use std::io;
use std::sync::mpsc::Sender;

use crate::engine::messages::{EngineUpdate, SessionStatus, StartRequest};
use crate::win32::device::open_physical_drive;
use crate::win32::ioctl::device_io_control;

const GENERIC_READ: u32 = 0x8000_0000;
const IOCTL_SFFDISK_DEVICE_COMMAND: u32 = 0x0007_C084;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CidInfo {
    pub manufacturer_id: u8,
    pub oem_id: u16,
    pub product_name: [u8; 5],
    pub revision: u8,
    pub serial_number: u32,
    pub manufacture_year: u16,
    pub manufacture_month: u8,
}

pub fn run_read_cid(request: StartRequest, tx: Sender<EngineUpdate>) -> io::Result<()> {
    if request.is_cancelled() {
        let _ = tx.send(EngineUpdate::status(
            request.session_id,
            request.target.id,
            SessionStatus::Cancelled,
            "CID read cancelled",
        ));
        return Ok(());
    }

    if !crate::win32::elevation::is_running_as_admin() {
        let _ = tx.send(EngineUpdate::status(
            request.session_id,
            request.target.id,
            SessionStatus::Failed,
            "admin required for CID read",
        ));
        return Ok(());
    }

    let device = request
        .target
        .physical_path
        .clone()
        .unwrap_or_else(|| r"\\.\PhysicalDrive0".to_owned());
    let handle = open_physical_drive(&device, GENERIC_READ)?;

    // Basic control packet for CMD10 (Read CID). Some controllers require richer payloads.
    let command_packet = [10u8, 0, 0, 0, 0, 0, 0, 0];
    let mut out_buf = [0u8; 256];
    let written =
        device_io_control(&handle, IOCTL_SFFDISK_DEVICE_COMMAND, Some(&command_packet), &mut out_buf)?;
    if written < 16 {
        return Err(io::Error::other("CID response too short"));
    }
    let mut cid_bytes = [0u8; 16];
    cid_bytes.copy_from_slice(&out_buf[..16]);
    let parsed = parse_cid(&cid_bytes);
    let _ = tx.send(EngineUpdate::status(
        request.session_id,
        request.target.id,
        SessionStatus::Completed,
        format!(
            "CID parsed (MID={:#04x}, OEM={:#06x}, PNM={})",
            parsed.manufacturer_id,
            parsed.oem_id,
            String::from_utf8_lossy(&parsed.product_name)
        ),
    ));
    Ok(())
}

pub fn parse_cid(raw: &[u8; 16]) -> CidInfo {
    let manufacturer_id = raw[0];
    let oem_id = u16::from_be_bytes([raw[1], raw[2]]);
    let product_name = [raw[3], raw[4], raw[5], raw[6], raw[7]];
    let revision = raw[8];
    let serial_number = u32::from_be_bytes([raw[9], raw[10], raw[11], raw[12]]);
    let mdt = u16::from_be_bytes([raw[13], raw[14]]);
    let year_offset = ((mdt >> 4) & 0xFF) as u16;
    let month = (mdt & 0x0F) as u8;

    CidInfo {
        manufacturer_id,
        oem_id,
        product_name,
        revision,
        serial_number,
        manufacture_year: 2000 + year_offset,
        manufacture_month: month,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_expected_fields() {
        let raw = [
            0x1B, 0x53, 0x44, b'A', b'B', b'C', b'D', b'E', 0x21, 0x00, 0x11, 0x22, 0x33, 0x01,
            0xA2, 0x00,
        ];
        let cid = parse_cid(&raw);
        assert_eq!(cid.manufacturer_id, 0x1B);
        assert_eq!(cid.oem_id, 0x5344);
        assert_eq!(cid.product_name, *b"ABCDE");
    }
}
