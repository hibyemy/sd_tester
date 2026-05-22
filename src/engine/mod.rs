pub mod benchmark;
pub mod messages;

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use messages::{EngineUpdate, SessionStatus, StartRequest, TestKind};

pub struct EngineRuntime {
    pub command_tx: Sender<StartRequest>,
    pub updates_rx: Receiver<EngineUpdate>,
}

impl EngineRuntime {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel::<StartRequest>();
        let (updates_tx, updates_rx) = mpsc::channel::<EngineUpdate>();

        thread::spawn(move || {
            while let Ok(request) = command_rx.recv() {
                let tx = updates_tx.clone();
                thread::spawn(move || {
                    let target_id = request.target.id.clone();
                    let _ = tx.send(EngineUpdate::status(
                        request.session_id,
                        target_id.clone(),
                        SessionStatus::Starting,
                        "test thread starting",
                    ));

                    let run_result = match request.test_kind {
                        TestKind::ActionCam | TestKind::OsDrive => {
                            benchmark::run_benchmark(request.clone(), tx.clone())
                        }
                        TestKind::CapacityLookBack => {
                            crate::capacity::lookback::run_capacity(request.clone(), tx.clone())
                        }
                        TestKind::CapacityBruteForce => {
                            crate::capacity::lookback::run_capacity_bruteforce(
                                request.clone(),
                                tx.clone(),
                            )
                        }
                        TestKind::RawStride => {
                            crate::raw::stride::run_raw_stride(request.clone(), tx.clone())
                        }
                        TestKind::ReadCid => {
                            crate::forensics::cid::run_read_cid(request.clone(), tx.clone())
                        }
                    };

                    if let Err(err) = run_result {
                        let _ = tx.send(EngineUpdate::status(
                            request.session_id,
                            target_id,
                            SessionStatus::Failed,
                            format!("failed: {err}"),
                        ));
                    }

                    cleanup_test_artifacts(&request);
                });
            }
        });

        Self {
            command_tx,
            updates_rx,
        }
    }
}

fn cleanup_test_artifacts(request: &StartRequest) {
    if let Some(path) = request.output_file.as_ref() {
        let _ = std::fs::remove_file(path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}
