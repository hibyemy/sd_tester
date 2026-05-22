use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

pub struct DeviceHandle(*mut core::ffi::c_void);

impl DeviceHandle {
    pub fn raw(&self) -> *mut core::ffi::c_void {
        self.0
    }
}

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn open_physical_drive(path: &str, desired_access: u32) -> io::Result<DeviceHandle> {
    open_device_path(path, desired_access)
}

pub fn open_device_path(path: &str, desired_access: u32) -> io::Result<DeviceHandle> {
    let mut wide: Vec<u16> = OsStr::new(path).encode_wide().collect();
    wide.push(0);

    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            desired_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    Ok(DeviceHandle(handle))
}
