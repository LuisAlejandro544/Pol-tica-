//! Propósito del archivo:
//! Barra superior de indicadores nacionales (Tesorería, Moneda, PIB, Estabilidad, Fecha),
//! barra inferior de navegación entre pantallas independientes y popup modal de eventos.

use crate::models::{GameEvent, GameState, ScreenRoute};
use crate::ui::theme::{
    format_population, format_usd_millions, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_CARD, BG_PANEL, BORDER_BRASS, BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Align, Color32, Layout, RichText, Stroke, Ui};

pub enum TopBarAction {
    None,
    TogglePause,
    AdvanceOneTurn,
    SetSpeed(u8),
}

/// Barra superior persistente durante la campaña (muestra métricas clave del país elegido).
pub fn render_top_hud(ui: &mut Ui, state: &GameState, sim_busy: bool) -> TopBarAction {
    let mut action = TopBarAction::None;

    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(pid) = &state.player_country_id {
                    if let Some(c) = state.countries.get(pid) {
                        ui.label(
                            RichText::new(format!("{} [{}]", c.name.to_uppercase(), c.id))
                                .strong()
                                .size(15.0)
                                .color(ACCENT_GOLD),
                        );
                        ui.separator();

                        metric_pill(ui, "Población", &format_population(c.population), TEXT_PRIMARY);
                        metric_pill(
                            ui,
                            "Tesorería",
                            &format_usd_millions(c.economy.treasury_usd_millions),
                            ACCENT_EMERALD,
                        );
                        metric_pill(
                            ui,
                            &format!("Moneda ({})", c.economy.currency_code),
                            &format!("{:.2}/USD", c.economy.exchange_rate_usd),
                            ACCENT_CYAN,
                        );
                        metric_pill(
                            ui,
                            "PIB",
                            &format!(
                                "{} ({:+.1}%)",
                                format_usd_millions(c.economy.gdp_usd_millions),
                                c.economy.gdp_growth_pct
                            ),
                            if c.economy.gdp_growth_pct >= 0.0 {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CRIMSON
                            },
                        );
                        metric_pill(
                            ui,
                            "Estabilidad",
                            &format!("{:.0}%", c.stability),
                            if c.stability >= 55.0 {
                                ACCENT_GOLD
                            } else {
                                ACCENT_CRIMSON
                            },
                        );
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(if sim_busy { "Simulando..." } else { "Siguiente Mes >>" }).clicked() && !sim_busy {
                        action = TopBarAction::AdvanceOneTurn;
                    }

                    for spd in [3_u8, 2, 1] {
                        let selected = state.speed_level == spd;
                        let label = format!("{spd}x");
                        if ui.selectable_label(selected, label).clicked() {
                            action = TopBarAction::SetSpeed(spd);
                        }
                    }

                    let pause_txt = if state.paused { "Reanudar" } else { "Pausar" };
                    if ui.button(pause_txt).clicked() {
                        action = TopBarAction::TogglePause;
                    }

                    ui.label(
                        RichText::new(format!("Turno {} | {}", state.turn, state.date_label()))
                            .strong()
                            .color(ACCENT_GOLD),
                    );
                });
            });
        });

    action
}

fn metric_pill(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(RichText::new(label).size(10.5).color(TEXT_MUTED));
        ui.label(RichText::new(value).strong().size(13.0).color(color));
    });
    ui.add_space(6.0);
}

/// Barra inferior horizontal con botones dedicados para cambiar entre pantallas.
pub fn render_bottom_nav(ui: &mut Ui, current: ScreenRoute) -> Option<ScreenRoute> {
    let mut next_route = None;
    let tabs = [
        (ScreenRoute::WorldMap, "Mapa Latam"),
        (ScreenRoute::Economy, "Economía y Moneda"),
        (ScreenRoute::Politics, "Política y Partidos"),
        (ScreenRoute::Diplomacy, "Diplomacia"),
        (ScreenRoute::Military, "Defensa y Fuerzas"),
        (ScreenRoute::Settings, "Opciones"),
    ];

    egui::Frame::new()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for (route, title) in tabs {
                    let active = current == route;
                    let text = RichText::new(title).size(13.5).strong().color(if active {
                        Color32::from_rgb(15, 22, 32)
                    } else {
                        TEXT_PRIMARY
                    });
                    let btn = egui::Button::new(text)
                        .fill(if active { ACCENT_GOLD } else { BG_CARD })
                        .stroke(Stroke::new(1.0_f32, if active { ACCENT_GOLD } else { BORDER_SUBTLE }));

                    if ui.add_sized([148.0, 36.0], btn).clicked() {
                        next_route = Some(route);
                    }
                }
            });
        });

    next_route
}

/// Popup modal para eventos dinámicos generados por Lua 5.4.
pub fn render_event_popup(ctx: &egui::Context, event: &GameEvent) -> Option<usize> {
    let mut chosen_idx = None;
    egui::Window::new(RichText::new(&event.title).strong().color(ACCENT_GOLD))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(520.0)
        .max_width(520.0)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(BG_PANEL)
                .stroke(Stroke::new(2.0_f32, BORDER_BRASS)),
        )
        .show(ctx, |ui| {
            ui.label(RichText::new(&event.description).size(14.0).color(TEXT_PRIMARY));
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            for (idx, opt) in event.options.iter().enumerate() {
                let summary = format!(
                    "{}\n  PIB {:+.2}% | Inflación {:+.2}% | Estabilidad {:+.1}%",
                    opt.label, opt.gdp_growth_delta, opt.inflation_delta, opt.stability_delta
                );
                if ui
                    .add_sized([490.0, 48.0], egui::Button::new(summary))
                    .clicked()
                {
                    chosen_idx = Some(idx);
                }
                ui.add_space(4.0);
            }
        });

    chosen_idx
}
