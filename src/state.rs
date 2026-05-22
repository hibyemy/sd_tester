use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::sync::mpsc::Sender;

use crate::engine::messages::{EngineUpdate, SessionStatus, StartRequest, TestKind};
use crate::targets::DriveTarget;

#[derive(Debug)]
pub struct SessionView {
    pub session_id: u64,
    pub target_id: String,
    pub target_name: String,
    pub test_kind: TestKind,
    pub status: SessionStatus,
    pub current_mbps: f64,
    pub latency_ms: f64,
    pub total_written: u64,
    pub verified_bytes: u64,
    pub usable_bytes: u64,
    pub target_bytes: u64,
    pub cancel_flag: Arc<AtomicBool>,
    pub last_message: String,
    pub speed_history: VecDeque<[f64; 2]>,
    pub latency_history: VecDeque<[f64; 2]>,
}

impl SessionView {
    pub fn new(session_id: u64, target: &DriveTarget, test_kind: TestKind) -> Self {
        Self {
            session_id,
            target_id: target.id.clone(),
            target_name: target.display_name.clone(),
            test_kind,
            status: SessionStatus::Starting,
            current_mbps: 0.0,
            latency_ms: 0.0,
            total_written: 0,
            verified_bytes: 0,
            usable_bytes: 0,
            target_bytes: target.advertised_bytes.unwrap_or_default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            last_message: "queued".to_owned(),
            speed_history: VecDeque::with_capacity(512),
            latency_history: VecDeque::with_capacity(512),
        }
    }
}

pub struct AppState {
    pub selected_targets: Vec<DriveTarget>,
    pub sessions: HashMap<u64, SessionView>,
    pub next_session_id: u64,
    pub max_concurrency: usize,
}

impl Default for AppState {
    fn default() -> Self {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);
        Self {
            selected_targets: Vec::new(),
            sessions: HashMap::new(),
            next_session_id: 1,
            max_concurrency: (threads / 2).max(1),
        }
    }
}

impl AppState {
    pub fn start_for_selected(
        &mut self,
        tx: &Sender<StartRequest>,
        test_kind: TestKind,
        duration_seconds: u64,
    ) {
        for target in &self.selected_targets {
            if self.sessions.values().any(|s| {
                s.target_id == target.id
                    && s.test_kind == test_kind
                    && matches!(s.status, SessionStatus::Starting | SessionStatus::Running)
            }) {
                continue;
            }

            let running = self
                .sessions
                .values()
                .filter(|s| matches!(s.status, SessionStatus::Starting | SessionStatus::Running))
                .count();
            if running >= self.max_concurrency {
                continue;
            }

            let session_id = self.next_session_id;
            self.next_session_id += 1;
            self.sessions
                .insert(session_id, SessionView::new(session_id, target, test_kind));

            let file_name = match test_kind {
                TestKind::ActionCam => format!("sdtester_actioncam_{session_id}.bin"),
                TestKind::OsDrive => format!("sdtester_osdrive_{session_id}.bin"),
                TestKind::CapacityLookBack => format!("sdtester_capacity_lookback_{session_id}.bin"),
                TestKind::CapacityBruteForce => {
                    format!("sdtester_capacity_bruteforce_{session_id}.bin")
                }
                TestKind::RawStride => format!("sdtester_raw_stride_{session_id}.bin"),
                TestKind::ReadCid => format!("sdtester_cid_{session_id}.bin"),
            };
            let output_dir = format!("{}\\sd_tester_data", target.drive_letter);
            let _ = std::fs::create_dir_all(&output_dir);
            let output_file = Some(std::path::PathBuf::from(format!("{output_dir}\\{file_name}")));
            let request = StartRequest {
                session_id,
                target: target.clone(),
                test_kind,
                output_file,
                duration_seconds,
                cancel_flag: self
                    .sessions
                    .get(&session_id)
                    .map(|s| s.cancel_flag.clone())
                    .unwrap_or_else(|| Arc::new(AtomicBool::new(false))),
            };
            let _ = tx.send(request);
        }
    }

    pub fn cancel_session(&mut self, session_id: u64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.cancel_flag.store(true, Ordering::Relaxed);
            if matches!(session.status, SessionStatus::Starting | SessionStatus::Running) {
                session.status = SessionStatus::Cancelled;
                session.last_message = "cancel requested".to_owned();
            }
        }
    }

    pub fn remove_finished_session(&mut self, session_id: u64) {
        let removable = self
            .sessions
            .get(&session_id)
            .map(|s| is_terminal_status(s.status))
            .unwrap_or(false);
        if removable {
            self.sessions.remove(&session_id);
        }
    }

    pub fn clear_finished_sessions(&mut self) {
        self.sessions
            .retain(|_, session| !is_terminal_status(session.status));
    }

    pub fn apply_update(&mut self, update: EngineUpdate) {
        if let Some(session) = self.sessions.get_mut(&update.session_id) {
            if session.target_id != update.target_id {
                return;
            }
            session.status = update.status;
            session.current_mbps = update.current_mbps;
            session.latency_ms = update.latency_ms;
            session.total_written = update.total_written;
            session.verified_bytes = update.verified_bytes;
            session.usable_bytes = update.usable_bytes;
            if update.target_bytes > 0 {
                session.target_bytes = update.target_bytes;
            }
            session.last_message = update.message;

            let t = session.speed_history.len() as f64;
            session.speed_history.push_back([t, update.current_mbps]);
            session.latency_history.push_back([t, update.latency_ms]);

            while session.speed_history.len() > 500 {
                session.speed_history.pop_front();
            }
            while session.latency_history.len() > 500 {
                session.latency_history.pop_front();
            }
        }
    }
}

fn is_terminal_status(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Cancelled
    )
}
