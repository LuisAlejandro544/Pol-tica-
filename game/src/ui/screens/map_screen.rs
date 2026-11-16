//! Propósito del archivo:
//! Pantalla principal del Mapa Estratégico (`WorldMap`).
//! Muestra el mapa SVG interactivo de Latinoamérica con 4 modos de mapa,
//! la crónica regional de decisiones de la IA local, y abre un popup emergente
//! (`Popup País`) al tocar una nación extranjera sin saturar la pantalla principal.

use crate::map::{Latam3dScene, LatamVectorMap};
use crate::models::{GameState, MapMode, ProvinceData, ScreenRoute};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use crate::ui::theme::{
    format_population, format_usd_millions, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{RichText, Stroke, Ui};
use std::collections::BTreeMap;

pub struct MapScreenResult {
    pub open_diplomacy_with: Option<String>,
}

pub fn render_map_screen(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
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
    ui.horizontal(|ui| {
        // Panel izquierdo compacto: Selector de modos de mapa y Crónica de la IA local
        egui::Frame::new()
            .fill(BG_PANEL)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(8.0)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_width(240.0);
                ui.set_height(avail.y - 14.0);

                ui.label(RichText::new("Capas del Mapa").strong().color(ACCENT_GOLD));
                ui.add_space(4.0);
                for (mode, title) in [
                    (MapMode::Political, "Político"),
                    (MapMode::GdpPerCapita, "PIB per cápita"),
                    (MapMode::Inflation, "Riesgo Inflacionario"),
                    (MapMode::Diplomacy, "Relaciones Diplomáticas"),
                ] {
                    if ui
                        .add_sized([220.0, 30.0], egui::Button::selectable(*map_mode == mode, title))
                        .clicked()
                    {
                        *map_mode = mode;
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.label(RichText::new("Crónica Geopolítica (IA Local)").strong().color(ACCENT_CYAN));
                ui.add_space(4.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for entry in &state.chronicle {
                        egui::Frame::new()
                            .fill(BG_CARD)
                            .corner_radius(5.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("[{} • {}]", entry.date_label, entry.country_id))
                                        .size(11.0)
                                        .strong()
                                        .color(ACCENT_GOLD),
                                );
                                ui.label(RichText::new(&entry.summary).size(12.0).color(TEXT_PRIMARY));
                            });
                        ui.add_space(4.0);
                    }
                });
            });

        // Área derecha: Visor 3D interactivo de Latinoamérica
        let map_w = (avail.x - 260.0).max(320.0);
        ui.allocate_ui_with_layout(
            egui::vec2(map_w, avail.y - 4.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if let Some(hit) = render_latam_map(
                    ui,
                    scene_3d,
                    target_format,
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

    // Popup emergente para inspeccionar un país tocado en el mapa
    if *show_country_popup {
        if let Some(sel_id) = state.selected_country_id.clone() {
            if let Some(country) = state.countries.get(&sel_id).cloned() {
                let is_player = state.player_country_id.as_deref() == Some(sel_id.as_str());
                let mut open = true;
                egui::Window::new(RichText::new(format!("{} [{}]", country.name, country.id)).strong().color(ACCENT_GOLD))
                    .open(&mut open)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::RIGHT_CENTER, [-18.0, 0.0])
                    .default_width(310.0)
                    .max_width(310.0)
                    .frame(
                        egui::Frame::window(&ui.ctx().style())
                            .fill(BG_PANEL)
                            .stroke(Stroke::new(1.5_f32, BORDER_BRASS)),
                    )
                    .show(ui.ctx(), |ui| {
                        ui.label(RichText::new(format!("Capital: {}", country.capital)).color(TEXT_MUTED));
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
                                                RichText::new(format!("Provincia: {} ({})", prov.name, prov.id))
                                                    .strong()
                                                    .color(ACCENT_CYAN),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • {} • Relieve {:.2}",
                                                    prov.admin_type, prov.terrain_type, prov.elevation_avg
                                                ))
                                                .size(11.5)
                                                .color(TEXT_MUTED),
                                            );
                                        });
                                }
                            }
                        }
                        ui.separator();
                        ui.label(format!("Población: {}", format_population(country.population)));
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
                            ui.label(format!("Gobierno: {} ({})", ruling.acronym, ruling.ideology));
                        }
                        ui.label(format!("Fuerzas Activas: {} efectivos", country.military.active_troops));

                        if !is_player {
                            if let Some(pid) = &state.player_country_id {
                                let rel = state.get_relation(pid, &sel_id);
                                ui.separator();
                                ui.label(
                                    RichText::new(format!("Opinión Bilateral: {:+.0}", rel.opinion))
                                        .color(if rel.opinion >= 0.0 { ACCENT_EMERALD } else { ACCENT_CRIMSON }),
                                );
                                if ui.button("Ir a Diplomacia con esta Nación >").clicked() {
                                    result.open_diplomacy_with = Some(sel_id.clone());
                                    *show_country_popup = false;
                                }
                            }
                        }
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
