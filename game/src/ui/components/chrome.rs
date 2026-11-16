//! Propósito del archivo:
//! Barra superior de mando (escudo nacional, indicadores estratégicos y reloj de turno/velocidad
//! siempre visible a la derecha), barra inferior de navegación tipo HOI4/Victoria 2 con íconos
//! vectoriales y popup modal de despacho diplomático para eventos de Lua 5.4.

use crate::models::{GameEvent, GameState, ScreenRoute};
use crate::ui::theme::{
    draw_corner_rivets, draw_country_shield, draw_gauge_bar, floating_dock_frame,
    floating_hud_frame, format_population, format_usd_millions, game_button, paint_icon,
    panel_frame, well_frame, GameIcon, ACCENT_CRIMSON, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD,
    BG_CARD, BG_PANEL, BG_WELL, BORDER_BRASS, BORDER_BRASS_DIM, BORDER_SUBTLE, SAFE_RIGHT_PAD,
    TEXT_DARK, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, Ui, Vec2};

pub enum TopBarAction {
    None,
    TogglePause,
    SetSpeed(u8),
}

/// Barra superior persistente para pantallas administrativas (Economía, Empresas, Política, etc.).
pub fn render_top_hud(ui: &mut Ui, state: &GameState, _sim_busy: bool) -> TopBarAction {
    let mut action = TopBarAction::None;

    panel_frame(true)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            let scroll = egui::ScrollArea::horizontal().id_salt("top_hud_scroll");
            action = render_top_hud_inner(ui, state, scroll);
        });

    action
}

/// Barra superior flotante translúcida sobre el mapa 3D a pantalla completa.
pub fn render_floating_top_hud(
    ctx: &egui::Context,
    state: &GameState,
    _sim_busy: bool,
) -> (TopBarAction, Rect) {
    let mut action = TopBarAction::None;
    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 12.0 - (SAFE_RIGHT_PAD as f32)).max(320.0);

    let area_resp = egui::Area::new(egui::Id::new("floating_top_hud_area"))
        .anchor(egui::Align2::LEFT_TOP, [6.0, 4.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_width(max_w);
            let frame_resp = floating_hud_frame(true)
                .inner_margin(Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    let scroll = egui::ScrollArea::horizontal().id_salt("floating_top_hud_scroll");
                    action = render_top_hud_inner(ui, state, scroll);
                });
            draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
        });

    (action, area_resp.response.rect)
}

