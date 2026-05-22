use egui::Ui;

use crate::targets::DriveTarget;

pub fn render_drive_picker(ui: &mut Ui, available: &[DriveTarget], selected: &mut Vec<DriveTarget>) {
    ui.label("Removable targets (SD/USB):");
    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
        for drive in available {
            let mut on = selected.iter().any(|d| d.id == drive.id);
            let label = format!(
                "{} [{}] ({})",
                drive.drive_letter,
                drive.kind_label(),
                drive.display_name
            );
            if ui.checkbox(&mut on, label).changed() {
                if on {
                    if !selected.iter().any(|d| d.id == drive.id) {
                        selected.push(drive.clone());
                    }
                } else {
                    selected.retain(|d| d.id != drive.id);
                }
            }
        }
    });
}
