use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    ActionCam,
    OsDrive,
    CapacityLookBack,
    CapacityBruteForce,
    RawStride,
    ReadCid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SessionStatus {
    Idle,
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct StartRequest {
    pub session_id: u64,
    pub target: crate::targets::DriveTarget,
    pub test_kind: TestKind,
    pub output_file: Option<PathBuf>,
    pub duration_seconds: u64,
    pub cancel_flag: Arc<AtomicBool>,
}

impl StartRequest {
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct EngineUpdate {
    pub session_id: u64,
    pub target_id: String,
    pub status: SessionStatus,
    pub current_mbps: f64,
    pub latency_ms: f64,
    pub total_written: u64,
    pub verified_bytes: u64,
    pub usable_bytes: u64,
    pub target_bytes: u64,
    pub message: String,
}

impl EngineUpdate {
    pub fn status(
        session_id: u64,
        target_id: impl Into<String>,
        status: SessionStatus,
        message: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            target_id: target_id.into(),
            status,
            current_mbps: 0.0,
            latency_ms: 0.0,
            total_written: 0,
            verified_bytes: 0,
            usable_bytes: 0,
            target_bytes: 0,
            message: message.into(),
        }
    }
}
