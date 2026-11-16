//! Propósito del archivo:
//! Pantalla principal del Mapa Estratégico a pantalla completa (`WorldMap`).
//! El visor 3D (`wgpu`) ocupa el 100% de la pantalla borde a borde, mientras los controles
//! tácticos (selector de capas cartográficas, botón desplegable de Crónica Regional,
//! tarjeta de unidad seleccionada y popup de despacho de país/estado) flotan encima
//! en paneles translúcidos de acero y latón.

use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode, ProvinceData, ScreenRoute};
use crate::ui::components::map_canvas::{render_latam_map, MapCanvasConfig, MapViewportState};
use crate::ui::theme::{
    card_frame, draw_corner_rivets, draw_country_shield, draw_gauge_bar, draw_section_header,
    floating_hud_frame, format_population, format_usd_millions, game_button, well_frame, GameIcon,
    ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_PANEL, BORDER_BRASS,
    BORDER_BRASS_DIM, SAFE_RIGHT_PAD, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, CornerRadius, Layout, Margin, Rect, RichText, Stroke, Ui, Vec2};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct MapScreenResult {
    pub open_diplomacy_with: Option<String>,
    pub order_unit_move: Option<(u64, String)>,
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
    state: &mut GameState,
    map_mode: &mut MapMode,
    viewport: &mut MapViewportState,
    show_country_popup: &mut bool,
) -> MapScreenResult {
    let mut result = MapScreenResult {
        open_diplomacy_with: None,
        order_unit_move: None,
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
            camera_bottom_offset: 60.0,
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

    // 2. Barra flotante superior izquierda: Modos de Mapa + botón desplegable de Crónica + titular en vivo
    let strip_max_w = (ctx.screen_rect().width() - 16.0 - (SAFE_RIGHT_PAD as f32)).max(300.0);
    let strip_resp = egui::Area::new(egui::Id::new("map_modes_floating_strip"))
        .anchor(egui::Align2::LEFT_TOP, [8.0, 62.0])
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            ui.set_max_width(strip_max_w);
            let frame_resp = floating_hud_frame(true)
                .inner_margin(Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().scroll.bar_width = 3.0;
                    egui::ScrollArea::horizontal()
                        .id_salt("map_modes_strip_scroll")
                        .auto_shrink([true, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for (mode, icon, title, accent) in [
                                    (MapMode::Political, GameIcon::Crown, "Político", ACCENT_GOLD),
                                    (
                                        MapMode::GdpPerCapita,
                                        GameIcon::Economy,
                                        "PIB",
                                        ACCENT_EMERALD,
                                    ),
                                    (
                                        MapMode::Inflation,
                                        GameIcon::Growth,
                                        "Inflación",
                                        ACCENT_CRIMSON,
                                    ),
                                    (
                                        MapMode::Diplomacy,
                                        GameIcon::Diplomacy,
                                        "Diplomacia",
                                        ACCENT_CYAN,
                                    ),
                                ] {
                                    let active = *map_mode == mode;
                                    if game_button(
                                        ui,
                                        Vec2::new(82.0, 26.0),
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

                                ui.separator();

                                let chron_lbl = if viewport.show_chronicle_drawer {
                                    "Ocultar"
                                } else {
                                    "Crónica"
                                };
                                if game_button(
                                    ui,
                                    Vec2::new(78.0, 26.0),
                                    chron_lbl,
                                    Some(GameIcon::Dispatch),
                                    viewport.show_chronicle_drawer,
                                    Some(ACCENT_CYAN),
                                )
                                .clicked()
                                {
                                    viewport.show_chronicle_drawer =
                                        !viewport.show_chronicle_drawer;
                                }

                                if !viewport.show_chronicle_drawer {
                                    if let Some(latest) = state.chronicle.first() {
                                        let preview: String =
                                            latest.summary.chars().take(42).collect();
                                        let suffix = if latest.summary.chars().count() > 42 {
                                            "..."
                                        } else {
                                            ""
                                        };
                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "[{}] {}{}",
                                                latest.country_id, preview, suffix
                                            ))
                                            .size(10.5)
                                            .color(TEXT_MUTED),
                                        );
                                    }
                                }
                            });
                        });
                });
            draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
        });
    result.overlay_rects.push(strip_resp.response.rect);

    // 3. Cajón desplegable flotante de Crónica Regional (solo visible cuando el jugador lo abre)
    if viewport.show_chronicle_drawer {
        let chron_resp = egui::Area::new(egui::Id::new("map_chronicle_floating_drawer"))
            .anchor(egui::Align2::LEFT_TOP, [8.0, 102.0])
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                ui.set_width(272.0);
                let frame_resp = floating_hud_frame(true).inner_margin(8.0).show(ui, |ui| {
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.horizontal(|ui| {
                            draw_section_header(
                                ui,
                                GameIcon::Dispatch,
                                "CRÓNICA REGIONAL",
                                ACCENT_CYAN,
                            );
                        });
                        egui::ScrollArea::vertical()
                            .id_salt("map_chronicle_scroll")
                            .max_height(210.0)
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

    // 4. Tarjeta flotante inferior izquierda para unidad militar seleccionada
    if let Some(uid) = state.selected_unit_id {
        if let Some(unit) = state.units.iter().find(|u| u.id == uid).cloned() {
            let is_mine = state.player_country_id.as_deref() == Some(unit.country_id.as_str());
            let unit_resp = egui::Area::new(egui::Id::new("map_selected_unit_floating"))
                .anchor(egui::Align2::LEFT_BOTTOM, [8.0, -58.0])
                .order(egui::Order::Foreground)
                .show(&ctx, |ui| {
                    ui.set_width(248.0);
                    let frame_resp = floating_hud_frame(true).inner_margin(8.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} ({})", unit.name, unit.troops))
                                    .strong()
                                    .size(12.0)
                                    .color(ACCENT_GOLD),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if game_button(
                                    ui,
                                    Vec2::new(24.0, 20.0),
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
                                    .size(11.0)
                                    .color(TEXT_PRIMARY),
                            );
                            draw_gauge_bar(
                                ui,
                                ui.available_width(),
                                12.0,
                                unit.march_progress,
                                ACCENT_CYAN,
                                &format!("{:.0}%", unit.march_progress * 100.0),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("Estacionada en {cur_name}"))
                                    .size(11.0)
                                    .color(TEXT_PRIMARY),
                            );
                        }

                        if is_mine {
                            ui.label(
                                RichText::new("Toca un estado en el mapa para mover")
                                    .size(10.0)
                                    .color(ACCENT_CYAN),
                            );
                        }
                    });
                    draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
                });
            result.overlay_rects.push(unit_resp.response.rect);
        }
    }

    // 5. Popup emergente de despacho diplomático al tocar un país/provincia en el mapa
    if *show_country_popup {
        if let Some(sel_id) = state.selected_country_id.clone() {
            if let Some(country) = state.countries.get(&sel_id).cloned() {
                let is_player = state.player_country_id.as_deref() == Some(sel_id.as_str());
                let mut open = true;
                let win_resp = egui::Window::new(
                    RichText::new(format!("{} [{}]", country.name.to_uppercase(), country.id))
                        .strong()
                        .color(ACCENT_GOLD),
                )
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_CENTER, [-((SAFE_RIGHT_PAD as f32) + 8.0), 0.0])
                .default_width(295.0)
                .max_width(305.0)
                .frame(
                    egui::Frame::window(&ctx.style())
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
                        .corner_radius(CornerRadius::same(5)),
                )
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("map_country_popup_scroll")
                        .max_height(255.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                draw_country_shield(ui, Vec2::new(36.0, 30.0), &country);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    ui.label(
                                        RichText::new(format!("Capital: {}", country.capital))
                                            .size(12.0)
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(&country.region)
                                            .size(11.0)
                                            .color(TEXT_MUTED),
                                    );
                                });
                            });

                            if let Some(prov_mesh) = &state.selected_province_mesh {
                                if let Some(prov) = provinces_catalog.get(prov_mesh) {
                                    if prov.country_id == country.id {
                                        let stationed: u64 = state
                                            .units
                                            .iter()
                                            .filter(|u| u.current_province == *prov_mesh)
                                            .map(|u| u.troops)
                                            .sum();
                                        ui.add_space(4.0);
                                        card_frame(Some(ACCENT_CYAN)).inner_margin(6.0).show(
                                            ui,
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Estado: {} ({})",
                                                        prov.name, prov.id
                                                    ))
                                                    .strong()
                                                    .size(12.0)
                                                    .color(ACCENT_CYAN),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} • {} • Tropas aquí: {}",
                                                        prov.admin_type,
                                                        prov.terrain_type,
                                                        stationed
                                                    ))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                                );
                                            },
                                        );
                                    }
                                }
                            }

                            ui.add_space(4.0);
                            well_frame().show(ui, |ui| {
                                ui.label(format!(
                                    "Población: {}",
                                    format_population(country.population)
                                ));
                                ui.label(format!(
                                    "Moneda: {} ({:.2}/USD)",
                                    country.economy.currency_code, country.economy.exchange_rate_usd
                                ));
                                ui.label(
                                    RichText::new(format!(
                                        "PIB: {} ({:+.1}%)",
                                        format_usd_millions(country.economy.gdp_usd_millions),
                                        country.economy.gdp_growth_pct
                                    ))
                                    .color(ACCENT_EMERALD),
                                );
                                ui.label(format!("Inflación: {:.1}%", country.economy.inflation_pct));
                                ui.label(format!(
                                    "Desempleo: {:.1}%",
                                    country.economy.unemployment_pct
                                ));
                                if let Some(ruling) = country.ruling_party() {
                                    ui.label(format!(
                                        "Gobierno: {} ({})",
                                        ruling.acronym, ruling.ideology
                                    ));
                                }
                                ui.label(format!(
                                    "Fuerzas Activas: {} efectivos",
                                    country.military.active_troops
                                ));
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
                                        .color(if rel.opinion >= 0.0 {
                                            ACCENT_EMERALD
                                        } else {
                                            ACCENT_CRIMSON
                                        }),
                                    );
                                    ui.add_space(4.0);
                                    if game_button(
                                        ui,
                                        Vec2::new(ui.available_width(), 36.0),
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
