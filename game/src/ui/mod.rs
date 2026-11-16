//! Propósito del archivo:
//! Coordinador raíz de la interfaz móvil en Rust (`HorizonteLatamApp`).
//! Conecta `eframe::App` (`wgpu`) con el `SimulationWorker` en segundo plano mediante
//! canales sin bloqueo (`crossbeam_channel`), carga el modelo 3D (`latinoamerica_3d.glb`)
//! y el catálogo provincial Lua 5.4 (`provinces_latam.lua`), y gestiona la navegación
//! multipantalla y los popups de eventos.

pub mod components;
pub mod screens;
pub mod theme;

use crate::engine::{SimResponse, SimulationWorker};
use crate::lua_bridge::LuaEngine;
use crate::map::Latam3dScene;
use crate::models::{
    CapitalMarkerData, Difficulty, GameState, MapMode, ProvinceData, ScreenRoute,
};
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
use eframe::egui_wgpu::wgpu;
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

pub struct HorizonteLatamApp {
    pub route: ScreenRoute,
    pub scene_3d: Option<Latam3dScene>,
    pub target_format: wgpu::TextureFormat,
    pub gpu_info: String,
    pub gpu_3d_error: Arc<Mutex<Option<String>>>,
    pub provinces_catalog: BTreeMap<String, ProvinceData>,
    pub capitals_catalog: Vec<CapitalMarkerData>,
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
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (target_format, gpu_info) = cc
            .wgpu_render_state
            .as_ref()
            .map(|rs| {
                let info = rs.adapter.get_info();
                (
                    rs.target_format,
                    format!("{} ({:?})", info.name, info.backend),
                )
            })
            .unwrap_or_else(|| {
                (
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    "wgpu pendiente de superficie".to_string(),
                )
            });

        let mut startup_err = None;
        let scene_3d = match Latam3dScene::load_embedded_or_disk() {
            Ok(scene) => scene,
            Err(err) => {
                log::error!("No se pudo inicializar el mapa 3D GLB: {err}");
                startup_err = Some(err);
                None
            }
        };

        let (provinces_catalog, capitals_catalog) = LuaEngine::new()
            .and_then(|lua| lua.load_provinces_and_capitals())
            .unwrap_or_else(|err| {
                log::warn!("No se pudo cargar provinces_latam.lua: {err}");
                (BTreeMap::new(), Vec::new())
            });

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
            selected_province_mesh: None,
            countries: BTreeMap::new(),
            relations: BTreeMap::new(),
            active_event: None,
            chronicle: Vec::new(),
        };

        Self {
            route: ScreenRoute::MainMenu,
            scene_3d,
            target_format,
            gpu_info,
            gpu_3d_error: Arc::new(Mutex::new(None)),
            provinces_catalog,
            capitals_catalog,
            state: placeholder_state,
            worker,
            map_mode: MapMode::Political,
            map_viewport: MapViewportState::default(),
            selected_difficulty: Difficulty::Normal,
            show_country_popup: false,
            last_tick_instant: Instant::now(),
            last_error: startup_err,
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
                    let prev_prov = self.state.selected_province_mesh.clone();
                    self.state = *updated;
                    if self.state.selected_country_id.is_none() {
                        self.state.selected_country_id = prev_selected;
                    }
                    if self.state.selected_province_mesh.is_none() {
                        self.state.selected_province_mesh = prev_prov;
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.theme_initialized {
            apply_tactical_theme(ctx);
            self.theme_initialized = true;
        }

        if let Some(rs) = frame.wgpu_render_state() {
            self.target_format = rs.target_format;
            if self.gpu_info.starts_with("wgpu pendiente") {
                let info = rs.adapter.get_info();
                self.gpu_info = format!("{} ({:?})", info.name, info.backend);
            }
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

        // Contenido principal según la pantalla activa (con margen seguro táctil en bordes)
        let central_frame = egui::Frame::central_panel(&ctx.style())
            .fill(crate::ui::theme::BG_DEEP)
            .inner_margin(egui::Margin::symmetric(14, 8));
        egui::CentralPanel::default().frame(central_frame).show(ctx, |ui| {
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
                        self.scene_3d.as_ref(),
                        self.target_format,
                        &self.gpu_info,
                        &self.gpu_3d_error,
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
                            if let Some(scene) = self.scene_3d.as_ref() {
                                if let Some(center) = scene.country_center(&country_id) {
                                    self.map_viewport.camera_3d.focus_point(center, 28.0);
                                }
                            }
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
                        self.scene_3d.as_ref(),
                        self.target_format,
                        &self.gpu_info,
                        &self.gpu_3d_error,
                        &self.provinces_catalog,
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
                ScreenRoute::Settings => {
                    match render_settings_screen(
                        ui,
                        &mut self.state,
                        &self.gpu_info,
                        self.target_format,
                    ) {
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
                    }
                }
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
