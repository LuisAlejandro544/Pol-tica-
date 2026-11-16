//! Propósito del archivo:
//! Orquestador de simulación en tiempo real en segundo plano (`SimulationWorker`).
//! Utiliza un hilo dedicado con `crossbeam-channel` para avanzar días de juego de forma
//! continua, mover unidades militares por el grafo de estados/provincias, fundar
//! empresas aleatorias con cotización bursátil y desempleo, y ejecutar reglas Lua 5.4
//! + IA paralela (`rayon`) sin bloquear el hilo de interfaz móvil.

use crate::ai::{apply_ai_decisions, evaluate_ai_countries, find_province_route, process_elections};
use crate::lua_bridge::LuaEngine;
use crate::models::{
    ArmyUnit, CompanyCategoryTemplate, CompanyEntity, Difficulty, GameState, PlayerAction,
    ProvinceGraph, ProvinceNode,
};
use crossbeam_channel::{Receiver, Sender};
use rand::{rngs::ChaCha8Rng, RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;

/// Comandos que la interfaz envía de forma asíncrona al trabajador de simulación.
pub enum SimCommand {
    SetProvinceGraph(Arc<ProvinceGraph>),
    AdvanceDays {
        state: Box<GameState>,
        days: u32,
    },
    NewCampaign {
        difficulty: Difficulty,
    },
}

/// Respuestas enviadas de regreso al hilo de interfaz cuando concluye el cálculo.
pub enum SimResponse {
    TickCompleted(Box<GameState>),
    CampaignReady(Box<GameState>),
    Error(String),
}

pub struct SimulationWorker {
    pub tx: Sender<SimCommand>,
    pub rx: Receiver<SimResponse>,
    pub is_busy: bool,
}

impl SimulationWorker {
    /// Arranca el hilo secundario dueño de la máquina virtual Lua 5.4 y el catálogo empresarial.
    pub fn spawn(initial_graph: Arc<ProvinceGraph>) -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<SimCommand>();
        let (res_tx, res_rx) = crossbeam_channel::unbounded::<SimResponse>();

        thread::Builder::new()
            .name("latam-sim-worker".to_string())
            .spawn(move || {
                let lua_engine = match LuaEngine::new() {
                    Ok(engine) => engine,
                    Err(e) => {
                        let _ = res_tx.send(SimResponse::Error(e.to_string()));
                        return;
                    }
                };
                let company_templates = lua_engine.load_company_catalog().unwrap_or_default();
                let mut graph = initial_graph;

                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        SimCommand::SetProvinceGraph(new_graph) => {
                            graph = new_graph;
                        }
                        SimCommand::NewCampaign { difficulty } => {
                            match build_initial_state(
                                &lua_engine,
                                &graph,
                                &company_templates,
                                difficulty,
                            ) {
                                Ok(state) => {
                                    let _ = res_tx.send(SimResponse::CampaignReady(Box::new(state)));
                                }
                                Err(e) => {
                                    let _ = res_tx.send(SimResponse::Error(e));
                                }
                            }
                        }
                        SimCommand::AdvanceDays { mut state, days } => {
                            let mut err_msg = None;
                            for _ in 0..days.max(1) {
                                if state.active_event.is_some() {
                                    break;
                                }
                                if let Err(e) = advance_single_day(
                                    &lua_engine,
                                    &graph,
                                    &company_templates,
                                    &mut state,
                                ) {
                                    err_msg = Some(e);
                                    break;
                                }
                            }
                            if let Some(e) = err_msg {
                                let _ = res_tx.send(SimResponse::Error(e));
                            } else {
                                let _ = res_tx.send(SimResponse::TickCompleted(state));
                            }
                        }
                    }
                }
            })
            .expect("No se pudo iniciar el hilo secundario de simulación");

        Self {
            tx: cmd_tx,
            rx: res_rx,
            is_busy: false,
        }
    }

    pub fn set_province_graph(&self, graph: Arc<ProvinceGraph>) {
        let _ = self.tx.send(SimCommand::SetProvinceGraph(graph));
    }

    pub fn request_new_campaign(&mut self, difficulty: Difficulty) {
        self.is_busy = true;
        let _ = self.tx.send(SimCommand::NewCampaign { difficulty });
    }

    pub fn request_advance_days(&mut self, state: &GameState, days: u32) {
        if self.is_busy {
            return;
        }
        self.is_busy = true;
        let _ = self.tx.send(SimCommand::AdvanceDays {
            state: Box::new(state.clone()),
            days,
        });
    }
}

