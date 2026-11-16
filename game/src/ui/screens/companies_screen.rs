//! Propósito del archivo:
//! Pantalla dedicada de Empresas Privadas, Empleo y Bolsa Regional (`Companies`).
//! Muestra las empresas fundadas aleatoriamente en los países latinoamericanos, su categoría,
//! qué productos o servicios venden, empleos generados, aporte al PIB nacional y sus precios
//! de acción en tiempo real con gráfica histórica bursátil.

use crate::models::{CompanyCategory, CompanyEntity, GameState};
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_section_header, format_usd_millions, game_button,
    panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD,
    BG_WELL, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone)]
pub struct CompaniesViewState {
    pub only_player_country: bool,
    pub category_filter: Option<CompanyCategory>,
    pub selected_company_id: Option<u64>,
}

impl Default for CompaniesViewState {
    fn default() -> Self {
        Self {
            only_player_country: false,
            category_filter: None,
            selected_company_id: None,
        }
    }
}

pub fn render_companies_screen(
    ui: &mut Ui,
    state: &GameState,
    view: &mut CompaniesViewState,
) {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return;
    };
    let Some(player_country) = state.countries.get(&pid) else {
        return;
    };

    let my_jobs = state.country_company_jobs(&pid);
    let my_gdp = state.country_company_gdp_bonus(&pid);

    let filtered: Vec<&CompanyEntity> = state
        .companies
        .iter()
        .filter(|c| !view.only_player_country || c.country_id == pid)
        .filter(|c| view.category_filter.map(|f| f == c.category).unwrap_or(true))
        .collect();

    if view.selected_company_id.is_none()
        || !filtered
            .iter()
            .any(|c| Some(c.id) == view.selected_company_id)
    {
        view.selected_company_id = filtered.first().map(|c| c.id);
    }

    let selected = view
        .selected_company_id
        .and_then(|id| filtered.iter().find(|c| c.id == id).copied());

    egui::ScrollArea::vertical()
        .id_salt("companies_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                draw_country_shield(ui, Vec2::new(34.0, 28.0), player_country);
                ui.label(
                    RichText::new("BOLSA REGIONAL Y SECTOR EMPRESARIAL")
                        .strong()
                        .size(16.5)
                        .color(ACCENT_GOLD),
                );
                ui.separator();
                well_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "Desempleo {}: {:.1}%  |  Empleos privados: {}  |  Aporte PIB: +{}",
                            player_country.id,
                            player_country.economy.unemployment_pct,
                            my_jobs,
                            format_usd_millions(my_gdp)
                        ))
                        .size(11.5)
                        .color(ACCENT_CYAN),
                    );
                });
            });
            ui.add_space(6.0);

            // Filtros horizontales táctiles de país y categoría
            panel_frame(false).inner_margin(7.0).show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("companies_filter_scroll")
                    .max_height(34.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if game_button(
                                ui,
                                Vec2::new(118.0, 28.0),
                                "Toda Latam",
                                None,
                                !view.only_player_country,
                                Some(ACCENT_GOLD),
                            )
                            .clicked()
                            {
                                view.only_player_country = false;
                            }
                            if game_button(
                                ui,
                                Vec2::new(112.0, 28.0),
                                &format!("Solo {}", player_country.id),
                                None,
                                view.only_player_country,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                            {
                                view.only_player_country = true;
                            }
                            ui.separator();
                            if game_button(
                                ui,
                                Vec2::new(82.0, 28.0),
                                "Todas",
                                None,
                                view.category_filter.is_none(),
                                Some(ACCENT_EMERALD),
                            )
                            .clicked()
                            {
                                view.category_filter = None;
                            }
                            for cat in [
                                CompanyCategory::Tecnologia,
                                CompanyCategory::Energia,
                                CompanyCategory::Agroindustria,
                                CompanyCategory::Manufactura,
                                CompanyCategory::Logistica,
                                CompanyCategory::Finanzas,
                                CompanyCategory::Salud,
                            ] {
                                let active = view.category_filter == Some(cat);
                                if game_button(
                                    ui,
                                    Vec2::new(108.0, 28.0),
                                    cat.label(),
                                    None,
                                    active,
                                    Some(ACCENT_GOLD),
                                )
                                .clicked()
                                {
                                    view.category_filter = Some(cat);
                                }
                            }
                        });
                    });
            });

            ui.add_space(7.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Companies,
                            &format!("EMPRESAS COTIZADAS ({})", filtered.len()),
                            ACCENT_GOLD,
                        );
                        if filtered.is_empty() {
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new(
                                        "Aún no hay empresas registradas bajo este filtro. \
                                         Con el avance del tiempo real se fundarán nuevas compañías.",
                                    )
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                                );
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt("companies_list_scroll")
                                .max_height(265.0)
                                .show(ui, |ui| {
                                    for comp in &filtered {
                                        let is_sel = view.selected_company_id == Some(comp.id);
                                        let delta = comp.share_delta_pct();
                                        let delta_col = if delta >= 0.0 {
                                            ACCENT_EMERALD
                                        } else {
                                            ACCENT_CRIMSON
                                        };
                                        let country_name = state
                                            .countries
                                            .get(&comp.country_id)
                                            .map(|c| c.name.as_str())
                                            .unwrap_or(&comp.country_id);

                                        card_frame(Some(if is_sel {
                                            ACCENT_GOLD
                                        } else {
                                            BORDER_BRASS_DIM
                                        }))
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "[{}] {}",
                                                        comp.country_id, comp.name
                                                    ))
                                                    .strong()
                                                    .size(12.5)
                                                    .color(if is_sel {
                                                        ACCENT_GOLD
                                                    } else {
                                                        TEXT_PRIMARY
                                                    }),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "${:.2} ({:+.2}%)",
                                                                comp.share_price, delta
                                                            ))
                                                            .strong()
                                                            .size(12.0)
                                                            .color(delta_col),
                                                        );
                                                    },
                                                );
                                            });
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • {} • {} empleos",
                                                    comp.category.label(),
                                                    country_name,
                                                    comp.employees
                                                ))
                                                .size(11.0)
                                                .color(ACCENT_CYAN),
                                            );
                                            ui.label(
                                                RichText::new(format!("Vende: {}", comp.sells))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.add_space(3.0);
                                            if game_button(
                                                ui,
                                                Vec2::new(ui.available_width(), 26.0),
                                                if is_sel {
                                                    "Inspeccionando Acción"
                                                } else {
                                                    "Ver Detalle y Gráfica"
                                                },
                                                None,
                                                is_sel,
                                                Some(ACCENT_GOLD),
                                            )
                                            .clicked()
                                            {
                                                view.selected_company_id = Some(comp.id);
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }
                                });
                        }
                    });
                });

                cols[1].vertical(|ui| {
                    panel_frame(false).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Growth,
                            "FICHA CORPORATIVA Y ACCIÓN EN VIVO",
                            ACCENT_CYAN,
                        );
                        if let Some(comp) = selected {
                            let cname = state
                                .countries
                                .get(&comp.country_id)
                                .map(|c| c.name.as_str())
                                .unwrap_or(&comp.country_id);
                            let delta = comp.share_delta_pct();
                            let delta_col = if delta >= 0.0 {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CRIMSON
                            };

                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new(&comp.name)
                                        .strong()
                                        .size(15.5)
                                        .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Sede: {} [{}]  |  Sector: {}  |  Fundada: {}",
                                        cname,
                                        comp.country_id,
                                        comp.category.label(),
                                        comp.founded_label
                                    ))
                                    .size(11.5)
                                    .color(TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(format!("Qué vende: {}", comp.sells))
                                        .size(12.0)
                                        .color(ACCENT_CYAN),
                                );
                            });

                            ui.add_space(6.0);
                            card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Precio de Acción: ${:.2} USD",
                                            comp.share_price
                                        ))
                                        .strong()
                                        .size(14.5)
                                        .color(TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!("{:+.2}%", delta))
                                                    .strong()
                                                    .size(13.5)
                                                    .color(delta_col),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(6.0);
                                draw_stock_sparkline(ui, &comp.price_history, delta_col);
                            });

                            ui.add_space(6.0);
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "Empleos directos generados: {} puestos",
                                        comp.employees
                                    ))
                                    .size(12.5)
                                    .color(ACCENT_EMERALD),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Aporte anual estimado al PIB: +{}",
                                        format_usd_millions(comp.annual_gdp_contribution_m)
                                    ))
                                    .size(12.5)
                                    .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(
                                        "Impacto lógico: reduce la tasa de desempleo y amplía la base tributaria del país sede.",
                                    )
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                            });
                        } else {
                            well_frame().show(ui, |ui| {
                                ui.label(
                                    RichText::new("Selecciona una empresa para ver su cotización.")
                                        .color(TEXT_MUTED),
                                );
                            });
                        }
                    });
                });
            });
        });
}

fn draw_stock_sparkline(ui: &mut Ui, history: &[f64], line_color: egui::Color32) {
    let w = ui.available_width().max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 90.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(3), BG_WELL);
    p.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0_f32, BORDER_BRASS_DIM),
        egui::StrokeKind::Inside,
    );

    if history.len() < 2 {
        return;
    }

    let min_v = history.iter().copied().fold(f64::INFINITY, f64::min);
    let max_v = history.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_v - min_v).max(0.5);
    let inner = rect.shrink(8.0);

    let mut pts = Vec::with_capacity(history.len());
    for (i, val) in history.iter().enumerate() {
        let tx = (i as f32) / ((history.len() - 1) as f32);
        let ty = ((*val - min_v) / span) as f32;
        pts.push(Pos2::new(
            inner.left() + tx * inner.width(),
            inner.bottom() - ty * inner.height(),
        ));
    }

    for pair in pts.windows(2) {
        p.line_segment([pair[0], pair[1]], Stroke::new(2.0_f32, line_color));
    }
    if let Some(&last) = pts.last() {
        p.circle_filled(last, 3.5, line_color);
    }
}
