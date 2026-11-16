//! Propósito del archivo:
//! Coordinador raíz de la interfaz móvil en Rust (`HorizonteLatamApp`).
//! Conecta `eframe::App` (`wgpu`) con el `SimulationWorker` en segundo plano mediante
//! canales sin bloqueo (`crossbeam_channel`), gestiona la simulación continua en tiempo real,
//! coordina la pantalla de Empresas/Bolsa, el reclutamiento y marcha de tropas por estados
//! y los popups de eventos geopolíticos.

pub mod components;
pub mod screens;
pub mod theme;

use crate::engine::{apply_player_action, ensure_fallback_graph, SimResponse, SimulationWorker};
use crate::lua_bridge::LuaEngine;
use crate::map::Latam3dScene;
use crate::models::{
    CapitalMarkerData, Difficulty, GameState, MapMode, PlayerAction, ProvinceData, ProvinceGraph,
    ScreenRoute,
};
use crate::ui::components::chrome::{
    render_bottom_nav, render_event_popup, render_floating_bottom_nav, render_floating_top_hud,
    render_top_hud, TopBarAction,
};
use crate::ui::components::map_canvas::MapViewportState;
use crate::ui::screens::companies_screen::{render_companies_screen, CompaniesViewState};
use crate::ui::screens::country_select::{render_country_select, CountrySelectAction};
use crate::ui::screens::diplomacy_screen::render_diplomacy_screen;
use crate::ui::screens::economy_screen::render_economy_screen;
use crate::ui::screens::main_menu::{render_main_menu, MainMenuAction};
use crate::ui::screens::map_screen::render_map_screen;
use crate::ui::screens::military_screen::{render_military_screen, MilitaryScreenAction};
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
    pub province_graph: Arc<ProvinceGraph>,
    pub capitals_catalog: Vec<CapitalMarkerData>,
    pub state: GameState,
    pub worker: SimulationWorker,
    pub map_mode: MapMode,
    pub map_viewport: MapViewportState,
    pub companies_view: CompaniesViewState,
    pub selected_difficulty: Difficulty,
    pub show_country_popup: bool,
    pub last_tick_instant: Instant,
    pub day_accumulator: f64,
    pub pending_actions: Vec<PlayerAction>,
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

        let graph = scene_3d
            .as_ref()
            .map(|s| s.build_province_graph(&provinces_catalog))
            .unwrap_or_default();
        let province_graph = Arc::new(graph);

        let mut worker = SimulationWorker::spawn(Arc::clone(&province_graph));
        worker.request_new_campaign(Difficulty::Normal);

        let placeholder_state = GameState {
            total_days: 1,
            year: 2026,
            month: 1,
            day: 1,
            paused: true,
            speed_level: 1,
            difficulty: Difficulty::Normal,
            player_country_id: None,
            selected_country_id: Some("MX".to_string()),
            selected_province_mesh: None,
            selected_unit_id: None,
            countries: BTreeMap::new(),
            relations: BTreeMap::new(),
            units: Vec::new(),
            next_unit_id: 1,
            companies: Vec::new(),
            next_company_id: 1,
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
            province_graph,
            capitals_catalog,
            state: placeholder_state,
            worker,
            map_mode: MapMode::Political,
            map_viewport: MapViewportState::default(),
            companies_view: CompaniesViewState::default(),
            selected_difficulty: Difficulty::Normal,
            show_country_popup: false,
            last_tick_instant: Instant::now(),
            day_accumulator: 0.0,
            pending_actions: Vec::new(),
            last_error: startup_err,
            theme_initialized: false,
        }
    }

    fn dispatch_player_action(&mut self, action: PlayerAction) {
        apply_player_action(&mut self.state, &self.province_graph, &action);
        if self.worker.is_busy {
            self.pending_actions.push(action);
        }
    }

    fn poll_worker_responses(&mut self) {
        while let Ok(msg) = self.worker.rx.try_recv() {
            self.worker.is_busy = false;
            match msg {
                SimResponse::CampaignReady(new_state) => {
                    self.state = *new_state;
                    self.pending_actions.clear();
                    self.day_accumulator = 0.0;

                    let mut graph_mut = (*self.province_graph).clone();
                    let prev_len = graph_mut.len();
                    ensure_fallback_graph(&self.state.countries, &mut graph_mut);
                    if graph_mut.len() != prev_len {
                        self.province_graph = Arc::new(graph_mut);
                        self.worker
                            .set_province_graph(Arc::clone(&self.province_graph));
                    }
                }
                SimResponse::TickCompleted(updated) => {
                    let prev_selected = self.state.selected_country_id.clone();
                    let prev_prov = self.state.selected_province_mesh.clone();
                    let prev_unit = self.state.selected_unit_id;
                    let prev_paused = self.state.paused;
                    let prev_speed = self.state.speed_level;
                    let prev_diff = self.state.difficulty;

                    self.state = *updated;
                    self.state.paused = prev_paused;
                    self.state.speed_level = prev_speed;
                    self.state.difficulty = prev_diff;
                    if self.state.selected_country_id.is_none() {
                        self.state.selected_country_id = prev_selected;
                    }
                    if self.state.selected_province_mesh.is_none() {
                        self.state.selected_province_mesh = prev_prov;
                    }
                    if self.state.selected_unit_id.is_none() {
                        self.state.selected_unit_id = prev_unit;
                    }

                    // Reaplicar órdenes tácticas emitidas mientras el hilo estaba ocupado
                    let replay = std::mem::take(&mut self.pending_actions);
                    for act in replay {
                        apply_player_action(&mut self.state, &self.province_graph, &act);
                    }
                }
                SimResponse::Error(err) => {
                    self.last_error = Some(err);
                }
            }
        }
    }

    fn maybe_advance_realtime(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let dt = (now - self.last_tick_instant).as_secs_f64().min(0.25);
        self.last_tick_instant = now;

        if self.state.paused
            || self.state.active_event.is_some()
            || self.state.player_country_id.is_none()
        {
            self.day_accumulator = 0.0;
            return;
        }

        let days_per_sec = match self.state.speed_level {
            3 => 6.0,
            2 => 2.8,
            _ => 1.2,
        };
        self.day_accumulator += dt * days_per_sec;

        if self.day_accumulator >= 1.0 && !self.worker.is_busy {
            let days_to_step = (self.day_accumulator.floor() as u32).clamp(1, 3);
            self.day_accumulator -= days_to_step as f64;
            self.worker.request_advance_days(&self.state, days_to_step);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
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

        let in_campaign = !matches!(self.route, ScreenRoute::MainMenu | ScreenRoute::CountrySelect)
            && self.state.player_country_id.is_some();
        let fullscreen_map = in_campaign && self.route == ScreenRoute::WorldMap;

        // En pantallas administrativas usamos paneles fijos; en Mapa usamos pantalla completa borde a borde
        if in_campaign && !fullscreen_map {
            let hud_frame = egui::Frame::new()
                .fill(crate::ui::theme::BG_DEEP)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: crate::ui::theme::SAFE_RIGHT_PAD,
                    top: 3,
                    bottom: 2,
                });
            egui::TopBottomPanel::top("top_hud_panel")
                .exact_height(54.0)
                .resizable(false)
                .frame(hud_frame)
                .show(ctx, |ui| {
                    match render_top_hud(ui, &self.state, self.worker.is_busy) {
                        TopBarAction::None => {}
                        TopBarAction::TogglePause => {
                            self.state.paused = !self.state.paused;
                            self.last_tick_instant = Instant::now();
                        }
                        TopBarAction::SetSpeed(spd) => {
                            self.state.speed_level = spd;
                        }
                    }
                });

            let nav_frame = egui::Frame::new()
                .fill(crate::ui::theme::BG_DEEP)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: crate::ui::theme::SAFE_RIGHT_PAD,
                    top: 2,
                    bottom: 3,
                });
            egui::TopBottomPanel::bottom("bottom_nav_panel")
                .exact_height(50.0)
                .resizable(false)
                .frame(nav_frame)
                .show(ctx, |ui| {
                    if let Some(next_route) = render_bottom_nav(ui, self.route) {
                        self.route = next_route;
                    }
                });
        }

        // Contenido principal según la pantalla activa (margen cero en Mapa para 100% pantalla completa)
        let central_frame = if fullscreen_map {
            egui::Frame::central_panel(&ctx.style())
                .fill(crate::ui::theme::BG_DEEP)
                .inner_margin(egui::Margin::ZERO)
        } else {
            egui::Frame::central_panel(&ctx.style())
                .fill(crate::ui::theme::BG_DEEP)
                .inner_margin(egui::Margin {
                    left: 10,
                    right: crate::ui::theme::SAFE_RIGHT_PAD,
                    top: 6,
                    bottom: 6,
                })
        };
        let mut next_hud_rects = Vec::new();
        egui::CentralPanel::default().frame(central_frame).show(ctx, |ui| {
            if !fullscreen_map {
                if let Some(err) = &self.last_error {
                    ui.colored_label(egui::Color32::RED, format!("Aviso del motor: {err}"));
                }
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
                            self.state.paused = false;
                            self.last_tick_instant = Instant::now();
                            self.state.push_chronicle(
                                country_id,
                                format!("Has asumido el liderazgo de {name} en tiempo real."),
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
                    next_hud_rects.extend(res.overlay_rects);
                    if let Some((unit_id, target_province)) = res.order_unit_move {
                        self.dispatch_player_action(PlayerAction::OrderUnitMove {
                            unit_id,
                            target_province,
                        });
                    }
                    if let Some(target_id) = res.open_diplomacy_with {
                        self.state.selected_country_id = Some(target_id);
                        self.route = ScreenRoute::Diplomacy;
                    }
                }
                ScreenRoute::Economy => {
                    if render_economy_screen(ui, &mut self.state) {
                        self.route = ScreenRoute::Companies;
                    }
                }
                ScreenRoute::Companies => {
                    render_companies_screen(ui, &self.state, &mut self.companies_view);
                }
                ScreenRoute::Politics => render_politics_screen(ui, &mut self.state),
                ScreenRoute::Diplomacy => render_diplomacy_screen(ui, &mut self.state),
                ScreenRoute::Military => {
                    match render_military_screen(ui, &mut self.state, &self.province_graph) {
                        MilitaryScreenAction::None => {}
                        MilitaryScreenAction::PlayerAction(act) => {
                            self.dispatch_player_action(act);
                        }
                        MilitaryScreenAction::OpenMapForUnit(uid) => {
                            self.state.selected_unit_id = Some(uid);
                            self.route = ScreenRoute::WorldMap;
                        }
                    }
                }
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

        // En modo mapa pantalla completa, flotamos el HUD superior y dock inferior encima del 3D
        if fullscreen_map {
            let (top_act, top_rect) = render_floating_top_hud(ctx, &self.state, self.worker.is_busy);
            next_hud_rects.push(top_rect);
            match top_act {
                TopBarAction::None => {}
                TopBarAction::TogglePause => {
                    self.state.paused = !self.state.paused;
                    self.last_tick_instant = Instant::now();
                }
                TopBarAction::SetSpeed(spd) => {
                    self.state.speed_level = spd;
                }
            }

            let (next_route, nav_rect) = render_floating_bottom_nav(ctx, self.route);
            next_hud_rects.push(nav_rect);
            if let Some(route) = next_route {
                self.route = route;
            }

            if let Some(err) = self.last_error.clone() {
                let err_resp = egui::Area::new(egui::Id::new("fullscreen_map_error_toast"))
                    .anchor(egui::Align2::CENTER_TOP, [-16.0, 102.0])
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        crate::ui::theme::floating_hud_frame(false)
                            .stroke(egui::Stroke::new(1.5_f32, crate::ui::theme::ACCENT_CRIMSON))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("Aviso del motor: {err}"))
                                            .size(11.5)
                                            .color(crate::ui::theme::ACCENT_CRIMSON),
                                    );
                                    if crate::ui::theme::game_button(
                                        ui,
                                        egui::Vec2::new(24.0, 20.0),
                                        "X",
                                        None,
                                        false,
                                        Some(crate::ui::theme::ACCENT_CRIMSON),
                                    )
                                    .clicked()
                                    {
                                        self.last_error = None;
                                    }
                                });
                            });
                    });
                next_hud_rects.push(err_resp.response.rect);
            }
        }
        self.map_viewport.hud_rects = next_hud_rects;

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

        // Avance continuo en tiempo real al final del frame para no perder interacciones del usuario
        self.maybe_advance_realtime(ctx);
    }
}