/// Construye un grafo mínimo de respaldo si aún no se cargó el modelo 3D `.glb`.
pub fn ensure_fallback_graph(countries: &BTreeMap<String, crate::models::CountryState>, graph: &mut ProvinceGraph) {
    for (idx, (cid, c)) in countries.iter().enumerate() {
        let has_any = graph.values().any(|p| p.country_id == *cid);
        if !has_any {
            let mesh_name = format!("PROV_{cid}_0_CAP");
            let x = ((idx % 5) as f32 - 2.0) * 6.0;
            let z = ((idx / 5) as f32 - 2.0) * 6.0;
            graph.insert(
                mesh_name.clone(),
                ProvinceNode {
                    mesh_name,
                    country_id: cid.clone(),
                    name: c.capital.clone(),
                    world_pos: [x, 0.4, z],
                    terrain_type: "Capital".to_string(),
                    elevation_avg: 0.25,
                    neighbors: Vec::new(),
                },
            );
        }
    }
}

/// Aplica una acción militar del jugador sobre el `GameState` (reclutar o mover unidad).
pub fn apply_player_action(
    state: &mut GameState,
    graph: &ProvinceGraph,
    action: &PlayerAction,
) {
    match action {
        PlayerAction::RecruitSoldiers {
            country_id,
            province_mesh,
            troops,
            cost_m,
        } => {
            let can_pay = state
                .countries
                .get(country_id)
                .map(|c| c.economy.treasury_usd_millions >= *cost_m)
                .unwrap_or(false);
            if !can_pay || *troops == 0 {
                return;
            }

            let added_upkeep = (*troops as f64 / 1_000.0) * 1.35;
            if let Some(c) = state.countries.get_mut(country_id) {
                c.economy.treasury_usd_millions -= *cost_m;
                c.military.active_troops += *troops;
                c.economy.troop_upkeep_usd_millions = c.estimated_troop_upkeep_m();
                // Descuenta inmediatamente del ingreso mensual neto proyectado
                c.economy.last_monthly_balance -= added_upkeep;
            }

            let (world_pos, prov_name) = graph
                .get(province_mesh)
                .map(|p| (p.world_pos, p.name.clone()))
                .unwrap_or(([0.0, 0.5, 0.0], province_mesh.clone()));

            let uid = state.next_unit_id;
            state.next_unit_id += 1;
            state.units.push(ArmyUnit {
                id: uid,
                name: format!("Regimiento #{uid}"),
                country_id: country_id.clone(),
                troops: *troops,
                current_province: province_mesh.clone(),
                target_province: None,
                route_queue: Vec::new(),
                march_progress: 0.0,
                world_pos,
                morale: 88.0,
            });
            state.selected_unit_id = Some(uid);
            state.push_chronicle(
                country_id.clone(),
                format!(
                    "Reclutados {} soldados en {} (costo ${:.0}M, mantenimiento -{:.2}M/mes).",
                    troops, prov_name, cost_m, added_upkeep
                ),
            );
        }
        PlayerAction::OrderUnitMove {
            unit_id,
            target_province,
        } => {
            let Some(unit) = state.units.iter().find(|u| u.id == *unit_id).cloned() else {
                return;
            };
            if unit.current_province == *target_province {
                return;
            }
            let route = find_province_route(
                graph,
                state,
                &unit.country_id,
                &unit.current_province,
                target_province,
            );
            if route.is_empty() {
                return;
            }
            let dest_label = graph
                .get(target_province)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| target_province.clone());

            if let Some(u) = state.units.iter_mut().find(|u| u.id == *unit_id) {
                let first_hop = route[0].clone();
                u.target_province = Some(first_hop);
                u.route_queue = route.into_iter().skip(1).collect();
                u.march_progress = 0.0;
            }
            state.push_chronicle(
                unit.country_id,
                format!("{} inicia marcha hacia {}.", unit.name, dest_label),
            );
        }
    }
}

