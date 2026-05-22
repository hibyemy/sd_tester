use crate::app::SdTesterApp;
use crate::engine::messages::TestKind;

pub fn render(app: &mut SdTesterApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Start Look-Back Capacity Test").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::CapacityLookBack, 30);
        }
        if ui.button("Start Brute-Force Full Verify").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::CapacityBruteForce, 600);
        }
        ui.label("Look-back verifies every 1GB. Brute-force writes full advertised size then verifies all.");
    });
    super::shared::drive_picker::render_drive_picker(
        ui,
        &app.available_targets,
        &mut app.state.selected_targets,
    );
    ui.separator();

    let session_ids: Vec<u64> = app.state.sessions.keys().copied().collect();
    for session_id in session_ids {
        let Some(session) = app.state.sessions.get(&session_id) else {
            continue;
        };
        if !matches!(session.test_kind, TestKind::CapacityLookBack | TestKind::CapacityBruteForce) {
            continue;
        }
        let mut cancel_clicked = false;
        let mut close_clicked = false;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} | {:?} | {:?}",
                    session.target_name, session.test_kind, session.status
                ));
                if matches!(
                    session.status,
                    crate::engine::messages::SessionStatus::Starting
                        | crate::engine::messages::SessionStatus::Running
                ) && ui.button("Cancel").clicked()
                {
                    cancel_clicked = true;
                }
                if matches!(
                    session.status,
                    crate::engine::messages::SessionStatus::Completed
                        | crate::engine::messages::SessionStatus::Failed
                        | crate::engine::messages::SessionStatus::Cancelled
                ) && ui.button("x").on_hover_text("Remove this finished test").clicked()
                {
                    close_clicked = true;
                }
            });
            ui.label(format!(
                "Write speed: {:.1} MB/s | Tested: {:.2} GiB | Verified: {:.2} GiB | Usable: {:.2} GiB",
                session.current_mbps,
                gib(session.total_written),
                gib(session.verified_bytes),
                gib(session.usable_bytes)
            ));
            if let Some(eta) = eta_string(session.target_bytes, session.total_written, session.current_mbps) {
                ui.label(format!("Estimated time remaining: {eta}"));
            }

            if session.target_bytes > 0 {
                let write_progress = (session.total_written as f64 / session.target_bytes as f64)
                    .clamp(0.0, 1.0) as f32;
                let verify_progress =
                    (session.verified_bytes as f64 / session.target_bytes as f64).clamp(0.0, 1.0)
                        as f32;
                ui.add(
                    egui::ProgressBar::new(write_progress)
                        .text(format!("Write Progress: {:.1}%", write_progress * 100.0)),
                );
                ui.add(
                    egui::ProgressBar::new(verify_progress)
                        .text(format!("Verify Progress: {:.1}%", verify_progress * 100.0)),
                );
                ui.label(format!(
                    "Advertised: {:.2} GiB | Tested: {:.2} GiB | Estimated usable: {:.2} GiB",
                    gib(session.target_bytes),
                    gib(session.total_written),
                    gib(session.usable_bytes)
                ));
            } else {
                ui.label("Advertised capacity unavailable on this target.");
            }

            ui.label(format!("Status: {}", session.last_message));
        });
        if cancel_clicked {
            app.state.cancel_session(session_id);
        }
        if close_clicked {
            app.state.remove_finished_session(session_id);
        }
        ui.separator();
    }
}

fn gib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

fn eta_string(target_bytes: u64, written_bytes: u64, mbps: f64) -> Option<String> {
    if target_bytes == 0 || written_bytes >= target_bytes || mbps <= 0.05 {
        return None;
    }
    let remaining_mb = (target_bytes - written_bytes) as f64 / (1024.0 * 1024.0);
    let seconds = (remaining_mb / mbps).max(0.0) as u64;
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    Some(format!("{h:02}:{m:02}:{s:02}"))
}
