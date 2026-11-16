//! Propósito del archivo:
//! Pantalla dedicada de Defensa Nacional y Operaciones Militares (`Military`) estilo HOI4.
//! Muestra aprestamiento operativo en medidor, presupuesto de defensa con botones `[-]` `[+]`,
//! reclutamiento y maniobras en botones táctiles y comparativa de fuerza con vecinos.

use crate::models::GameState;
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, game_button,
    panel_frame, step_button, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{RichText, Ui, Vec2};

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
    let shield_country = country.clone();
    let player_power = country.military_power_score().max(1.0);
    let max_power = neighbors
        .iter()
        .map(|(_, _, _, s)| *s)
        .fold(player_power, f64::max)
        .max(10.0);

    egui::ScrollArea::vertical()
        .id_salt("military_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                draw_country_shield(ui, Vec2::new(34.0, 28.0), &shield_country);
                ui.label(
                    RichText::new(format!(
                        "ESTADO MAYOR CONJUNTO — {}",
                        shield_country.name.to_uppercase()
                    ))
                    .strong()
                    .size(16.5)
                    .color(ACCENT_GOLD),
                );
            });
            ui.add_space(6.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                        let btn_w = ui.available_width().min(340.0);
                        draw_section_header(
                            ui,
                            GameIcon::Military,
                            "CAPACIDAD DE DEFENSA",
                            ACCENT_CYAN,
                        );

                        well_frame().show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Efectivos Activos: {} tropas",
                                    country.military.active_troops
                                ))
                                .size(15.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Índice de Poder Militar: {:.1}",
                                    country.military_power_score()
                                ))
                                .size(12.5)
                                .color(ACCENT_GOLD),
                            );
                        });
                        ui.add_space(6.0);

                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            18.0,
                            (country.military.readiness / 100.0) as f32,
                            ACCENT_EMERALD,
                            &format!("Aprestamiento Operativo: {:.0}%", country.military.readiness),
                        );
                        ui.add_space(8.0);

                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Presupuesto de Defensa (% PIB)")
                                        .strong()
                                        .size(12.5),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{:.2}%",
                                                country.military.defense_budget_pct
                                            ))
                                            .strong()
                                            .color(ACCENT_GOLD),
                                        );
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if step_button(
                                    ui,
                                    Vec2::new(32.0, 28.0),
                                    "",
                                    Some(GameIcon::Minus),
                                )
                                .clicked()
                                {
                                    country.military.defense_budget_pct =
                                        (country.military.defense_budget_pct - 0.25).clamp(0.0, 6.0);
                                }
                                let slider_w = (ui.available_width() - 42.0).max(80.0);
                                ui.add_sized(
                                    [slider_w, 24.0],
                                    egui::Slider::new(
                                        &mut country.military.defense_budget_pct,
                                        0.0..=6.0,
                                    )
                                    .show_value(false),
                                );
                                if step_button(
                                    ui,
                                    Vec2::new(32.0, 28.0),
                                    "",
                                    Some(GameIcon::Plus),
                                )
                                .clicked()
                                {
                                    country.military.defense_budget_pct =
                                        (country.military.defense_budget_pct + 0.25).clamp(0.0, 6.0);
                                }
                            });
                        });

                        ui.add_space(8.0);

                        let can_recruit = country.economy.treasury_usd_millions >= 350.0;
                        if game_button(
                            ui,
                            Vec2::new(btn_w, 42.0),
                            "Reclutar 5,000 Efectivos (-350 M USD)",
                            Some(GameIcon::Population),
                            can_recruit,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                            && can_recruit
                        {
                            country.economy.treasury_usd_millions -= 350.0;
                            country.military.active_troops += 5_000;
                            recruit_log = Some(
                                "Se incorporaron 5,000 nuevos efectivos a las fuerzas armadas."
                                    .to_string(),
                            );
                        }
                        ui.add_space(6.0);

                        let can_drills = country.economy.treasury_usd_millions >= 200.0;
                        if game_button(
                            ui,
                            Vec2::new(btn_w, 42.0),
                            "Maniobras Conjuntas (-200 M, +6% Aprest.)",
                            Some(GameIcon::Military),
                            can_drills,
                            Some(ACCENT_EMERALD),
                        )
                        .clicked()
                            && can_drills
                        {
                            country.economy.treasury_usd_millions -= 200.0;
                            country.military.readiness =
                                (country.military.readiness + 6.0).min(100.0);
                            recruit_log =
                                Some("Maniobras militares completadas con éxito.".to_string());
                        }
                    });
                });

                cols[1].vertical(|ui| {
                    panel_frame(false).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Dispatch,
                            "BALANCE MILITAR EN FRONTERAS",
                            ACCENT_GOLD,
                        );
                        if neighbors.is_empty() {
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new("Sin fronteras terrestres directas.")
                                        .color(TEXT_MUTED),
                                );
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("military_neighbors_scroll")
                                .max_height(225.0)
                                .show(ui, |ui| {
                                    for (nid, nname, troops, score) in neighbors {
                                        card_frame(Some(BORDER_BRASS_DIM)).inner_margin(8.0).show(
                                            ui,
                                            |ui| {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(
                                                        RichText::new(format!("{nname} [{nid}]"))
                                                            .strong()
                                                            .color(TEXT_PRIMARY),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("• {troops} tropas"))
                                                            .size(11.5)
                                                            .color(TEXT_MUTED),
                                                    );
                                                });
                                                ui.add_space(3.0);
                                                draw_gauge_bar(
                                                    ui,
                                                    ui.available_width(),
                                                    15.0,
                                                    (score / max_power) as f32,
                                                    if score > player_power {
                                                        ACCENT_CRIMSON
                                                    } else {
                                                        ACCENT_EMERALD
                                                    },
                                                    &format!("Poder: {score:.1}"),
                                                );
                                            },
                                        );
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
