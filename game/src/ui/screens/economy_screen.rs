//! Propósito del archivo:
//! Pantalla dedicada de Economía, Moneda y Banco Central (`Economy`) estilo Victoria 2.
//! Combina libros contables nacionales, medidores de inflación y deuda, y palancas de
//! política fiscal/monetaria con botones táctiles `[-]` / `[+]` además de deslizadores.

use crate::models::GameState;
use crate::ui::theme::{
    card_frame, draw_country_shield, draw_gauge_bar, draw_section_header, format_usd_millions,
    game_button, panel_frame, step_button, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN,
    ACCENT_EMERALD, ACCENT_GOLD, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{RichText, Ui, Vec2};

pub fn render_economy_screen(ui: &mut Ui, state: &mut GameState) -> bool {
    let mut open_companies = false;
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return false;
    };
    let corp_count = state.companies.iter().filter(|c| c.country_id == pid).count();
    let corp_jobs = state.country_company_jobs(&pid);
    let corp_gdp = state.country_company_gdp_bonus(&pid);
    let corp_exports = state.country_company_export_bonus(&pid);
    let global_cycle = state.global_cycle.clone();

    let Some(country) = state.countries.get_mut(&pid) else {
        return false;
    };

    let shield_country = country.clone();

    egui::ScrollArea::vertical()
        .id_salt("economy_screen_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                draw_country_shield(ui, Vec2::new(34.0, 28.0), &shield_country);
                ui.label(
                    RichText::new(format!(
                        "MINISTERIO DE ECONOMÍA Y BANCO CENTRAL — {}",
                        shield_country.name.to_uppercase()
                    ))
                    .strong()
                    .size(16.5)
                    .color(ACCENT_GOLD),
                );
            });
            ui.add_space(6.0);

            ui.columns(2, |cols| {
                // Columna izquierda: Libro de Moneda, PIB y Tesorería
                cols[0].vertical(|ui| {
                    panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Currency,
                            "MONEDA NACIONAL Y PRECIOS",
                            ACCENT_CYAN,
                        );
                        well_frame().show(ui, |ui| {
                            ui.label(format!(
                                "Divisa: {} ({})",
                                country.economy.currency_name, country.economy.currency_code
                            ));
                            ui.label(
                                RichText::new(format!(
                                    "Tipo de Cambio: {:.2} {} / 1 USD",
                                    country.economy.exchange_rate_usd,
                                    country.economy.currency_code
                                ))
                                .strong()
                                .size(15.0)
                                .color(TEXT_PRIMARY),
                            );
                            let real_rate =
                                country.economy.interest_rate_pct - country.economy.inflation_pct;
                            ui.label(
                                RichText::new(format!(
                                    "Meta BC: {:.1}%  |  Tasa Real: {:+.2}%  |  Riesgo País: {:.0} pb",
                                    country.economy.inflation_target_pct,
                                    real_rate,
                                    country.economy.country_risk_bps
                                ))
                                .size(11.0)
                                .color(if country.economy.country_risk_bps >= 500.0 {
                                    ACCENT_CRIMSON
                                } else {
                                    ACCENT_CYAN
                                }),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Ciclo Global: {} (Fed {:.2}% | Comm. {:.0})",
                                    global_cycle.cycle_label,
                                    global_cycle.fed_rate_pct,
                                    global_cycle.commodity_index
                                ))
                                .size(10.5)
                                .color(TEXT_MUTED),
                            );
                        });
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Inflación Anualizada")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            18.0,
                            (country.economy.inflation_pct / 50.0) as f32,
                            if country.economy.inflation_pct > 10.0 {
                                ACCENT_CRIMSON
                            } else {
                                ACCENT_EMERALD
                            },
                            &format!("{:.2}%", country.economy.inflation_pct),
                        );
                    });

                    ui.add_space(8.0);

                    panel_frame(false).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Treasury,
                            "CUENTAS NACIONALES Y TESORERÍA",
                            ACCENT_GOLD,
                        );
                        well_frame().show(ui, |ui| {
                            ui.label(format!(
                                "PIB Nominal: {}",
                                format_usd_millions(country.economy.gdp_usd_millions)
                            ));
                            ui.label(format!(
                                "PIB per Cápita: ${:.0} USD",
                                country.gdp_per_capita_usd()
                            ));
                            ui.label(
                                RichText::new(format!(
                                    "Crecimiento Anual: {:+.2}% (Potencial: {:.2}%)",
                                    country.economy.gdp_growth_pct,
                                    country.economy.potential_gdp_growth_pct
                                ))
                                .strong()
                                .color(if country.economy.gdp_growth_pct >= 0.0 {
                                    ACCENT_EMERALD
                                } else {
                                    ACCENT_CRIMSON
                                }),
                            );
                            ui.label(format!(
                                "Reservas / Tesorería: {}",
                                format_usd_millions(country.economy.treasury_usd_millions)
                            ));
                            ui.label(
                                RichText::new(format!(
                                    "Balance Fiscal: {:+.1} M/mes  |  Balanza Com.: {:+.1} M/mes",
                                    country.economy.last_monthly_balance,
                                    country.economy.trade_balance_usd_millions
                                ))
                                .strong()
                                .size(11.5)
                                .color(if country.economy.last_monthly_balance >= 0.0 {
                                    ACCENT_EMERALD
                                } else {
                                    ACCENT_CRIMSON
                                }),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Mantenimiento Militar: -${:.2} M USD/mes",
                                    country.economy.troop_upkeep_usd_millions
                                ))
                                .size(11.5)
                                .color(ACCENT_CRIMSON),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "Sector Privado: {} emp. ({} puestos | +{} PIB | +{} Exp/año | Imp. +${:.1}M/mes)",
                                    corp_count,
                                    corp_jobs,
                                    format_usd_millions(corp_gdp),
                                    format_usd_millions(corp_exports),
                                    country.economy.corporate_tax_revenue_m
                                ))
                                .size(11.0)
                                .color(ACCENT_CYAN),
                            );
                        });
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Tasa de Desempleo Nacional")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            18.0,
                            (country.economy.unemployment_pct / 25.0) as f32,
                            if country.economy.unemployment_pct > 9.5 {
                                ACCENT_CRIMSON
                            } else {
                                ACCENT_EMERALD
                            },
                            &format!("{:.1}% Desempleo", country.economy.unemployment_pct),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Deuda Pública / PIB")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            18.0,
                            (country.economy.debt_to_gdp_pct / 150.0) as f32,
                            if country.economy.debt_to_gdp_pct > 85.0 {
                                ACCENT_CRIMSON
                            } else {
                                ACCENT_GOLD
                            },
                            &format!("{:.1}% PIB", country.economy.debt_to_gdp_pct),
                        );
                        ui.add_space(8.0);
                        if game_button(
                            ui,
                            Vec2::new(ui.available_width(), 36.0),
                            "Ver Bolsa y Empresas Nacionales >",
                            Some(GameIcon::Companies),
                            true,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                        {
                            open_companies = true;
                        }
                    });
                });

                // Columna derecha: Palancas táctiles de Política Monetaria y Fiscal
                cols[1].vertical(|ui| {
                    panel_frame(true).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Economy,
                            "PALANCAS MONETARIAS Y FISCALES",
                            ACCENT_GOLD,
                        );

                        policy_lever(
                            ui,
                            "Tasa de Interés Referencial",
                            "Subir tasas frena la inflación pero desacelera el PIB.",
                            &mut country.economy.interest_rate_pct,
                            1.0,
                            50.0,
                            0.5,
                            "%",
                        );

                        ui.add_space(6.0);

                        policy_lever(
                            ui,
                            "Presión Tributaria (% del PIB)",
                            "Aumenta ingresos; tasas muy altas frenan crecimiento y apoyo.",
                            &mut country.economy.tax_rate_pct,
                            10.0,
                            40.0,
                            0.5,
                            "%",
                        );

                        ui.add_space(6.0);

                        policy_lever(
                            ui,
                            "Inversión en Infraestructura",
                            "Impulsa el crecimiento estructural del PIB a mediano plazo.",
                            &mut country.economy.infra_investment_pct,
                            0.5,
                            10.0,
                            0.25,
                            "%",
                        );

                        ui.add_space(6.0);

                        policy_lever(
                            ui,
                            "Inversión Social y Educación",
                            "Mejora estabilidad interna y aprobación ciudadana.",
                            &mut country.economy.social_investment_pct,
                            0.5,
                            12.0,
                            0.25,
                            "%",
                        );
                    });
                });
            });
        });

    open_companies
}

#[allow(clippy::too_many_arguments)]
fn policy_lever(
    ui: &mut Ui,
    title: &str,
    hint: &str,
    value: &mut f64,
    min: f64,
    max: f64,
    step: f64,
    suffix: &str,
) {
    card_frame(Some(BORDER_BRASS_DIM)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().size(13.0).color(TEXT_PRIMARY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{:.2}{suffix}", *value))
                        .strong()
                        .size(13.0)
                        .color(ACCENT_GOLD),
                );
            });
        });
        ui.label(RichText::new(hint).size(11.0).color(TEXT_MUTED));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if step_button(ui, Vec2::new(32.0, 28.0), "", Some(GameIcon::Minus)).clicked() {
                *value = (*value - step).clamp(min, max);
            }
            let slider_w = (ui.available_width() - 42.0).max(80.0);
            ui.add_sized(
                [slider_w, 24.0],
                egui::Slider::new(value, min..=max).show_value(false),
            );
            if step_button(ui, Vec2::new(32.0, 28.0), "", Some(GameIcon::Plus)).clicked() {
                *value = (*value + step).clamp(min, max);
            }
        });
    });
}
