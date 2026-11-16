//! Propósito del archivo:
//! Pantalla dedicada de Política Interna, Partidos Políticos y Congreso (`Politics`).
//! Muestra el reparto visual de escaños parlamentarios, ideologías, popularidad de
//! cada partido, cuenta regresiva electoral y decretos de gobernabilidad.

use crate::models::GameState;
use crate::ui::theme::{
    ideology_color, ACCENT_CYAN, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE,
    TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Rect, RichText, Stroke, Ui, Vec2};

pub fn render_politics_screen(ui: &mut Ui, state: &mut GameState) {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return;
    };

    let mut decree_msg: Option<String> = None;

    {
        let Some(country) = state.countries.get_mut(&pid) else {
            return;
        };

        egui::ScrollArea::vertical()
            .id_salt("politics_screen_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("CONGRESO Y PARTIDOS POLÍTICOS — {}", country.name))
                            .strong()
                            .size(18.0)
                            .color(ACCENT_GOLD),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "Estabilidad: {:.1}% • Elecciones en {} meses",
                            country.stability, country.months_to_election
                        ))
                        .color(ACCENT_CYAN),
                    );
                });
                ui.add_space(8.0);

                // Barra proporcional de escaños en el Congreso
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                    .corner_radius(8.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Composición del Parlamento (100% Escaños)").strong());
                        ui.add_space(6.0);
                        let bar_w = ui.available_width().min(900.0);
                        let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 24.0), egui::Sense::hover());
                        let painter = ui.painter_at(bar_rect);
                        let mut cursor_x = bar_rect.min.x;

                        for party in &country.political_parties {
                            let seg_w = (party.seats_pct as f32 / 100.0) * bar_w;
                            let seg_rect = Rect::from_min_size(
                                egui::pos2(cursor_x, bar_rect.min.y),
                                Vec2::new(seg_w.max(2.0), 24.0),
                            );
                            painter.rect_filled(seg_rect, 2.0, ideology_color(&party.ideology));
                            cursor_x += seg_w;
                        }
                    });

                ui.add_space(8.0);

                // Listado detallado de partidos y acciones ejecutivas
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        egui::Frame::new()
                            .fill(BG_PANEL)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(8.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Fuerzas Políticas Activas").strong().color(ACCENT_GOLD));
                                ui.separator();
                                egui::ScrollArea::vertical()
                                    .id_salt("politics_parties_scroll")
                                    .max_height(210.0)
                                    .show(ui, |ui| {
                                        for party in &country.political_parties {
                                            egui::Frame::new()
                                                .fill(BG_CARD)
                                                .corner_radius(6.0)
                                                .inner_margin(10.0)
                                                .show(ui, |ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        let role = if party.is_ruling { "GOBIERNO" } else { "OPOSICIÓN" };
                                                        ui.label(
                                                            RichText::new(format!("[{}] {}", party.acronym, party.name))
                                                                .strong()
                                                                .color(ideology_color(&party.ideology)),
                                                        );
                                                        ui.label(RichText::new(role).size(11.0).color(ACCENT_GOLD));
                                                    });
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Ideología: {}  |  Escaños: {:.1}%  |  Popularidad: {:.1}%",
                                                            party.ideology, party.seats_pct, party.approval_pct
                                                        ))
                                                        .size(12.5)
                                                        .color(TEXT_PRIMARY),
                                                    );
                                                });
                                            ui.add_space(6.0);
                                        }
                                    });
                            });
                    });

                    cols[1].vertical(|ui| {
                        egui::Frame::new()
                            .fill(BG_PANEL)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(8.0)
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                let btn_w = ui.available_width().min(320.0);
                                ui.label(RichText::new("Acciones de Gobernabilidad").strong().color(ACCENT_CYAN));
                                ui.separator();
                                ui.label(
                                    RichText::new("Ejecuta medidas rápidas para mantener la estabilidad parlamentaria:")
                                        .size(12.5)
                                        .color(TEXT_MUTED),
                                );
                                ui.add_space(8.0);

                                if ui
                                    .add_sized([btn_w, 44.0], egui::Button::new("Mesa de Diálogo (-250 M USD, +5% Estab.)"))
                                    .clicked()
                                    && country.economy.treasury_usd_millions >= 250.0
                                {
                                    country.economy.treasury_usd_millions -= 250.0;
                                    country.stability = (country.stability + 5.0).min(100.0);
                                    decree_msg = Some("Se celebró una mesa de diálogo nacional en el Congreso.".to_string());
                                }

                                ui.add_space(6.0);

                                if ui
                                    .add_sized([btn_w, 44.0], egui::Button::new("Campaña de Apoyo (-180 M USD, +4% Aprob.)"))
                                    .clicked()
                                    && country.economy.treasury_usd_millions >= 180.0
                                {
                                    country.economy.treasury_usd_millions -= 180.0;
                                    if let Some(ruling) = country.political_parties.iter_mut().find(|p| p.is_ruling) {
                                        ruling.approval_pct = (ruling.approval_pct + 4.0).min(95.0);
                                    }
                                    decree_msg = Some("El partido de gobierno impulsó una campaña ciudadana.".to_string());
                                }
                            });
                    });
                });
            });
    }

    if let Some(msg) = decree_msg {
        state.push_chronicle(pid, msg);
    }
}
