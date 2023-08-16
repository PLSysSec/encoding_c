

// pub fn safe_deref<T>(ptr: *const T) -> T {
//     unsafe{
//         std::ptr::read(ptr)
//     }
// }

pub fn safe_as_ref<'a, T>(ptr: *const T) -> &'a T {
    unsafe{
        &*ptr
    }
}

pub fn safe_as_ref_mut<'a, T>(ptr: *mut T) -> &'a mut T {
    unsafe{
        &mut *ptr
    }
}

pub fn safe_slice_from_raw_parts<'a, T>(data: *const T, len: usize) -> &'a [T]
{
    unsafe {
        std::slice::from_raw_parts(data, len)
    }
}

pub fn safe_slice_from_raw_parts_mut<'a, T>(data: *mut T, len: usize) -> &'a mut [T]
{
    unsafe {
        std::slice::from_raw_parts_mut(data, len)
    }
}

pub fn safe_from_utf8_unchecked(v: &[u8]) -> &str
{
    unsafe {
        std::str::from_utf8_unchecked(v)
    }
}


pub fn safe_copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize)
{
    unsafe {
        std::ptr::copy_nonoverlapping(src, dst, count)
    }
}

pub fn safe_box_from_raw<T>(raw: *mut T) -> Box<T>
{
    unsafe {
        Box::from_raw(raw)
    }
}

