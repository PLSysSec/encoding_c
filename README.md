# encoding_c

[![crates.io](https://meritbadge.herokuapp.com/encoding_rs)](https://crates.io/crates/encoding_c)
[![docs.rs](https://docs.rs/encoding_rs/badge.svg)](https://docs.rs/encoding_c/)
[![Apache 2 / MIT dual-licensed](https://img.shields.io/badge/license-Apache%202%20%2F%20MIT-blue.svg)](https://github.com/hsivonen/encoding_c/blob/master/COPYRIGHT)

encoding_c is an FFI wrapper for [encoding_rs](https://github.com/hsivonen/encoding_rs).

## Licensing

Please see the file named
[COPYRIGHT](https://github.com/hsivonen/encoding_c/blob/master/COPYRIGHT).

## C/C++ Headers

`include/encoding_rs.h` and `include/encoding_rs_statics.h` are needed for C
usage.

`include/encoding_rs_cpp.h` is a sample C++ API built on top of the C API using
GSL and the C++ standard library. Since C++ project typically roll their own
string classes, etc., it's probably necessary for C++ projects to manually
adapt the header to their replacements of standard-library types.

## Release Notes

### 0.7.0

* Map `None` to `SIZE_MAX` in the max length calculation functions.

### 0.6.0

* Check in the `cheddar`-generated header and comment out the `cheddar`-using
  `build.rs`.

### 0.5.0

* Initial release of encoding_c. (I.e. first release with FFI in a distinct
  crate.)

