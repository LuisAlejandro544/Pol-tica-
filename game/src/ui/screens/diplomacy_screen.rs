//! Propósito del archivo:
//! Pantalla dedicada de Relaciones Exteriores, Cancillería y Diplomacia (`Diplomacy`).
//! Permite asignar un Ministro de Relaciones Exteriores con estadísticas aleatorias
//! (mediante popup dedicado), enviar misiones diplomáticas instantáneas (0 días de espera:
//! mejora de relaciones, pacto de no agresión, alianza, acuerdo de paz y paz blanca),
//! y gestionar tratados comerciales, sanciones o declaraciones de guerra con las 20 naciones.

use crate::engine::{
    compute_minister_mission_chance_pct, compute_minister_relation_gain, generate_runtime_seed,
};
use crate::models::{ForeignMinister, GameState, MinisterMissionKind, PlayerAction};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, format_usd_millions,
    game_button, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_BTN, BG_BTN_ACTIVE, BG_BTN_HOVER, BG_CARD, BG_PANEL, BORDER_BRASS,
    BORDER_BRASS_DIM, BORDER_SUBTLE, SAFE_RIGHT_PAD, TEXT_DARK, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Color32, CornerRadius, RichText, Sense, Stroke, Ui, Vec2};

pub struct DiplomacyScreenResult {
    pub actions: Vec<PlayerAction>,
}

