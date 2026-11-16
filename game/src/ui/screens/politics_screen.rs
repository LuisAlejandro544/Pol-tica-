//! Propósito del archivo:
//! Pantalla dedicada de Política Interna, Partidos Políticos y Congreso (`Politics`).
//! Muestra la mesa circular del Congreso con sillas pintadas según los escaños de cada partido,
//! barra ideológica, tarjetas de partidos con medidor de aprobación, cuenta regresiva electoral
//! y decretos ejecutivos en botones táctiles.

use crate::models::{CountryState, GameState, PlayerAction, PoliticalDecreeKind, PoliticalParty};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, game_button,
    ideology_color, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_WELL, BORDER_BRASS, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

const TOTAL_CONGRESS_CHAIRS: usize = 50;

pub fn render_politics_screen(ui: &mut Ui, state: &mut GameState) -> Vec<PlayerAction> {
    let mut actions = Vec::new();
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return actions;
    };

    {
        let Some(country) = state.countries.get_mut(&pid) else {
            return actions;
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

                if let Some(pres) = shield_country.president() {
                    let ruling_acronym = shield_country
                        .ruling_party()
                        .map(|p| p.acronym.as_str())
                        .unwrap_or("GOB");
                    well_frame().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("PRESIDENCIA EN EJERCICIO:")
                                    .strong()
                                    .size(11.5)
                                    .color(ACCENT_GOLD),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{}  •  {} años  •  Nac.: {}  •  Partido [{}]",
                                    pres.full_name, pres.age, pres.nationality, ruling_acronym
                                ))
                                .strong()
                                .size(12.0)
                                .color(TEXT_PRIMARY),
                            );
                        });
                    });
                    ui.add_space(6.0);
                }

                // Mesa circular del Congreso con sillas pintadas por partido + quórum de coalición y estabilidad
                let chair_counts =
                    allocate_parliament_chairs(&country.political_parties, TOTAL_CONGRESS_CHAIRS);
                let support_pct = country.compute_legislative_support_pct();
                country.legislative_support_pct = support_pct;
                let has_majority = support_pct >= 50.0;

                panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("MESA CIRCULAR DEL CONGRESO (SILLAS Y COALICIÓN)")
                                .strong()
                                .size(12.5)
                                .color(ACCENT_GOLD),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "Quórum: {:.1}% ({})  •  Cap. Político: {:.0}",
                                support_pct,
                                if has_majority { "MAYORÍA" } else { "MINORÍA" },
                                country.political_capital
                            ))
                            .strong()
                            .size(11.5)
                            .color(if has_majority {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CRIMSON
                            }),
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
                    ui.add_space(6.0);

                    ui.horizontal_top(|ui| {
                        let table_size = Vec2::new(220.0, 200.0);
                        let hovered_party = draw_congress_round_table(
                            ui,
                            table_size,
                            &shield_country,
                            &chair_counts,
                        );

                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            let right_w = ui.available_width().max(180.0);
                            let (bar_rect, _) = ui.allocate_exact_size(
                                Vec2::new(right_w, 20.0),
                                Sense::hover(),
                            );
                            let painter = ui.painter_at(bar_rect);
                            let mut cursor_x = bar_rect.min.x;

                            for party in &country.political_parties {
                                let seg_w = (party.seats_pct as f32 / 100.0) * right_w;
                                let seg_rect = Rect::from_min_size(
                                    Pos2::new(cursor_x, bar_rect.min.y),
                                    Vec2::new(seg_w.max(2.0), 20.0),
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

                            ui.add_space(6.0);
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Curules pintadas en mesa circular ({TOTAL_CONGRESS_CHAIRS} sillas):"
                                    ))
                                    .strong()
                                    .size(11.0)
                                    .color(ACCENT_CYAN),
                                );
                                ui.add_space(3.0);
                                for (idx, party) in country.political_parties.iter().enumerate() {
                                    let chairs = chair_counts.get(idx).copied().unwrap_or(0);
                                    let col = ideology_color(&party.ideology);
                                    let is_hi = hovered_party == Some(idx);
                                    ui.horizontal(|ui| {
                                        let (dot_rect, _) = ui.allocate_exact_size(
                                            Vec2::splat(11.0),
                                            Sense::hover(),
                                        );
                                        ui.painter().rect_filled(
                                            dot_rect,
                                            CornerRadius::same(2),
                                            col,
                                        );
                                        let is_coalition = !party.is_ruling
                                            && country
                                                .coalition_parties
                                                .iter()
                                                .any(|ac| ac == &party.acronym);
                                        if party.is_ruling || is_hi || is_coalition {
                                            ui.painter().rect_stroke(
                                                dot_rect,
                                                CornerRadius::same(2),
                                                Stroke::new(
                                                    1.2_f32,
                                                    if party.is_ruling || is_hi {
                                                        ACCENT_GOLD
                                                    } else {
                                                        ACCENT_EMERALD
                                                    },
                                                ),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                        let tag = if party.is_ruling {
                                            "GOB"
                                        } else if is_coalition {
                                            "COALICIÓN"
                                        } else {
                                            "OPO"
                                        };
                                        ui.label(
                                            RichText::new(format!(
                                                "[{}] {} sillas ({:.1}%) • {} • {}",
                                                party.acronym,
                                                chairs,
                                                party.seats_pct,
                                                tag,
                                                party.leader.full_name
                                            ))
                                            .size(11.5)
                                            .strong()
                                            .color(if is_hi { ACCENT_GOLD } else { TEXT_PRIMARY }),
                                        );
                                    });
                                }
                            });

                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(if has_majority {
                                    "Mayoría legislativa activa: fortalece estabilidad institucional y capital político."
                                } else {
                                    "Gobierno en minoría: pacta coalición con partidos opositores para asegurar quórum."
                                })
                                .size(10.5)
                                .color(if has_majority { ACCENT_EMERALD } else { ACCENT_CRIMSON }),
                            );
                        });
                    });
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
                                    let mut coalition_action: Option<(String, String, bool)> = None;
                                    let current_cap = country.political_capital;

                                    for party in &country.political_parties {
                                        let col = ideology_color(&party.ideology);
                                        let is_coalition = !party.is_ruling
                                            && country
                                                .coalition_parties
                                                .iter()
                                                .any(|ac| ac == &party.acronym);
                                        card_frame(Some(if party.is_ruling {
                                            ACCENT_GOLD
                                        } else if is_coalition {
                                            ACCENT_EMERALD
                                        } else {
                                            BORDER_BRASS_DIM
                                        }))
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                let role = if party.is_ruling {
                                                    "GOBIERNO"
                                                } else if is_coalition {
                                                    "ALIADO COALICIÓN"
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
                                                        } else if is_coalition {
                                                            ACCENT_EMERALD
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
                                            let leader_role = if party.is_ruling {
                                                "Presidente"
                                            } else {
                                                "Líder oposición"
                                            };
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}: {} ({} años, {})",
                                                    leader_role,
                                                    party.leader.full_name,
                                                    party.leader.age,
                                                    party.leader.nationality
                                                ))
                                                .size(11.0)
                                                .color(ACCENT_CYAN),
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
                                            if !party.is_ruling {
                                                ui.add_space(4.0);
                                                let can_pact = is_coalition || current_cap >= 20.0;
                                                let btn_lbl = if is_coalition {
                                                    "Disolver Pacto de Coalición"
                                                } else {
                                                    "Pactar Coalición (-20 Cap.)"
                                                };
                                                if game_button(
                                                    ui,
                                                    Vec2::new(ui.available_width(), 26.0),
                                                    btn_lbl,
                                                    None,
                                                    can_pact,
                                                    Some(if is_coalition {
                                                        ACCENT_CRIMSON
                                                    } else {
                                                        ACCENT_EMERALD
                                                    }),
                                                )
                                                .clicked()
                                                    && can_pact
                                                {
                                                    coalition_action = Some((
                                                        party.acronym.clone(),
                                                        party.leader.full_name.clone(),
                                                        !is_coalition,
                                                    ));
                                                }
                                            }
                                        });
                                        ui.add_space(5.0);
                                    }

                                    if let Some((acronym, _leader_name, _join)) = coalition_action {
                                        actions.push(PlayerAction::ToggleCoalitionParty {
                                            country_id: pid.clone(),
                                            party_acronym: acronym,
                                        });
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
                                actions.push(PlayerAction::ExecutePoliticalDecree {
                                    country_id: pid.clone(),
                                    decree: PoliticalDecreeKind::NationalDialogue,
                                });
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
                                actions.push(PlayerAction::ExecutePoliticalDecree {
                                    country_id: pid.clone(),
                                    decree: PoliticalDecreeKind::PublicCampaign,
                                });
                            }
                        });
                    });
                });
            });
    }

    actions
}

