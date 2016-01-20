// Copyright 2015-2016 Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use Decoder;
use DecoderResult;
use handles::*;
use data::*;

macro_rules! decoder_function {
    ($preamble:block,
     $eof:block,
     $body:block,
     $slf:ident,
     $src_consumed:ident,
     $dest:ident,
     $b:ident,
     $destination_handle:ident,
     $unread_handle:ident,
     $name:ident,
     $code_unit:ty,
     $dest_struct:ident) => (
    fn $name(&mut $slf,
             src: &[u8],
             dst: &mut [$code_unit],
             last: bool)
             -> (DecoderResult, usize, usize) {
        let mut source = ByteSource::new(src);
        let mut $dest = $dest_struct::new(dst);
        loop {
            {
                // Start non-boilerplate
                $preamble
                // End non-boilerplate
            }
            loop {
                match source.check_available() {
                    Space::Full($src_consumed) => {
                        if last {
                            // Start non-boilerplate
                            $eof
                            // End non-boilerplate
                        }
                        return (DecoderResult::InputEmpty, $src_consumed, $dest.written());
                    }
                    Space::Available(source_handle) => {
                        match $dest.check_space_astral() {
                            Space::Full(dst_written) => {
                                return (DecoderResult::OutputFull,
                                        source_handle.consumed(),
                                        dst_written);
                            }
                            Space::Available($destination_handle) => {
                                let ($b, $unread_handle) = source_handle.read();
                                // Start non-boilerplate
                                $body
                                // End non-boilerplate
                            }
                        }
                    }
                }
            }
        }
    });
}

macro_rules! decoder_functions {
    ($preamble:block,
     $eof:block,
     $body:block,
     $slf:ident,
     $src_consumed:ident,
     $dest:ident,
     $b:ident,
     $destination_handle:ident,
     $unread_handle:ident) => (
    decoder_function!($preamble,
                      $eof,
                      $body,
                      $slf,
                      $src_consumed,
                      $dest,
                      $b,
                      $destination_handle,
                      $unread_handle,
                      decode_to_utf8,
                      u8,
                      Utf8Destination);
    decoder_function!($preamble,
                      $eof,
                      $body,
                      $slf,
                      $src_consumed,
                      $dest,
                      $b,
                      $destination_handle,
                      $unread_handle,
                      decode_to_utf16,
                      u16,
                      Utf16Destination);
    );
}

pub struct Big5Decoder {
    lead: u8,
}

impl Big5Decoder {
    pub fn new() -> Big5Decoder {
        Big5Decoder { lead: 0 }
    }

    fn plus_one_if_lead(&self, byte_length: usize) -> usize {
        byte_length +
        if self.lead == 0 {
            0
        } else {
            1
        }
    }
}

impl Decoder for Big5Decoder {
    fn reset(&mut self) {
        self.lead = 0u8;
    }

    fn max_utf16_buffer_length(&self, byte_length: usize) -> usize {
        self.plus_one_if_lead(byte_length) + 1
    }

    fn max_utf8_buffer_length(&self, byte_length: usize) -> usize {
        let len = self.plus_one_if_lead(byte_length);
        (len * 2) + (len / 2) + 4 // XXX tail
    }

    fn max_utf8_buffer_length_with_replacement(&self, byte_length: usize) -> usize {
        3 * self.plus_one_if_lead(byte_length) + 4 // XXX tail
    }