pub fn render_diplomacy_screen(
    ui: &mut Ui,
    state: &mut GameState,
    show_minister_popup: &mut bool,
) -> DiplomacyScreenResult {
    let mut out = DiplomacyScreenResult {
        actions: Vec::new(),
    };

    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return out;
    };

    let assigned_minister: Option<ForeignMinister> = state
        .countries
        .get(&pid)
        .and_then(|c| c.foreign_minister.clone());

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

            // Cabecera del Ministro de Relaciones Exteriores asignado (o aviso para asignarlo)
            panel_frame(true).inner_margin(8.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(m) = &assigned_minister {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new(format!(
                                    "MINISTRO DE RELACIONES: {} ({} años, {})",
                                    m.profile.full_name, m.profile.age, m.profile.nationality
                                ))
                                .strong()
                                .size(13.0)
                                .color(ACCENT_GOLD),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Especialidad: {}  |  Mejora Rel.: +{:.0}%  •  Paz: {:.0}%  •  Paz Blanca: {:.0}%  •  Alianza: {:.0}%  •  No Agresión: {:.0}%",
                                    m.specialty,
                                    m.stats.relation_boost_pct,
                                    m.stats.peace_treaty_chance_pct,
                                    m.stats.white_peace_chance_pct,
                                    m.stats.alliance_chance_pct,
                                    m.stats.non_aggression_chance_pct,
                                ))
                                .size(11.0)
                                .color(TEXT_PRIMARY),
                            );
                        });
                    } else {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("MINISTRO DE RELACIONES EXTERIORES: SIN ASIGNAR")
                                    .strong()
                                    .size(13.0)
                                    .color(ACCENT_CRIMSON),
                            );
                            ui.label(
                                RichText::new(
                                    "Asigna un Canciller con estadísticas aleatorias para enviar misiones diplomáticas instantáneas (0 días).",
                                )
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_lbl = if assigned_minister.is_some() {
                            "Cambiar Ministro"
                        } else {
                            "Asignar Ministro >"
                        };
                        if game_button(
                            ui,
                            Vec2::new(158.0, 32.0),
                            btn_lbl,
                            Some(GameIcon::Crown),
                            assigned_minister.is_none(),
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                        {
                            *show_minister_popup = true;
                        }
                    });
                });
            });
            ui.add_space(6.0);

            if let Some((ok, msg)) = state.last_diplomacy_feedback.clone() {
                let border_col = if ok { ACCENT_EMERALD } else { ACCENT_CRIMSON };
                card_frame(Some(border_col)).inner_margin(7.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(msg).size(11.5).strong().color(border_col));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if game_button(
                                ui,
                                Vec2::new(24.0, 20.0),
                                "X",
                                None,
                                false,
                                Some(border_col),
                            )
                            .clicked()
                            {
                                state.last_diplomacy_feedback = None;
                            }
                        });
                    });
                });
                ui.add_space(6.0);
            }

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
                            .max_height(265.0)
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
                                    } else if rel.non_aggression_pact {
                                        "NO AG."
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

                // Columna derecha: Embajada, misiones instantáneas del Ministro y tratados
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
                                if let Some(pres) = target.president() {
                                    ui.label(
                                        RichText::new(format!(
                                            "Presidente: {} ({} años, {})",
                                            pres.full_name, pres.age, pres.nationality
                                        ))
                                        .size(11.5)
                                        .color(ACCENT_CYAN),
                                    );
                                }
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
                                    "TLC: {}  |  No Agresión: {}  |  Alianza: {}  |  Guerra: {}",
                                    if rel.trade_pact { "SÍ" } else { "NO" },
                                    if rel.non_aggression_pact { "SÍ" } else { "NO" },
                                    if rel.alliance { "SÍ" } else { "NO" },
                                    if rel.at_war { "ACTIVA" } else { "NO" },
                                ))
                                .size(11.5)
                                .color(if rel.at_war { ACCENT_CRIMSON } else { TEXT_PRIMARY }),
                            );
                            let opp = target.opposition_leaders();
                            if !opp.is_empty() {
                                let opp_line = opp
                                    .iter()
                                    .map(|(p, l)| {
                                        format!(
                                            "[{}] {} ({} años, {})",
                                            p.acronym, l.full_name, l.age, l.nationality
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("  •  ");
                                ui.label(
                                    RichText::new(format!("Oposición: {opp_line}"))
                                        .size(10.5)
                                        .color(TEXT_MUTED),
                                );
                            }
                        });
                        ui.add_space(6.0);

                        // Bloque 1: Misiones del Ministro de Relaciones (0 días)
                        card_frame(Some(ACCENT_GOLD)).show(ui, |ui| {
                            let btn_w = ui.available_width().min(360.0);
                            draw_section_header(
                                ui,
                                GameIcon::Crown,
                                "MISIONES DEL MINISTRO (INSTANTÁNEAS • 0 DÍAS)",
                                ACCENT_GOLD,
                            );

                            if let Some(minister) = &assigned_minister {
                                let gain = compute_minister_relation_gain(minister);
                                let improve_lbl = format!(
                                    "Mejorar Relaciones (+{gain:.1} Opinión • 0 días)"
                                );
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 34.0),
                                    &improve_lbl,
                                    Some(GameIcon::Diplomacy),
                                    true,
                                    Some(ACCENT_CYAN),
                                )
                                .clicked()
                                {
                                    out.actions.push(PlayerAction::ExecuteMinisterMission {
                                        player_country_id: pid.clone(),
                                        target_country_id: target_id.clone(),
                                        mission: MinisterMissionKind::ImproveRelations,
                                        roll_seed: generate_runtime_seed(&format!(
                                            "IMP:{pid}:{target_id}:{}",
                                            state.total_days
                                        )),
                                    });
                                }
                                ui.add_space(4.0);

                                if !rel.at_war {
                                    if !rel.non_aggression_pact {
                                        let p_nap = compute_minister_mission_chance_pct(
                                            state,
                                            minister,
                                            &pid,
                                            &target_id,
                                            MinisterMissionKind::ProposeNonAggression,
                                        );
                                        let lbl = format!(
                                            "Pacto de No Agresión (Prob. {p_nap:.0}% • 0 días)"
                                        );
                                        if game_button(
                                            ui,
                                            Vec2::new(btn_w, 34.0),
                                            &lbl,
                                            Some(GameIcon::Stability),
                                            false,
                                            Some(ACCENT_EMERALD),
                                        )
                                        .clicked()
                                        {
                                            out.actions.push(PlayerAction::ExecuteMinisterMission {
                                                player_country_id: pid.clone(),
                                                target_country_id: target_id.clone(),
                                                mission: MinisterMissionKind::ProposeNonAggression,
                                                roll_seed: generate_runtime_seed(&format!(
                                                    "NAP:{pid}:{target_id}:{}",
                                                    state.total_days
                                                )),
                                            });
                                        }
                                        ui.add_space(4.0);
                                    }

                                    if !rel.alliance {
                                        let p_ally = compute_minister_mission_chance_pct(
                                            state,
                                            minister,
                                            &pid,
                                            &target_id,
                                            MinisterMissionKind::ProposeAlliance,
                                        );
                                        let lbl = format!(
                                            "Proponer Alianza (Prob. {p_ally:.0}% • 0 días)"
                                        );
                                        if game_button(
                                            ui,
                                            Vec2::new(btn_w, 34.0),
                                            &lbl,
                                            Some(GameIcon::Military),
                                            false,
                                            Some(ACCENT_GOLD),
                                        )
                                        .clicked()
                                        {
                                            out.actions.push(PlayerAction::ExecuteMinisterMission {
                                                player_country_id: pid.clone(),
                                                target_country_id: target_id.clone(),
                                                mission: MinisterMissionKind::ProposeAlliance,
                                                roll_seed: generate_runtime_seed(&format!(
                                                    "ALLY:{pid}:{target_id}:{}",
                                                    state.total_days
                                                )),
                                            });
                                        }
                                    }
                                } else {
                                    let p_peace = compute_minister_mission_chance_pct(
                                        state,
                                        minister,
                                        &pid,
                                        &target_id,
                                        MinisterMissionKind::ProposePeaceTreaty,
                                    );
                                    let lbl_peace = format!(
                                        "Acuerdo de Paz (Prob. {p_peace:.0}% • 0 días)"
                                    );
                                    if game_button(
                                        ui,
                                        Vec2::new(btn_w, 34.0),
                                        &lbl_peace,
                                        Some(GameIcon::Diplomacy),
                                        true,
                                        Some(ACCENT_EMERALD),
                                    )
                                    .clicked()
                                    {
                                        out.actions.push(PlayerAction::ExecuteMinisterMission {
                                            player_country_id: pid.clone(),
                                            target_country_id: target_id.clone(),
                                            mission: MinisterMissionKind::ProposePeaceTreaty,
                                            roll_seed: generate_runtime_seed(&format!(
                                                "PEACE:{pid}:{target_id}:{}",
                                                state.total_days
                                            )),
                                        });
                                    }
                                    ui.add_space(4.0);

                                    let p_white = compute_minister_mission_chance_pct(
                                        state,
                                        minister,
                                        &pid,
                                        &target_id,
                                        MinisterMissionKind::ProposeWhitePeace,
                                    );
                                    let lbl_white = format!(
                                        "Paz Blanca (Prob. {p_white:.0}% • 0 días)"
                                    );
                                    if game_button(
                                        ui,
                                        Vec2::new(btn_w, 34.0),
                                        &lbl_white,
                                        Some(GameIcon::Crown),
                                        false,
                                        Some(ACCENT_CYAN),
                                    )
                                    .clicked()
                                    {
                                        out.actions.push(PlayerAction::ExecuteMinisterMission {
                                            player_country_id: pid.clone(),
                                            target_country_id: target_id.clone(),
                                            mission: MinisterMissionKind::ProposeWhitePeace,
                                            roll_seed: generate_runtime_seed(&format!(
                                                "WPEACE:{pid}:{target_id}:{}",
                                                state.total_days
                                            )),
                                        });
                                    }
                                }
                            } else {
                                ui.label(
                                    RichText::new(
                                        "No hay Ministro de Relaciones asignado. Asígnalo para negociar paz, alianzas, no agresión o mejorar relaciones al instante.",
                                    )
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(5.0);
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 34.0),
                                    "Asignar Ministro Ahora >",
                                    Some(GameIcon::Crown),
                                    true,
                                    Some(ACCENT_GOLD),
                                )
                                .clicked()
                                {
                                    *show_minister_popup = true;
                                }
                            }
                        });

                        ui.add_space(6.0);

                        // Bloque 2: Tratados comerciales, sanciones y declaración de guerra
                        card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                            let btn_w = ui.available_width().min(360.0);
                            if !rel.at_war && !rel.trade_pact {
                                let can_tlc = rel.opinion >= 20.0;
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 32.0),
                                    "Firmar Tratado Comercial (Req. >= +20)",
                                    Some(GameIcon::Economy),
                                    can_tlc,
                                    Some(ACCENT_EMERALD),
                                )
                                .clicked()
                                    && can_tlc
                                {
                                    out.actions.push(PlayerAction::SignTradePact {
                                        player_country_id: pid.clone(),
                                        target_country_id: target_id.clone(),
                                    });
                                }
                                ui.add_space(4.0);
                            }

                            if !rel.at_war {
                                if game_button(
                                    ui,
                                    Vec2::new(btn_w, 32.0),
                                    "Emitir Sanción / Protesta (-20 Opinión)",
                                    Some(GameIcon::Dispatch),
                                    false,
                                    Some(ACCENT_CRIMSON),
                                )
                                .clicked()
                                {
                                    out.actions.push(PlayerAction::IssueSanction {
                                        player_country_id: pid.clone(),
                                        target_country_id: target_id.clone(),
                                    });
                                }

                                if !rel.alliance && !rel.non_aggression_pact {
                                    ui.add_space(4.0);
                                    if game_button(
                                        ui,
                                        Vec2::new(btn_w, 32.0),
                                        "Declarar Guerra",
                                        Some(GameIcon::Military),
                                        false,
                                        Some(ACCENT_CRIMSON),
                                    )
                                    .clicked()
                                    {
                                        out.actions.push(PlayerAction::DeclareWar {
                                            player_country_id: pid.clone(),
                                            target_country_id: target_id.clone(),
                                        });
                                    }
                                }
                            }
                        });
                    });
                });
            });
        });

    out
}

