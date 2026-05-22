#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    Sd,
    Usb,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveTarget {
    pub id: String,
    pub display_name: String,
    pub drive_letter: String,
    pub kind: DriveKind,
    pub physical_path: Option<String>,
    pub advertised_bytes: Option<u64>,
}

impl DriveTarget {
    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            DriveKind::Sd => "SD",
            DriveKind::Usb => "USB",
            DriveKind::Unknown => "Unknown",
        }
    }
}