/// Reparte exactamente `total_chairs` sillas proporcionalmente entre los partidos usando resto mayor.
fn allocate_parliament_chairs(parties: &[PoliticalParty], total_chairs: usize) -> Vec<usize> {
    if parties.is_empty() || total_chairs == 0 {
        return Vec::new();
    }
    let total_pct: f64 = parties.iter().map(|p| p.seats_pct.max(0.0)).sum();
    if total_pct <= 0.001 {
        let mut even = vec![total_chairs / parties.len(); parties.len()];
        for slot in even.iter_mut().take(total_chairs % parties.len()) {
            *slot += 1;
        }
        return even;
    }

    let mut counts = vec![0_usize; parties.len()];
    let mut remainders: Vec<(usize, f64)> = Vec::with_capacity(parties.len());
    let mut assigned = 0_usize;

    for (idx, p) in parties.iter().enumerate() {
        let exact = (p.seats_pct.max(0.0) / total_pct) * (total_chairs as f64);
        let floor_val = exact.floor() as usize;
        counts[idx] = floor_val;
        assigned += floor_val;
        remainders.push((idx, exact - (floor_val as f64)));
    }

    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut rem_idx = 0_usize;
    while assigned < total_chairs && !remainders.is_empty() {
        let party_idx = remainders[rem_idx % remainders.len()].0;
        counts[party_idx] += 1;
        assigned += 1;
        rem_idx += 1;
    }

    counts
}

