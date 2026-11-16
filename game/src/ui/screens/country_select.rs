//! Propósito del archivo:
//! Pantalla independiente de Selección de País (`CountrySelect`).
//! Divide la pantalla horizontal en tres columnas verticales independientes:
//! - Izquierda: Lista vertical deslizable de las 21 naciones latinoamericanas.
//! - Centro: Mapa interactivo 3D (o respaldo SVG 2D) para explorar y tocar países.
//! - Derecha: Dossier deslizable con indicadores, partidos y botón para iniciar campaña.

use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use crate::ui::theme::{
    country_base_color, format_population, format_usd_millions, ideology_color, ACCENT_CYAN,
    ACCENT_EMERALD, ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, Color32, Layout, RichText, Stroke, Ui};
use parking_lot::Mutex;
use std::sync::Arc;

pub enum CountrySelectAction {
    None,
    BackToMenu,
    ConfirmCountry(String),
}

pub fn render_country_select(
    ui: &mut Ui,
    scene_3d: Option<&Latam3dScene>,
    target_format: wgpu::TextureFormat,
    gpu_info: &str,
    gpu_3d_error: &Arc<Mutex<Option<String>>>,
    state: &mut GameState,
    viewport: &mut MapViewportState,
) -> CountrySelectAction {
    let mut action = CountrySelectAction::None;

    // Barra superior de la pantalla de selección (compacta para evitar desbordes horizontales)
    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("< Menú").clicked() {
                    action = CountrySelectAction::BackToMenu;
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new("SELECCIÓN DE NACIÓN")
                        .strong()
                        .size(16.0)
                        .color(ACCENT_GOLD),
                );
                ui.separator();
                ui.label(
                    RichText::new("Toca un país en el mapa o desliza la lista")
                        .size(12.5)
                        .color(TEXT_MUTED),
                );
            });
        });

    ui.add_space(6.0);

    // Distribución horizontal en 3 columnas proporcionales con Layout::top_down explícito en cada una
    let avail = ui.available_size();
    let usable_w = (avail.x - 16.0).max(480.0);
    let mut list_w = (usable_w * 0.22).clamp(140.0, 210.0);
    let mut dossier_w = (usable_w * 0.34).clamp(220.0, 350.0);
    if list_w + dossier_w + 180.0 > usable_w {
        let excess = (list_w + dossier_w + 180.0) - usable_w;
        dossier_w = (dossier_w - excess * 0.65).max(185.0);
        list_w = (list_w - excess * 0.35).max(130.0);
    }
    let map_w = (usable_w - list_w - dossier_w).max(160.0);
    let col_h = (avail.y - 6.0).max(180.0);

    ui.horizontal(|ui| {
        // 1. Lista vertical deslizable de 21 naciones
        ui.allocate_ui_with_layout(
            egui::vec2(list_w, col_h),
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
                            ui.set_height((col_h - 18.0).max(120.0));

                            ui.label(RichText::new("21 Naciones").strong().color(ACCENT_GOLD));
                            ui.separator();

                            egui::ScrollArea::vertical()
                                .id_salt("country_select_list_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for (id, country) in &state.countries {
                                        let is_sel =
                                            state.selected_country_id.as_deref() == Some(id.as_str());
                                        ui.horizontal(|ui| {
                                            let (dot_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(10.0, 10.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                dot_rect.center(),
                                                5.0,
                                                country_base_color(country),
                                            );
                                            if ui
                                                .add_sized(
                                                    [(ui.available_width() - 4.0).max(90.0), 28.0],
                                                    egui::Button::selectable(is_sel, &country.name),
                                                )
                                                .clicked()
                                            {
                                                state.selected_country_id = Some(id.clone());
                                                state.selected_province_mesh = None;
                                                if let Some(scene) = scene_3d {
                                                    if let Some(center) = scene.country_center(id) {
                                                        viewport.camera_3d.focus_point(center, 30.0);
                                                    }
                                                }
                                            }
                                        });
                                        ui.add_space(2.0);
                                    }
                                });
                        });
                    });
            },
        );

        // 2. Visor interactivo 3D (wgpu)
        ui.allocate_ui_with_layout(
            egui::vec2(map_w, col_h),
            Layout::top_down(Align::Min),
            |ui| {
                if let Some(hit) = render_latam_map(
                    ui,
                    scene_3d,
                    target_format,
                    gpu_info,
                    gpu_3d_error,
                    state,
                    MapMode::Political,
                    viewport,
                ) {
                    state.selected_country_id = Some(hit.country_id);
                    state.selected_province_mesh = hit.province_mesh;
                }
            },
        );

        // 3. Dossier deslizable del país seleccionado
        ui.allocate_ui_with_layout(
            egui::vec2(dossier_w, col_h),
            Layout::top_down(Align::Min),
            |ui| {
                egui::Frame::new()
                    .fill(BG_PANEL)
                    .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                    .corner_radius(8.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            ui.set_height((col_h - 22.0).max(120.0));

                            let Some(sel_id) = state.selected_country_id.clone() else {
                                ui.label("Selecciona un país en el mapa.");
                                return;
                            };
                            let Some(country) = state.countries.get(&sel_id).cloned() else {
                                return;
                            };

                            // Encabezado fijo + botón principal accesible sin tener que buscarlo abajo
                            ui.label(
                                RichText::new(format!("{} ({})", country.name, country.id))
                                    .size(19.0)
                                    .strong()
                                    .color(ACCENT_GOLD),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Capital: {} • {}",
                                    country.capital, country.region
                                ))
                                .size(12.0)
                                .color(TEXT_MUTED),
                            );
                            ui.add_space(6.0);

                            let btn_w = ui.available_width();
                            let start_btn = egui::Button::new(
                                RichText::new(format!("Gobernar {} >", country.name))
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::from_rgb(15, 22, 32)),
                            )
                            .fill(ACCENT_GOLD);

                            if ui.add_sized([btn_w, 40.0], start_btn).clicked() {
                                action = CountrySelectAction::ConfirmCountry(sel_id.clone());
                            }

                            ui.add_space(6.0);
                            ui.separator();

                            egui::ScrollArea::vertical()
                                .id_salt("country_select_dossier_scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    // Bloque: Población, Moneda y PIB
                                    egui::Frame::new()
                                        .fill(BG_CARD)
                                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                        .corner_radius(6.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("Indicadores Económicos")
                                                    .strong()
                                                    .color(ACCENT_CYAN),
                                            );
                                            ui.add_space(4.0);
                                            egui::Grid::new("select_stats_grid")
                                                .num_columns(2)
                                                .spacing([10.0, 4.0])
                                                .show(ui, |ui| {
                                                    ui.label("Población:");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} ({:+.2}%)",
                                                            format_population(country.population),
                                                            country.pop_growth_pct
                                                        ))
                                                        .strong(),
                                                    );
                                                    ui.end_row();

                                                    ui.label("Moneda:");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} ({})",
                                                            country.economy.currency_name,
                                                            country.economy.currency_code
                                                        ))
                                                        .strong(),
                                                    );
                                                    ui.end_row();

                                                    ui.label("Cambio:");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:.2} {} / USD",
                                                            country.economy.exchange_rate_usd,
                                                            country.economy.currency_code
                                                        ))
                                                        .strong(),
                                                    );
                                                    ui.end_row();

                                                    ui.label("PIB:");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} ({:+.1}%)",
                                                            format_usd_millions(
                                                                country.economy.gdp_usd_millions
                                                            ),
                                                            country.economy.gdp_growth_pct
                                                        ))
                                                        .strong()
                                                        .color(ACCENT_EMERALD),
                                                    );
                                                    ui.end_row();

                                                    ui.label("Per cápita:");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "${:.0} USD",
                                                            country.gdp_per_capita_usd()
                                                        ))
                                                        .strong(),
                                                    );
                                                    ui.end_row();

                                                    ui.label("Inflación / Deuda:");
                                                    ui.label(format!(
                                                        "{:.1}% / {:.1}% PIB",
                                                        country.economy.inflation_pct,
                                                        country.economy.debt_to_gdp_pct
                                                    ));
                                                    ui.end_row();
                                                });
                                        });

                                    ui.add_space(6.0);

                                    // Bloque: Partidos Políticos
                                    egui::Frame::new()
                                        .fill(BG_CARD)
                                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                        .corner_radius(6.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new("Partidos en el Congreso")
                                                    .strong()
                                                    .color(ACCENT_GOLD),
                                            );
                                            ui.add_space(4.0);
                                            for party in &country.political_parties {
                                                ui.horizontal_wrapped(|ui| {
                                                    let badge =
                                                        if party.is_ruling { "GOB" } else { "OPO" };
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "[{badge}] {}",
                                                            party.acronym
                                                        ))
                                                        .strong()
                                                        .color(ideology_color(&party.ideology)),
                                                    );
                                                    ui.label(
                                                        RichText::new(&party.name)
                                                            .size(12.0)
                                                            .color(TEXT_PRIMARY),
                                                    );
                                                });
                                                ui.label(
                                                    RichText::new(format!(
                                                        "  {} • Escaños {:.0}% • Aprob. {:.0}%",
                                                        party.ideology,
                                                        party.seats_pct,
                                                        party.approval_pct
                                                    ))
                                                    .size(11.5)
                                                    .color(TEXT_MUTED),
                                                );
                                                ui.add_space(2.0);
                                            }
                                        });
                                });
                        });
                    });
            },
        );
    });

    action
}