    decoder_functions!({},
                       {
                           if self.lead != 0 {
                               self.lead = 0;
                               return (DecoderResult::Malformed(1), src_consumed, dest.written());
                           }
                       },
                       {
                           if self.lead == 0 {
                               if b <= 0x7f {
                                   // TODO optimize ASCII run
                                   destination_handle.write_ascii(b);
                                   continue;
                               }
                               if b >= 0x81 && b <= 0xFE {
                                   self.lead = b;
                                   continue;
                               }
                               return (DecoderResult::Malformed(1),
                                       unread_handle.consumed(),
                                       destination_handle.written());
                           }
                           let lead = self.lead as usize;
                           self.lead = 0;
                           let offset = if b < 0x7F {
                               0x40usize
                           } else {
                               0x62usize
                           };
                           if (b >= 0x40 && b <= 0x7E) || (b >= 0xA1 && b <= 0xFE) {
                               let pointer = (lead - 0x81usize) * 157usize + (b as usize - offset);
                               match pointer {
                                   1133 => {
                                       destination_handle.write_big5_combination(0x00CAu16,
                                                                                 0x0304u16);
                                       continue;
                                   }
                                   1135 => {
                                       destination_handle.write_big5_combination(0x00CAu16,
                                                                                 0x030Cu16);
                                       continue;
                                   }
                                   1164 => {
                                       destination_handle.write_big5_combination(0x00EAu16,
                                                                                 0x0304u16);
                                       continue;
                                   }
                                   1166 => {
                                       destination_handle.write_big5_combination(0x00EAu16,
                                                                                 0x030Cu16);
                                       continue;
                                   }
                                   _ => {
                                       let low_bits = big5_low_bits(pointer);
                                       if low_bits == 0 {
                                           if b <= 0x7F {
                                               return (DecoderResult::Malformed(1),
                                                       unread_handle.unread(),
                                                       destination_handle.written());
                                           }
                                           return (DecoderResult::Malformed(2),
                                                   unread_handle.consumed(),
                                                   destination_handle.written());
                                       }
                                       if big5_is_astral(pointer) {
                                           destination_handle.write_astral(low_bits as u32 |
                                                                           0x20000u32);
                                           continue;
                                       }
                                       destination_handle.write_bmp_excl_ascii(low_bits);
                                       continue;
                                   }
                               }
                           }
                           // pointer is null
                           if b <= 0x7F {
                               return (DecoderResult::Malformed(1),
                                       unread_handle.unread(),
                                       destination_handle.written());
                           }
                           return (DecoderResult::Malformed(2),
                                   unread_handle.consumed(),
                                   destination_handle.written());


                       },
                       self,
                       src_consumed,
                       dest,
                       b,
                       destination_handle,
                       unread_handle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    fn decode_big5_to_utf16(bytes: &[u8], expect: &[u16]) {
        let mut decoder = Big5Decoder::new();
        let mut dest: Vec<u16> = Vec::with_capacity(decoder.max_utf16_buffer_length(expect.len()));
        let capacity = dest.capacity();
        dest.resize(capacity, 0u16);
        let (complete, read, written, _) = decoder.decode_to_utf16_with_replacement(bytes,
                                                                                    &mut dest,
                                                                                    true);
        assert_eq!(complete, WithReplacementResult::InputEmpty);
        assert_eq!(read, bytes.len());
        assert_eq!(written, expect.len());
        dest.truncate(written);
        assert_eq!(&dest[..], expect);
    }

    fn decode_big5_to_utf8(bytes: &[u8], expect: &str) {
        let mut decoder = Big5Decoder::new();
        let mut dest: Vec<u8> = Vec::with_capacity(decoder.max_utf8_buffer_length(expect.len()));
        let capacity = dest.capacity();
        dest.resize(capacity, 0u8);
        let (complete, read, written, _) = decoder.decode_to_utf8_with_replacement(bytes,
                                                                                   &mut dest,
                                                                                   true);
        assert_eq!(complete, WithReplacementResult::InputEmpty);
        assert_eq!(read, bytes.len());
        assert_eq!(written, expect.len());
        dest.truncate(written);
        assert_eq!(&dest[..], expect.as_bytes());
    }

    #[test]
    fn test_big5_decode() {
        // ASCII
        decode_big5_to_utf16(&[0x61u8, 0x62u8], &[0x0061u16, 0x0062u16]);
        // Edge cases
        decode_big5_to_utf16(&[0x87u8, 0x40u8], &[0x43F0u16]);
        decode_big5_to_utf16(&[0xFEu8, 0xFEu8], &[0x79D4u16]);
        decode_big5_to_utf16(&[0xFEu8, 0xFDu8], &[0xD864u16, 0xDD0Du16]);
        decode_big5_to_utf16(&[0x88u8, 0x62u8], &[0x00CAu16, 0x0304u16]);
        decode_big5_to_utf16(&[0x88u8, 0x64u8], &[0x00CAu16, 0x030Cu16]);
        decode_big5_to_utf16(&[0x88u8, 0x66u8], &[0x00CAu16]);
        decode_big5_to_utf16(&[0x88u8, 0xA3u8], &[0x00EAu16, 0x0304u16]);
        decode_big5_to_utf16(&[0x88u8, 0xA5u8], &[0x00EAu16, 0x030Cu16]);
        decode_big5_to_utf16(&[0x88u8, 0xA7u8], &[0x00EAu16]);
        decode_big5_to_utf16(&[0x99u8, 0xD4u8], &[0x8991u16]);
        decode_big5_to_utf16(&[0x99u8, 0xD5u8], &[0xD85Eu16, 0xDD67u16]);
        decode_big5_to_utf16(&[0x99u8, 0xD6u8], &[0x8A29u16]);
        // Edge cases surrounded with ASCII
        decode_big5_to_utf16(&[0x61u8, 0x87u8, 0x40u8, 0x62u8],
                             &[0x0061u16, 0x43F0u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0xFEu8, 0xFEu8, 0x62u8],
                             &[0x0061u16, 0x79D4u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0xFEu8, 0xFDu8, 0x62u8],
                             &[0x0061u16, 0xD864u16, 0xDD0Du16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0x62u8, 0x62u8],
                             &[0x0061u16, 0x00CAu16, 0x0304u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0x64u8, 0x62u8],
                             &[0x0061u16, 0x00CAu16, 0x030Cu16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0x66u8, 0x62u8],
                             &[0x0061u16, 0x00CAu16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0xA3u8, 0x62u8],
                             &[0x0061u16, 0x00EAu16, 0x0304u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0xA5u8, 0x62u8],
                             &[0x0061u16, 0x00EAu16, 0x030Cu16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x88u8, 0xA7u8, 0x62u8],
                             &[0x0061u16, 0x00EAu16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x99u8, 0xD4u8, 0x62u8],
                             &[0x0061u16, 0x8991u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x99u8, 0xD5u8, 0x62u8],
                             &[0x0061u16, 0xD85Eu16, 0xDD67u16, 0x0062u16]);
        decode_big5_to_utf16(&[0x61u8, 0x99u8, 0xD6u8, 0x62u8],
                             &[0x0061u16, 0x8A29u16, 0x0062u16]);
        // Bad sequences
        decode_big5_to_utf16(&[0x80u8, 0x61u8], &[0xFFFDu16, 0x0061u16]);
        decode_big5_to_utf16(&[0xFFu8, 0x61u8], &[0xFFFDu16, 0x0061u16]);
        decode_big5_to_utf16(&[0xFEu8, 0x39u8], &[0xFFFDu16, 0x0039u16]);
        decode_big5_to_utf16(&[0x87u8, 0x66u8], &[0xFFFDu16, 0x0066u16]);
        decode_big5_to_utf16(&[0x81u8, 0x40u8], &[0xFFFDu16, 0x0040u16]);
        decode_big5_to_utf16(&[0x61u8, 0x81u8], &[0x0061u16, 0xFFFDu16]);

        // ASCII
        decode_big5_to_utf8(&[0x61u8, 0x62u8], &"\u{0061}\u{0062}");
        // Edge cases
        decode_big5_to_utf8(&[0x87u8, 0x40u8], &"\u{43F0}");
        decode_big5_to_utf8(&[0xFEu8, 0xFEu8], &"\u{79D4}");
        decode_big5_to_utf8(&[0xFEu8, 0xFDu8], &"\u{2910D}");
        decode_big5_to_utf8(&[0x88u8, 0x62u8], &"\u{00CA}\u{0304}");
        decode_big5_to_utf8(&[0x88u8, 0x64u8], &"\u{00CA}\u{030C}");
        decode_big5_to_utf8(&[0x88u8, 0x66u8], &"\u{00CA}");
        decode_big5_to_utf8(&[0x88u8, 0xA3u8], &"\u{00EA}\u{0304}");
        decode_big5_to_utf8(&[0x88u8, 0xA5u8], &"\u{00EA}\u{030C}");
        decode_big5_to_utf8(&[0x88u8, 0xA7u8], &"\u{00EA}");
        decode_big5_to_utf8(&[0x99u8, 0xD4u8], &"\u{8991}");
        decode_big5_to_utf8(&[0x99u8, 0xD5u8], &"\u{27967}");
        decode_big5_to_utf8(&[0x99u8, 0xD6u8], &"\u{8A29}");
        // Edge cases surrounded with ASCII
        decode_big5_to_utf8(&[0x61u8, 0x87u8, 0x40u8, 0x62u8], &"\u{0061}\u{43F0}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0xFEu8, 0xFEu8, 0x62u8], &"\u{0061}\u{79D4}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0xFEu8, 0xFDu8, 0x62u8], &"\u{0061}\u{2910D}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0x62u8, 0x62u8], &"\u{0061}\u{00CA}\u{0304}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0x64u8, 0x62u8], &"\u{0061}\u{00CA}\u{030C}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0x66u8, 0x62u8], &"\u{0061}\u{00CA}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0xA3u8, 0x62u8], &"\u{0061}\u{00EA}\u{0304}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0xA5u8, 0x62u8], &"\u{0061}\u{00EA}\u{030C}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x88u8, 0xA7u8, 0x62u8], &"\u{0061}\u{00EA}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x99u8, 0xD4u8, 0x62u8], &"\u{0061}\u{8991}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x99u8, 0xD5u8, 0x62u8], &"\u{0061}\u{27967}\u{0062}");
        decode_big5_to_utf8(&[0x61u8, 0x99u8, 0xD6u8, 0x62u8], &"\u{0061}\u{8A29}\u{0062}");
        // Bad sequences
        decode_big5_to_utf8(&[0x80u8, 0x61u8], &"\u{FFFD}\u{0061}");
        decode_big5_to_utf8(&[0xFFu8, 0x61u8], &"\u{FFFD}\u{0061}");
        decode_big5_to_utf8(&[0xFEu8, 0x39u8], &"\u{FFFD}\u{0039}");
        decode_big5_to_utf8(&[0x87u8, 0x66u8], &"\u{FFFD}\u{0066}");
        decode_big5_to_utf8(&[0x81u8, 0x40u8], &"\u{FFFD}\u{0040}");
        decode_big5_to_utf8(&[0x61u8, 0x81u8], &"\u{0061}\u{FFFD}");
    }
}
