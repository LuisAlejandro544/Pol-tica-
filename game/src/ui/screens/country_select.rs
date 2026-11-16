//! Propósito del archivo:
//! Pantalla independiente de Selección de País (`CountrySelect`).
//! Divide la pantalla horizontal en:
//! - Izquierda/Centro: Mapa SVG interactivo de Latinoamérica + lista rápida por región.
//! - Derecha: Ficha detallada de la nación seleccionada (Población, Moneda, PIB, Partidos Políticos)
//!   y botón principal para asumir el liderazgo e iniciar la partida.

use crate::map::{Latam3dScene, LatamVectorMap};
use crate::models::{GameState, MapMode};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use eframe::egui_wgpu::wgpu;
use crate::ui::theme::{
    country_base_color, format_population, format_usd_millions, ideology_color, ACCENT_CYAN,
    ACCENT_EMERALD, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY,
};
use egui::{Color32, RichText, Stroke, Ui};

pub enum CountrySelectAction {
    None,
    BackToMenu,
    ConfirmCountry(String),
}

pub fn render_country_select(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    vector_map: &LatamVectorMap,
    state: &mut GameState,
    viewport: &mut MapViewportState,
) -> CountrySelectAction {
    let mut action = CountrySelectAction::None;

    // Barra superior de la pantalla de selección
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("< Menú Principal").clicked() {
                    action = CountrySelectAction::BackToMenu;
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new("SELECCIÓN DE NACIÓN — LATINOAMÉRICA")
                        .strong()
                        .size(17.0)
                        .color(ACCENT_GOLD),
                );
                ui.label(
                    RichText::new("Toca un país en el mapa 3D o en el listado para revisar su dossier")
                        .size(13.0)
                        .color(TEXT_MUTED),
                );
            });
        });

    ui.add_space(6.0);

    // Distribución horizontal en 3 columnas proporcionales (Lista rápida | Mapa SVG | Dossier Nacional)
    let avail = ui.available_size();
    ui.horizontal(|ui| {
        // 1. Lista rápida de países (190 px)
        egui::Frame::new()
            .fill(BG_PANEL)
            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
            .corner_radius(8.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_width(185.0);
                ui.set_height(avail.y - 12.0);
                ui.label(RichText::new("21 Naciones").strong().color(ACCENT_GOLD));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (id, country) in &state.countries {
                        let is_sel = state.selected_country_id.as_deref() == Some(id.as_str());
                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(
                                egui::vec2(10.0, 10.0),
                                egui::Sense::hover(),
                            );
                            ui.painter()
                                .circle_filled(dot_rect.center(), 5.0, country_base_color(country));
                            if ui.selectable_label(is_sel, &country.name).clicked() {
                                state.selected_country_id = Some(id.clone());
                                state.selected_province_mesh = None;
                                if let Some(scene) = scene_3d {
                                    if let Some(center) = scene.country_center(id) {
                                        viewport.camera_3d.focus_point(center, 30.0);
                                    }
                                }
                            }
                        });
                    }
                });
            });

        // 2. Visor interactivo 3D (o respaldo SVG)
        let map_w = (avail.x - 185.0 - 380.0 - 28.0).max(260.0);
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
                    MapMode::Political,
                    viewport,
                ) {
                    state.selected_country_id = Some(hit.country_id);
                    state.selected_province_mesh = hit.province_mesh;
                }
            },
        );

        // 3. Dossier completo del país seleccionado (365 px)
        egui::Frame::new()
            .fill(BG_PANEL)
            .stroke(Stroke::new(1.5, BORDER_BRASS))
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_width(365.0);
                ui.set_height(avail.y - 20.0);

                let Some(sel_id) = state.selected_country_id.clone() else {
                    ui.label("Selecciona un país en el mapa.");
                    return;
                };
                let Some(country) = state.countries.get(&sel_id) else {
                    return;
                };

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} ({})", country.name, country.id))
                                .size(22.0)
                                .strong()
                                .color(ACCENT_GOLD),
                        );
                    });
                    ui.label(
                        RichText::new(format!("Capital: {} • Región: {}", country.capital, country.region))
                            .size(13.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(8.0);

                    // Bloque: Población, Moneda y PIB
                    egui::Frame::new()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(6.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Indicadores Demográficos y Económicos").strong().color(ACCENT_CYAN));
                            ui.add_space(4.0);
                            egui::Grid::new("select_stats_grid").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                                ui.label("Población:");
                                ui.label(RichText::new(format!(
                                    "{} ({:+.2}%/año)",
                                    format_population(country.population),
                                    country.pop_growth_pct
                                )).strong());
                                ui.end_row();

                                ui.label("Moneda Nacional:");
                                ui.label(RichText::new(format!(
                                    "{} ({})",
                                    country.economy.currency_name, country.economy.currency_code
                                )).strong());
                                ui.end_row();

                                ui.label("Tipo de Cambio:");
                                ui.label(RichText::new(format!(
                                    "{:.2} {} / 1 USD",
                                    country.economy.exchange_rate_usd, country.economy.currency_code
                                )).strong());
                                ui.end_row();

                                ui.label("PIB Nominal:");
                                ui.label(RichText::new(format!(
                                    "{} ({:+.1}%)",
                                    format_usd_millions(country.economy.gdp_usd_millions),
                                    country.economy.gdp_growth_pct
                                )).strong().color(ACCENT_EMERALD));
                                ui.end_row();

                                ui.label("PIB per cápita:");
                                ui.label(RichText::new(format!("${:.0} USD", country.gdp_per_capita_usd())).strong());
                                ui.end_row();

                                ui.label("Inflación / Deuda:");
                                ui.label(RichText::new(format!(
                                    "{:.1}% anual / {:.1}% PIB",
                                    country.economy.inflation_pct, country.economy.debt_to_gdp_pct
                                )));
                                ui.end_row();
                            });
                        });

                    ui.add_space(8.0);

                    // Bloque: Partidos Políticos
                    egui::Frame::new()
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(6.0)
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Partidos Políticos en el Congreso").strong().color(ACCENT_GOLD));
                            ui.add_space(4.0);
                            for party in &country.political_parties {
                                ui.horizontal(|ui| {
                                    let badge = if party.is_ruling { "GOB" } else { "OPO" };
                                    ui.label(
                                        RichText::new(format!("[{badge}] {}", party.acronym))
                                            .strong()
                                            .color(ideology_color(&party.ideology)),
                                    );
                                    ui.label(RichText::new(&party.name).size(12.5).color(TEXT_PRIMARY));
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "   Ideología: {} • Escaños: {:.0}% • Aprobación: {:.0}%",
                                        party.ideology, party.seats_pct, party.approval_pct
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(2.0);
                            }
                        });

                    ui.add_space(10.0);

                    let btn = egui::Button::new(
                        RichText::new(format!("Gobernar {}", country.name))
                            .size(16.0)
                            .strong()
                            .color(Color32::from_rgb(15, 22, 32)),
                    )
                    .fill(ACCENT_GOLD);

                    if ui.add_sized([340.0, 46.0], btn).clicked() {
                        action = CountrySelectAction::ConfirmCountry(sel_id);
                    }
                });
            });
    });

    action
}
