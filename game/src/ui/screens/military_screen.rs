//! Propósito del archivo:
//! Pantalla dedicada de Defensa Nacional y Operaciones Militares (`Military`).
//! Gestiona presupuesto de defensa, alistamiento de nuevas divisiones, preparación
//! operativa y comparación del balance militar regional frente a países vecinos.

use crate::models::GameState;
use crate::ui::theme::{
    ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE,
    TEXT_MUTED,
};
use egui::{RichText, Stroke, Ui};

pub fn render_military_screen(ui: &mut Ui, state: &mut GameState) {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return;
    };

    let mut recruit_log: Option<String> = None;
    let neighbors: Vec<(String, String, u64, f64)> = {
        let Some(c) = state.countries.get(&pid) else {
            return;
        };
        c.neighbors
            .iter()
            .filter_map(|nid| {
                state.countries.get(nid).map(|nc| {
                    (
                        nc.id.clone(),
                        nc.name.clone(),
                        nc.military.active_troops,
                        nc.military_power_score(),
                    )
                })
            })
            .collect()
    };

    let Some(country) = state.countries.get_mut(&pid) else {
        return;
    };

    egui::ScrollArea::vertical()
        .id_salt("military_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("ESTADO MAYOR CONJUNTO — {}", country.name))
                    .strong()
                    .size(18.0)
                    .color(ACCENT_GOLD),
            );
            ui.add_space(6.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let btn_w = ui.available_width().min(320.0);
                            ui.label(RichText::new("Capacidad de Defensa").strong().color(ACCENT_CYAN));
                            ui.separator();
                            ui.label(
                                RichText::new(format!("Efectivos Activos: {} tropas", country.military.active_troops))
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.label(format!("Aprestamiento Operativo: {:.0}%", country.military.readiness));
                            ui.label(format!(
                                "Índice de Poder Militar: {:.1}",
                                country.military_power_score()
                            ));
                            ui.add_space(8.0);

                            ui.label("Presupuesto de Defensa (% PIB)");
                            ui.add(egui::Slider::new(&mut country.military.defense_budget_pct, 0.0..=6.0).suffix("%"));
                            ui.add_space(10.0);

                            if ui
                                .add_sized([btn_w, 42.0], egui::Button::new("Reclutar 5,000 Efectivos (-350 M USD)"))
                                .clicked()
                                && country.economy.treasury_usd_millions >= 350.0
                            {
                                country.economy.treasury_usd_millions -= 350.0;
                                country.military.active_troops += 5_000;
                                recruit_log = Some("Se incorporaron 5,000 nuevos efectivos a las fuerzas armadas.".to_string());
                            }
                            ui.add_space(6.0);
                            if ui
                                .add_sized([btn_w, 42.0], egui::Button::new("Maniobras Conjuntas (-200 M USD, +6% Aprest.)"))
                                .clicked()
                                && country.economy.treasury_usd_millions >= 200.0
                            {
                                country.economy.treasury_usd_millions -= 200.0;
                                country.military.readiness = (country.military.readiness + 6.0).min(100.0);
                                recruit_log = Some("Maniobras militares completadas con éxito.".to_string());
                            }
                        });
                });

                cols[1].vertical(|ui| {
                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Balance Militar en Fronteras Vecinas").strong().color(ACCENT_GOLD));
                            ui.separator();
                            if neighbors.is_empty() {
                                ui.label(RichText::new("Sin fronteras terrestres directas.").color(TEXT_MUTED));
                            } else {
                                egui::ScrollArea::vertical()
                                    .id_salt("military_neighbors_scroll")
                                    .max_height(210.0)
                                    .show(ui, |ui| {
                                        for (nid, nname, troops, score) in neighbors {
                                            egui::Frame::new().fill(BG_CARD).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(RichText::new(format!("{nname} [{nid}]")).strong());
                                                    ui.label(RichText::new(format!("• {troops} tropas")).color(TEXT_MUTED));
                                                    ui.label(RichText::new(format!("• Poder: {score:.1}")).color(ACCENT_EMERALD));
                                                });
                                            });
                                            ui.add_space(4.0);
                                        }
                                    });
                            }
                        });
                });
            });
        });

    if let Some(msg) = recruit_log {
        state.push_chronicle(pid, msg);
    }
}
