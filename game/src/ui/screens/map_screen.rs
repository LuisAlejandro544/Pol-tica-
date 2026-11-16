//! Propósito del archivo:
//! Pantalla principal del Mapa Estratégico a pantalla completa (`WorldMap`).
//! El visor 3D (`wgpu`) ocupa el 100% de la pantalla borde a borde, mientras los controles
//! tácticos (selector de capas cartográficas, botón desplegable de Crónica Regional,
//! tarjeta de unidad seleccionada y popup de despacho de país/estado) flotan encima
//! en paneles translúcidos de acero y latón.

use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode, ProvinceData, ProvinceGraph, ScreenRoute};
use crate::ui::components::map_canvas::{render_latam_map, MapCanvasConfig, MapViewportState};
use crate::ui::theme::{
    card_frame, draw_corner_rivets, draw_country_shield, draw_gauge_bar, draw_section_header,
    floating_hud_frame, format_exact_count, format_population, format_usd_millions, game_button,
    step_button, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD,
    BG_PANEL, BORDER_BRASS, BORDER_BRASS_DIM, SAFE_RIGHT_PAD, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, CornerRadius, Layout, Margin, Rect, RichText, Stroke, Ui, Vec2};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct MapScreenResult {
    pub open_diplomacy_with: Option<String>,
    pub order_unit_move: Option<(u64, String)>,
    pub recruit_in_province: Option<(String, u64, f64)>,
    pub overlay_rects: Vec<Rect>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_map_screen(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    gpu_info: &str,
    gpu_3d_error: &Arc<Mutex<Option<String>>>,
    provinces_catalog: &BTreeMap<String, ProvinceData>,
    province_graph: &ProvinceGraph,
    state: &mut GameState,
    map_mode: &mut MapMode,
    viewport: &mut MapViewportState,
    show_country_popup: &mut bool,
    recruit_troops_amount: &mut u64,
) -> MapScreenResult {
    let mut result = MapScreenResult {
        open_diplomacy_with: None,
        order_unit_move: None,
        recruit_in_province: None,
        overlay_rects: Vec::new(),
    };

    // 1. Lienzo 3D al 100% de pantalla completa (borde a borde)
    let hit_opt = render_latam_map(
        ui,
        scene_3d,
        target_format,
        gpu_info,
        gpu_3d_error,
        state,
        *map_mode,
        viewport,
        MapCanvasConfig {
            fullscreen_borderless: true,
            show_mode_badge: false,
            camera_bottom_offset: 48.0,
            camera_right_inset: (SAFE_RIGHT_PAD as f32) + 8.0,
        },
    );

    if let Some(hit) = hit_opt {
        if let Some(uid) = hit.unit_id {
            state.selected_unit_id = Some(uid);
            state.selected_country_id = Some(hit.country_id);
        } else {
            let mut moved_unit = false;
            if let (Some(uid), Some(prov_mesh), Some(pid)) = (
                state.selected_unit_id,
                hit.province_mesh.as_ref(),
                state.player_country_id.as_ref(),
            ) {
                let owns_unit = state
                    .units
                    .iter()
                    .any(|u| u.id == uid && u.country_id == *pid);
                let rel = state.get_relation(pid, &hit.country_id);
                let can_enter = hit.country_id == *pid || rel.alliance || rel.at_war;
                if owns_unit && can_enter {
                    result.order_unit_move = Some((uid, prov_mesh.clone()));
                    moved_unit = true;
                }
            }

            state.selected_country_id = Some(hit.country_id);
            state.selected_province_mesh = hit.province_mesh;
            if !moved_unit {
                *show_country_popup = true;
            }
        }
    }

    let ctx = ui.ctx().clone();
    let screen_h = ctx.screen_rect().height();

    // 2. Carril vertical compacto en el borde izquierdo: deja todo el centro y derecha libres
    let rail_resp = egui::Area::new(egui::Id::new("map_modes_floating_rail"))
        .anchor(egui::Align2::LEFT_TOP, [6.0, 48.0])
        .order(egui::Order::Middle)
        .show(&ctx, |ui| {
            let frame_resp = floating_hud_frame(true)
                .inner_margin(Margin::symmetric(5, 5))
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for (mode, icon, title, accent) in [
                            (MapMode::Political, GameIcon::Crown, "Político", ACCENT_GOLD),
                            (MapMode::GdpPerCapita, GameIcon::Economy, "PIB", ACCENT_EMERALD),
                            (MapMode::Inflation, GameIcon::Growth, "Inflac.", ACCENT_CRIMSON),
                            (MapMode::Diplomacy, GameIcon::Diplomacy, "Diplo.", ACCENT_CYAN),
                        ] {
                            let active = *map_mode == mode;
                            if game_button(
                                ui,
                                Vec2::new(78.0, 24.0),
                                title,
                                Some(icon),
                                active,
                                Some(accent),
                            )
                            .clicked()
                            {
                                *map_mode = mode;
                            }
                        }

                        ui.add_space(1.0);
                        let chron_lbl = if viewport.show_chronicle_drawer {
                            "Cerrar"
                        } else {
                            "Crónica"
                        };
                        if game_button(
                            ui,
                            Vec2::new(78.0, 24.0),
                            chron_lbl,
                            Some(GameIcon::Dispatch),
                            viewport.show_chronicle_drawer,
                            Some(ACCENT_CYAN),
                        )
                        .clicked()
                        {
                            viewport.show_chronicle_drawer = !viewport.show_chronicle_drawer;
                        }
                    });
                });
            draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
        });
    result.overlay_rects.push(rail_resp.response.rect);

    // 3. Cajón desplegable flotante de Crónica Regional junto al carril izquierdo
    if viewport.show_chronicle_drawer {
        let chron_max_h = (screen_h - 140.0).clamp(115.0, 210.0);
        let chron_resp = egui::Area::new(egui::Id::new("map_chronicle_floating_drawer"))
            .anchor(egui::Align2::LEFT_TOP, [100.0, 48.0])
            .order(egui::Order::Middle)
            .show(&ctx, |ui| {
                ui.set_width(256.0);
                let frame_resp = floating_hud_frame(true).inner_margin(8.0).show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Dispatch,
                            "CRÓNICA REGIONAL",
                            ACCENT_CYAN,
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("map_chronicle_scroll")
                            .max_height(chron_max_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for entry in &state.chronicle {
                                    card_frame(Some(BORDER_BRASS_DIM)).inner_margin(6.0).show(
                                        ui,
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "[{} • {}]",
                                                    entry.date_label, entry.country_id
                                                ))
                                                .size(10.0)
                                                .strong()
                                                .color(ACCENT_GOLD),
                                            );
                                            ui.label(
                                                RichText::new(&entry.summary)
                                                    .size(11.0)
                                                    .color(TEXT_PRIMARY),
                                            );
                                        },
                                    );
                                    ui.add_space(3.0);
                                }
                            });
                    });
                });
                draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
            });
        result.overlay_rects.push(chron_resp.response.rect);
    }

    // 4. Tarjeta flotante inferior izquierda (a la derecha del carril) para unidad militar seleccionada
    if let Some(uid) = state.selected_unit_id {
        if let Some(unit) = state.units.iter().find(|u| u.id == uid).cloned() {
            let is_mine = state.player_country_id.as_deref() == Some(unit.country_id.as_str());
            let unit_resp = egui::Area::new(egui::Id::new("map_selected_unit_floating"))
                .anchor(egui::Align2::LEFT_BOTTOM, [100.0, -48.0])
                .order(egui::Order::Middle)
                .show(&ctx, |ui| {
                    ui.set_width(238.0);
                    let frame_resp = floating_hud_frame(true).inner_margin(7.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} ({})", unit.name, unit.troops))
                                    .strong()
                                    .size(11.5)
                                    .color(ACCENT_GOLD),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if game_button(
                                    ui,
                                    Vec2::new(22.0, 19.0),
                                    "X",
                                    None,
                                    false,
                                    Some(ACCENT_CRIMSON),
                                )
                                .clicked()
                                {
                                    state.selected_unit_id = None;
                                }
                            });
                        });

                        let cur_name = provinces_catalog
                            .get(&unit.current_province)
                            .map(|p| p.name.as_str())
                            .unwrap_or(&unit.current_province);

                        if let Some(tgt) = &unit.target_province {
                            let tgt_name = provinces_catalog
                                .get(tgt)
                                .map(|p| p.name.as_str())
                                .unwrap_or(tgt);
                            ui.label(
                                RichText::new(format!("Marcha -> {tgt_name}"))
                                    .size(10.5)
                                    .color(TEXT_PRIMARY),
                            );
                            draw_gauge_bar(
                                ui,
                                ui.available_width(),
                                11.0,
                                unit.march_progress,
                                ACCENT_CYAN,
                                &format!("{:.0}%", unit.march_progress * 100.0),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("En {cur_name}"))
                                    .size(10.5)
                                    .color(TEXT_PRIMARY),
                            );
                        }

                        if is_mine {
                            ui.label(
                                RichText::new("Toca un estado para mover")
                                    .size(9.5)
                                    .color(ACCENT_CYAN),
                            );
                        }
                    });
                    draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
                });
            result.overlay_rects.push(unit_resp.response.rect);
        }
    }

    // 5. Popup emergente de país/provincia: en Order::Foreground, compacto y libre de HUDs/cámara
    if *show_country_popup {
        if let Some(sel_id) = state.selected_country_id.clone() {
            if let Some(country) = state.countries.get(&sel_id).cloned() {
                let is_player = state.player_country_id.as_deref() == Some(sel_id.as_str());
                let popup_scroll_h = (screen_h - 165.0).clamp(130.0, 215.0);
                let mut open = true;
                let win_resp = egui::Window::new(
                    RichText::new(format!("{} [{}]", country.name.to_uppercase(), country.id))
                        .strong()
                        .color(ACCENT_GOLD),
                )
                .order(egui::Order::Foreground)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_TOP, [-((SAFE_RIGHT_PAD as f32) + 8.0), 48.0])
                .default_width(282.0)
                .max_width(290.0)
                .frame(
                    egui::Frame::window(&ctx.style())
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
                        .corner_radius(CornerRadius::same(5)),
                )
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("map_country_popup_scroll")
                        .max_height(popup_scroll_h)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                draw_country_shield(ui, Vec2::new(34.0, 27.0), &country);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    ui.label(
                                        RichText::new(format!("Capital: {}", country.capital))
                                            .size(11.5)
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(&country.region)
                                            .size(10.5)
                                            .color(TEXT_MUTED),
                                    );
                                });
                            });

                            if let Some(prov_mesh) = &state.selected_province_mesh {
                                let prov_meta = provinces_catalog
                                    .get(prov_mesh)
                                    .filter(|p| p.country_id == country.id)
                                    .map(|p| {
                                        (
                                            p.name.clone(),
                                            p.id.clone(),
                                            p.admin_type.clone(),
                                            p.terrain_type.clone(),
                                        )
                                    })
                                    .or_else(|| {
                                        province_graph
                                            .get(prov_mesh)
                                            .filter(|p| p.country_id == country.id)
                                            .map(|p| {
                                                (
                                                    p.name.clone(),
                                                    country.id.clone(),
                                                    "Región".to_string(),
                                                    p.terrain_type.clone(),
                                                )
                                            })
                                    });

                                if let Some((prov_name, prov_id, admin_type, terrain_type)) =
                                    prov_meta
                                {
                                    let stationed: u64 = state
                                        .units
                                        .iter()
                                        .filter(|u| u.current_province == *prov_mesh)
                                        .map(|u| u.troops)
                                        .sum();
                                    let region_opt = state.regions.get(prov_mesh).cloned();
                                    ui.add_space(4.0);
                                    card_frame(Some(ACCENT_CYAN)).inner_margin(6.0).show(
                                        ui,
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Región: {} ({})",
                                                    prov_name, prov_id
                                                ))
                                                .strong()
                                                .size(11.5)
                                                .color(ACCENT_CYAN),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • {} • Tropas aquí: {}",
                                                    admin_type, terrain_type, stationed
                                                ))
                                                .size(10.5)
                                                .color(TEXT_MUTED),
                                            );

                                            if let Some(reg) = region_opt.as_ref() {
                                                let pop_share =
                                                    reg.population_share_pct(country.population);
                                                let unemp_rate = reg.unemployment_rate_pct();
                                                let unemp_pop_pct =
                                                    reg.unemployed_of_population_pct();
                                                let reg_cap = reg.max_recruitable();
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Población: {} ({:.1}% del país)",
                                                        format_exact_count(reg.population),
                                                        pop_share
                                                    ))
                                                    .size(10.5)
                                                    .color(TEXT_PRIMARY),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Desempleados: {} ({:.1}% tasa | {:.1}% pob.)",
                                                        format_exact_count(reg.unemployed),
                                                        unemp_rate,
                                                        unemp_pop_pct
                                                    ))
                                                    .size(10.5)
                                                    .color(if reg.unemployed > 0 {
                                                        ACCENT_GOLD
                                                    } else {
                                                        ACCENT_CRIMSON
                                                    }),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Cupo reclutable: {} soldados",
                                                        format_exact_count(reg_cap)
                                                    ))
                                                    .size(10.0)
                                                    .color(if reg_cap > 0 {
                                                        ACCENT_EMERALD
                                                    } else {
                                                        ACCENT_CRIMSON
                                                    }),
                                                );
                                            }

                                            if is_player {
                                                ui.add_space(4.0);
                                                ui.label(
                                                    RichText::new("Reclutar tropas aquí:")
                                                        .strong()
                                                        .size(10.5)
                                                        .color(ACCENT_GOLD),
                                                );

                                                let max_by_region = region_opt
                                                    .as_ref()
                                                    .map(|r| r.max_recruitable())
                                                    .unwrap_or(0);
                                                let max_by_treasury =
                                                    ((country.economy.treasury_usd_millions / 70.0)
                                                        * 1_000.0)
                                                        .floor() as u64;
                                                let effective_max =
                                                    max_by_region.min(max_by_treasury);

                                                if max_by_region == 0 {
                                                    ui.label(
                                                        RichText::new(
                                                            "Región sin desempleados o sin cupo demográfico disponible.",
                                                        )
                                                        .size(10.0)
                                                        .color(ACCENT_CRIMSON),
                                                    );
                                                } else if max_by_treasury == 0 {
                                                    ui.label(
                                                        RichText::new(
                                                            "Tesorería insuficiente para reclutar.",
                                                        )
                                                        .size(10.0)
                                                        .color(ACCENT_CRIMSON),
                                                    );
                                                } else {
                                                    *recruit_troops_amount =
                                                        (*recruit_troops_amount)
                                                            .clamp(1, effective_max);
                                                    ui.horizontal(|ui| {
                                                        ui.add(
                                                            egui::DragValue::new(
                                                                recruit_troops_amount,
                                                            )
                                                            .range(1..=effective_max)
                                                            .speed(50.0),
                                                        );
                                                        let slider_w =
                                                            (ui.available_width() - 6.0).max(80.0);
                                                        ui.add_sized(
                                                            [slider_w, 20.0],
                                                            egui::Slider::new(
                                                                recruit_troops_amount,
                                                                1..=effective_max,
                                                            )
                                                            .show_value(false),
                                                        );
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 3.0;
                                                        let bw = ((ui.available_width() - 12.0)
                                                            / 5.0)
                                                            .max(36.0);
                                                        for (lbl, delta) in [
                                                            ("-1K", -1_000_i64),
                                                            ("-250", -250_i64),
                                                            ("+250", 250_i64),
                                                            ("+1K", 1_000_i64),
                                                        ] {
                                                            if step_button(
                                                                ui,
                                                                Vec2::new(bw, 22.0),
                                                                lbl,
                                                                None,
                                                            )
                                                            .clicked()
                                                            {
                                                                let next = ((*recruit_troops_amount
                                                                    as i64)
                                                                    + delta)
                                                                    .clamp(1, effective_max as i64)
                                                                    as u64;
                                                                *recruit_troops_amount = next;
                                                            }
                                                        }
                                                        if step_button(
                                                            ui,
                                                            Vec2::new(bw, 22.0),
                                                            "Máx",
                                                            None,
                                                        )
                                                        .clicked()
                                                        {
                                                            *recruit_troops_amount = effective_max;
                                                        }
                                                    });

                                                    let chosen = *recruit_troops_amount;
                                                    let cost_m = (chosen as f64 / 1_000.0) * 70.0;
                                                    let upkeep_m = (chosen as f64 / 1_000.0) * 1.35;
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Costo: ${cost_m:.1}M | Mant.: -${upkeep_m:.2}M/mes"
                                                        ))
                                                        .size(9.5)
                                                        .color(TEXT_MUTED),
                                                    );
                                                    let btn_lbl = format!(
                                                        "Reclutar {} de desempleados",
                                                        format_exact_count(chosen)
                                                    );
                                                    if game_button(
                                                        ui,
                                                        Vec2::new(ui.available_width(), 26.0),
                                                        &btn_lbl,
                                                        Some(GameIcon::Population),
                                                        true,
                                                        Some(ACCENT_GOLD),
                                                    )
                                                    .clicked()
                                                    {
                                                        result.recruit_in_province = Some((
                                                            prov_mesh.clone(),
                                                            chosen,
                                                            cost_m,
                                                        ));
                                                    }
                                                }
                                            }
                                        },
                                    );
                                }
                            } else if is_player {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "Toca una de tus regiones en el mapa para reclutar tropas directamente allí.",
                                    )
                                    .size(10.5)
                                    .color(ACCENT_CYAN),
                                );
                            }

                            ui.add_space(4.0);
                            well_frame().inner_margin(Margin::symmetric(6, 4)).show(ui, |ui| {
                                egui::Grid::new("country_popup_stats_grid")
                                    .num_columns(2)
                                    .spacing([12.0, 3.0])
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "Pob: {}",
                                                format_population(country.population)
                                            ))
                                            .size(11.0),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{}: {:.2}/USD",
                                                country.economy.currency_code,
                                                country.economy.exchange_rate_usd
                                            ))
                                            .size(11.0),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            RichText::new(format!(
                                                "PIB: {} ({:+.1}%)",
                                                format_usd_millions(country.economy.gdp_usd_millions),
                                                country.economy.gdp_growth_pct
                                            ))
                                            .size(11.0)
                                            .color(ACCENT_EMERALD),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "Inflac: {:.1}%",
                                                country.economy.inflation_pct
                                            ))
                                            .size(11.0),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            RichText::new(format!(
                                                "Desemp: {:.1}%",
                                                country.economy.unemployment_pct
                                            ))
                                            .size(11.0),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "Tropas: {}",
                                                country.military.active_troops
                                            ))
                                            .size(11.0),
                                        );
                                        ui.end_row();
                                    });

                                if let Some(ruling) = country.ruling_party() {
                                    ui.add_space(2.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "Gobierno: {} ({})",
                                            ruling.acronym, ruling.ideology
                                        ))
                                        .size(10.5)
                                        .color(TEXT_MUTED),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "Presidente: {} ({} años, {})",
                                            ruling.leader.full_name,
                                            ruling.leader.age,
                                            ruling.leader.nationality
                                        ))
                                        .size(10.5)
                                        .color(ACCENT_CYAN),
                                    );
                                }
                            });

                            if !is_player {
                                if let Some(pid) = &state.player_country_id {
                                    let rel = state.get_relation(pid, &sel_id);
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "Opinión Bilateral: {:+.0}",
                                            rel.opinion
                                        ))
                                        .strong()
                                        .size(11.5)
                                        .color(if rel.opinion >= 0.0 {
                                            ACCENT_EMERALD
                                        } else {
                                            ACCENT_CRIMSON
                                        }),
                                    );
                                    ui.add_space(4.0);
                                    if game_button(
                                        ui,
                                        Vec2::new(ui.available_width(), 32.0),
                                        "ABRIR EMBAJADA >",
                                        Some(GameIcon::Diplomacy),
                                        true,
                                        Some(ACCENT_GOLD),
                                    )
                                    .clicked()
                                    {
                                        result.open_diplomacy_with = Some(sel_id.clone());
                                        *show_country_popup = false;
                                    }
                                }
                            }
                        });
                });
                if let Some(wr) = win_resp {
                    result.overlay_rects.push(wr.response.rect);
                }
                if !open {
                    *show_country_popup = false;
                }
            }
        }
    }

    let _ = ScreenRoute::WorldMap;
    result
}
