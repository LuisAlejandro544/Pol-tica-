//! Propósito del archivo:
//! Pantalla dedicada de Política Interna, Partidos Políticos y Congreso (`Politics`).
//! Muestra la composición parlamentaria con barra ideológica, tarjetas de partidos con
//! medidor de aprobación, cuenta regresiva electoral y decretos ejecutivos en botones táctiles.

use crate::models::GameState;
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, game_button,
    ideology_color, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BORDER_BRASS, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{CornerRadius, Rect, RichText, Stroke, Ui, Vec2};

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
        let shield_country = country.clone();

        egui::ScrollArea::vertical()
            .id_salt("politics_screen_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    draw_country_shield(ui, Vec2::new(34.0, 28.0), &shield_country);
                    ui.label(
                        RichText::new(format!(
                            "CONGRESO Y PARTIDOS POLÍTICOS — {}",
                            country.name.to_uppercase()
                        ))
                        .strong()
                        .size(16.5)
                        .color(ACCENT_GOLD),
                    );
                    ui.separator();
                    well_frame().show(ui, |ui| {
                        ui.label(
                            RichText::new(format!(
                                "Elecciones en {} meses",
                                country.months_to_election
                            ))
                            .strong()
                            .size(12.0)
                            .color(ACCENT_CYAN),
                        );
                    });
                });
                ui.add_space(6.0);

                // Barra proporcional de escaños en el Congreso + medidor de estabilidad
                panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("COMPOSICIÓN DEL PARLAMENTO (100% ESCAÑOS)")
                                .strong()
                                .size(12.5)
                                .color(ACCENT_GOLD),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            draw_gauge_bar(
                                ui,
                                155.0,
                                18.0,
                                (country.stability / 100.0) as f32,
                                if country.stability >= 55.0 {
                                    ACCENT_EMERALD
                                } else {
                                    ACCENT_CRIMSON
                                },
                                &format!("Estabilidad {:.0}%", country.stability),
                            );
                        });
                    });
                    ui.add_space(5.0);
                    let bar_w = ui.available_width().min(900.0);
                    let (bar_rect, _) =
                        ui.allocate_exact_size(Vec2::new(bar_w, 24.0), egui::Sense::hover());
                    let painter = ui.painter_at(bar_rect);
                    let mut cursor_x = bar_rect.min.x;

                    for party in &country.political_parties {
                        let seg_w = (party.seats_pct as f32 / 100.0) * bar_w;
                        let seg_rect = Rect::from_min_size(
                            egui::pos2(cursor_x, bar_rect.min.y),
                            Vec2::new(seg_w.max(2.0), 24.0),
                        );
                        painter.rect_filled(
                            seg_rect,
                            CornerRadius::same(2),
                            ideology_color(&party.ideology),
                        );
                        cursor_x += seg_w;
                    }
                    painter.rect_stroke(
                        bar_rect,
                        CornerRadius::same(2),
                        Stroke::new(1.5_f32, BORDER_BRASS),
                        egui::StrokeKind::Inside,
                    );
                });

                ui.add_space(8.0);

                // Listado detallado de partidos y acciones ejecutivas
                ui.columns(2, |cols| {
                    cols[0].vertical(|ui| {
                        panel_frame(false).inner_margin(10.0).show(ui, |ui| {
                            draw_section_header(
                                ui,
                                GameIcon::Politics,
                                "FUERZAS POLÍTICAS ACTIVAS",
                                ACCENT_GOLD,
                            );
                            egui::ScrollArea::vertical()
                                .id_salt("politics_parties_scroll")
                                .max_height(215.0)
                                .show(ui, |ui| {
                                    for party in &country.political_parties {
                                        let col = ideology_color(&party.ideology);
                                        card_frame(Some(if party.is_ruling {
                                            ACCENT_GOLD
                                        } else {
                                            BORDER_BRASS_DIM
                                        }))
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                let role = if party.is_ruling {
                                                    "GOBIERNO"
                                                } else {
                                                    "OPOSICIÓN"
                                                };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "[{}] {}",
                                                        party.acronym, party.name
                                                    ))
                                                    .strong()
                                                    .size(12.5)
                                                    .color(col),
                                                );
                                                ui.label(
                                                    RichText::new(role)
                                                        .size(10.5)
                                                        .strong()
                                                        .color(if party.is_ruling {
                                                            ACCENT_GOLD
                                                        } else {
                                                            TEXT_MUTED
                                                        }),
                                                );
                                            });
                                            ui.label(
                                                RichText::new(format!(
                                                    "Ideología: {}  |  Escaños: {:.1}%",
                                                    party.ideology, party.seats_pct
                                                ))
                                                .size(11.5)
                                                .color(TEXT_PRIMARY),
                                            );
                                            ui.add_space(3.0);
                                            draw_gauge_bar(
                                                ui,
                                                ui.available_width(),
                                                15.0,
                                                (party.approval_pct / 100.0) as f32,
                                                col,
                                                &format!("Aprobación {:.1}%", party.approval_pct),
                                            );
                                        });
                                        ui.add_space(5.0);
                                    }
                                });
                        });
                    });

                    cols[1].vertical(|ui| {
                        panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                            let btn_w = ui.available_width().min(340.0);
                            draw_section_header(
                                ui,
                                GameIcon::Crown,
                                "DECRETOS DE GOBERNABILIDAD",
                                ACCENT_CYAN,
                            );
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new(
                                        "Impulsa pactos o campañas ejecutivas para sostener la \
                                         estabilidad parlamentaria antes de las próximas elecciones:",
                                    )
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                                );
                            });
                            ui.add_space(8.0);

                            let can_dialogue = country.economy.treasury_usd_millions >= 250.0;
                            if game_button(
                                ui,
                                Vec2::new(btn_w, 44.0),
                                "Mesa de Diálogo (-250 M, +5% Estab.)",
                                Some(GameIcon::Diplomacy),
                                can_dialogue,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                                && can_dialogue
                            {
                                country.economy.treasury_usd_millions -= 250.0;
                                country.stability = (country.stability + 5.0).min(100.0);
                                decree_msg = Some(
                                    "Se celebró una mesa de diálogo nacional en el Congreso."
                                        .to_string(),
                                );
                            }

                            ui.add_space(7.0);

                            let can_campaign = country.economy.treasury_usd_millions >= 180.0;
                            if game_button(
                                ui,
                                Vec2::new(btn_w, 44.0),
                                "Campaña de Apoyo (-180 M, +4% Aprob.)",
                                Some(GameIcon::Politics),
                                can_campaign,
                                Some(ACCENT_GOLD),
                            )
                            .clicked()
                                && can_campaign
                            {
                                country.economy.treasury_usd_millions -= 180.0;
                                if let Some(ruling) =
                                    country.political_parties.iter_mut().find(|p| p.is_ruling)
                                {
                                    ruling.approval_pct = (ruling.approval_pct + 4.0).min(95.0);
                                }
                                decree_msg = Some(
                                    "El partido de gobierno impulsó una campaña ciudadana."
                                        .to_string(),
                                );
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
