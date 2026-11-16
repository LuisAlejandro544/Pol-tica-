//! Propósito del archivo:
//! Coordinador raíz de la interfaz móvil en Rust (`HorizonteLatamApp`).
//! Conecta `eframe::App` con el `SimulationWorker` en segundo plano mediante
//! canales sin bloqueo (`crossbeam_channel`), gestiona la navegación multipantalla,
//! los popups de eventos Lua 5.4 y el temporizador automático de turnos según velocidad.

pub mod components;
pub mod screens;
pub mod theme;

use crate::engine::{SimResponse, SimulationWorker};
use crate::map::LatamVectorMap;
use crate::models::{Difficulty, GameState, MapMode, ScreenRoute};
use crate::ui::components::chrome::{
    render_bottom_nav, render_event_popup, render_top_hud, TopBarAction,
};
use crate::ui::components::map_canvas::MapViewportState;
use crate::ui::screens::country_select::{render_country_select, CountrySelectAction};
use crate::ui::screens::diplomacy_screen::render_diplomacy_screen;
use crate::ui::screens::economy_screen::render_economy_screen;
use crate::ui::screens::main_menu::{render_main_menu, MainMenuAction};
use crate::ui::screens::map_screen::render_map_screen;
use crate::ui::screens::military_screen::render_military_screen;
use crate::ui::screens::politics_screen::render_politics_screen;
use crate::ui::screens::settings_screen::{render_settings_screen, SettingsAction};
use crate::ui::theme::apply_tactical_theme;
use std::collections::BTreeMap;
use std::time::Instant;

pub struct HorizonteLatamApp {
    pub route: ScreenRoute,
    pub vector_map: LatamVectorMap,
    pub state: GameState,
    pub worker: SimulationWorker,
    pub map_mode: MapMode,
    pub map_viewport: MapViewportState,
    pub selected_difficulty: Difficulty,
    pub show_country_popup: bool,
    pub last_tick_instant: Instant,
    pub last_error: Option<String>,
    theme_initialized: bool,
}

impl HorizonteLatamApp {
    pub fn new() -> Self {
        let vector_map = LatamVectorMap::load_embedded()
            .expect("No se pudo cargar assets/maps/latinoamerica.svg");
        let mut worker = SimulationWorker::spawn();
        worker.request_new_campaign(Difficulty::Normal);

        let placeholder_state = GameState {
            turn: 1,
            year: 2026,
            month: 1,
            paused: true,
            speed_level: 1,
            difficulty: Difficulty::Normal,
            player_country_id: None,
            selected_country_id: Some("MX".to_string()),
            countries: BTreeMap::new(),
            relations: BTreeMap::new(),
            active_event: None,
            chronicle: Vec::new(),
        };

        Self {
            route: ScreenRoute::MainMenu,
            vector_map,
            state: placeholder_state,
            worker,
            map_mode: MapMode::Political,
            map_viewport: MapViewportState::default(),
            selected_difficulty: Difficulty::Normal,
            show_country_popup: false,
            last_tick_instant: Instant::now(),
            last_error: None,
            theme_initialized: false,
        }
    }

    fn poll_worker_responses(&mut self) {
        while let Ok(msg) = self.worker.rx.try_recv() {
            self.worker.is_busy = false;
            match msg {
                SimResponse::CampaignReady(new_state) => {
                    let keep_player = self.state.player_country_id.clone();
                    self.state = *new_state;
                    if self.route == ScreenRoute::MainMenu {
                        let _ = keep_player;
                    }
                }
                SimResponse::TurnCompleted(updated) => {
                    let prev_selected = self.state.selected_country_id.clone();
                    self.state = *updated;
                    if self.state.selected_country_id.is_none() {
                        self.state.selected_country_id = prev_selected;
                    }
                }
                SimResponse::Error(err) => {
                    self.last_error = Some(err);
                }
            }
        }
    }

    fn maybe_auto_advance_turn(&mut self, ctx: &egui::Context) {
        if self.state.paused || self.worker.is_busy || self.state.active_event.is_some() {
            return;
        }
        if self.state.player_country_id.is_none() {
            return;
        }
        let interval_secs = match self.state.speed_level {
            3 => 0.9,
            2 => 1.8,
            _ => 3.2,
        };
        if self.last_tick_instant.elapsed().as_secs_f32() >= interval_secs {
            self.last_tick_instant = Instant::now();
            self.worker.request_advance_turn(&self.state);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(120));
    }
}