fn render_top_hud_inner(
    ui: &mut Ui,
    state: &GameState,
    scroll: egui::ScrollArea,
) -> TopBarAction {
    let mut action = TopBarAction::None;
    ui.set_height(33.0);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    ui.spacing_mut().scroll.bar_width = 3.0;

    let total_w = ui.available_width().max(320.0);
    let controls_w = 212.0_f32.min((total_w - 140.0).max(195.0));
    let stats_w = (total_w - controls_w - 8.0).max(120.0);

    ui.horizontal_centered(|ui| {
        // Bloque izquierdo desplazable: Escudo nacional + cajas de métricas tácticas
        ui.allocate_ui_with_layout(
            Vec2::new(stats_w, 32.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_max_height(33.0);
                scroll
                    .max_height(36.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.5;
                            if let Some(pid) = &state.player_country_id {
                                if let Some(c) = state.countries.get(pid) {
                                    draw_country_shield(ui, Vec2::new(30.0, 25.0), c);
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        ui.label(
                                            RichText::new(c.name.to_uppercase())
                                                .strong()
                                                .size(11.5)
                                                .color(ACCENT_GOLD),
                                        );
                                        if let Some(ruling) = c.ruling_party() {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • {}",
                                                    ruling.acronym, ruling.ideology
                                                ))
                                                .size(9.0)
                                                .color(TEXT_MUTED),
                                            );
                                        }
                                    });
                                    ui.add_space(2.0);

                                    stat_slot(
                                        ui,
                                        GameIcon::Population,
                                        "POB",
                                        &format_population(c.population),
                                        TEXT_PRIMARY,
                                    );
                                    let treasury_lbl = format!(
                                        "{} ({:+.1}M/m)",
                                        format_usd_millions(c.economy.treasury_usd_millions),
                                        c.economy.last_monthly_balance
                                    );
                                    let treasury_col = if c.economy.last_monthly_balance >= 0.0 {
                                        ACCENT_EMERALD
                                    } else if c.economy.treasury_usd_millions > 0.0 {
                                        ACCENT_GOLD
                                    } else {
                                        ACCENT_CRIMSON
                                    };
                                    stat_slot(
                                        ui,
                                        GameIcon::Treasury,
                                        "TESORERÍA",
                                        &treasury_lbl,
                                        treasury_col,
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
                                    stat_slot(
                                        ui,
                                        GameIcon::Companies,
                                        "DESEMPLEO",
                                        &format!("{:.1}%", c.economy.unemployment_pct),
                                        if c.economy.unemployment_pct <= 8.0 {
                                            ACCENT_EMERALD
                                        } else {
                                            ACCENT_CRIMSON
                                        },
                                    );

                                    well_frame()
                                        .inner_margin(Margin::symmetric(5, 2))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing.y = 1.0;
                                                ui.label(
                                                    RichText::new("ESTABILIDAD")
                                                        .size(8.5)
                                                        .color(TEXT_MUTED),
                                                );
                                                draw_gauge_bar(
                                                    ui,
                                                    60.0,
                                                    12.0,
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

        // Bloque derecho fijado: Reloj en tiempo real, Pausa/Play y Velocidad [1x][2x][3x]
        ui.allocate_ui_with_layout(
            Vec2::new(controls_w, 32.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.set_max_height(33.0);
                ui.spacing_mut().item_spacing.x = 3.5;

                for spd in [3_u8, 2, 1] {
                    let selected = state.speed_level == spd;
                    if game_button(
                        ui,
                        Vec2::new(26.0, 28.0),
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
                    Vec2::new(28.0, 28.0),
                    "",
                    Some(pause_icon),
                    !state.paused,
                    Some(ACCENT_EMERALD),
                )
                .clicked()
                {
                    action = TopBarAction::TogglePause;
                }

                well_frame()
                    .inner_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.label(
                                RichText::new(state.date_label().to_uppercase())
                                    .strong()
                                    .size(10.5)
                                    .color(ACCENT_GOLD),
                            );
                            let status_lbl = if state.paused {
                                format!("DÍA {} • PAUSA", state.total_days)
                            } else {
                                format!("DÍA {} • EN VIVO", state.total_days)
                            };
                            ui.label(
                                RichText::new(status_lbl)
                                    .size(8.5)
                                    .color(if state.paused {
                                        TEXT_MUTED
                                    } else {
                                        ACCENT_EMERALD
                                    }),
                            );
                        });
                    });
            },
        );
    });

    action
}

fn stat_slot(ui: &mut Ui, icon: GameIcon, label: &str, value: &str, color: Color32) {
    well_frame()
        .inner_margin(Margin::symmetric(5, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let (ic_rect, _) = ui.allocate_exact_size(Vec2::splat(11.5), Sense::hover());
                paint_icon(ui.painter(), ic_rect, icon, color);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(label).size(8.5).color(TEXT_MUTED));
                    ui.label(RichText::new(value).strong().size(10.5).color(color));
                });
            });
        });
}

/// Barra inferior horizontal fija para pantallas administrativas.
pub fn render_bottom_nav(ui: &mut Ui, current: ScreenRoute) -> Option<ScreenRoute> {
    let mut next_route = None;
    panel_frame(false)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            let scroll = egui::ScrollArea::horizontal().id_salt("bottom_nav_scroll");
            next_route = render_bottom_nav_inner(ui, current, scroll, ui.available_width());
        });
    next_route
}

/// Dock inferior flotante estilo Gran Estrategia sobre el mapa 3D a pantalla completa.
pub fn render_floating_bottom_nav(
    ctx: &egui::Context,
    current: ScreenRoute,
) -> (Option<ScreenRoute>, Rect) {
    let mut next_route = None;
    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 20.0 - (SAFE_RIGHT_PAD as f32)).clamp(320.0, 780.0);
    let right_offset = -((SAFE_RIGHT_PAD as f32) * 0.45);

    let area_resp = egui::Area::new(egui::Id::new("floating_bottom_nav_area"))
        .anchor(egui::Align2::CENTER_BOTTOM, [right_offset, -4.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_max_width(max_w);
            let frame_resp = floating_dock_frame()
                .inner_margin(Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    let scroll =
                        egui::ScrollArea::horizontal().id_salt("floating_bottom_nav_scroll");
                    next_route = render_bottom_nav_inner(ui, current, scroll, max_w - 14.0);
                });
            draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
        });

    (next_route, area_resp.response.rect)
}

