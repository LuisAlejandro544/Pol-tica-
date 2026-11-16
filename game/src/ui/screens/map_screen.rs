//! Propósito del archivo:
//! Pantalla principal del Mapa Estratégico (`WorldMap`).
//! Muestra en columna vertical izquierda las 4 capas del mapa y la crónica regional
//! deslizable de la IA local, a la derecha el visor 3D/SVG interactivo, y abre un
//! popup deslizable (`Popup País`) al tocar una nación extranjera.

use crate::map::{Latam3dScene, LatamVectorMap};
use crate::models::{GameState, MapMode, ProvinceData, ScreenRoute};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use crate::ui::theme::{
    format_population, format_usd_millions, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, Layout, RichText, Stroke, Ui};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct MapScreenResult {
    pub open_diplomacy_with: Option<String>,
}

pub fn render_map_screen(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    gpu_3d_failed: &Arc<AtomicBool>,
    vector_map: &LatamVectorMap,
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
    let left_w = (avail.x * 0.27).clamp(185.0, 245.0);
    let map_w = (avail.x - left_w - 14.0).max(240.0);
    let panel_h = (avail.y - 4.0).max(180.0);

    ui.horizontal(|ui| {
        // Panel izquierdo vertical explícito: Capas de mapa + Crónica deslizable
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, panel_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(8.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            ui.set_height((panel_h - 18.0).max(120.0));

                            ui.label(RichText::new("Capas del Mapa").strong().color(ACCENT_GOLD));
                            ui.add_space(4.0);
                            let btn_w = ui.available_width();
                            for (mode, title) in [
                                (MapMode::Political, "Político"),
                                (MapMode::GdpPerCapita, "PIB per cápita"),
                                (MapMode::Inflation, "Riesgo Inflacionario"),
                                (MapMode::Diplomacy, "Relaciones Diplomáticas"),
                            ] {
                                if ui
                                    .add_sized([btn_w, 28.0], egui::Button::selectable(*map_mode == mode, title))
                                    .clicked()
                                {
                                    *map_mode = mode;
                                }
                            }

                            ui.add_space(6.0);
                            ui.separator();
                            ui.label(
                                RichText::new("Crónica Geopolítica")
                                    .strong()
                                    .color(ACCENT_CYAN),
                            );
                            ui.add_space(4.0);

                            egui::ScrollArea::vertical()
                                .id_salt("map_chronicle_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for entry in &state.chronicle {
                                        egui::Frame::new()
                                            .fill(BG_CARD)
                                            .corner_radius(5.0)
                                            .inner_margin(6.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "[{} • {}]",
                                                        entry.date_label, entry.country_id
                                                    ))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(ACCENT_GOLD),
                                                );
                                                ui.label(
                                                    RichText::new(&entry.summary)
                                                        .size(12.0)
                                                        .color(TEXT_PRIMARY),
                                                );
                                            });
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
                    gpu_3d_failed,
                    vector_map,
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

    // Popup emergente deslizable para inspeccionar un país tocado en el mapa
    if *show_country_popup {
        if let Some(sel_id) = state.selected_country_id.clone() {
            if let Some(country) = state.countries.get(&sel_id).cloned() {
                let is_player = state.player_country_id.as_deref() == Some(sel_id.as_str());
                let mut open = true;
                egui::Window::new(
                    RichText::new(format!("{} [{}]", country.name, country.id))
                        .strong()
                        .color(ACCENT_GOLD),
                )
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::RIGHT_CENTER, [-16.0, 0.0])
                .default_width(300.0)
                .max_width(310.0)
                .frame(
                    egui::Frame::window(&ui.ctx().style())
                        .fill(BG_PANEL)
                        .stroke(Stroke::new(1.5_f32, BORDER_BRASS)),
                )
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("map_country_popup_scroll")
                        .max_height(250.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Capital: {}", country.capital))
                                    .color(TEXT_MUTED),
                            );
                            if let Some(prov_mesh) = &state.selected_province_mesh {
                                if let Some(prov) = provinces_catalog.get(prov_mesh) {
                                    if prov.country_id == country.id {
                                        ui.add_space(4.0);
                                        egui::Frame::new()
                                            .fill(BG_CARD)
                                            .corner_radius(5.0)
                                            .inner_margin(6.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Provincia: {} ({})",
                                                        prov.name, prov.id
                                                    ))
                                                    .strong()
                                                    .color(ACCENT_CYAN),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} • {} • Relieve {:.2}",
                                                        prov.admin_type,
                                                        prov.terrain_type,
                                                        prov.elevation_avg
                                                    ))
                                                    .size(11.5)
                                                    .color(TEXT_MUTED),
                                                );
                                            });
                                    }
                                }
                            }
                            ui.separator();
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

                            if !is_player {
                                if let Some(pid) = &state.player_country_id {
                                    let rel = state.get_relation(pid, &sel_id);
                                    ui.separator();
                                    ui.label(
                                        RichText::new(format!(
                                            "Opinión Bilateral: {:+.0}",
                                            rel.opinion
                                        ))
                                        .color(if rel.opinion >= 0.0 {
                                            ACCENT_EMERALD
                                        } else {
                                            ACCENT_CRIMSON
                                        }),
                                    );
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 34.0],
                                            egui::Button::new("Ir a Diplomacia >"),
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
