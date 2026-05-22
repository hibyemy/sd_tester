use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
};

use crate::targets::{DriveKind, DriveTarget};
use crate::win32::device::open_device_path;
use crate::win32::ioctl::device_io_control;

const GENERIC_READ: u32 = 0x8000_0000;
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x0056_0000;
const STORAGE_DEVICE_PROPERTY: u32 = 0;
const PROPERTY_STANDARD_QUERY: u32 = 0;
const BUS_TYPE_USB: u8 = 0x07;
const BUS_TYPE_SD: u8 = 0x0C;
const BUS_TYPE_MMC: u8 = 0x0D;

#[repr(C)]
struct StoragePropertyQuery {
    property_id: u32,
    query_type: u32,
    additional: [u8; 1],
}

#[repr(C)]
struct StorageDeviceDescriptorHead {
    version: u32,
    size: u32,
    device_type: u8,
    device_type_modifier: u8,
    removable_media: u8,
    command_queueing: u8,
    vendor_id_offset: u32,
    product_id_offset: u32,
    product_revision_offset: u32,
    serial_number_offset: u32,
    bus_type: u8,
    raw_properties_length: u32,
}

#[repr(C)]
struct VolumeDiskExtentsHead {
    number_of_disk_extents: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DiskExtent {
    disk_number: u32,
    starting_offset: i64,
    extent_length: i64,
}

pub fn discover_removable_targets() -> Vec<DriveTarget> {
    let mask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();

    for i in 0..26u32 {
        if (mask & (1u32 << i)) == 0 {
            continue;
        }

        let letter = (b'A' + i as u8) as char;
        let root = format!("{letter}:\\");
        let mut wide: Vec<u16> = root.encode_utf16().collect();
        wide.push(0);

        let drive_type = unsafe { GetDriveTypeW(wide.as_ptr()) };
        if drive_type != DRIVE_REMOVABLE && drive_type != DRIVE_FIXED {
            continue;
        }

        let volume_path = format!(r"\\.\{letter}:");
        let kind = query_bus_kind(&volume_path).unwrap_or(DriveKind::Unknown);
        if kind == DriveKind::Unknown && drive_type != DRIVE_REMOVABLE {
            continue;
        };
        let physical_path = query_physical_drive_path(&volume_path);

        out.push(DriveTarget {
            id: format!("vol-{letter}"),
            display_name: format!("{letter}: [{:?}] ({})", kind, kind_name(kind)),
            drive_letter: format!("{letter}:"),
            kind,
            physical_path,
            advertised_bytes: query_volume_capacity_bytes(&root),
        });
    }

    out
}

fn query_volume_capacity_bytes(root_path: &str) -> Option<u64> {
    let mut wide: Vec<u16> = root_path.encode_utf16().collect();
    wide.push(0);
    let mut total_bytes = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            &mut total_bytes,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        None
    } else {
        Some(total_bytes)
    }
}

fn query_bus_kind(volume_path: &str) -> Option<DriveKind> {
    let handle = open_device_path(volume_path, GENERIC_READ).ok()?;
    let query = StoragePropertyQuery {
        property_id: STORAGE_DEVICE_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        additional: [0],
    };
    let query_bytes = unsafe {
        std::slice::from_raw_parts(
            (&query as *const StoragePropertyQuery) as *const u8,
            std::mem::size_of::<StoragePropertyQuery>(),
        )
    };

    let mut out = [0u8; 1024];
    let returned = device_io_control(&handle, IOCTL_STORAGE_QUERY_PROPERTY, Some(query_bytes), &mut out).ok()?;
    if returned < std::mem::size_of::<StorageDeviceDescriptorHead>() as u32 {
        return None;
    }

    let head = unsafe { &*(out.as_ptr() as *const StorageDeviceDescriptorHead) };
    match head.bus_type {
        BUS_TYPE_USB => Some(DriveKind::Usb),
        BUS_TYPE_SD | BUS_TYPE_MMC => Some(DriveKind::Sd),
        _ => Some(DriveKind::Unknown),
    }
}

fn query_physical_drive_path(volume_path: &str) -> Option<String> {
    let handle = open_device_path(volume_path, GENERIC_READ).ok()?;
    let mut out = [0u8; 64];
    let returned = device_io_control(&handle, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, None, &mut out).ok()?;
    if returned < (std::mem::size_of::<VolumeDiskExtentsHead>() + std::mem::size_of::<DiskExtent>()) as u32 {
        return None;
    }

    let extent_ptr = unsafe { out.as_ptr().add(std::mem::size_of::<VolumeDiskExtentsHead>()) as *const DiskExtent };
    let extent = unsafe { *extent_ptr };
    Some(format!(r"\\.\PhysicalDrive{}", extent.disk_number))
}

fn kind_name(kind: DriveKind) -> &'static str {
    match kind {
        DriveKind::Sd => "SD",
        DriveKind::Usb => "USB",
        DriveKind::Unknown => "Unknown",
    }
}