fn build_initial_state(
    lua: &LuaEngine,
    graph: &ProvinceGraph,
    company_templates: &[CompanyCategoryTemplate],
    difficulty: Difficulty,
) -> Result<GameState, String> {
    let mut countries = lua
        .load_initial_countries()
        .map_err(|e| format!("Error cargando países desde Lua 5.4: {e}"))?;

    let mut relations = BTreeMap::new();
    let ids: Vec<String> = countries.keys().cloned().collect();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = &ids[i];
            let b = &ids[j];
            let c_a = &countries[a];
            let is_neighbor = c_a.neighbors.iter().any(|n| n == b);
            relations.insert(
                GameState::ordered_pair(a, b),
                crate::models::DiplomaticRelation {
                    opinion: if is_neighbor { 25.0 } else { 10.0 },
                    trade_pact: is_neighbor && (i + j) % 3 == 0,
                    alliance: false,
                    at_war: false,
                },
            );
        }
    }

    // Sembrar unidades militares reales distribuidas en las provincias de cada país
    let mut units = Vec::new();
    let mut next_unit_id = 1_u64;
    for (cid, country) in &countries {
        let provs: Vec<&ProvinceNode> = graph
            .values()
            .filter(|p| p.country_id == *cid)
            .collect();
        let div_count = if provs.len() >= 3 && country.military.active_troops >= 60_000 {
            3
        } else if provs.len() >= 2 && country.military.active_troops >= 20_000 {
            2
        } else {
            1
        };

        let mut remaining = country.military.active_troops.max(1_000);
        for d in 0..div_count {
            let troops = if d + 1 == div_count {
                remaining
            } else {
                let share = (remaining / ((div_count - d) as u64)).max(1_000);
                remaining = remaining.saturating_sub(share);
                share
            };
            let (prov_mesh, world_pos) = if !provs.is_empty() {
                let p = provs[(d * (provs.len() / div_count).max(1)) % provs.len()];
                (p.mesh_name.clone(), p.world_pos)
            } else {
                (format!("PROV_{cid}_0_CAP"), [0.0, 0.4, 0.0])
            };
            units.push(ArmyUnit {
                id: next_unit_id,
                name: format!("Cuerpo {} #{}", cid, d + 1),
                country_id: cid.clone(),
                troops,
                current_province: prov_mesh,
                target_province: None,
                route_queue: Vec::new(),
                march_progress: 0.0,
                world_pos,
                morale: 82.0,
            });
            next_unit_id += 1;
        }
    }

    // Calcular balance inicial realista con mantenimiento de soldados incluido
    for country in countries.values_mut() {
        let monthly_gdp = country.economy.gdp_usd_millions / 12.0;
        let tax_rev = monthly_gdp * (country.economy.tax_rate_pct / 100.0);
        let def_cost = monthly_gdp * (country.military.defense_budget_pct / 100.0);
        let inf_cost = monthly_gdp * (country.economy.infra_investment_pct / 100.0);
        let soc_cost = monthly_gdp * (country.economy.social_investment_pct / 100.0);
        let debt_svc = (country.economy.gdp_usd_millions * (country.economy.debt_to_gdp_pct / 100.0)) * 0.0045;
        let upkeep = country.estimated_troop_upkeep_m();
        country.economy.troop_upkeep_usd_millions = upkeep;
        country.economy.last_monthly_balance =
            tax_rev - (def_cost + upkeep + inf_cost + soc_cost + debt_svc);
    }

    let mut state = GameState {
        total_days: 1,
        year: 2026,
        month: 1,
        day: 1,
        paused: true,
        speed_level: 1,
        difficulty,
        player_country_id: None,
        selected_country_id: Some("MX".to_string()),
        selected_province_mesh: None,
        selected_unit_id: None,
        countries,
        relations,
        units,
        next_unit_id,
        companies: Vec::new(),
        next_company_id: 1,
        active_event: None,
        chronicle: Vec::new(),
    };

    // Sembrar 4 empresas pioneras en distintos países (no en todos) para que el mercado bursátil arranque vivo
    let seed_countries = ["MX", "BR", "CO", "CL"];
    for (idx, cid) in seed_countries.iter().enumerate() {
        if state.countries.contains_key(*cid) && !company_templates.is_empty() {
            let mut rng = ChaCha8Rng::seed_from_u64(0xCAFE_0001 + idx as u64);
            let _ = try_found_company_in(
                &mut state,
                company_templates,
                cid,
                &mut rng,
                idx % company_templates.len(),
            );
        }
    }

    state.push_chronicle(
        "LATAM",
        "Simulación en tiempo real lista. Elige una nación para gobernar.",
    );
    Ok(state)
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn advance_single_day(
    lua: &LuaEngine,
    graph: &ProvinceGraph,
    company_templates: &[CompanyCategoryTemplate],
    state: &mut GameState,
) -> Result<(), String> {
    // 1. Avanzar reloj diario en tiempo real
    state.total_days += 1;
    let max_d = days_in_month(state.year, state.month);
    let mut new_month = false;
    if state.day >= max_d {
        state.day = 1;
        new_month = true;
        if state.month >= 12 {
            state.month = 1;
            state.year += 1;
        } else {
            state.month += 1;
        }
    } else {
        state.day += 1;
    }

    // 2. Movimiento continuo de unidades militares por los estados/provincias
    step_unit_movements(state, graph);

    // 3. Flujo diario de caja (refleja de inmediato ingresos reducidos por mantenimiento de soldados)
    for country in state.countries.values_mut() {
        let daily_net = country.economy.last_monthly_balance / 30.0;
        let next_treasury = country.economy.treasury_usd_millions + daily_net;
        if next_treasury < 0.0 {
            let deficit_pct =
                (next_treasury.abs() / country.economy.gdp_usd_millions.max(1.0)) * 100.0;
            country.economy.debt_to_gdp_pct =
                (country.economy.debt_to_gdp_pct + deficit_pct).min(240.0);
            country.economy.treasury_usd_millions = 0.0;
        } else {
            country.economy.treasury_usd_millions = next_treasury;
        }
    }

    // 4. Tick bursátil en tiempo real y fundación aleatoria de empresas en algunos países
    step_companies_and_stocks(state, company_templates);

    // 5. Cada 7 días: evaluación semanal de la IA local en paralelo (reclutamiento y marcha)
    if state.total_days % 7 == 0 {
        let ai_decisions = evaluate_ai_countries(state, graph);
        apply_ai_decisions(state, graph, ai_decisions);
    }

    // 6. Al cambiar de mes: reglas macroeconómicas en Lua 5.4, elecciones y eventos
    if new_month {
        let ids: Vec<String> = state.countries.keys().cloned().collect();
        for id in ids {
            let corp_gdp = state.country_company_gdp_bonus(&id);
            let corp_jobs = state.country_company_jobs(&id);
            if let Some(country) = state.countries.get_mut(&id) {
                // Preservamos la tesorería diaria para no duplicar el cobro mensual en Lua
                let daily_treasury = country.economy.treasury_usd_millions;
                lua.step_country_via_lua(country, corp_gdp, corp_jobs)
                    .map_err(|e| format!("Error ejecutando reglas Lua para {id}: {e}"))?;
                country.economy.treasury_usd_millions = daily_treasury;
            }
        }

        process_elections(state);

        if state.active_event.is_none()
            && state.player_country_id.is_some()
            && state.month % 4 == 0
        {
            let ev = lua
                .pick_event(state.total_days, state.countries.len() as u64)
                .map_err(|e| format!("Error generando evento Lua: {e}"))?;
            state.active_event = Some(ev);
        }
    }

    Ok(())
}

