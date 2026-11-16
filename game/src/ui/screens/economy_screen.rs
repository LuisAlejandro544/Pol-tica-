//! Propósito del archivo:
//! Pantalla dedicada de Economía, Moneda y Banco Central (`Economy`).
//! Permite ajustar impuestos, tasa de interés monetaria, inversión en infraestructura
//! y gasto social, además de monitorear el tipo de cambio, inflación, deuda y PIB.

use crate::models::GameState;
use crate::ui::theme::{
    format_usd_millions, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD, BG_CARD,
    BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED,
};
use egui::{RichText, Stroke, Ui};

pub fn render_economy_screen(ui: &mut Ui, state: &mut GameState) {
    let Some(pid) = state.player_country_id.clone() else {
        ui.label("Selecciona un país primero.");
        return;
    };
    let Some(country) = state.countries.get_mut(&pid) else {
        return;
    };

    ui.label(
        RichText::new(format!("MINISTERIO DE ECONOMÍA Y BANCO CENTRAL — {}", country.name))
            .strong()
            .size(18.0)
            .color(ACCENT_GOLD),
    );
    ui.add_space(6.0);

    ui.columns(2, |cols| {
        // Columna izquierda: Indicadores de Moneda, PIB y Tesorería
        cols[0].vertical(|ui| {
            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.5_f32, BORDER_BRASS))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Moneda Nacional y Precios").strong().color(ACCENT_CYAN));
                    ui.separator();
                    ui.label(format!(
                        "Divisa: {} ({})",
                        country.economy.currency_name, country.economy.currency_code
                    ));
                    ui.label(
                        RichText::new(format!(
                            "Tipo de Cambio: {:.2} {} por 1 USD",
                            country.economy.exchange_rate_usd, country.economy.currency_code
                        ))
                        .strong()
                        .size(16.0),
                    );
                    ui.label(
                        RichText::new(format!("Inflación Anualizada: {:.2}%", country.economy.inflation_pct))
                            .color(if country.economy.inflation_pct > 10.0 {
                                ACCENT_CRIMSON
                            } else {
                                ACCENT_EMERALD
                            }),
                    );
                });

            ui.add_space(10.0);

            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Cuentas Nacionales y PIB").strong().color(ACCENT_GOLD));
                    ui.separator();
                    ui.label(format!(
                        "PIB Nominal: {}",
                        format_usd_millions(country.economy.gdp_usd_millions)
                    ));
                    ui.label(format!("PIB per Cápita: ${:.0} USD", country.gdp_per_capita_usd()));
                    ui.label(
                        RichText::new(format!("Crecimiento Anual: {:+.2}%", country.economy.gdp_growth_pct))
                            .color(if country.economy.gdp_growth_pct >= 0.0 {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CRIMSON
                            }),
                    );
                    ui.label(format!("Deuda Pública / PIB: {:.1}%", country.economy.debt_to_gdp_pct));
                    ui.label(format!(
                        "Reservas / Tesorería: {}",
                        format_usd_millions(country.economy.treasury_usd_millions)
                    ));
                    ui.label(
                        RichText::new(format!(
                            "Balance Último Mes: {:+.1} M USD",
                            country.economy.last_monthly_balance
                        ))
                        .color(if country.economy.last_monthly_balance >= 0.0 {
                            ACCENT_EMERALD
                        } else {
                            ACCENT_CRIMSON
                        }),
                    );
                });
        });

        // Columna derecha: Palancas interactivas de Política Fiscal y Monetaria
        cols[1].vertical(|ui| {
            egui::Frame::new()
                .fill(BG_PANEL)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(8.0)
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Política Monetaria y Fiscal").strong().color(ACCENT_GOLD));
                    ui.separator();

                    egui::Frame::new().fill(BG_CARD).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.label(RichText::new("Tasa de Interés Referencial (%)").strong());
                        ui.label(
                            RichText::new("Subir tasas frena la inflación pero desacelera el PIB.")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(egui::Slider::new(&mut country.economy.interest_rate_pct, 1.0..=50.0).suffix("%"));
                    });

                    ui.add_space(6.0);

                    egui::Frame::new().fill(BG_CARD).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.label(RichText::new("Presión Tributaria (% del PIB)").strong());
                        ui.label(
                            RichText::new("Aumenta la recaudación; tasas muy altas reducen inversión y aprobación.")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(egui::Slider::new(&mut country.economy.tax_rate_pct, 10.0..=40.0).suffix("%"));
                    });

                    ui.add_space(6.0);

                    egui::Frame::new().fill(BG_CARD).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                        ui.label(RichText::new("Inversión en Infraestructura (% PIB)").strong());
                        ui.add(egui::Slider::new(&mut country.economy.infra_investment_pct, 0.5..=10.0).suffix("%"));

                        ui.add_space(4.0);
                        ui.label(RichText::new("Inversión Social y Educación (% PIB)").strong());
                        ui.add(egui::Slider::new(&mut country.economy.social_investment_pct, 0.5..=12.0).suffix("%"));
                    });
                });
        });
    });
}
