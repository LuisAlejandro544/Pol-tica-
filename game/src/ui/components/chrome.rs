//! Propósito del archivo:
//! Barra superior de mando (escudo nacional, indicadores estratégicos y reloj de turno/velocidad
//! siempre visible a la derecha), barra inferior de navegación tipo HOI4/Victoria 2 con íconos
//! vectoriales y popup modal de despacho diplomático para eventos de Lua 5.4.

use crate::models::{GameEvent, GameState, ScreenRoute};
use crate::ui::theme::{
    draw_country_shield, draw_gauge_bar, format_population, format_usd_millions, game_button,
    paint_icon, panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD,
    ACCENT_GOLD, BG_CARD, BG_PANEL, BG_WELL, BORDER_BRASS, BORDER_BRASS_DIM, BORDER_SUBTLE,
    TEXT_DARK, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Color32, CornerRadius, Margin, RichText, Sense, Stroke, Ui, Vec2};

pub enum TopBarAction {
    None,
    TogglePause,
    AdvanceOneTurn,
    SetSpeed(u8),
}

/// Barra superior persistente durante la campaña: escudo + indicadores a la izquierda y reloj/velocidad fijado a la derecha.
pub fn render_top_hud(ui: &mut Ui, state: &GameState, sim_busy: bool) -> TopBarAction {
    let mut action = TopBarAction::None;

    panel_frame(true)
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            let total_w = ui.available_width().max(320.0);
            let controls_w = 304.0_f32.min(total_w * 0.44);
            let stats_w = (total_w - controls_w - 10.0).max(120.0);

            ui.horizontal(|ui| {
                // Bloque izquierdo desplazable: Escudo nacional + cajas de métricas tácticas
                ui.allocate_ui_with_layout(
                    Vec2::new(stats_w, 36.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt("top_hud_scroll")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if let Some(pid) = &state.player_country_id {
                                        if let Some(c) = state.countries.get(pid) {
                                            draw_country_shield(ui, Vec2::new(34.0, 28.0), c);
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing.y = 0.0;
                                                ui.label(
                                                    RichText::new(c.name.to_uppercase())
                                                        .strong()
                                                        .size(13.0)
                                                        .color(ACCENT_GOLD),
                                                );
                                                if let Some(ruling) = c.ruling_party() {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{} • {}",
                                                            ruling.acronym, ruling.ideology
                                                        ))
                                                        .size(10.0)
                                                        .color(TEXT_MUTED),
                                                    );
                                                }
                                            });
                                            ui.add_space(4.0);

                                            stat_slot(
                                                ui,
                                                GameIcon::Population,
                                                "POB",
                                                &format_population(c.population),
                                                TEXT_PRIMARY,
                                            );
                                            stat_slot(
                                                ui,
                                                GameIcon::Treasury,
                                                "TESORERÍA",
                                                &format_usd_millions(c.economy.treasury_usd_millions),
                                                ACCENT_EMERALD,
                                            );
                                            stat_slot(
                                                ui,
                                                GameIcon::Currency,
                                                &c.economy.currency_code,
                                                &format!("{:.2}/USD", c.economy.exchange_rate_usd),
                                                ACCENT_CYAN,
                                            );
                                            stat_slot(
                                                ui,
                                                GameIcon::Growth,
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

                                            well_frame().show(ui, |ui| {
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 1.0;
                                                    ui.label(
                                                        RichText::new("ESTABILIDAD")
                                                            .size(9.5)
                                                            .color(TEXT_MUTED),
                                                    );
                                                    draw_gauge_bar(
                                                        ui,
                                                        68.0,
                                                        13.0,
                                                        (c.stability / 100.0) as f32,
                                                        if c.stability >= 55.0 {
                                                            ACCENT_EMERALD
                                                        } else {
                                                            ACCENT_CRIMSON
                                                        },
                                                        &format!("{:.0}%", c.stability),
                                                    );
                                                });
                                            });
                                        }
                                    }
                                });
                            });
                    },
                );

                // Bloque derecho fijado: Cronómetro, Pausa/Play, Velocidad [1x][2x][3x] y Siguiente Mes
                ui.allocate_ui_with_layout(
                    Vec2::new(controls_w, 36.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let next_lbl = if sim_busy { "..." } else { "+1 MES" };
                        if game_button(
                            ui,
                            Vec2::new(76.0, 32.0),
                            next_lbl,
                            Some(GameIcon::NextTurn),
                            !sim_busy,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                            && !sim_busy
                        {
                            action = TopBarAction::AdvanceOneTurn;
                        }

                        for spd in [3_u8, 2, 1] {
                            let selected = state.speed_level == spd;
                            if game_button(
                                ui,
                                Vec2::new(30.0, 32.0),
                                &format!("{spd}x"),
                                None,
                                selected,
                                Some(ACCENT_CYAN),
                            )
                            .clicked()
                            {
                                action = TopBarAction::SetSpeed(spd);
                            }
                        }

                        let pause_icon = if state.paused {
                            GameIcon::Play
                        } else {
                            GameIcon::Pause
                        };
                        if game_button(
                            ui,
                            Vec2::new(34.0, 32.0),
                            "",
                            Some(pause_icon),
                            !state.paused,
                            Some(ACCENT_EMERALD),
                        )
                        .clicked()
                        {
                            action = TopBarAction::TogglePause;
                        }

                        well_frame().show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(
                                    RichText::new(state.date_label().to_uppercase())
                                        .strong()
                                        .size(12.0)
                                        .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(format!("TURNO {}", state.turn))
                                        .size(9.5)
                                        .color(TEXT_MUTED),
                                );
                            });
                        });
                    },
                );
            });
        });

    action
}

