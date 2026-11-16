//! Propósito del archivo:
//! Pantalla independiente de Selección de País (`CountrySelect`) estilo Victoria 2 / HOI4.
//! Divide la pantalla horizontal en tres columnas verticales explícitas (`Layout::top_down`):
//! - Izquierda: Lista vertical deslizable donde cada país es un botón táctil con escudo heráldico.
//! - Centro: Mapa 3D interactivo (`wgpu`) para explorar y seleccionar países/provincias.
//! - Derecha: Dossier diplomático-económico con escudo, botón principal de mando e indicadores.

use crate::map::Latam3dScene;
use crate::models::{GameState, MapMode};
use crate::ui::components::map_canvas::{render_latam_map, MapViewportState};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_section_header, format_population, format_usd_millions,
    game_button, ideology_color, panel_frame, well_frame, GameIcon, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_BTN, BG_BTN_ACTIVE, BG_BTN_HOVER, BORDER_BRASS, BORDER_BRASS_DIM,
    BORDER_SUBTLE, TEXT_DARK, TEXT_MUTED, TEXT_PRIMARY,
};
use eframe::egui_wgpu::wgpu;
use egui::{Align, CornerRadius, Layout, Margin, RichText, Sense, Stroke, Ui, Vec2};
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

    // Barra superior de selección estilo sala de guerra
    panel_frame(true)
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if game_button(ui, Vec2::new(92.0, 30.0), "< MENÚ", None, false, None).clicked() {
                    action = CountrySelectAction::BackToMenu;
                }
                ui.add_space(6.0);
                ui.label(
                    RichText::new("SELECCIÓN DE NACIÓN")
                        .strong()
                        .size(15.5)
                        .color(ACCENT_GOLD),
                );
                ui.separator();
                ui.label(
                    RichText::new("Elige una nación en el mapa 3D o pulsa un botón de la lista")
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
            });
        });

    ui.add_space(5.0);

    let avail = ui.available_size();
    let usable_w = (avail.x - 16.0).max(460.0);
    let mut list_w = (usable_w * 0.23).clamp(150.0, 215.0);
    let mut dossier_w = (usable_w * 0.34).clamp(220.0, 340.0);
    if list_w + dossier_w + 180.0 > usable_w {
        let excess = (list_w + dossier_w + 180.0) - usable_w;
        dossier_w = (dossier_w - excess * 0.65).max(185.0);
        list_w = (list_w - excess * 0.35).max(135.0);
    }
    let map_w = (usable_w - list_w - dossier_w).max(160.0);
    let col_h = (avail.y - 4.0).max(180.0);

    ui.horizontal(|ui| {
        // 1. Lista vertical de 21 naciones — cada país es un botón táctil con escudo propio
        ui.allocate_ui_with_layout(
            egui::vec2(list_w, col_h),
            Layout::top_down(Align::Min),
            |ui| {
                panel_frame(false).inner_margin(8.0).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height((col_h - 18.0).max(120.0));

                        draw_section_header(ui, GameIcon::Crown, "21 NACIONES", ACCENT_GOLD);

                        egui::ScrollArea::vertical()
                            .id_salt("country_select_list_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let row_w = (ui.available_width() - 4.0).max(110.0);
                                for (id, country) in &state.countries {
                                    let is_sel =
                                        state.selected_country_id.as_deref() == Some(id.as_str());
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
                                        draw_country_shield(ui, Vec2::new(26.0, 22.0), country);
                                        ui.label(
                                            RichText::new(&country.name)
                                                .strong()
                                                .size(12.5)
                                                .color(if is_sel { TEXT_DARK } else { TEXT_PRIMARY }),
                                        );
                                    });

                                    if row_resp.clicked() {
                                        state.selected_country_id = Some(id.clone());
                                        state.selected_province_mesh = None;
                                        if let Some(scene) = scene_3d {
                                            if let Some(center) = scene.country_center(id) {
                                                viewport.camera_3d.focus_point(center, 30.0);
                                            }
                                        }
                                    }
                                    ui.add_space(3.0);
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
                panel_frame(true).inner_margin(9.0).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height((col_h - 20.0).max(120.0));

                        let Some(sel_id) = state.selected_country_id.clone() else {
                            ui.label("Selecciona un país en el mapa.");
                            return;
                        };
                        let Some(country) = state.countries.get(&sel_id).cloned() else {
                            return;
                        };

                        ui.horizontal(|ui| {
                            draw_country_shield(ui, Vec2::new(40.0, 34.0), &country);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new(format!("{} ({})", country.name, country.id))
                                        .size(16.5)
                                        .strong()
                                        .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} • {}",
                                        country.capital, country.region
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                        });
                        ui.add_space(6.0);

                        let btn_w = ui.available_width();
                        if game_button(
                            ui,
                            Vec2::new(btn_w, 40.0),
                            &format!("GOBERNAR {}", country.name.to_uppercase()),
                            Some(GameIcon::Crown),
                            true,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                        {
                            action = CountrySelectAction::ConfirmCountry(sel_id.clone());
                        }

                        ui.add_space(6.0);

                        egui::ScrollArea::vertical()
                            .id_salt("country_select_dossier_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                                    draw_section_header(
                                        ui,
                                        GameIcon::Economy,
                                        "INDICADORES",
                                        ACCENT_CYAN,
                                    );
                                    egui::Grid::new("select_stats_grid")
                                        .num_columns(2)
                                        .spacing([8.0, 4.0])
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("Población:").size(12.0).color(TEXT_MUTED));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} ({:+.2}%)",
                                                    format_population(country.population),
                                                    country.pop_growth_pct
                                                ))
                                                .size(12.0)
                                                .strong(),
                                            );
                                            ui.end_row();

                                            ui.label(RichText::new("Divisa:").size(12.0).color(TEXT_MUTED));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} ({:.2}/USD)",
                                                    country.economy.currency_code,
                                                    country.economy.exchange_rate_usd
                                                ))
                                                .size(12.0)
                                                .strong(),
                                            );
                                            ui.end_row();

                                            ui.label(RichText::new("PIB:").size(12.0).color(TEXT_MUTED));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} ({:+.1}%)",
                                                    format_usd_millions(
                                                        country.economy.gdp_usd_millions
                                                    ),
                                                    country.economy.gdp_growth_pct
                                                ))
                                                .size(12.0)
                                                .strong()
                                                .color(ACCENT_EMERALD),
                                            );
                                            ui.end_row();

                                            ui.label(RichText::new("Per cápita:").size(12.0).color(TEXT_MUTED));
                                            ui.label(
                                                RichText::new(format!(
                                                    "${:.0} USD",
                                                    country.gdp_per_capita_usd()
                                                ))
                                                .size(12.0)
                                                .strong(),
                                            );
                                            ui.end_row();

                                            ui.label(RichText::new("Inflación/Deuda:").size(12.0).color(TEXT_MUTED));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:.1}% / {:.1}%",
                                                    country.economy.inflation_pct,
                                                    country.economy.debt_to_gdp_pct
                                                ))
                                                .size(12.0),
                                            );
                                            ui.end_row();
                                        });
                                });

                                ui.add_space(6.0);

                                card_frame(None).show(ui, |ui| {
                                    draw_section_header(
                                        ui,
                                        GameIcon::Politics,
                                        "CONGRESO ACTUAL",
                                        ACCENT_GOLD,
                                    );
                                    for party in &country.political_parties {
                                        well_frame().show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                let badge =
                                                    if party.is_ruling { "GOB" } else { "OPO" };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "[{badge}] {}",
                                                        party.acronym
                                                    ))
                                                    .strong()
                                                    .size(12.0)
                                                    .color(ideology_color(&party.ideology)),
                                                );
                                                ui.label(
                                                    RichText::new(&party.name)
                                                        .size(11.5)
                                                        .color(TEXT_PRIMARY),
                                                );
                                            });
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • Escaños {:.0}% • Aprob. {:.0}%",
                                                    party.ideology,
                                                    party.seats_pct,
                                                    party.approval_pct
                                                ))
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                            );
                                        });
                                        ui.add_space(3.0);
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