/// Popup modal dedicado para elegir y asignar al Ministro de Relaciones Exteriores (0 días).
pub fn render_minister_assign_popup(
    ctx: &egui::Context,
    state: &GameState,
    open: &mut bool,
) -> Option<PlayerAction> {
    if !*open {
        return None;
    }
    let Some(pid) = state.player_country_id.as_deref() else {
        *open = false;
        return None;
    };
    let Some(country) = state.countries.get(pid) else {
        *open = false;
        return None;
    };

    let mut action: Option<PlayerAction> = None;
    let screen = ctx.screen_rect();
    let win_w = (screen.width() - (SAFE_RIGHT_PAD as f32) - 48.0).clamp(360.0, 620.0);
    let max_scroll_h = (screen.height() - 150.0).clamp(150.0, 290.0);

    egui::Area::new(egui::Id::new("minister_assign_popup_scrim"))
        .fixed_pos(screen.min)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(6, 10, 16, 155),
            );
        });

    let mut win_open = *open;
    egui::Window::new(
        RichText::new("ASIGNAR MINISTRO DE RELACIONES EXTERIORES")
            .strong()
            .color(ACCENT_GOLD),
    )
    .order(egui::Order::Tooltip)
    .open(&mut win_open)
    .collapsible(false)
    .resizable(false)
    .anchor(
        egui::Align2::CENTER_CENTER,
        [-((SAFE_RIGHT_PAD as f32) * 0.45), 0.0],
    )
    .default_width(win_w)
    .max_width(win_w)
    .frame(
        egui::Frame::window(&ctx.style())
            .fill(BG_PANEL)
            .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
            .corner_radius(CornerRadius::same(5)),
    )
    .show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(
                    "Cada candidato posee estadísticas aleatorias. Todas sus misiones se ejecutan al instante (0 días).",
                )
                .size(11.5)
                .color(TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if game_button(
                    ui,
                    Vec2::new(155.0, 26.0),
                    "Renovar Candidatos",
                    None,
                    false,
                    Some(ACCENT_CYAN),
                )
                .clicked()
                {
                    action = Some(PlayerAction::RefreshMinisterCandidates {
                        country_id: pid.to_string(),
                        seed: generate_runtime_seed(&format!("MREFRESH:{pid}:{}", state.total_days)),
                    });
                }
            });
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("diplomacy_minister_candidates_scroll")
            .max_height(max_scroll_h)
            .show(ui, |ui| {
                for cand in &country.minister_candidates {
                    let s = &cand.stats;
                    egui::Frame::new()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0_f32, BORDER_BRASS_DIM))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.label(
                                        RichText::new(format!(
                                            "{} ({} años, {})",
                                            cand.profile.full_name,
                                            cand.profile.age,
                                            cand.profile.nationality
                                        ))
                                        .strong()
                                        .size(12.5)
                                        .color(ACCENT_GOLD),
                                    );
                                    ui.label(
                                        RichText::new(format!("Especialidad: {}", cand.specialty))
                                            .size(11.0)
                                            .color(ACCENT_CYAN),
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if game_button(
                                            ui,
                                            Vec2::new(136.0, 30.0),
                                            "Asignar Canciller",
                                            Some(GameIcon::Crown),
                                            true,
                                            Some(ACCENT_EMERALD),
                                        )
                                        .clicked()
                                        {
                                            action = Some(PlayerAction::AssignForeignMinister {
                                                country_id: pid.to_string(),
                                                candidate_id: cand.id,
                                            });
                                            *open = false;
                                        }
                                    },
                                );
                            });

                            ui.add_space(4.0);
                            egui::Grid::new(egui::Id::new(("minister_cand_stats", cand.id)))
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    let bar_w = ((ui.available_width() - 16.0) * 0.5).clamp(110.0, 250.0);
                                    draw_gauge_bar(
                                        ui,
                                        bar_w,
                                        14.0,
                                        (s.relation_boost_pct / 75.0) as f32,
                                        ACCENT_CYAN,
                                        &format!("Mejora Relaciones +{:.0}%", s.relation_boost_pct),
                                    );
                                    draw_gauge_bar(
                                        ui,
                                        bar_w,
                                        14.0,
                                        (s.non_aggression_chance_pct / 100.0) as f32,
                                        ACCENT_EMERALD,
                                        &format!("Pacto No Agresión {:.0}%", s.non_aggression_chance_pct),
                                    );
                                    ui.end_row();

                                    draw_gauge_bar(
                                        ui,
                                        bar_w,
                                        14.0,
                                        (s.alliance_chance_pct / 100.0) as f32,
                                        ACCENT_GOLD,
                                        &format!("Alianza {:.0}%", s.alliance_chance_pct),
                                    );
                                    draw_gauge_bar(
                                        ui,
                                        bar_w,
                                        14.0,
                                        (s.peace_treaty_chance_pct / 100.0) as f32,
                                        ACCENT_EMERALD,
                                        &format!("Acuerdo de Paz {:.0}%", s.peace_treaty_chance_pct),
                                    );
                                    ui.end_row();

                                    draw_gauge_bar(
                                        ui,
                                        bar_w,
                                        14.0,
                                        (s.white_peace_chance_pct / 100.0) as f32,
                                        ACCENT_CYAN,
                                        &format!("Paz Blanca {:.0}%", s.white_peace_chance_pct),
                                    );
                                    ui.label(
                                        RichText::new("Ejecución: 0 días")
                                            .size(10.5)
                                            .color(TEXT_MUTED),
                                    );
                                    ui.end_row();
                                });
                        });
                    ui.add_space(5.0);
                }
            });
    });

    if !win_open {
        *open = false;
    }
    action
}