fn step_unit_movements(state: &mut GameState, graph: &ProvinceGraph) {
    let mut arrivals: Vec<(String, String, String)> = Vec::new();
    let mut country_map_updates: Vec<(String, String)> = Vec::new();

    for unit in &mut state.units {
        let Some(target_mesh) = unit.target_province.clone() else {
            if let Some(p) = graph.get(&unit.current_province) {
                unit.world_pos = p.world_pos;
            }
            continue;
        };

        let from_pos = graph
            .get(&unit.current_province)
            .map(|p| p.world_pos)
            .unwrap_or(unit.world_pos);
        let (to_pos, elev, dest_owner, dest_name) = graph
            .get(&target_mesh)
            .map(|p| {
                (
                    p.world_pos,
                    p.elevation_avg,
                    p.country_id.clone(),
                    p.name.clone(),
                )
            })
            .unwrap_or((unit.world_pos, 0.2, unit.country_id.clone(), target_mesh.clone()));

        let terrain_factor = (1.0 + elev * 0.45).clamp(1.0, 2.2) as f32;
        let speed_per_day = 0.30 / terrain_factor;
        unit.march_progress = (unit.march_progress + speed_per_day).min(1.0);

        let t = unit.march_progress;
        unit.world_pos = [
            from_pos[0] + (to_pos[0] - from_pos[0]) * t,
            from_pos[1] + (to_pos[1] - from_pos[1]) * t + (t * std::f32::consts::PI).sin() * 0.15,
            from_pos[2] + (to_pos[2] - from_pos[2]) * t,
        ];

        if unit.march_progress >= 1.0 {
            unit.current_province = target_mesh.clone();
            unit.world_pos = to_pos;
            unit.march_progress = 0.0;

            if dest_owner != unit.country_id && state.get_relation(&unit.country_id, &dest_owner).at_war {
                country_map_updates.push((unit.country_id.clone(), dest_name.clone()));
            }

            if !unit.route_queue.is_empty() {
                unit.target_province = Some(unit.route_queue.remove(0));
            } else {
                unit.target_province = None;
                if state.player_country_id.as_deref() == Some(unit.country_id.as_str()) {
                    arrivals.push((unit.country_id.clone(), unit.name.clone(), dest_name));
                }
            }
        }
    }

    for (cid, uname, pname) in arrivals {
        state.push_chronicle(cid, format!("{uname} llegó a {pname}."));
    }
    for (cid, pname) in country_map_updates {
        state.push_chronicle(cid, format!("Fuerzas desplegadas en zona disputada: {pname}."));
    }
}

