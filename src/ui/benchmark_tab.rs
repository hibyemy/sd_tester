use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::app::SdTesterApp;
use crate::engine::messages::{SessionStatus, TestKind};

pub fn render(app: &mut SdTesterApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Start Action Cam").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::ActionCam, 20);
        }
        if ui.button("Start OS Drive 4K Random").clicked() {
            app.state
                .start_for_selected(&app.engine.command_tx, TestKind::OsDrive, 20);
        }
        ui.label(format!("Max concurrent: {}", app.state.max_concurrency));
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
        if !matches!(session.test_kind, TestKind::ActionCam | TestKind::OsDrive) {
            continue;
        }
        let mut cancel_clicked = false;
        let mut close_clicked = false;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} {:?} | {:?} | {}",
                    session.target_name, session.test_kind, session.status, session.last_message
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
                "{:.2} MB/s | {:.2} ms | {:.2} GiB written",
                session.current_mbps,
                session.latency_ms,
                session.total_written as f64 / (1024.0 * 1024.0 * 1024.0)
            ));
            let (peak_mbps, avg_mbps) = speed_stats(session.speed_history.iter().map(|p| p[1]));
            let (peak_latency, avg_latency) =
                speed_stats(session.latency_history.iter().map(|p| p[1]));
            ui.label(format!(
                "Stats | Peak throughput: {:.1} MB/s | Avg throughput: {:.1} MB/s | Peak latency: {:.2} ms | Avg latency: {:.2} ms",
                peak_mbps, avg_mbps, peak_latency, avg_latency
            ));

            let analysis = assess_benchmark(
                session.test_kind,
                avg_mbps,
                peak_mbps,
                avg_latency,
                peak_latency,
                session.status,
            );
            let color = match analysis.grade {
                Grade::Pass => egui::Color32::from_rgb(60, 180, 75),
                Grade::Caution => egui::Color32::from_rgb(255, 200, 0),
                Grade::Fail => egui::Color32::from_rgb(230, 80, 80),
                Grade::Pending => egui::Color32::GRAY,
            };
            ui.colored_label(
                color,
                format!("Assessment: {} - {}", analysis.grade.label(), analysis.summary),
            );
            ui.label(analysis.context);

            let speed = PlotPoints::from_iter(session.speed_history.iter().copied());
            let latency = PlotPoints::from_iter(session.latency_history.iter().copied());
            Plot::new(format!("benchmark-{}", session.session_id))
                .legend(Legend::default())
                .height(180.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new("MB/s", speed));
                    plot_ui.line(Line::new("Latency (ms)", latency));
                });
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

fn speed_stats(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let mut count = 0u64;
    let mut sum = 0.0f64;
    let mut peak = 0.0f64;
    for v in values {
        count += 1;
        sum += v;
        peak = peak.max(v);
    }
    let avg = if count == 0 { 0.0 } else { sum / count as f64 };
    (peak, avg)
}

#[derive(Clone, Copy)]
enum Grade {
    Pending,
    Pass,
    Caution,
    Fail,
}

impl Grade {
    fn label(self) -> &'static str {
        match self {
            Grade::Pending => "Pending",
            Grade::Pass => "Pass",
            Grade::Caution => "Caution",
            Grade::Fail => "Fail",
        }
    }
}

struct Assessment {
    grade: Grade,
    summary: String,
    context: String,
}

fn assess_benchmark(
    test_kind: TestKind,
    avg_mbps: f64,
    peak_mbps: f64,
    avg_latency: f64,
    peak_latency: f64,
    status: SessionStatus,
) -> Assessment {
    if !matches!(status, SessionStatus::Completed | SessionStatus::Failed) {
        return Assessment {
            grade: Grade::Pending,
            summary: "Running benchmark... collecting enough samples for verdict".to_owned(),
            context: "Verdict is finalized after completion.".to_owned(),
        };
    }

    match test_kind {
        TestKind::ActionCam => {
            // Action-cam profile prioritizes sustained sequential throughput and low spikes.
            let grade = if avg_mbps >= 40.0 && peak_latency <= 60.0 {
                Grade::Pass
            } else if avg_mbps >= 22.0 && peak_latency <= 120.0 {
                Grade::Caution
            } else {
                Grade::Fail
            };

            let summary = match grade {
                Grade::Pass => "Likely suitable for high-bitrate sequential recording".to_owned(),
                Grade::Caution => {
                    "Borderline for high-bitrate capture; may drop frames during spikes".to_owned()
                }
                Grade::Fail => "Not reliable for demanding continuous recording workloads".to_owned(),
                Grade::Pending => String::new(),
            };
            let context = format!(
                "Reference: sustained sequential write is key. Avg {:.1} MB/s, peak {:.1} MB/s, peak latency {:.1} ms.",
                avg_mbps, peak_mbps, peak_latency
            );
            Assessment {
                grade,
                summary,
                context,
            }
        }
        TestKind::OsDrive => {
            // 4K random: convert throughput to approximate IOPS.
            let approx_iops = (avg_mbps * 1024.0 / 4.0).max(0.0);
            let grade = if avg_mbps >= 8.0 && avg_latency <= 8.0 {
                Grade::Pass
            } else if avg_mbps >= 3.0 && avg_latency <= 20.0 {
                Grade::Caution
            } else {
                Grade::Fail
            };

            let summary = match grade {
                Grade::Pass => "Good random I/O responsiveness for OS/app usage".to_owned(),
                Grade::Caution => "Usable for light random access; expect stutter under load".to_owned(),
                Grade::Fail => "Poor random I/O; not recommended for OS-like workloads".to_owned(),
                Grade::Pending => String::new(),
            };
            let context = format!(
                "Reference: 4K random favors low latency. Avg {:.1} MB/s (~{approx_iops:.0} IOPS), avg latency {:.2} ms, peak latency {:.2} ms.",
                avg_mbps, avg_latency, peak_latency
            );
            Assessment {
                grade,
                summary,
                context,
            }
        }
        _ => Assessment {
            grade: Grade::Pending,
            summary: "Assessment not defined for this test type".to_owned(),
            context: String::new(),
        },
    }
}