impl eframe::App for HorizonteLatamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_initialized {
            apply_tactical_theme(ctx);
            self.theme_initialized = true;
        }

        self.poll_worker_responses();
        self.maybe_auto_advance_turn(ctx);

        let in_campaign = !matches!(self.route, ScreenRoute::MainMenu | ScreenRoute::CountrySelect)
            && self.state.player_country_id.is_some();

        // Barra superior persistente durante la campaña
        if in_campaign {
            egui::TopBottomPanel::top("top_hud_panel").show(ctx, |ui| {
                match render_top_hud(ui, &self.state, self.worker.is_busy) {
                    TopBarAction::None => {}
                    TopBarAction::TogglePause => {
                        self.state.paused = !self.state.paused;
                        self.last_tick_instant = Instant::now();
                    }
                    TopBarAction::AdvanceOneTurn => {
                        self.worker.request_advance_turn(&self.state);
                    }
                    TopBarAction::SetSpeed(spd) => {
                        self.state.speed_level = spd;
                    }
                }
            });

            // Barra inferior de navegación entre pantallas independientes
            egui::TopBottomPanel::bottom("bottom_nav_panel").show(ctx, |ui| {
                if let Some(next_route) = render_bottom_nav(ui, self.route) {
                    self.route = next_route;
                }
            });
        }

        // Contenido principal según la pantalla activa
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.last_error {
                ui.colored_label(egui::Color32::RED, format!("Aviso del motor: {err}"));
            }

            match self.route {
                ScreenRoute::MainMenu => {
                    let has_campaign = self.state.player_country_id.is_some();
                    match render_main_menu(
                        ui,
                        &mut self.selected_difficulty,
                        has_campaign,
                        &self.state,
                    ) {
                        MainMenuAction::None => {}
                        MainMenuAction::StartNewCampaign(diff) => {
                            self.worker.request_new_campaign(diff);
                            self.route = ScreenRoute::CountrySelect;
                        }
                        MainMenuAction::ResumeCampaign => {
                            self.route = ScreenRoute::WorldMap;
                        }
                        MainMenuAction::OpenSettings => {
                            self.route = ScreenRoute::Settings;
                        }
                    }
                }
                ScreenRoute::CountrySelect => {
                    match render_country_select(
                        ui,
                        &self.vector_map,
                        &mut self.state,
                        &mut self.map_viewport,
                    ) {
                        CountrySelectAction::None => {}
                        CountrySelectAction::BackToMenu => {
                            self.route = ScreenRoute::MainMenu;
                        }
                        CountrySelectAction::ConfirmCountry(country_id) => {
                            let name = self
                                .state
                                .countries
                                .get(&country_id)
                                .map(|c| c.name.clone())
                                .unwrap_or_else(|| country_id.clone());
                            self.state.player_country_id = Some(country_id.clone());
                            self.state.selected_country_id = Some(country_id.clone());
                            self.state.push_chronicle(
                                country_id,
                                format!("Has asumido el liderazgo de {name}."),
                            );
                            self.route = ScreenRoute::WorldMap;
                        }
                    }
                }
                ScreenRoute::WorldMap => {
                    let res = render_map_screen(
                        ui,
                        &self.vector_map,
                        &mut self.state,
                        &mut self.map_mode,
                        &mut self.map_viewport,
                        &mut self.show_country_popup,
                    );
                    if let Some(target_id) = res.open_diplomacy_with {
                        self.state.selected_country_id = Some(target_id);
                        self.route = ScreenRoute::Diplomacy;
                    }
                }
                ScreenRoute::Economy => render_economy_screen(ui, &mut self.state),
                ScreenRoute::Politics => render_politics_screen(ui, &mut self.state),
                ScreenRoute::Diplomacy => render_diplomacy_screen(ui, &mut self.state),
                ScreenRoute::Military => render_military_screen(ui, &mut self.state),
                ScreenRoute::Settings => match render_settings_screen(ui, &mut self.state) {
                    SettingsAction::None => {}
                    SettingsAction::ReturnToMap => {
                        self.route = ScreenRoute::WorldMap;
                    }
                    SettingsAction::RestartSelectCountry => {
                        self.worker.request_new_campaign(self.state.difficulty);
                        self.route = ScreenRoute::CountrySelect;
                    }
                    SettingsAction::ReturnToMainMenu => {
                        self.route = ScreenRoute::MainMenu;
                    }
                },
            }
        });

        // Popup modal de evento geopolítico generado por Lua 5.4
        if let Some(ev) = self.state.active_event.clone() {
            if let Some(opt_idx) = render_event_popup(ctx, &ev) {
                if let Some(opt) = ev.options.get(opt_idx) {
                    if let Some(pid) = self.state.player_country_id.clone() {
                        if let Some(country) = self.state.countries.get_mut(&pid) {
                            let treasury_delta =
                                country.economy.gdp_usd_millions * opt.treasury_delta_pct;
                            country.economy.treasury_usd_millions =
                                (country.economy.treasury_usd_millions + treasury_delta).max(0.0);
                            country.economy.gdp_growth_pct += opt.gdp_growth_delta;
                            country.economy.inflation_pct =
                                (country.economy.inflation_pct + opt.inflation_delta).max(0.4);
                            country.stability =
                                (country.stability + opt.stability_delta).clamp(10.0, 100.0);
                        }
                        self.state.push_chronicle(
                            pid,
                            format!("Evento '{}' -> {}", ev.title, opt.label),
                        );
                    }
                }
                self.state.active_event = None;
            }
        }
    }
}