fn render_bottom_nav_inner(
    ui: &mut Ui,
    current: ScreenRoute,
    scroll: egui::ScrollArea,
    avail_w: f32,
) -> Option<ScreenRoute> {
    let mut next_route = None;
    let tabs = [
        (ScreenRoute::WorldMap, GameIcon::Map, "Mapa"),
        (ScreenRoute::Economy, GameIcon::Economy, "Economía"),
        (ScreenRoute::Companies, GameIcon::Companies, "Empresas"),
        (ScreenRoute::Politics, GameIcon::Politics, "Política"),
        (ScreenRoute::Diplomacy, GameIcon::Diplomacy, "Diplomacia"),
        (ScreenRoute::Military, GameIcon::Military, "Defensa"),
        (ScreenRoute::Settings, GameIcon::Settings, "Opciones"),
    ];

    ui.set_height(31.0);
    ui.spacing_mut().scroll.bar_width = 3.0;
    let tab_w = ((avail_w.max(320.0) - 42.0) / 7.0).clamp(80.0, 114.0);
    scroll
        .max_height(32.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                for (route, icon, title) in tabs {
                    let active = current == route;
                    if game_button(
                        ui,
                        Vec2::new(tab_w, 29.0),
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

    next_route
}

/// Popup modal estilo Despacho Diplomático / Periódico Victoria 2 para eventos de Lua 5.4.
pub fn render_event_popup(ctx: &egui::Context, event: &GameEvent) -> Option<usize> {
    let mut chosen_idx = None;
    let screen = ctx.screen_rect();
    let max_scroll_h = (screen.height() - 122.0).clamp(130.0, 235.0);
    let win_w = (screen.width() - (SAFE_RIGHT_PAD as f32) - 40.0).clamp(320.0, 460.0);

    // Telón translúcido bloqueante para que el evento quede limpio por encima de las barras
    egui::Area::new(egui::Id::new("event_popup_scrim"))
        .fixed_pos(screen.min)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(
                rect,
                CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(6, 10, 16, 145),
            );
        });

    egui::Window::new(RichText::new("DESPACHO DE GOBIERNO").strong().color(ACCENT_GOLD))
        .order(egui::Order::Tooltip)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [-((SAFE_RIGHT_PAD as f32) * 0.45), 0.0])
        .default_width(win_w)
        .max_width(win_w)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(BG_PANEL)
                .stroke(Stroke::new(2.0_f32, BORDER_BRASS))
                .corner_radius(CornerRadius::same(5)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("event_popup_scroll")
                .max_height(max_scroll_h)
                .show(ui, |ui| {
                    let btn_w = ui.available_width().min(win_w - 16.0);
                    egui::Frame::new()
                        .fill(BG_WELL)
                        .stroke(Stroke::new(1.0_f32, BORDER_BRASS_DIM))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (ic_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                paint_icon(ui.painter(), ic_rect, GameIcon::Dispatch, ACCENT_GOLD);
                                ui.label(
                                    RichText::new(&event.title)
                                        .strong()
                                        .size(14.5)
                                        .color(ACCENT_GOLD),
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(&event.description)
                                    .size(12.5)
                                    .color(TEXT_PRIMARY),
                            );
                        });
                    ui.add_space(8.0);

                    for (idx, opt) in event.options.iter().enumerate() {
                        let summary = format!(
                            "{}\n  PIB {:+.2}%  |  Inflación {:+.2}%  |  Estabilidad {:+.1}%",
                            opt.label,
                            opt.gdp_growth_delta,
                            opt.inflation_delta,
                            opt.stability_delta
                        );
                        let resp = egui::Button::new(
                            RichText::new(summary).size(12.0).color(TEXT_PRIMARY),
                        )
                        .fill(BG_CARD)
                        .stroke(Stroke::new(1.5_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(4));
                        if ui.add_sized([btn_w, 42.0], resp).clicked() {
                            chosen_idx = Some(idx);
                        }
                        ui.add_space(4.0);
                    }
                });
        });

    let _ = (TEXT_DARK, BG_PANEL);
    chosen_idx
}
