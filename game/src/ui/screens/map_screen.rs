//! Propósito del archivo:
//! Pantalla principal del Mapa Estratégico (`WorldMap`) estilo Victoria 2 / HOI4.
//! Columna izquierda (`Layout::top_down`): 4 botones físicos de capas cartográficas + crónica
//! geopolítica en tarjetas con marco. Derecha: visor 3D táctico (`wgpu`) y popup inspector
//! de país/provincia anclado fuera de la zona de navegación Android.

use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode, ProvinceData, ScreenRoute};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_section_header, format_population, format_usd_millions,
    game_button, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_PANEL, BORDER_BRASS, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, CornerRadius, Layout, RichText, Stroke, Ui, Vec2};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct MapScreenResult {
    pub open_diplomacy_with: Option<String>,
}

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
    };

    let avail = ui.available_size();
    let left_w = (avail.x * 0.27).clamp(185.0, 240.0);
    let map_w = (avail.x - left_w - 10.0).max(240.0);
    let panel_h = (avail.y - 2.0).max(180.0);

    ui.horizontal(|ui| {
        // Panel izquierdo vertical explícito: Botones reales de capas + Crónica deslizable
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, panel_h),
            Layout::top_down(Align::Min),
            |ui| {
                panel_frame(true).inner_margin(8.0).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height((panel_h - 18.0).max(120.0));

                        draw_section_header(ui, GameIcon::Map, "MODOS DE MAPA", ACCENT_GOLD);

                        let btn_w = ui.available_width();
                        for (mode, icon, title, accent) in [
                            (MapMode::Political, GameIcon::Crown, "Político", ACCENT_GOLD),
                            (
                                MapMode::GdpPerCapita,
                                GameIcon::Economy,
                                "PIB per cápita",
                                ACCENT_EMERALD,
                            ),
                            (
                                MapMode::Inflation,
                                GameIcon::Growth,
                                "Riesgo Inflación",
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
                                Vec2::new(btn_w, 32.0),
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

                        ui.add_space(6.0);
                        draw_section_header(
                            ui,
                            GameIcon::Dispatch,
                            "CRÓNICA REGIONAL",
                            ACCENT_CYAN,
                        );

                        egui::ScrollArea::vertical()
                            .id_salt("map_chronicle_scroll")
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
                                                .size(10.5)
                                                .strong()
                                                .color(ACCENT_GOLD),
                                            );
                                            ui.label(
                                                RichText::new(&entry.summary)
                                                    .size(11.5)
                                                    .color(TEXT_PRIMARY),
                                            );
                                        },
                                    );
                                    ui.add_space(4.0);
                                }
                            });
                    });
                });
            },
        );

        // Área derecha: Visor 3D interactivo de Latinoamérica
        ui.allocate_ui_with_layout(
            egui::vec2(map_w, panel_h),
            Layout::top_down(Align::Min),
            |ui| {
                if let Some(hit) = render_latam_map(
                    ui,
                    scene_3d,
                    target_format,
                    gpu_info,
                    gpu_3d_error,
                    state,
                    *map_mode,
                    viewport,
                ) {
                    state.selected_country_id = Some(hit.country_id);
                    state.selected_province_mesh = hit.province_mesh;
                    *show_country_popup = true;
                }
            },
        );
    });

    // Popup emergente de despacho diplomático al tocar un país/provincia en el mapa
    if *show_country_popup {
        if let Some(sel_id) = state.selected_country_id.clone() {
            if let Some(country) = state.countries.get(&sel_id).cloned() {
                let is_player = state.player_country_id.as_deref() == Some(sel_id.as_str());
                let mut open = true;
                egui::Window::new(
                    RichText::new(format!("{} [{}]", country.name.to_uppercase(), country.id))
                        .strong()
                        .color(ACCENT_GOLD),
                )
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_CENTER, [-48.0, 0.0])
                .default_width(295.0)
                .max_width(305.0)
                .frame(
                    egui::Frame::window(&ui.ctx().style())
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
                        .corner_radius(CornerRadius::same(4)),
                )
                .show(ui.ctx(), |ui| {
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
                                        ui.add_space(4.0);
                                        card_frame(Some(ACCENT_CYAN)).inner_margin(6.0).show(
                                            ui,
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Provincia: {} ({})",
                                                        prov.name, prov.id
                                                    ))
                                                    .strong()
                                                    .size(12.0)
                                                    .color(ACCENT_CYAN),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} • {} • Relieve {:.2}",
                                                        prov.admin_type,
                                                        prov.terrain_type,
                                                        prov.elevation_avg
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
                if !open {
                    *show_country_popup = false;
                }
            }
        }
    }

    let _ = ScreenRoute::WorldMap;
    result
}
