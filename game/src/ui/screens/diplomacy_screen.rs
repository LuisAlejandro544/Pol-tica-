//! Propósito del archivo:
//! Pantalla dedicada de Relaciones Exteriores y Diplomacia (`Diplomacy`).
//! Lista las 20 naciones en botones físicos con escudo heráldico, estado bilateral
//! y medidor de opinión, junto a una embajada con botones de acción diplomática.

use crate::models::GameState;
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, format_usd_millions,
    game_button, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_BTN, BG_BTN_ACTIVE, BG_BTN_HOVER, BORDER_BRASS_DIM, BORDER_SUBTLE, TEXT_DARK,
    TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{CornerRadius, RichText, Sense, Stroke, Ui, Vec2};

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
            draw_section_header(
                ui,
                GameIcon::Diplomacy,
                "CANCILLERÍA Y DIPLOMACIA REGIONAL",
                ACCENT_GOLD,
            );

            let other_ids: Vec<String> = state
                .countries
                .keys()
                .filter(|id| *id != &pid)
                .cloned()
                .collect();

            if state.selected_country_id.as_deref() == Some(pid.as_str())
                || state.selected_country_id.is_none()
            {
                state.selected_country_id = other_ids.first().cloned();
            }

            ui.columns(2, |cols| {
                // Columna izquierda: Botones físicos de embajadas latinoamericanas
                cols[0].vertical(|ui| {
                    panel_frame(false).inner_margin(8.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Map,
                            "NACIONES DE LATINOAMÉRICA",
                            ACCENT_CYAN,
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("diplomacy_countries_scroll")
                            .max_height(235.0)
                            .show(ui, |ui| {
                                let row_w = (ui.available_width() - 4.0).max(140.0);
                                for oid in &other_ids {
                                    let c = &state.countries[oid];
                                    let rel = state.get_relation(&pid, oid);
                                    let is_sel =
                                        state.selected_country_id.as_deref() == Some(oid.as_str());
                                    let status = if rel.at_war {
                                        "GUERRA"
                                    } else if rel.alliance {
                                        "ALIADO"
                                    } else if rel.trade_pact {
                                        "TLC"
                                    } else {
                                        "NEUTRAL"
                                    };

                                    let (row_rect, row_resp) = ui.allocate_exact_size(
                                        Vec2::new(row_w, 34.0),
                                        Sense::click(),
                                    );
                                    if ui.is_rect_visible(row_rect) {
                                        let fill = if is_sel {
                                            BG_BTN_ACTIVE
                                        } else if row_resp.hovered() {
                                            BG_BTN_HOVER
                                        } else {
                                            BG_BTN
                                        };
                                        let border = if is_sel {
                                            ACCENT_GOLD
                                        } else {
                                            BORDER_SUBTLE
                                        };
                                        let p = ui.painter();
                                        p.rect_filled(row_rect, CornerRadius::same(4), fill);
                                        p.rect_stroke(
                                            row_rect,
                                            CornerRadius::same(4),
                                            Stroke::new(
                                                if is_sel { 1.5_f32 } else { 1.0_f32 },
                                                border,
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                    }

                                    let mut row_ui = ui.new_child(
                                        egui::UiBuilder::new().max_rect(row_rect.shrink(4.0)),
                                    );
                                    row_ui.horizontal_centered(|ui| {
                                        draw_country_shield(ui, Vec2::new(26.0, 22.0), c);
                                        ui.label(
                                            RichText::new(format!("{} • {}", c.name, status))
                                                .strong()
                                                .size(12.0)
                                                .color(if is_sel { TEXT_DARK } else { TEXT_PRIMARY }),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!("{:+.0}", rel.opinion))
                                                        .strong()
                                                        .size(11.5)
                                                        .color(if is_sel {
                                                            TEXT_DARK
                                                        } else if rel.opinion >= 0.0 {
                                                            ACCENT_EMERALD
                                                        } else {
                                                            ACCENT_CRIMSON
                                                        }),
                                                );
                                            },
                                        );
                                    });

                                    if row_resp.clicked() {
                                        state.selected_country_id = Some(oid.clone());
                                    }
                                    ui.add_space(3.0);
                                }
                            });
                    });
                });

                // Columna derecha: Embajada y tratados bilaterales
                cols[1].vertical(|ui| {
                    let Some(target_id) = state.selected_country_id.clone() else {
                        return;
                    };
                    let Some(target) = state.countries.get(&target_id).cloned() else {
                        return;
                    };
                    let rel = state.get_relation(&pid, &target_id);

                    panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            draw_country_shield(ui, Vec2::new(38.0, 32.0), &target);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(
                                    RichText::new(format!(
                                        "EMBAJADA EN {} ({})",
                                        target.name.to_uppercase(),
                                        target.id
                                    ))
                                    .size(15.5)
                                    .strong()
                                    .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "PIB: {} • Fuerzas: {} efectivos",
                                        format_usd_millions(target.economy.gdp_usd_millions),
                                        target.military.active_troops
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                        });
                        ui.add_space(6.0);

                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            18.0,
                            ((rel.opinion + 100.0) / 200.0) as f32,
                            if rel.opinion >= 0.0 {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CRIMSON
                            },
                            &format!("Relación Bilateral: {:+.0} / 100", rel.opinion),
                        );
                        ui.add_space(5.0);

                        well_frame().show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Tratado Comercial: {}   |   Alianza Defensiva: {}",
                                    if rel.trade_pact { "VIGENTE" } else { "NO" },
                                    if rel.alliance { "VIGENTE" } else { "NO" }
                                ))
                                .size(12.0)
                                .color(TEXT_PRIMARY),
                            );
                        });
                        ui.add_space(6.0);

                        let key = GameState::ordered_pair(&pid, &target_id);

                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            let btn_w = ui.available_width().min(340.0);
                            if game_button(
                                ui,
                                Vec2::new(btn_w, 36.0),
                                "Enviar Misión Diplomática (+12 Opinión)",
                                Some(GameIcon::Diplomacy),
                                false,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                            {
                                let entry = state.relations.entry(key.clone()).or_default();
                                entry.opinion = (entry.opinion + 12.0).min(100.0);
                                action_log =
                                    Some(format!("Misión diplomática enviada a {}.", target.name));
                            }
                            ui.add_space(5.0);

                            if !rel.trade_pact {
                                let can_tlc = rel.opinion >= 20.0;
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 36.0),
                                    "Firmar Tratado Comercial (Req. >= +20)",
                                    Some(GameIcon::Economy),
                                    can_tlc,
                                    Some(ACCENT_EMERALD),
                                )
                                .clicked()
                                    && can_tlc
                                {
                                    let entry = state.relations.entry(key.clone()).or_default();
                                    entry.trade_pact = true;
                                    entry.opinion = (entry.opinion + 10.0).min(100.0);
                                    action_log = Some(format!(
                                        "Tratado comercial firmado con {}.",
                                        target.name
                                    ));
                                }
                                ui.add_space(5.0);
                            }

                            if !rel.alliance {
                                let can_ally = rel.opinion >= 55.0;
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 36.0),
                                    "Proponer Alianza Estratégica (Req. >= +55)",
                                    Some(GameIcon::Military),
                                    can_ally,
                                    Some(ACCENT_GOLD),
                                )
                                .clicked()
                                    && can_ally
                                {
                                    let entry = state.relations.entry(key.clone()).or_default();
                                    entry.alliance = true;
                                    entry.opinion = (entry.opinion + 15.0).min(100.0);
                                    action_log = Some(format!(
                                        "Alianza estratégica sellada con {}.",
                                        target.name
                                    ));
                                }
                                ui.add_space(5.0);
                            }

                            if game_button(
                                ui,
                                Vec2::new(btn_w, 36.0),
                                "Emitir Sanción / Protesta (-20 Opinión)",
                                Some(GameIcon::Dispatch),
                                false,
                                Some(ACCENT_CRIMSON),
                            )
                            .clicked()
                            {
                                let entry = state.relations.entry(key).or_default();
                                entry.opinion = (entry.opinion - 20.0).max(-100.0);
                                entry.alliance = false;
                                entry.trade_pact = false;
                                action_log =
                                    Some(format!("Tensión diplomática con {}.", target.name));
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
