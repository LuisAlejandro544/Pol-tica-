//! Propósito del archivo:
//! Pantalla dedicada de Defensa Nacional y Operaciones Militares (`Military`) estilo HOI4.
//! Muestra aprestamiento operativo, impacto directo del mantenimiento militar en los ingresos,
//! reclutamiento de soldados por estado/provincia y control de movimiento de unidades entre estados.

use crate::models::{ArmyUnit, GameState, PlayerAction, ProvinceGraph};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, game_button,
    panel_frame, step_button, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{RichText, Ui, Vec2};

pub enum MilitaryScreenAction {
    None,
    PlayerAction(PlayerAction),
    OpenMapForUnit(u64),
}

pub fn render_military_screen(
    ui: &mut Ui,
    state: &mut GameState,
    graph: &ProvinceGraph,
) -> MilitaryScreenAction {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return MilitaryScreenAction::None;
    };

    let mut out_action = MilitaryScreenAction::None;
    let mut drills_msg: Option<String> = None;

    // Recolectar estados propios del jugador y estados accesibles (propios, aliados o en guerra)
    let mut own_provinces: Vec<(String, String)> = graph
        .values()
        .filter(|p| p.country_id == pid)
        .map(|p| (p.mesh_name.clone(), p.name.clone()))
        .collect();
    own_provinces.sort_by(|a, b| a.1.cmp(&b.1));

    let mut accessible_destinations: Vec<(String, String, String)> = graph
        .values()
        .filter(|p| {
            if p.country_id == pid {
                return true;
            }
            let rel = state.get_relation(&pid, &p.country_id);
            rel.alliance || rel.at_war
        })
        .map(|p| (p.mesh_name.clone(), p.name.clone(), p.country_id.clone()))
        .collect();
    accessible_destinations.sort_by(|a, b| a.1.cmp(&b.1));

    let recruit_prov_mesh = state
        .selected_province_mesh
        .clone()
        .filter(|m| own_provinces.iter().any(|(pm, _)| pm == m))
        .or_else(|| own_provinces.first().map(|(pm, _)| pm.clone()))
        .unwrap_or_else(|| format!("PROV_{pid}_0_CAP"));

    let recruit_prov_idx = own_provinces
        .iter()
        .position(|(pm, _)| *pm == recruit_prov_mesh)
        .unwrap_or(0);

    let recruit_prov_name = own_provinces
        .get(recruit_prov_idx)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| "Capital".to_string());

    let player_units: Vec<ArmyUnit> = state
        .units
        .iter()
        .filter(|u| u.country_id == pid)
        .cloned()
        .collect();

    if state.selected_unit_id.is_none()
        || !player_units
            .iter()
            .any(|u| Some(u.id) == state.selected_unit_id)
    {
        state.selected_unit_id = player_units.first().map(|u| u.id);
    }

    let selected_unit = state
        .selected_unit_id
        .and_then(|uid| player_units.iter().find(|u| u.id == uid).cloned());

    let neighbors: Vec<(String, String, u64, f64)> = {
        let Some(c) = state.countries.get(&pid) else {
            return MilitaryScreenAction::None;
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
        return MilitaryScreenAction::None;
    };
    let shield_country = country.clone();
    let player_power = country.military_power_score().max(1.0);
    let max_power = neighbors
        .iter()
        .map(|(_, _, _, s)| *s)
        .fold(player_power, f64::max)
        .max(10.0);

    let mut next_selected_prov: Option<String> = None;
    let mut selected_uid = state.selected_unit_id;

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
                        let btn_w = ui.available_width().min(360.0);
                        draw_section_header(
                            ui,
                            GameIcon::Military,
                            "RECLUTAMIENTO Y MANTENIMIENTO",
                            ACCENT_CYAN,
                        );

                        well_frame().show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Efectivos Activos: {} tropas ({} unidades)",
                                    country.military.active_troops,
                                    player_units.len()
                                ))
                                .size(14.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Costo Mensual de Tropas: -${:.2} M USD/mes (reduce tus ingresos netos)",
                                    country.economy.troop_upkeep_usd_millions
                                ))
                                .size(11.5)
                                .color(ACCENT_CRIMSON),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Balance Neto Actual: {:+.2} M USD/mes  |  Poder: {:.1}",
                                    country.economy.last_monthly_balance,
                                    country.military_power_score()
                                ))
                                .size(11.5)
                                .color(if country.economy.last_monthly_balance >= 0.0 {
                                    ACCENT_EMERALD
                                } else {
                                    ACCENT_CRIMSON
                                }),
                            );
                        });
                        ui.add_space(6.0);

                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            17.0,
                            (country.military.readiness / 100.0) as f32,
                            ACCENT_EMERALD,
                            &format!("Aprestamiento Operativo: {:.0}%", country.military.readiness),
                        );
                        ui.add_space(6.0);

                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Presupuesto de Defensa (% PIB)")
                                        .strong()
                                        .size(12.0),
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

                        ui.add_space(6.0);

                        // Selector de estado destino para reclutar soldados
                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            ui.label(
                                RichText::new("Estado de Reclutamiento:")
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                            );
                            ui.horizontal(|ui| {
                                if own_provinces.len() > 1
                                    && step_button(
                                        ui,
                                        Vec2::new(30.0, 28.0),
                                        "<",
                                        None,
                                    )
                                    .clicked()
                                {
                                    let prev = if recruit_prov_idx == 0 {
                                        own_provinces.len() - 1
                                    } else {
                                        recruit_prov_idx - 1
                                    };
                                    next_selected_prov = Some(own_provinces[prev].0.clone());
                                }
                                ui.label(
                                    RichText::new(&recruit_prov_name)
                                        .strong()
                                        .size(13.0)
                                        .color(ACCENT_CYAN),
                                );
                                if own_provinces.len() > 1
                                    && step_button(
                                        ui,
                                        Vec2::new(30.0, 28.0),
                                        ">",
                                        None,
                                    )
                                    .clicked()
                                {
                                    let next = (recruit_prov_idx + 1) % own_provinces.len();
                                    next_selected_prov = Some(own_provinces[next].0.clone());
                                }
                            });
                        });

                        ui.add_space(6.0);

                        for (troops, cost_m, upkeep_m) in [
                            (1_000_u64, 70.0_f64, 1.35_f64),
                            (2_500_u64, 175.0_f64, 3.38_f64),
                            (5_000_u64, 350.0_f64, 6.75_f64),
                        ] {
                            let can_recruit = country.economy.treasury_usd_millions >= cost_m;
                            let lbl = format!(
                                "Reclutar {troops} (-${cost_m:.0}M | -{upkeep_m:.2}M/mes)"
                            );
                            if game_button(
                                ui,
                                Vec2::new(btn_w, 36.0),
                                &lbl,
                                Some(GameIcon::Population),
                                can_recruit,
                                Some(ACCENT_GOLD),
                            )
                            .clicked()
                                && can_recruit
                            {
                                out_action = MilitaryScreenAction::PlayerAction(
                                    PlayerAction::RecruitSoldiers {
                                        country_id: pid.clone(),
                                        province_mesh: recruit_prov_mesh.clone(),
                                        troops,
                                        cost_m,
                                    },
                                );
                            }
                            ui.add_space(4.0);
                        }

                        let can_drills = country.economy.treasury_usd_millions >= 200.0;
                        if game_button(
                            ui,
                            Vec2::new(btn_w, 38.0),
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
                            drills_msg =
                                Some("Maniobras militares completadas con éxito.".to_string());
                        }
                    });
                });

                cols[1].vertical(|ui| {
                    panel_frame(false).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Map,
                            "DESPLIEGUE Y MARCHA POR ESTADOS",
                            ACCENT_GOLD,
                        );

                        egui::ScrollArea::vertical()
                            .id_salt("military_units_scroll")
                            .max_height(155.0)
                            .show(ui, |ui| {
                                for unit in &player_units {
                                    let is_sel = selected_uid == Some(unit.id);
                                    let cur_name = graph
                                        .get(&unit.current_province)
                                        .map(|p| p.name.as_str())
                                        .unwrap_or(&unit.current_province);
                                    let status = if let Some(tgt) = &unit.target_province {
                                        let tgt_name = graph
                                            .get(tgt)
                                            .map(|p| p.name.as_str())
                                            .unwrap_or(tgt);
                                        format!(
                                            "Marchando a {} ({:.0}%)",
                                            tgt_name,
                                            unit.march_progress * 100.0
                                        )
                                    } else {
                                        format!("Estacionada en {cur_name}")
                                    };

                                    card_frame(Some(if is_sel {
                                        ACCENT_GOLD
                                    } else {
                                        BORDER_BRASS_DIM
                                    }))
                                    .inner_margin(7.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • {} tropas",
                                                    unit.name, unit.troops
                                                ))
                                                .strong()
                                                .size(12.0)
                                                .color(if is_sel {
                                                    ACCENT_GOLD
                                                } else {
                                                    TEXT_PRIMARY
                                                }),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if game_button(
                                                        ui,
                                                        Vec2::new(72.0, 24.0),
                                                        if is_sel { "Activa" } else { "Elegir" },
                                                        None,
                                                        is_sel,
                                                        Some(ACCENT_CYAN),
                                                    )
                                                    .clicked()
                                                    {
                                                        selected_uid = Some(unit.id);
                                                    }
                                                },
                                            );
                                        });
                                        ui.label(
                                            RichText::new(status).size(11.0).color(TEXT_MUTED),
                                        );
                                    });
                                    ui.add_space(3.0);
                                }
                            });

                        if let Some(unit) = selected_unit {
                            ui.add_space(6.0);
                            if game_button(
                                ui,
                                Vec2::new(ui.available_width(), 34.0),
                                "Mover Unidad en el Mapa 3D >",
                                Some(GameIcon::Map),
                                true,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                            {
                                out_action = MilitaryScreenAction::OpenMapForUnit(unit.id);
                            }

                            ui.add_space(5.0);
                            ui.label(
                                RichText::new("O elige estado destino directamente:")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            egui::ScrollArea::vertical()
                                .id_salt("military_destinations_scroll")
                                .max_height(105.0)
                                .show(ui, |ui| {
                                    for (dest_mesh, dest_name, owner_cid) in
                                        &accessible_destinations
                                    {
                                        if *dest_mesh == unit.current_province {
                                            continue;
                                        }
                                        let lbl = if *owner_cid == pid {
                                            format!("Marchar a {dest_name}")
                                        } else {
                                            format!("Marchar a {dest_name} [{owner_cid}]")
                                        };
                                        if game_button(
                                            ui,
                                            Vec2::new(ui.available_width(), 28.0),
                                            &lbl,
                                            None,
                                            false,
                                            Some(ACCENT_GOLD),
                                        )
                                        .clicked()
                                        {
                                            out_action = MilitaryScreenAction::PlayerAction(
                                                PlayerAction::OrderUnitMove {
                                                    unit_id: unit.id,
                                                    target_province: dest_mesh.clone(),
                                                },
                                            );
                                        }
                                        ui.add_space(2.0);
                                    }
                                });
                        }

                        ui.add_space(8.0);
                        draw_section_header(
                            ui,
                            GameIcon::Dispatch,
                            "BALANCE EN FRONTERAS",
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
                                .max_height(140.0)
                                .show(ui, |ui| {
                                    for (nid, nname, troops, score) in neighbors {
                                        card_frame(Some(BORDER_BRASS_DIM)).inner_margin(7.0).show(
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

    state.selected_unit_id = selected_uid;
    if let Some(prov) = next_selected_prov {
        state.selected_province_mesh = Some(prov);
    }
    if let Some(msg) = drills_msg {
        state.push_chronicle(pid, msg);
    }

    out_action
}