fn step_companies_and_stocks(
    state: &mut GameState,
    templates: &[CompanyCategoryTemplate],
) {
    let mut rng = ChaCha8Rng::seed_from_u64(
        state.total_days.wrapping_mul(0x517C_C1B7_2722_0A95) ^ 0xA5A5_5A5A,
    );

    // 1. Actualización diaria del precio de acción de cada empresa según macroeconomía del país
    for comp in &mut state.companies {
        let Some(c) = state.countries.get(&comp.country_id) else {
            continue;
        };
        comp.prev_share_price = comp.share_price;

        let growth_drift = (c.economy.gdp_growth_pct - 1.8) * 0.0018;
        let inflation_drag = (c.economy.inflation_pct - 5.5).max(0.0)
            * 0.0007
            * comp.inflation_sensitivity;
        let rate_effect = -(c.economy.interest_rate_pct - 7.0)
            * 0.0005
            * comp.rate_sensitivity;
        let stability_drift = (c.stability - 55.0) * 0.00018;
        let noise = rng.random_range(-0.022..=0.022);

        let pct_change = (growth_drift - inflation_drag + rate_effect + stability_drift + noise)
            .clamp(-0.06, 0.06);
        comp.share_price = (comp.share_price * (1.0 + pct_change)).clamp(2.5, 2_500.0);

        comp.price_history.push(comp.share_price);
        if comp.price_history.len() > 24 {
            comp.price_history.remove(0);
        }
    }

    // 2. Cada 10 días de juego, evaluar si nace aleatoriamente una nueva empresa en algún país (no en todos)
    if state.total_days % 10 != 0 || templates.is_empty() || state.companies.len() >= 65 {
        return;
    }

    // Probabilidad moderada (~55% cada 10 días) de que brote una empresa en algún país propicio
    if !rng.random_bool(0.55) {
        return;
    }

    let candidate_ids: Vec<String> = state.countries.keys().cloned().collect();
    if candidate_ids.is_empty() {
        return;
    }

    let mut weights: Vec<(String, f64)> = Vec::new();
    for cid in candidate_ids {
        let Some(c) = state.countries.get(&cid) else {
            continue;
        };
        let existing_count = state
            .companies
            .iter()
            .filter(|co| co.country_id == cid)
            .count();
        // Limitar concentración para que no aparezcan en todos igual ni saturen un solo país
        if existing_count >= 6 {
            continue;
        }
        let score = (c.stability * 0.6
            + (c.economy.gdp_growth_pct + 4.0).max(0.5) * 8.0
            + c.economy.infra_investment_pct * 4.0
            - (c.economy.inflation_pct - 8.0).max(0.0) * 1.4
            - (c.economy.tax_rate_pct - 22.0).max(0.0) * 1.6)
            / ((existing_count as f64) * 0.75 + 1.0);
        if score > 10.0 {
            weights.push((cid, score));
        }
    }

    if weights.is_empty() {
        return;
    }

    let total_w: f64 = weights.iter().map(|(_, w)| *w).sum();
    let mut pick = rng.random_range(0.0..total_w);
    let mut chosen_cid = weights[0].0.clone();
    for (cid, w) in weights {
        if pick <= w {
            chosen_cid = cid;
            break;
        }
        pick -= w;
    }

    let cat_idx = rng.random_range(0..templates.len());
    let _ = try_found_company_in(state, templates, &chosen_cid, &mut rng, cat_idx);
}

