//! Propósito del archivo:
//! Pantalla dedicada de Relaciones Exteriores y Diplomacia (`Diplomacy`).
//! Lista las 20 naciones restantes de Latinoamérica con su opinión bilateral, tratados
//! comerciales, alianzas y botones de negociación diplomática.

use crate::models::GameState;
use crate::ui::theme::{
    format_usd_millions, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_CARD,
    BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED,
};
use egui::{RichText, Stroke, Ui};

pub fn render_diplomacy_screen(ui: &mut Ui, state: &mut GameState) {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return;
    };

    let mut action_log: Option<String> = None;

    egui::ScrollArea::vertical()
        .id_salt("diplomacy_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(
                RichText::new("CANCILLERÍA Y DIPLOMACIA REGIONAL")
                    .strong()
                    .size(18.0)
                    .color(ACCENT_GOLD),
            );
            ui.add_space(6.0);

            let other_ids: Vec<String> = state
                .countries
                .keys()
                .filter(|id| *id != &pid)
                .cloned()
                .collect();

            if state.selected_country_id.as_deref() == Some(pid.as_str()) || state.selected_country_id.is_none() {
                state.selected_country_id = other_ids.first().cloned();
            }

            ui.columns(2, |cols| {
                // Columna izquierda: Listado de embajadas latinoamericanas
                cols[0].vertical(|ui| {
                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(8.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Naciones de Latinoamérica").strong().color(ACCENT_CYAN));
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("diplomacy_countries_scroll")
                                .max_height(220.0)
                                .show(ui, |ui| {
                                    for oid in &other_ids {
                                        let c = &state.countries[oid];
                                        let rel = state.get_relation(&pid, oid);
                                        let is_sel = state.selected_country_id.as_deref() == Some(oid.as_str());
                                        let status = if rel.at_war {
                                            "GUERRA"
                                        } else if rel.alliance {
                                            "ALIADO"
                                        } else if rel.trade_pact {
                                            "TLC"
                                        } else {
                                            "NEUTRAL"
                                        };
                                        let label = format!("{} [{}] • {} ({:+.0})", c.name, c.id, status, rel.opinion);
                                        if ui
                                            .add_sized([ui.available_width(), 28.0], egui::Button::selectable(is_sel, label))
                                            .clicked()
                                        {
                                            state.selected_country_id = Some(oid.clone());
                                        }
                                    }
                                });
                        });
                });

                // Columna derecha: Acciones diplomáticas con la nación seleccionada
                cols[1].vertical(|ui| {
                    let Some(target_id) = state.selected_country_id.clone() else {
                        return;
                    };
                    let Some(target) = state.countries.get(&target_id).cloned() else {
                        return;
                    };
                    let rel = state.get_relation(&pid, &target_id);

                    egui::Frame::new()
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Embajada en {} ({})", target.name, target.id))
                                    .size(18.0)
                                    .strong()
                                    .color(ACCENT_GOLD),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "PIB: {} • Fuerzas: {} efectivos",
                                    format_usd_millions(target.economy.gdp_usd_millions),
                                    target.military.active_troops
                                ))
                                .color(TEXT_MUTED),
                            );
                            ui.separator();

                            ui.label(
                                RichText::new(format!("Relación Bilateral: {:+.0} / 100", rel.opinion))
                                    .size(15.5)
                                    .color(if rel.opinion >= 0.0 { ACCENT_EMERALD } else { ACCENT_CRIMSON }),
                            );
                            ui.label(format!(
                                "Tratado Comercial: {} | Alianza Defensiva: {}",
                                if rel.trade_pact { "Vigente" } else { "No" },
                                if rel.alliance { "Vigente" } else { "No" }
                            ));
                            ui.add_space(8.0);

                            let key = GameState::ordered_pair(&pid, &target_id);

                            egui::Frame::new()
                                .fill(BG_CARD)
                                .corner_radius(6.0)
                                .inner_margin(8.0)
                                .show(ui, |ui| {
                                    let btn_w = ui.available_width().min(320.0);
                                    if ui
                                        .add_sized([btn_w, 38.0], egui::Button::new("Enviar Misión Diplomática (+12 Opinión)"))
                                        .clicked()
                                    {
                                        let entry = state.relations.entry(key.clone()).or_default();
                                        entry.opinion = (entry.opinion + 12.0).min(100.0);
                                        action_log = Some(format!("Misión diplomática enviada a {}.", target.name));
                                    }
                                    ui.add_space(5.0);

                                    if !rel.trade_pact
                                        && ui
                                            .add_sized([btn_w, 38.0], egui::Button::new("Firmar Tratado Comercial (Req. > +20)"))
                                            .clicked()
                                        && rel.opinion >= 20.0
                                    {
                                        let entry = state.relations.entry(key.clone()).or_default();
                                        entry.trade_pact = true;
                                        entry.opinion = (entry.opinion + 10.0).min(100.0);
                                        action_log = Some(format!("Tratado comercial firmado con {}.", target.name));
                                    }
                                    ui.add_space(5.0);

                                    if !rel.alliance
                                        && ui
                                            .add_sized([btn_w, 38.0], egui::Button::new("Proponer Alianza Estratégica (Req. > +55)"))
                                            .clicked()
                                        && rel.opinion >= 55.0
                                    {
                                        let entry = state.relations.entry(key.clone()).or_default();
                                        entry.alliance = true;
                                        entry.opinion = (entry.opinion + 15.0).min(100.0);
                                        action_log = Some(format!("Alianza estratégica sellada con {}.", target.name));
                                    }
                                    ui.add_space(5.0);

                                    if ui
                                        .add_sized([btn_w, 38.0], egui::Button::new("Emitir Sanción / Protesta (-20 Opinión)"))
                                        .clicked()
                                    {
                                        let entry = state.relations.entry(key).or_default();
                                        entry.opinion = (entry.opinion - 20.0).max(-100.0);
                                        entry.alliance = false;
                                        entry.trade_pact = false;
                                        action_log = Some(format!("Tensión diplomática con {}.", target.name));
                                    }
                                });
                        });
                });
            });
        });

    if let Some(msg) = action_log {
        state.push_chronicle(&pid, msg);
    }
}
