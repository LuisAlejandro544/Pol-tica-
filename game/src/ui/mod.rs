//! Propósito del archivo:
//! Coordinador raíz de la interfaz móvil en Rust (`HorizonteLatamApp`).
//! Conecta `eframe::App` (`wgpu`) con el `SimulationWorker` en segundo plano mediante
//! canales sin bloqueo (`crossbeam_channel`), gestiona la simulación continua en tiempo real,
//! coordina la pantalla de Empresas/Bolsa, el reclutamiento y marcha de tropas por estados
//! y los popups de eventos geopolíticos.

pub mod components;
pub mod screens;
pub mod theme;

use crate::engine::{
    apply_player_action, ensure_fallback_graph, generate_runtime_seed, SimResponse,
    SimulationWorker,
};
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
use crate::ui::screens::diplomacy_screen::{
    render_diplomacy_screen, render_minister_assign_popup,
};
use crate::ui::screens::economy_screen::render_economy_screen;
use crate::ui::screens::loading_screen::{
    render_campaign_loading_screen, CampaignLoadingState, LoadingScreenAction,
};
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
    pub province_graph: Arc<ProvinceGraph>,
    pub capitals_catalog: Vec<CapitalMarkerData>,
    pub state: GameState,
    pub worker: SimulationWorker,
    pub map_mode: MapMode,
    pub map_viewport: MapViewportState,
    pub companies_view: CompaniesViewState,
    pub campaign_loading: CampaignLoadingState,
    pub selected_difficulty: Difficulty,
    pub show_country_popup: bool,
    pub show_minister_popup: bool,
    pub recruit_troops_amount: u64,
    pub last_tick_instant: Instant,
    pub last_frame_instant: Instant,
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
                let cache_key = wgpu::util::pipeline_cache_key(&info).unwrap_or_else(|| {
                    format!("wgpu_{:?}_{:x}_{:x}", info.backend, info.vendor, info.device)
                });
                crate::cache::set_wgpu_pipeline_cache_key(cache_key);
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
        let glb_bytes_opt = Latam3dScene::read_embedded_or_disk_glb_bytes();
        let prov_lua_bytes = LuaEngine::read_provinces_lua_bytes();

        let (scene_3d, provinces_catalog, capitals_catalog, graph) =
            if let Some(glb_bytes) = glb_bytes_opt.as_deref() {
                let scene_fp = crate::cache::compute_scene_fingerprint(glb_bytes, &prov_lua_bytes);
                if let Some((baked_scene, baked_provs, baked_caps, baked_graph)) =
                    crate::cache::load_baked_map_bundle(scene_fp)
                {
                    log::info!(
                        "Mapa 3D y grafo provincial cargados desde caché binaria ({} mallas).",
                        baked_scene.pick_meshes.len()
                    );
                    (Some(baked_scene), baked_provs, baked_caps, baked_graph)
                } else {
                    let loaded_scene = match Latam3dScene::from_glb_bytes(glb_bytes) {
                        Ok(scene) => Some(scene),
                        Err(err) => {
                            log::error!("No se pudo inicializar el mapa 3D GLB: {err}");
                            startup_err = Some(err);
                            None
                        }
                    };
                    let (provs, caps) = LuaEngine::new()
                        .and_then(|lua| lua.load_provinces_and_capitals())
                        .unwrap_or_else(|err| {
                            log::warn!("No se pudo cargar provinces_latam.lua: {err}");
                            (BTreeMap::new(), Vec::new())
                        });
                    let built_graph = loaded_scene
                        .as_ref()
                        .map(|s| s.build_province_graph(&provs))
                        .unwrap_or_default();

                    if let Some(scene_ref) = loaded_scene.as_ref() {
                        let scene_clone = scene_ref.clone();
                        let provs_clone = provs.clone();
                        let caps_clone = caps.clone();
                        let graph_clone = built_graph.clone();
                        let _ = std::thread::Builder::new()
                            .name("latam-map-baker".to_string())
                            .spawn(move || {
                                crate::cache::save_baked_map_bundle(
                                    scene_fp,
                                    &scene_clone,
                                    &provs_clone,
                                    &caps_clone,
                                    &graph_clone,
                                );
                            });
                    }

                    (loaded_scene, provs, caps, built_graph)
                }
            } else {
                let (provs, caps) = LuaEngine::new()
                    .and_then(|lua| lua.load_provinces_and_capitals())
                    .unwrap_or_else(|err| {
                        log::warn!("No se pudo cargar provinces_latam.lua: {err}");
                        (BTreeMap::new(), Vec::new())
                    });
                (None, provs, caps, BTreeMap::new())
            };

        let province_graph = Arc::new(graph);

        let mut worker = SimulationWorker::spawn(Arc::clone(&province_graph));
        worker.request_new_campaign(Difficulty::Normal);

        let placeholder_state = GameState {
            campaign_seed: 1,
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
            regions: BTreeMap::new(),
            relations: BTreeMap::new(),
            units: Vec::new(),
            next_unit_id: 1,
            companies: Vec::new(),
            next_company_id: 1,
            active_event: None,
            chronicle: Vec::new(),
            global_cycle: crate::models::GlobalMacroCycle::default(),
            last_event_id: None,
            last_event_day: 0,
            next_minister_id: 1,
            last_diplomacy_feedback: None,
            pending_chronicle_archive: Vec::new(),
            pending_stock_archive: Vec::new(),
            pending_macro_archive: Vec::new(),
        };

        let now = Instant::now();
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
            campaign_loading: CampaignLoadingState::default(),
            selected_difficulty: Difficulty::Normal,
            show_country_popup: false,
            show_minister_popup: false,
            recruit_troops_amount: 1_000,
            last_tick_instant: now,
            last_frame_instant: now,
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
            match msg {
                SimResponse::CampaignReady(new_state) => {
                    self.worker.is_busy = false;
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
                SimResponse::CampaignLoadProgress {
                    progress,
                    stage,
                    detail,
                    units_configured,
                    companies_founded,
                } => {
                    self.campaign_loading.worker_progress = progress;
                    self.campaign_loading.stage = stage;
                    self.campaign_loading.detail = detail;
                    self.campaign_loading.units_configured = units_configured;
                    self.campaign_loading.companies_founded = companies_founded;
                }
                SimResponse::CampaignConfigured(configured_state) => {
                    self.worker.is_busy = false;
                    self.campaign_loading.worker_progress = 1.0;
                    self.campaign_loading.units_configured = configured_state.units.len();
                    self.campaign_loading.companies_founded = configured_state.companies.len();
                    self.campaign_loading.ready_state = Some(configured_state);
                }
                SimResponse::TickCompleted(mut updated) => {
                    self.worker.is_busy = false;
                    let prev_selected = self.state.selected_country_id.clone();
                    let prev_prov = self.state.selected_province_mesh.clone();
                    let prev_unit = self.state.selected_unit_id;
                    let prev_paused = self.state.paused;
                    let prev_speed = self.state.speed_level;
                    let prev_diff = self.state.difficulty;
                    let prev_diplomacy_feedback = self.state.last_diplomacy_feedback.take();
                    let ui_chronicle_spool =
                        std::mem::take(&mut self.state.pending_chronicle_archive);
                    if !ui_chronicle_spool.is_empty() {
                        updated.pending_chronicle_archive.extend(ui_chronicle_spool);
                    }

                    self.state = *updated;
                    self.state.paused = prev_paused;
                    self.state.speed_level = prev_speed;
                    self.state.difficulty = prev_diff;
                    if self.state.last_diplomacy_feedback.is_none() {
                        self.state.last_diplomacy_feedback = prev_diplomacy_feedback;
                    }
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
                    self.worker.is_busy = false;
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
            || self.route == ScreenRoute::CampaignLoading
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
            if crate::cache::wgpu_pipeline_cache_key().is_none() {
                let info = rs.adapter.get_info();
                let cache_key = wgpu::util::pipeline_cache_key(&info).unwrap_or_else(|| {
                    format!("wgpu_{:?}_{:x}_{:x}", info.backend, info.vendor, info.device)
                });
                crate::cache::set_wgpu_pipeline_cache_key(cache_key);
            }
            if self.gpu_info.starts_with("wgpu pendiente") {
                let info = rs.adapter.get_info();
                self.gpu_info = format!("{} ({:?})", info.name, info.backend);
            }
        }

        self.poll_worker_responses();

        let now_frame = Instant::now();
        let frame_dt = (now_frame - self.last_frame_instant)
            .as_secs_f32()
            .clamp(0.001, 0.1);
        self.last_frame_instant = now_frame;

        let in_campaign = !matches!(
            self.route,
            ScreenRoute::MainMenu | ScreenRoute::CountrySelect | ScreenRoute::CampaignLoading
        ) && self.state.player_country_id.is_some();
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
                            let seed = generate_runtime_seed(&country_id);
                            self.campaign_loading = CampaignLoadingState::begin(country_id.clone());
                            self.pending_actions.clear();
                            self.day_accumulator = 0.0;
                            self.worker.request_configure_campaign(
                                &self.state,
                                country_id,
                                seed,
                            );
                            self.route = ScreenRoute::CampaignLoading;
                        }
                    }
                }
                ScreenRoute::CampaignLoading => {
                    match render_campaign_loading_screen(
                        ui,
                        &self.state,
                        &mut self.campaign_loading,
                        frame_dt,
                    ) {
                        LoadingScreenAction::None => {}
                        LoadingScreenAction::EnterWorldMap => {
                            if let Some(configured) = self.campaign_loading.ready_state.take() {
                                self.state = *configured;
                                if let Some(pid) = self.state.player_country_id.as_deref() {
                                    if let Some(scene) = self.scene_3d.as_ref() {
                                        if let Some(center) = scene.country_center(pid) {
                                            self.map_viewport.camera_3d.focus_point(center, 28.0);
                                        }
                                    }
                                }
                                self.show_country_popup = false;
                                self.show_minister_popup = false;
                                self.last_tick_instant = Instant::now();
                                self.route = ScreenRoute::WorldMap;
                            }
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
                        &self.province_graph,
                        &mut self.state,
                        &mut self.map_mode,
                        &mut self.map_viewport,
                        &mut self.show_country_popup,
                        &mut self.recruit_troops_amount,
                    );
                    next_hud_rects.extend(res.overlay_rects);
                    if let Some((unit_id, target_province)) = res.order_unit_move {
                        self.dispatch_player_action(PlayerAction::OrderUnitMove {
                            unit_id,
                            target_province,
                        });
                    }
                    if let Some((province_mesh, troops, cost_m)) = res.recruit_in_province {
                        if let Some(pid) = self.state.player_country_id.clone() {
                            self.dispatch_player_action(PlayerAction::RecruitSoldiers {
                                country_id: pid,
                                province_mesh,
                                troops,
                                cost_m,
                            });
                        }
                    }
                    if let Some(target_id) = res.open_diplomacy_with {
                        self.state.selected_country_id = Some(target_id);
                        self.route = ScreenRoute::Diplomacy;
                    }
                }
                ScreenRoute::Economy => {
                    let res = render_economy_screen(ui, &mut self.state);
                    if let Some(act) = res.policy_action {
                        self.dispatch_player_action(act);
                    }
                    if res.open_companies {
                        self.route = ScreenRoute::Companies;
                    }
                }
                ScreenRoute::Companies => {
                    render_companies_screen(ui, &self.state, &mut self.companies_view);
                }
                ScreenRoute::Politics => {
                    for act in render_politics_screen(ui, &mut self.state) {
                        self.dispatch_player_action(act);
                    }
                }
                ScreenRoute::Diplomacy => {
                    let res = render_diplomacy_screen(
                        ui,
                        &mut self.state,
                        &mut self.show_minister_popup,
                    );
                    for act in res.actions {
                        self.dispatch_player_action(act);
                    }
                }
                ScreenRoute::Military => {
                    let res = render_military_screen(
                        ui,
                        &mut self.state,
                        &self.province_graph,
                        &mut self.recruit_troops_amount,
                    );
                    for act in res.actions {
                        self.dispatch_player_action(act);
                    }
                    if let Some(uid) = res.open_map_for_unit {
                        self.state.selected_unit_id = Some(uid);
                        self.route = ScreenRoute::WorldMap;
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
                    .anchor(egui::Align2::CENTER_TOP, [-16.0, 48.0])
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

        // Popup modal de asignación del Ministro de Relaciones Exteriores
        if in_campaign && self.route == ScreenRoute::Diplomacy {
            if let Some(act) =
                render_minister_assign_popup(ctx, &self.state, &mut self.show_minister_popup)
            {
                self.dispatch_player_action(act);
            }
        } else {
            self.show_minister_popup = false;
        }

        // Popup modal de evento geopolítico generado por Lua 5.4
        if let Some(ev) = self.state.active_event.clone() {
            if let Some(opt_idx) = render_event_popup(ctx, &ev) {
                if let Some(opt) = ev.options.get(opt_idx) {
                    if let Some(pid) = self.state.player_country_id.clone() {
                        if let Some(country) = self.state.countries.get_mut(&pid) {
                            let monthly_gdp = (country.economy.gdp_usd_millions / 12.0).max(1.0);
                            let treasury_delta = monthly_gdp * (opt.treasury_delta_pct * 4.0);
                            country.economy.treasury_usd_millions =
                                (country.economy.treasury_usd_millions + treasury_delta).max(0.0);
                            country.economy.gdp_growth_pct += opt.gdp_growth_delta;
                            country.economy.inflation_pct =
                                (country.economy.inflation_pct + opt.inflation_delta).clamp(-0.4, 210.0);
                            country.stability =
                                (country.stability + opt.stability_delta).clamp(12.0, 98.0);
                            country.recompute_fiscal_snapshot();
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