/// Dibuja una mesa circular central rodeada de sillas individuales pintadas según cada partido.
fn draw_congress_round_table(
    ui: &mut Ui,
    size: Vec2,
    country: &CountryState,
    chair_counts: &[usize],
) -> Option<usize> {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if !ui.is_rect_visible(rect) {
        return None;
    }

    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::same(6), BG_WELL);
    p.rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, BORDER_BRASS_DIM),
        egui::StrokeKind::Inside,
    );

    let center = rect.center();
    let max_r = (rect.width().min(rect.height()) * 0.5) - 10.0;
    let table_r = (max_r * 0.42).max(26.0);
    let inner_ring_r = max_r * 0.65;
    let outer_ring_r = max_r * 0.89;

    // 1. Mesa circular central estilo sala de consejo
    p.circle_filled(center, table_r, Color32::from_rgb(36, 29, 24));
    p.circle_stroke(center, table_r, Stroke::new(2.0_f32, BORDER_BRASS));
    p.circle_stroke(
        center,
        (table_r - 6.0).max(12.0),
        Stroke::new(1.0_f32, BORDER_BRASS_DIM),
    );
    p.text(
        Pos2::new(center.x, center.y - 7.0),
        Align2::CENTER_CENTER,
        &country.id,
        FontId::proportional(13.0),
        ACCENT_GOLD,
    );
    p.text(
        Pos2::new(center.x, center.y + 8.0),
        Align2::CENTER_CENTER,
        "CONGRESO",
        FontId::proportional(8.5),
        TEXT_MUTED,
    );

    // 2. Construir la secuencia ordenada de sillas asignadas a cada partido
    let mut seat_owners: Vec<usize> = Vec::with_capacity(TOTAL_CONGRESS_CHAIRS);
    for (party_idx, &count) in chair_counts.iter().enumerate() {
        for _ in 0..count {
            seat_owners.push(party_idx);
        }
    }
    if seat_owners.is_empty() {
        return None;
    }

    // 3. Posicionar las sillas en 2 anillos concéntricos alrededor de la mesa circular
    let inner_slots = 20_usize.min(seat_owners.len());
    let outer_slots = seat_owners.len().saturating_sub(inner_slots);

    let mut positions: Vec<(Pos2, f32, usize)> = Vec::with_capacity(seat_owners.len());
    let start_angle = -std::f32::consts::FRAC_PI_2;

    for i in 0..inner_slots {
        let angle = start_angle + (i as f32 / inner_slots as f32) * std::f32::consts::TAU;
        let pos = Pos2::new(
            center.x + angle.cos() * inner_ring_r,
            center.y + angle.sin() * inner_ring_r,
        );
        positions.push((pos, angle, seat_owners[i]));
    }
    for j in 0..outer_slots {
        let angle = start_angle
            + ((j as f32 + 0.5) / outer_slots.max(1) as f32) * std::f32::consts::TAU;
        let pos = Pos2::new(
            center.x + angle.cos() * outer_ring_r,
            center.y + angle.sin() * outer_ring_r,
        );
        positions.push((pos, angle, seat_owners[inner_slots + j]));
    }

    let hover_pos = response.hover_pos();
    let mut hovered_party: Option<usize> = None;
    if let Some(hp) = hover_pos {
        let mut best_d = 14.0_f32;
        for (pos, _, party_idx) in &positions {
            let d = pos.distance(hp);
            if d < best_d {
                best_d = d;
                hovered_party = Some(*party_idx);
            }
        }
    }

    // 4. Pintar cada silla (asiento + respaldo hacia afuera mirando la mesa circular)
    for (pos, angle, party_idx) in positions {
        let Some(party) = country.political_parties.get(party_idx) else {
            continue;
        };
        let base_col = ideology_color(&party.ideology);
        let is_hovered = hovered_party == Some(party_idx);
        let radial = Vec2::new(angle.cos(), angle.sin());
        let tangent = Vec2::new(-angle.sin(), angle.cos());

        // Asiento mirando al centro de la mesa
        let seat_r = if is_hovered { 5.4 } else { 4.4 };
        p.circle_filled(pos, seat_r, base_col);

        let is_coalition = !party.is_ruling
            && country
                .coalition_parties
                .iter()
                .any(|ac| ac == &party.acronym);
        let rim_col = if is_hovered {
            Color32::WHITE
        } else if party.is_ruling {
            ACCENT_GOLD
        } else if is_coalition {
            ACCENT_EMERALD
        } else {
            Color32::from_rgb(16, 22, 30)
        };
        p.circle_stroke(
            pos,
            seat_r,
            Stroke::new(
                if is_hovered || party.is_ruling || is_coalition {
                    1.4_f32
                } else {
                    0.9_f32
                },
                rim_col,
            ),
        );

        // Respaldo de la silla en el lado exterior
        let back_center = pos + radial * (seat_r + 1.4);
        let back_a = back_center - tangent * 3.8;
        let back_b = back_center + tangent * 3.8;
        p.line_segment(
            [back_a, back_b],
            Stroke::new(2.2_f32, if is_hovered { ACCENT_GOLD } else { base_col }),
        );
    }

    hovered_party
}
