use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use crate::state::AppState;
use crate::targets::DriveTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainTab {
    Benchmark,
    Capacity,
    Forensics,
}

pub struct SdTesterApp {
    pub engine: crate::engine::EngineRuntime,
    pub state: AppState,
    pub available_targets: Vec<DriveTarget>,
    tab: MainTab,
    last_refresh: Instant,
}

impl Default for SdTesterApp {
    fn default() -> Self {
        Self {
            engine: crate::engine::EngineRuntime::new(),
            state: AppState::default(),
            available_targets: crate::win32::discovery::discover_removable_targets(),
            tab: MainTab::Benchmark,
            last_refresh: Instant::now(),
        }
    }
}

impl eframe::App for SdTesterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut processed = 0usize;
        loop {
            match self.engine.updates_rx.try_recv() {
                Ok(update) => {
                    self.state.apply_update(update);
                    processed += 1;
                    if processed >= 256 {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        if self.last_refresh.elapsed() > Duration::from_secs(5) {
            self.available_targets = crate::win32::discovery::discover_removable_targets();
            self.last_refresh = Instant::now();
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, MainTab::Benchmark, "Benchmark");
                ui.selectable_value(&mut self.tab, MainTab::Capacity, "Capacity");
                ui.selectable_value(&mut self.tab, MainTab::Forensics, "Forensics");
                if ui.button("Clear Finished").clicked() {
                    self.state.clear_finished_sessions();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            MainTab::Benchmark => crate::ui::benchmark_tab::render(self, ui),
            MainTab::Capacity => crate::ui::capacity_tab::render(self, ui),
            MainTab::Forensics => crate::ui::forensics_tab::render(self, ui),
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