fn try_found_company_in(
    state: &mut GameState,
    templates: &[CompanyCategoryTemplate],
    country_id: &str,
    rng: &mut ChaCha8Rng,
    cat_idx: usize,
) -> Option<()> {
    let tmpl = templates.get(cat_idx)?;
    let country_snapshot = state.countries.get(country_id)?.clone();

    let prefix = &tmpl.prefixes[rng.random_range(0..tmpl.prefixes.len())];
    let suffix = &tmpl.suffixes[rng.random_range(0..tmpl.suffixes.len())];
    let product = &tmpl.products[rng.random_range(0..tmpl.products.len())];
    let name = format!("{prefix} {suffix}");

    // Evitar duplicados exactos dentro del mismo país
    let final_name = if state
        .companies
        .iter()
        .any(|c| c.country_id == country_id && c.name == name)
    {
        format!("{name} {}", state.next_company_id)
    } else {
        name
    };

    let pop_scale = (country_snapshot.population as f64 / 35_000_000.0).clamp(0.25, 1.85);
    let raw_jobs = rng.random_range(tmpl.jobs_min..=tmpl.jobs_max) as f64;
    let employees = ((raw_jobs * pop_scale).round() as u64).max(800);

    let initial_price = rng.random_range(tmpl.base_price_min..=tmpl.base_price_max);
    let annual_gdp_contribution_m = (employees as f64) * tmpl.gdp_per_job_m;

    let cid = state.next_company_id;
    state.next_company_id += 1;

    let company = CompanyEntity {
        id: cid,
        name: final_name.clone(),
        country_id: country_id.to_string(),
        category: tmpl.category,
        sells: product.clone(),
        founded_label: state.date_label(),
        employees,
        share_price: initial_price,
        prev_share_price: initial_price,
        price_history: vec![initial_price],
        annual_gdp_contribution_m,
        inflation_sensitivity: tmpl.inflation_sensitivity,
        rate_sensitivity: tmpl.rate_sensitivity,
    };
    state.companies.push(company);

    // Impacto inmediato y lógico en empleo (baja desempleo) y PIB del país
    if let Some(c) = state.countries.get_mut(country_id) {
        let workforce = ((c.population as f64) * 0.48).max(120_000.0);
        let unemp_reduction = ((employees as f64 / workforce) * 100.0 * 1.4).clamp(0.05, 1.4);
        c.economy.unemployment_pct = (c.economy.unemployment_pct - unemp_reduction).max(2.0);
        c.economy.corporate_gdp_bonus_m += annual_gdp_contribution_m;
        c.economy.gdp_usd_millions += annual_gdp_contribution_m * 0.25;
        c.economy.gdp_growth_pct = (c.economy.gdp_growth_pct + 0.08).min(13.5);
    }

    state.push_chronicle(
        country_id.to_string(),
        format!(
            "Nueva empresa en {}: {} ({}) — +{} empleos, +${:.0}M PIB.",
            country_snapshot.name,
            final_name,
            tmpl.category.label(),
            employees,
            annual_gdp_contribution_m
        ),
    );
    Some(())
}