fn stat_slot(ui: &mut Ui, icon: GameIcon, label: &str, value: &str, color: Color32) {
    well_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let (ic_rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
            paint_icon(ui.painter(), ic_rect, icon, color);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(label).size(9.5).color(TEXT_MUTED));
                ui.label(RichText::new(value).strong().size(11.5).color(color));
            });
        });
    });
}

/// Barra inferior horizontal con botones táctiles remachados e íconos para cambiar entre pantallas.
pub fn render_bottom_nav(ui: &mut Ui, current: ScreenRoute) -> Option<ScreenRoute> {
    let mut next_route = None;
    let tabs = [
        (ScreenRoute::WorldMap, GameIcon::Map, "Mapa"),
        (ScreenRoute::Economy, GameIcon::Economy, "Economía"),
        (ScreenRoute::Politics, GameIcon::Politics, "Política"),
        (ScreenRoute::Diplomacy, GameIcon::Diplomacy, "Diplomacia"),
        (ScreenRoute::Military, GameIcon::Military, "Defensa"),
        (ScreenRoute::Settings, GameIcon::Settings, "Opciones"),
    ];

    panel_frame(false)
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            let avail_w = ui.available_width().max(320.0);
            let tab_w = ((avail_w - 42.0) / 6.0).clamp(96.0, 152.0);
            egui::ScrollArea::horizontal()
                .id_salt("bottom_nav_scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        for (route, icon, title) in tabs {
                            let active = current == route;
                            if game_button(
                                ui,
                                Vec2::new(tab_w, 34.0),
                                title,
                                Some(icon),
                                active,
                                Some(ACCENT_GOLD),
                            )
                            .clicked()
                            {
                                next_route = Some(route);
                            }
                        }
                    });
                });
        });

    next_route
}

/// Popup modal estilo Despacho Diplomático / Periódico Victoria 2 para eventos de Lua 5.4.
pub fn render_event_popup(ctx: &egui::Context, event: &GameEvent) -> Option<usize> {
    let mut chosen_idx = None;
    egui::Window::new(RichText::new("DESPACHO DE GOBIERNO").strong().color(ACCENT_GOLD))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [-16.0, 0.0])
        .default_width(470.0)
        .max_width(490.0)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(BG_PANEL)
                .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
                .corner_radius(CornerRadius::same(4)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("event_popup_scroll")
                .max_height(260.0)
                .show(ui, |ui| {
                    let btn_w = ui.available_width().min(450.0);
                    egui::Frame::new()
                        .fill(BG_WELL)
                        .stroke(Stroke::new(1.0_f32, BORDER_BRASS_DIM))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (ic_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                paint_icon(ui.painter(), ic_rect, GameIcon::Dispatch, ACCENT_GOLD);
                                ui.label(
                                    RichText::new(&event.title)
                                        .strong()
                                        .size(16.0)
                                        .color(ACCENT_GOLD),
                                );
                            });
                            ui.add_space(5.0);
                            ui.label(
                                RichText::new(&event.description)
                                    .size(13.5)
                                    .color(TEXT_PRIMARY),
                            );
                        });
                    ui.add_space(10.0);

                    for (idx, opt) in event.options.iter().enumerate() {
                        let summary = format!(
                            "{}\n  PIB {:+.2}%  |  Inflación {:+.2}%  |  Estabilidad {:+.1}%",
                            opt.label,
                            opt.gdp_growth_delta,
                            opt.inflation_delta,
                            opt.stability_delta
                        );
                        let resp = egui::Button::new(
                            RichText::new(summary).size(13.0).color(TEXT_PRIMARY),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.5_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(4));
                        if ui.add_sized([btn_w, 48.0], resp).clicked() {
                            chosen_idx = Some(idx);
                        }
                        ui.add_space(5.0);
                    }
                });
        });

    let _ = (TEXT_DARK, BG_PANEL);
    chosen_idx
}
