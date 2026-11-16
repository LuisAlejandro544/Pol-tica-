//! Propósito del archivo:
//! Pantalla dedicada de Carga y Preparación de Campaña (`CampaignLoading`).
//! Se muestra tras seleccionar una nación en `CountrySelect` mientras el hilo de simulación
//! (`SimulationWorker`) configura las unidades/soldados predeterminados, despliega los regimientos
//! en el grafo provincial y forma empresas privadas aleatorias en distintos países de Latinoamérica.

use crate::models::GameState;
use crate::ui::theme::{
    card_frame, draw_corner_rivets, draw_country_shield, draw_gauge_bar, draw_section_header,
    game_button, panel_frame, well_frame, GameIcon, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_GOLD,
    BORDER_BRASS, BORDER_BRASS_DIM, TEXT_MUTED, TEXT_PRIMARY,
};
use egui::{Align, Layout, RichText, Ui, Vec2};

#[derive(Debug, Clone)]
pub struct CampaignLoadingState {
    pub target_country_id: String,
    pub worker_progress: f32,
    pub displayed_progress: f32,
    pub stage: String,
    pub detail: String,
    pub units_configured: usize,
    pub companies_founded: usize,
    pub ready_state: Option<Box<GameState>>,
    pub settle_timer_secs: f32,
}

impl Default for CampaignLoadingState {
    fn default() -> Self {
        Self {
            target_country_id: "MX".to_string(),
            worker_progress: 0.08,
            displayed_progress: 0.05,
            stage: "Inicializando campaña...".to_string(),
            detail: "Preparando simulación geopolítica y tablero regional.".to_string(),
            units_configured: 0,
            companies_founded: 0,
            ready_state: None,
            settle_timer_secs: 0.0,
        }
    }
}

impl CampaignLoadingState {
    pub fn begin(country_id: String) -> Self {
        Self {
            target_country_id: country_id,
            worker_progress: 0.12,
            displayed_progress: 0.06,
            stage: "Conectando estado mayor...".to_string(),
            detail: "Calculando soldados predeterminados y empresas aleatorias regionales."
                .to_string(),
            units_configured: 0,
            companies_founded: 0,
            ready_state: None,
            settle_timer_secs: 0.0,
        }
    }
}

pub enum LoadingScreenAction {
    None,
    EnterWorldMap,
}

pub fn render_campaign_loading_screen(
    ui: &mut Ui,
    preview_state: &GameState,
    loading: &mut CampaignLoadingState,
    dt_secs: f32,
) -> LoadingScreenAction {
    let mut action = LoadingScreenAction::None;

    let target = if loading.ready_state.is_some() {
        1.0_f32
    } else {
        loading.worker_progress.clamp(0.10, 0.96)
    };
    if loading.displayed_progress < target {
        loading.displayed_progress = (loading.displayed_progress + dt_secs * 1.15).min(target);
    }

    if loading.ready_state.is_some() && loading.displayed_progress >= 0.995 {
        loading.settle_timer_secs += dt_secs;
        if loading.settle_timer_secs >= 0.30 {
            return LoadingScreenAction::EnterWorldMap;
        }
    }

    let country_opt = preview_state.countries.get(&loading.target_country_id);
    let avail = ui.available_size();
    let card_w = (avail.x - 40.0).clamp(360.0, 660.0);

    ui.centered_and_justified(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(card_w, 300.0_f32.min((avail.y - 20.0).max(220.0))),
            Layout::top_down(Align::Center),
            |ui| {
                let frame_resp = panel_frame(true).inner_margin(16.0).show(ui, |ui| {
                    ui.set_width((card_w - 32.0).max(300.0));

                    ui.horizontal(|ui| {
                        if let Some(c) = country_opt {
                            draw_country_shield(ui, Vec2::new(46.0, 38.0), c);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(
                                    RichText::new(format!(
                                        "CONFIGURANDO CAMPAÑA — {}",
                                        c.name.to_uppercase()
                                    ))
                                    .strong()
                                    .size(17.0)
                                    .color(ACCENT_GOLD),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Capital: {}  •  Región: {}  •  Dificultad: {:?}",
                                        c.capital, c.region, preview_state.difficulty
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                        } else {
                            ui.label(
                                RichText::new("CONFIGURANDO CAMPAÑA REGIONAL")
                                    .strong()
                                    .size(17.0)
                                    .color(ACCENT_GOLD),
                                );
                        }
                    });

                    ui.add_space(10.0);

                    card_frame(Some(BORDER_BRASS_DIM)).inner_margin(10.0).show(ui, |ui| {
                        draw_section_header(
                            ui,
                            GameIcon::Dispatch,
                            &loading.stage.to_uppercase(),
                            ACCENT_CYAN,
                        );
                        ui.label(
                            RichText::new(&loading.detail)
                                .size(12.5)
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(8.0);
                        draw_gauge_bar(
                            ui,
                            ui.available_width(),
                            22.0,
                            loading.displayed_progress,
                            if loading.ready_state.is_some() {
                                ACCENT_EMERALD
                            } else {
                                ACCENT_CYAN
                            },
                            &format!("{:.0}%", loading.displayed_progress * 100.0),
                        );
                    });

                    ui.add_space(8.0);

                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            well_frame().inner_margin(8.0).show(ui, |ui| {
                                ui.label(
                                    RichText::new("FUERZAS INICIALES")
                                        .strong()
                                        .size(11.0)
                                        .color(ACCENT_GOLD),
                                );
                                let units_lbl = if loading.units_configured > 0 {
                                    format!("{} unidades desplegadas", loading.units_configured)
                                } else {
                                    "Asignando guarniciones...".to_string()
                                };
                                ui.label(RichText::new(units_lbl).size(12.0).color(TEXT_PRIMARY));
                            });
                        });
                        cols[1].vertical(|ui| {
                            well_frame().inner_margin(8.0).show(ui, |ui| {
                                ui.label(
                                    RichText::new("EMPRESAS ALEATORIAS")
                                        .strong()
                                        .size(11.0)
                                        .color(ACCENT_EMERALD),
                                );
                                let comp_lbl = if loading.companies_founded > 0 {
                                    format!(
                                        "{} compañías en plazas elegidas",
                                        loading.companies_founded
                                    )
                                } else {
                                    "Evaluando mercados...".to_string()
                                };
                                ui.label(RichText::new(comp_lbl).size(12.0).color(TEXT_PRIMARY));
                            });
                        });
                    });

                    ui.add_space(10.0);

                    if loading.ready_state.is_some() {
                        if game_button(
                            ui,
                            Vec2::new(ui.available_width(), 38.0),
                            "DESPLEGAR EN EL MAPA >",
                            Some(GameIcon::Map),
                            true,
                            Some(ACCENT_GOLD),
                        )
                        .clicked()
                        {
                            action = LoadingScreenAction::EnterWorldMap;
                        }
                    } else {
                        ui.label(
                            RichText::new(
                                "Configurando tropas predeterminadas y empresas privadas en segundo plano...",
                            )
                            .size(11.0)
                            .color(TEXT_MUTED),
                        );
                    }
                });
                draw_corner_rivets(ui.painter(), frame_resp.response.rect, BORDER_BRASS);
            },
        );
    });

    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(33));

    action
}
