//! egui rendering. Builds an owned `Snapshot`, renders from it (so no borrow on
//! `App` is held during rendering), collects user intents into locals, then
//! applies them as mutations to `App` afterwards.

use crate::app::App;
use eframe::egui;

pub fn draw(app: &mut App, ctx: &egui::Context) {
    let s = app.snapshot();

    // Intents collected during rendering, applied after.
    let mut toggle: Option<String> = None;
    let mut want_refresh = false;
    let mut want_reclaim = false;
    let mut do_confirm = false;
    let mut cancel_confirm = false;

    egui::TopBottomPanel::top("bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("WSL2 VHDX Compactor");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!s.running, egui::Button::new("⟳ Refresh"))
                    .clicked()
                {
                    want_refresh = true;
                }
                let can_reclaim = !s.running && !s.selected.is_empty();
                if ui
                    .add_enabled(can_reclaim, egui::Button::new("Reclaim ▶"))
                    .clicked()
                {
                    want_reclaim = true;
                }
            });
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        if s.distros.is_empty() {
            ui.label("No WSL2 distros found (WSL operations require Windows).");
        }

        egui::Grid::new("distros")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                ui.label("");
                ui.label("Distro");
                ui.label("State");
                ui.label("ext4.vhdx");
                ui.end_row();

                for d in &s.distros {
                    let mut checked = s.selected.contains(&d.name);
                    if ui.checkbox(&mut checked, "").changed() {
                        toggle = Some(d.name.clone());
                    }
                    ui.label(&d.name);
                    ui.label(if d.running { "Running" } else { "Stopped" });
                    ui.label(d.size.map(|sz| sz.human()).unwrap_or_else(|| "—".into()));
                    ui.end_row();
                }
            });

        if let Some((distro, pct)) = &s.progress {
            ui.separator();
            ui.label(format!("Compacting {distro}…"));
            ui.add(egui::ProgressBar::new(*pct as f32 / 100.0).show_percentage());
        }

        ui.separator();
        ui.label("Log");
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in &s.log {
                    ui.monospace(line);
                }
            });
    });

    // The destructive part: make the global WSL shutdown explicit.
    if s.confirm_open {
        egui::Window::new("Confirm reclaim")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(
                    "This runs `wsl --shutdown`, stopping ALL WSL2 distros and \
                     Docker Desktop's WSL backend.",
                );
                ui.label("Unsaved work in any running distro will be lost. Continue?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel_confirm = true;
                    }
                    if ui.button("Shut down & compact").clicked() {
                        do_confirm = true;
                    }
                });
            });
    }

    // Apply collected intents now that no borrow on `App` is held.
    if let Some(name) = toggle {
        app.toggle(&name);
    }
    if want_refresh {
        app.refresh();
    }
    if want_reclaim {
        app.open_confirm();
    }
    if cancel_confirm {
        app.close_confirm();
    }
    if do_confirm {
        app.confirm_and_start(ctx.clone());
    }
}
