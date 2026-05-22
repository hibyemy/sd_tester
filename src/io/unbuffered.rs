use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_NORMAL, FILE_FLAG_NO_BUFFERING, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

pub fn open_unbuffered_file(path: &Path) -> std::io::Result<File> {
    let result = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .attributes(FILE_ATTRIBUTE_NORMAL)
        .custom_flags(FILE_FLAG_NO_BUFFERING)
        .open(path);

    match result {
        Ok(file) => Ok(file),
        Err(err) if matches!(err.raw_os_error(), Some(5) | Some(32)) => OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .attributes(FILE_ATTRIBUTE_NORMAL)
            .open(path),
        Err(err) => Err(err),
    }
}
