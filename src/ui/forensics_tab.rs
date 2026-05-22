use crate::app::SdTesterApp;
use crate::engine::messages::TestKind;

pub fn render(app: &mut SdTesterApp, ui: &mut egui::Ui) {
    ui.heading("Forensics / Raw Operations");
    ui.colored_label(
        egui::Color32::RED,
        "WARNING: Raw stride test is destructive. It writes directly to PhysicalDriveX.",
    );

    ui.horizontal(|ui| {
        if ui.button("Run Raw Stride (Admin)").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::RawStride, 5);
        }
        if ui.button("Read CID (Admin)").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::ReadCid, 5);
        }
        if ui.button("Relaunch as Admin").clicked() {
            let _ = crate::win32::elevation::relaunch_as_admin();
        }
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
        if !matches!(session.test_kind, TestKind::RawStride | TestKind::ReadCid) {
            continue;
        }
        let mut cancel_clicked = false;
        let mut close_clicked = false;
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} | {:?} | {}",
                session.target_name, session.status, session.last_message
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
        if cancel_clicked {
            app.state.cancel_session(session_id);
        }
        if close_clicked {
            app.state.remove_finished_session(session_id);
        }
    }
}
