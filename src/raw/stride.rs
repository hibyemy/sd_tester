use std::io;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::sync::mpsc::Sender;

use crate::engine::messages::{EngineUpdate, SessionStatus, StartRequest};
use crate::win32::device::open_physical_drive;
use crate::win32::ioctl::device_io_control;
use xxhash_rust::xxh3::xxh3_64;

const GENERIC_READ: u32 = 0x8000_0000;
const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const STRIDE: u64 = 1024 * 1024 * 1024;
const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007_405c;
const MARKER_SIZE: usize = 4096;

pub fn run_raw_stride(request: StartRequest, tx: Sender<EngineUpdate>) -> io::Result<()> {
    if !crate::win32::elevation::is_running_as_admin() {
        let _ = tx.send(EngineUpdate::status(
            request.session_id,
            request.target.id,
            SessionStatus::Failed,
            "admin privileges required for raw stride",
        ));
        return Ok(());
    }

    let device_path = request
        .target
        .physical_path
        .clone()
        .unwrap_or_else(|| r"\\.\PhysicalDrive0".to_owned());
    let total_bytes = query_device_length(&device_path).unwrap_or(4 * STRIDE);
    let final_offset = aligned_final_offset(total_bytes);

    let mut dev = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH)
        .open(&device_path)?;

    let mut marker = [0u8; MARKER_SIZE];
    for i in 0..=(final_offset / STRIDE) {
        if request.is_cancelled() {
            let _ = tx.send(EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Cancelled,
                "raw stride cancelled",
            ));
            return Ok(());
        }
        let offset = i * STRIDE;
        if offset > final_offset {
            break;
        }
        marker.fill(0);
        let hash = xxh3_64(&(offset.to_le_bytes()));
        marker[..8].copy_from_slice(&hash.to_le_bytes());
        marker[8..16].copy_from_slice(&request.session_id.to_le_bytes());
        dev.seek(SeekFrom::Start(offset))?;
        dev.write_all(&marker)?;
    }

    marker.fill(0);
    let final_hash = xxh3_64(&(final_offset.to_le_bytes()));
    marker[..8].copy_from_slice(&final_hash.to_le_bytes());
    marker[8..16].copy_from_slice(&request.session_id.to_le_bytes());
    dev.seek(SeekFrom::Start(final_offset))?;
    dev.write_all(&marker)?;
    dev.seek(SeekFrom::Start(0))?;
    let mut check = [0u8; MARKER_SIZE];
    dev.read_exact(&mut check)?;
    let wrap_around = check[8..16] != request.session_id.to_le_bytes();

    let _ = tx.send(EngineUpdate::status(
        request.session_id,
        request.target.id,
        SessionStatus::Completed,
        if wrap_around {
            format!("raw stride complete: wrap-around suspected after marker at {final_offset} bytes")
        } else {
            format!("raw stride complete: 0GB marker intact after marker at {final_offset} bytes")
        },
    ));
    Ok(())
}

fn query_device_length(device_path: &str) -> io::Result<u64> {
    let handle = open_physical_drive(device_path, GENERIC_READ)?;
    let mut out = [0u8; 8];
    let returned = device_io_control(&handle, IOCTL_DISK_GET_LENGTH_INFO, None, &mut out)?;
    if returned < 8 {
        return Err(io::Error::other("IOCTL_DISK_GET_LENGTH_INFO returned short buffer"));
    }
    Ok(i64::from_le_bytes(out) as u64)
}

fn aligned_final_offset(total_bytes: u64) -> u64 {
    let min_size = MARKER_SIZE as u64;
    if total_bytes <= min_size {
        return 0;
    }
    let last = total_bytes - min_size;
    (last / min_size) * min_size
}
