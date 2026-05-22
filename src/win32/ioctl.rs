use std::io;

use windows_sys::Win32::System::IO::DeviceIoControl;

use crate::win32::device::DeviceHandle;

pub fn device_io_control(
    handle: &DeviceHandle,
    code: u32,
    in_buf: Option<&[u8]>,
    out_buf: &mut [u8],
) -> io::Result<u32> {
    let mut returned = 0u32;
    let (in_ptr, in_len) = if let Some(buf) = in_buf {
        (buf.as_ptr() as *mut _, buf.len() as u32)
    } else {
        (std::ptr::null_mut(), 0)
    };

    let ok = unsafe {
        DeviceIoControl(
            handle.raw(),
            code,
            in_ptr,
            in_len,
            out_buf.as_mut_ptr() as *mut _,
            out_buf.len() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(returned)
    }
}
