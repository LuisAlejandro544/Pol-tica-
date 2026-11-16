//! Propósito del archivo:
//! Orquestador de simulación en tiempo real en segundo plano (`SimulationWorker`).
//! Utiliza un hilo dedicado con `crossbeam-channel` para avanzar días de juego de forma
//! continua, mover unidades militares por el grafo de estados/provincias, fundar
//! empresas aleatorias con cotización bursátil y desempleo, y ejecutar reglas Lua 5.4
//! + IA paralela (`rayon`) sin bloquear el hilo de interfaz móvil.

pub mod leaders;

use crate::ai::{apply_ai_decisions, evaluate_ai_countries, find_province_route, process_elections};
use crate::lua_bridge::{LuaEngine, LuaEventContext, LuaStepContext};
use crate::models::{
    ArchivedStockSample, ArmyUnit, CampaignThumbnail, CompanyCategory, CompanyCategoryTemplate,
    CompanyEntity, Difficulty, ForeignMinister, GameState, GlobalMacroCycle, MacroHistorySnapshot,
    MinisterMissionKind, PlayerAction, PoliticalDecreeKind, ProvinceGraph, ProvinceNode,
    RegionState,
};
use crossbeam_channel::{Receiver, Sender};
use leaders::{
    assign_initial_minister_pools, assign_random_leaders_to_countries,
    generate_minister_candidates,
};
use rand::{rngs::ChaCha8Rng, RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

static CAMPAIGN_SEED_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Genera una semilla pseudoaleatoria irrepetible combinando reloj del sistema, contador y país elegido.
pub fn generate_runtime_seed(salt: &str) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let seq = CAMPAIGN_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut seed = nanos ^ seq.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    for b in salt.bytes() {
        seed = seed.rotate_left(9) ^ (b as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    }
    if seed == 0 {
        0xCAFE_BABE_1234_5678
    } else {
        seed
    }
}

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
    ConfigureCampaign {
        state: Box<GameState>,
        player_country_id: String,
        seed: u64,
    },
}

/// Respuestas enviadas de regreso al hilo de interfaz cuando concluye el cálculo.
pub enum SimResponse {
    TickCompleted(Box<GameState>),
    CampaignReady(Box<GameState>),
    CampaignLoadProgress {
        progress: f32,
        stage: String,
        detail: String,
        units_configured: usize,
        companies_founded: usize,
    },
    CampaignConfigured(Box<GameState>),
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
                            match build_initial_state(&lua_engine, &graph, difficulty) {
                                Ok(state) => {
                                    let _ = res_tx.send(SimResponse::CampaignReady(Box::new(state)));
                                }
                                Err(e) => {
                                    let _ = res_tx.send(SimResponse::Error(e));
                                }
                            }
                        }
                        SimCommand::ConfigureCampaign {
                            state,
                            player_country_id,
                            seed,
                        } => {
                            match configure_campaign_for_country(
                                *state,
                                &graph,
                                &company_templates,
                                &player_country_id,
                                seed,
                                &res_tx,
                            ) {
                                Ok(configured) => {
                                    let _ = res_tx.send(SimResponse::CampaignConfigured(
                                        Box::new(configured),
                                    ));
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

    pub fn request_configure_campaign(
        &mut self,
        state: &GameState,
        player_country_id: String,
        seed: u64,
    ) {
        self.is_busy = true;
        let _ = self.tx.send(SimCommand::ConfigureCampaign {
            state: Box::new(state.clone()),
            player_country_id,
            seed,
        });
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

/// Calcula cuántos puntos de opinión bilateral aporta inmediatamente una misión de mejora de relaciones.
pub fn compute_minister_relation_gain(minister: &ForeignMinister) -> f64 {
    let raw = 10.0 * (1.0 + minister.stats.relation_boost_pct / 100.0);
    (raw * 10.0).round() / 10.0
}

/// Calcula la probabilidad efectiva (0..100) de que el país objetivo acepte de inmediato
/// una misión enviada por el Ministro de Relaciones Exteriores asignado (0 días de espera).
pub fn compute_minister_mission_chance_pct(
    state: &GameState,
    minister: &ForeignMinister,
    player_id: &str,
    target_id: &str,
    mission: MinisterMissionKind,
) -> f64 {
    let rel = state.get_relation(player_id, target_id);
    let my_power = state
        .countries
        .get(player_id)
        .map(|c| c.military_power_score())
        .unwrap_or(10.0);
    let their_power = state
        .countries
        .get(target_id)
        .map(|c| c.military_power_score())
        .unwrap_or(10.0)
        .max(1.0);
    let power_ratio = (my_power / their_power).clamp(0.4, 2.2);

    match mission {
        MinisterMissionKind::ImproveRelations => 100.0,
        MinisterMissionKind::ProposeNonAggression => {
            let opinion_mod = rel.opinion * 0.18;
            let trade_bonus = if rel.trade_pact { 5.0 } else { 0.0 };
            (minister.stats.non_aggression_chance_pct + opinion_mod + trade_bonus).clamp(8.0, 96.0)
        }
        MinisterMissionKind::ProposeAlliance => {
            let opinion_mod = (rel.opinion - 20.0) * 0.25;
            let pact_bonus = (if rel.trade_pact { 5.0 } else { 0.0 })
                + (if rel.non_aggression_pact { 7.0 } else { 0.0 });
            (minister.stats.alliance_chance_pct + opinion_mod + pact_bonus).clamp(5.0, 95.0)
        }
        MinisterMissionKind::ProposePeaceTreaty => {
            let mil_mod = (power_ratio - 1.0) * 14.0;
            let opinion_mod = rel.opinion * 0.12;
            (minister.stats.peace_treaty_chance_pct + mil_mod + opinion_mod).clamp(8.0, 96.0)
        }
        MinisterMissionKind::ProposeWhitePeace => {
            let mil_mod = (power_ratio - 1.0) * 10.0;
            let opinion_mod = rel.opinion * 0.08;
            (minister.stats.white_peace_chance_pct + mil_mod + opinion_mod).clamp(10.0, 97.0)
        }
    }
}

/// Recluta tropas directamente desde la población desempleada de una región propia,
/// respetando su tope demográfico regional y la tesorería nacional.
pub fn recruit_from_region(
    state: &mut GameState,
    graph: &ProvinceGraph,
    country_id: &str,
    province_mesh: &str,
    requested_troops: u64,
) -> Option<(u64, f64, f64, String)> {
    if requested_troops == 0 {
        return None;
    }

    let reg_cap = state
        .regions
        .get(province_mesh)
        .filter(|r| r.country_id == country_id)
        .map(|r| r.max_recruitable())
        .unwrap_or(0);

    let treasury = state
        .countries
        .get(country_id)
        .map(|c| c.economy.treasury_usd_millions)
        .unwrap_or(0.0);
    let max_affordable = ((treasury / 70.0) * 1_000.0).floor() as u64;
    let actual_troops = requested_troops.min(reg_cap).min(max_affordable);
    if actual_troops == 0 {
        return None;
    }

    let actual_cost_m = (actual_troops as f64 / 1_000.0) * 70.0;
    let added_upkeep = (actual_troops as f64 / 1_000.0) * 1.35;

    let prov_name = if let Some(region) = state.regions.get_mut(province_mesh) {
        region.unemployed = region.unemployed.saturating_sub(actual_troops);
        region.stationed_recruited = region.stationed_recruited.saturating_add(actual_troops);
        region.name.clone()
    } else {
        graph
            .get(province_mesh)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| province_mesh.to_string())
    };

    if let Some(c) = state.countries.get_mut(country_id) {
        c.economy.treasury_usd_millions = (c.economy.treasury_usd_millions - actual_cost_m).max(0.0);
        c.military.active_troops += actual_troops;
        let workforce = ((c.population as f64) * 0.48).max(1.0);
        let unemp_drop = ((actual_troops as f64) / workforce) * 100.0;
        c.economy.unemployment_pct = (c.economy.unemployment_pct - unemp_drop).clamp(0.5, 34.0);
        c.recompute_fiscal_snapshot();
    }

    Some((actual_troops, actual_cost_m, added_upkeep, prov_name))
}

/// Aplica una acción del jugador sobre el `GameState` (militar, fiscal, política o diplomática instantánea).
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
            cost_m: _,
        } => {
            let Some((actual_troops, actual_cost_m, added_upkeep, prov_name)) =
                recruit_from_region(state, graph, country_id, province_mesh, *troops)
            else {
                return;
            };

            let world_pos = graph
                .get(province_mesh)
                .map(|p| p.world_pos)
                .unwrap_or([0.0, 0.5, 0.0]);

            let uid = state.next_unit_id;
            state.next_unit_id += 1;
            state.units.push(ArmyUnit {
                id: uid,
                name: format!("Regimiento #{uid}"),
                country_id: country_id.clone(),
                troops: actual_troops,
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
                    "Reclutados {} soldados desde desempleados en {} (costo ${:.1}M, mant. -{:.2}M/mes).",
                    actual_troops, prov_name, actual_cost_m, added_upkeep
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
        PlayerAction::UpdateEconomicPolicy {
            country_id,
            interest_rate_pct,
            tax_rate_pct,
            infra_investment_pct,
            social_investment_pct,
        } => {
            if let Some(c) = state.countries.get_mut(country_id) {
                c.economy.interest_rate_pct = interest_rate_pct.clamp(1.0, 50.0);
                c.economy.tax_rate_pct = tax_rate_pct.clamp(10.0, 40.0);
                c.economy.infra_investment_pct = infra_investment_pct.clamp(0.5, 10.0);
                c.economy.social_investment_pct = social_investment_pct.clamp(0.5, 12.0);
                c.recompute_fiscal_snapshot();
            }
        }
        PlayerAction::UpdateDefenseBudget {
            country_id,
            defense_budget_pct,
        } => {
            if let Some(c) = state.countries.get_mut(country_id) {
                c.military.defense_budget_pct = defense_budget_pct.clamp(0.0, 6.0);
                c.recompute_fiscal_snapshot();
            }
        }
        PlayerAction::RunMilitaryDrills { country_id } => {
            let mut ran = false;
            if let Some(c) = state.countries.get_mut(country_id) {
                if c.economy.treasury_usd_millions >= 200.0 {
                    c.economy.treasury_usd_millions -= 200.0;
                    c.military.readiness = (c.military.readiness + 6.0).min(100.0);
                    ran = true;
                }
            }
            if ran {
                state.push_chronicle(
                    country_id.clone(),
                    "Maniobras militares completadas con éxito (+6% aprestamiento).",
                );
            }
        }
        PlayerAction::ExecutePoliticalDecree { country_id, decree } => {
            let mut msg: Option<String> = None;
            if let Some(c) = state.countries.get_mut(country_id) {
                match decree {
                    PoliticalDecreeKind::NationalDialogue => {
                        if c.economy.treasury_usd_millions >= 250.0 {
                            c.economy.treasury_usd_millions -= 250.0;
                            c.stability = (c.stability + 5.0).min(98.0);
                            msg = Some(
                                "Se celebró una mesa de diálogo nacional en el Congreso (+5% estabilidad)."
                                    .to_string(),
                            );
                        }
                    }
                    PoliticalDecreeKind::PublicCampaign => {
                        if c.economy.treasury_usd_millions >= 180.0 {
                            c.economy.treasury_usd_millions -= 180.0;
                            if let Some(ruling) =
                                c.political_parties.iter_mut().find(|p| p.is_ruling)
                            {
                                ruling.approval_pct = (ruling.approval_pct + 4.0).min(95.0);
                            }
                            msg = Some(
                                "El partido de gobierno impulsó una campaña ciudadana (+4% aprobación)."
                                    .to_string(),
                            );
                        }
                    }
                }
            }
            if let Some(m) = msg {
                state.push_chronicle(country_id.clone(), m);
            }
        }
        PlayerAction::ToggleCoalitionParty {
            country_id,
            party_acronym,
        } => {
            let mut msg: Option<String> = None;
            if let Some(c) = state.countries.get_mut(country_id) {
                let is_partner = c.coalition_parties.iter().any(|a| a == party_acronym);
                let leader_name = c
                    .political_parties
                    .iter()
                    .find(|p| &p.acronym == party_acronym)
                    .map(|p| p.leader.full_name.clone())
                    .unwrap_or_else(|| "Líder parlamentario".to_string());
                if is_partner {
                    c.coalition_parties.retain(|a| a != party_acronym);
                    c.stability = (c.stability - 2.0).max(12.0);
                    msg = Some(format!(
                        "[{party_acronym}] abandonó la coalición en el Congreso."
                    ));
                } else if c.political_capital >= 20.0 {
                    c.coalition_parties.push(party_acronym.clone());
                    c.political_capital = (c.political_capital - 20.0).max(0.0);
                    c.stability = (c.stability + 2.5).min(98.0);
                    msg = Some(format!(
                        "Coalición legislativa firmada con [{party_acronym}] (líder {leader_name})."
                    ));
                }
                c.legislative_support_pct = c.compute_legislative_support_pct();
            }
            if let Some(m) = msg {
                state.push_chronicle(country_id.clone(), m);
            }
        }
        PlayerAction::AssignForeignMinister {
            country_id,
            candidate_id,
        } => {
            let assigned = if let Some(c) = state.countries.get_mut(country_id) {
                if let Some(pos) = c.minister_candidates.iter().position(|m| m.id == *candidate_id) {
                    let chosen = c.minister_candidates.remove(pos);
                    if let Some(prev) = c.foreign_minister.replace(chosen.clone()) {
                        c.minister_candidates.push(prev);
                    }
                    Some(chosen)
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(m) = assigned {
                let msg = format!(
                    "Canciller asignado en 0 días: {} ({}, {} años).",
                    m.profile.full_name, m.specialty, m.profile.age
                );
                state.last_diplomacy_feedback = Some((true, msg.clone()));
                state.push_chronicle(country_id.clone(), msg);
            }
        }
        PlayerAction::RefreshMinisterCandidates { country_id, seed } => {
            let mut next_id = state.next_minister_id.max(1);
            if let Some(c) = state.countries.get_mut(country_id) {
                let mut used: Vec<String> = c
                    .political_parties
                    .iter()
                    .map(|p| p.leader.full_name.clone())
                    .collect();
                if let Some(curr) = &c.foreign_minister {
                    used.push(curr.profile.full_name.clone());
                }
                c.minister_candidates =
                    generate_minister_candidates(country_id, *seed, &mut next_id, 4, &used);
                state.last_diplomacy_feedback = Some((
                    true,
                    "Nueva terna de candidatos a Ministro de Relaciones Exteriores disponible."
                        .to_string(),
                ));
            }
            state.next_minister_id = next_id;
        }
        PlayerAction::ExecuteMinisterMission {
            player_country_id,
            target_country_id,
            mission,
            roll_seed,
        } => {
            let Some(minister) = state
                .countries
                .get(player_country_id)
                .and_then(|c| c.foreign_minister.clone())
            else {
                state.last_diplomacy_feedback = Some((
                    false,
                    "Debes asignar primero un Ministro de Relaciones Exteriores.".to_string(),
                ));
                return;
            };
            let target_name = state
                .countries
                .get(target_country_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| target_country_id.clone());

            let chance_pct = compute_minister_mission_chance_pct(
                state,
                &minister,
                player_country_id,
                target_country_id,
                *mission,
            );
            let mut rng = ChaCha8Rng::seed_from_u64(*roll_seed);
            let roll_val = rng.random_range(0.0_f64..100.0_f64);
            let accepted = roll_val < chance_pct;

            let key = GameState::ordered_pair(player_country_id, target_country_id);
            let rel = state.relations.entry(key).or_default();

            let (ok, summary) = match mission {
                MinisterMissionKind::ImproveRelations => {
                    let gain = compute_minister_relation_gain(&minister);
                    rel.opinion = (rel.opinion + gain).min(100.0);
                    (
                        true,
                        format!(
                            "Misión completada al instante (0 días): {} mejoró relaciones con {} (+{:.1} opinión, +{:.0}% bono ministerial).",
                            minister.profile.full_name,
                            target_name,
                            gain,
                            minister.stats.relation_boost_pct
                        ),
                    )
                }
                MinisterMissionKind::ProposeNonAggression => {
                    if rel.at_war {
                        (
                            false,
                            format!(
                                "No se puede firmar Pacto de No Agresión mientras exista guerra con {target_name}."
                            ),
                        )
                    } else if accepted {
                        rel.non_aggression_pact = true;
                        rel.opinion = (rel.opinion + 8.0).min(100.0);
                        (
                            true,
                            format!(
                                "ÉXITO ({chance_pct:.0}% • 0 días): {} selló un Pacto de No Agresión con {}.",
                                minister.profile.full_name, target_name
                            ),
                        )
                    } else {
                        rel.opinion = (rel.opinion - 2.0).max(-100.0);
                        (
                            false,
                            format!(
                                "RECHAZADO ({chance_pct:.0}% • 0 días): {} declinó el Pacto de No Agresión propuesto por {}.",
                                target_name, minister.profile.full_name
                            ),
                        )
                    }
                }
                MinisterMissionKind::ProposeAlliance => {
                    if rel.at_war {
                        (
                            false,
                            format!(
                                "No es posible pactar una Alianza en estado de guerra con {target_name}."
                            ),
                        )
                    } else if accepted {
                        rel.alliance = true;
                        rel.non_aggression_pact = true;
                        rel.opinion = (rel.opinion + 15.0).min(100.0);
                        (
                            true,
                            format!(
                                "ÉXITO ({chance_pct:.0}% • 0 días): {} logró una Alianza Estratégica con {}.",
                                minister.profile.full_name, target_name
                            ),
                        )
                    } else {
                        rel.opinion = (rel.opinion - 3.0).max(-100.0);
                        (
                            false,
                            format!(
                                "RECHAZADO ({chance_pct:.0}% • 0 días): {} rechazó la propuesta de Alianza de {}.",
                                target_name, minister.profile.full_name
                            ),
                        )
                    }
                }
                MinisterMissionKind::ProposePeaceTreaty => {
                    if !rel.at_war {
                        (
                            false,
                            format!("No existe conflicto bélico activo con {target_name}."),
                        )
                    } else if accepted {
                        rel.at_war = false;
                        rel.non_aggression_pact = true;
                        rel.opinion = (rel.opinion + 20.0).clamp(-100.0, 100.0);
                        (
                            true,
                            format!(
                                "ACUERDO DE PAZ ({chance_pct:.0}% • 0 días): {} negoció el fin de la guerra y tregua con {}.",
                                minister.profile.full_name, target_name
                            ),
                        )
                    } else {
                        (
                            false,
                            format!(
                                "RECHAZADO ({chance_pct:.0}% • 0 días): {} rechazó el Acuerdo de Paz de {}.",
                                target_name, minister.profile.full_name
                            ),
                        )
                    }
                }
                MinisterMissionKind::ProposeWhitePeace => {
                    if !rel.at_war {
                        (
                            false,
                            format!("No existe conflicto bélico activo con {target_name}."),
                        )
                    } else if accepted {
                        rel.at_war = false;
                        rel.opinion = (rel.opinion + 10.0).clamp(-100.0, 100.0);
                        (
                            true,
                            format!(
                                "PAZ BLANCA ({chance_pct:.0}% • 0 días): {} cerró una Paz Blanca sin condiciones con {}.",
                                minister.profile.full_name, target_name
                            ),
                        )
                    } else {
                        (
                            false,
                            format!(
                                "RECHAZADO ({chance_pct:.0}% • 0 días): {} no aceptó la Paz Blanca enviada por {}.",
                                target_name, minister.profile.full_name
                            ),
                        )
                    }
                }
            };

            state.last_diplomacy_feedback = Some((ok, summary.clone()));
            state.push_chronicle(player_country_id.clone(), summary);
        }
        PlayerAction::SignTradePact {
            player_country_id,
            target_country_id,
        } => {
            let target_name = state
                .countries
                .get(target_country_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| target_country_id.clone());
            let key = GameState::ordered_pair(player_country_id, target_country_id);
            let rel = state.relations.entry(key).or_default();
            if !rel.at_war && rel.opinion >= 20.0 {
                rel.trade_pact = true;
                rel.opinion = (rel.opinion + 10.0).min(100.0);
                let msg = format!("Tratado comercial firmado con {target_name}.");
                state.last_diplomacy_feedback = Some((true, msg.clone()));
                state.push_chronicle(player_country_id.clone(), msg);
            }
        }
        PlayerAction::IssueSanction {
            player_country_id,
            target_country_id,
        } => {
            let target_name = state
                .countries
                .get(target_country_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| target_country_id.clone());
            let key = GameState::ordered_pair(player_country_id, target_country_id);
            let rel = state.relations.entry(key).or_default();
            rel.opinion = (rel.opinion - 20.0).max(-100.0);
            rel.alliance = false;
            rel.trade_pact = false;
            rel.non_aggression_pact = false;
            let msg = format!("Tensión diplomática y sanciones emitidas hacia {target_name}.");
            state.last_diplomacy_feedback = Some((false, msg.clone()));
            state.push_chronicle(player_country_id.clone(), msg);
        }
        PlayerAction::DeclareWar {
            player_country_id,
            target_country_id,
        } => {
            let target_name = state
                .countries
                .get(target_country_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| target_country_id.clone());
            let key = GameState::ordered_pair(player_country_id, target_country_id);
            let rel = state.relations.entry(key).or_default();
            if !rel.at_war {
                rel.at_war = true;
                rel.alliance = false;
                rel.trade_pact = false;
                rel.non_aggression_pact = false;
                rel.opinion = (rel.opinion - 45.0).max(-100.0);
                let msg = format!(
                    "¡Estado de guerra declarado contra {target_name}! Puedes negociar Acuerdo de Paz o Paz Blanca con tu Ministro."
                );
                state.last_diplomacy_feedback = Some((false, msg.clone()));
                state.push_chronicle(player_country_id.clone(), msg);
            }
        }
    }
}

fn build_initial_state(
    lua: &LuaEngine,
    graph: &ProvinceGraph,
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
                    non_aggression_pact: false,
                    alliance: false,
                    at_war: false,
                },
            );
        }
    }

    recompute_country_balances(&mut countries);

    let seed = generate_runtime_seed("PREVIEW");
    assign_random_leaders_to_countries(&mut countries, seed);
    let mut next_minister_id = 1_u64;
    assign_initial_minister_pools(&mut countries, seed ^ 0x4D49_4E49, &mut next_minister_id);
    let mut state = GameState {
        campaign_seed: seed,
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
        regions: BTreeMap::new(),
        relations,
        units: Vec::new(),
        next_unit_id: 1,
        companies: Vec::new(),
        next_company_id: 1,
        active_event: None,
        chronicle: Vec::new(),
        global_cycle: GlobalMacroCycle::default(),
        last_event_id: None,
        last_event_day: 0,
        next_minister_id,
        last_diplomacy_feedback: None,
        pending_chronicle_archive: Vec::new(),
        pending_stock_archive: Vec::new(),
        pending_macro_archive: Vec::new(),
    };

    let mut working_graph = graph.clone();
    ensure_fallback_graph(&state.countries, &mut working_graph);
    setup_initial_regions(&mut state, &working_graph, seed);

    state.push_chronicle(
        "LATAM",
        "Selecciona una nación para configurar líderes, regiones, unidades, mercados y empresas iniciales.",
    );
    Ok(state)
}

fn recompute_country_balances(countries: &mut BTreeMap<String, crate::models::CountryState>) {
    for country in countries.values_mut() {
        country.recompute_fiscal_snapshot();
    }
}

/// Distribuye la población nacional y los desempleados reales entre todas las regiones/provincias
/// de cada país (ponderando capital, terreno y elevación) garantizando que la suma coincida.
pub fn setup_initial_regions(state: &mut GameState, graph: &ProvinceGraph, seed: u64) {
    state.regions.clear();
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xD340_69AF_2026);

    let country_ids: Vec<String> = state.countries.keys().cloned().collect();
    for cid in country_ids {
        let Some(country) = state.countries.get(&cid) else {
            continue;
        };
        let total_pop = country.population;
        let target_unemp_pct = country.economy.unemployment_pct;
        let capital_lower = country.capital.to_lowercase();

        let mut provs: Vec<&ProvinceNode> =
            graph.values().filter(|p| p.country_id == cid).collect();
        provs.sort_by(|a, b| a.mesh_name.cmp(&b.mesh_name));
        if provs.is_empty() {
            continue;
        }

        let has_named_capital = provs
            .iter()
            .any(|p| p.name.to_lowercase().contains(&capital_lower) || p.terrain_type.eq_ignore_ascii_case("Capital"));

        let mut weights: Vec<f64> = Vec::with_capacity(provs.len());
        let mut capital_idx = 0_usize;
        let mut best_weight = -1.0_f64;

        for (idx, p) in provs.iter().enumerate() {
            let is_cap = p.name.to_lowercase().contains(&capital_lower)
                || p.terrain_type.eq_ignore_ascii_case("Capital")
                || (!has_named_capital && idx == 0);
            let cap_boost = if is_cap { 2.40 } else { 1.0 };
            let terrain_factor = match p.terrain_type.as_str() {
                "Capital" | "Llanura" | "Valle" | "Costa" => 1.25,
                "Meseta" | "Altiplano" => 0.92,
                "Selva" | "Desierto" | "Montaña" => 0.58,
                _ => 1.0,
            };
            let elev_factor = (1.18 - p.elevation_avg * 0.22).clamp(0.55, 1.25);
            let jitter = rng.random_range(0.75_f64..=1.32_f64);
            let w = (cap_boost * terrain_factor * elev_factor * jitter).max(0.15);
            if w > best_weight {
                best_weight = w;
                capital_idx = idx;
            }
            weights.push(w);
        }

        let weight_sum: f64 = weights.iter().sum::<f64>().max(0.001);
        let mut reg_pops: Vec<u64> = Vec::with_capacity(provs.len());
        let mut assigned_pop = 0_u64;
        for w in &weights {
            let share = ((total_pop as f64) * (*w / weight_sum)).floor() as u64;
            let val = if total_pop >= provs.len() as u64 {
                share.max(1)
            } else {
                share
            };
            assigned_pop = assigned_pop.saturating_add(val);
            reg_pops.push(val);
        }
        if assigned_pop < total_pop {
            reg_pops[capital_idx] = reg_pops[capital_idx].saturating_add(total_pop - assigned_pop);
        } else if assigned_pop > total_pop {
            let mut excess = assigned_pop - total_pop;
            for pop_slot in &mut reg_pops {
                if excess == 0 {
                    break;
                }
                let take = (*pop_slot).saturating_sub(1).min(excess);
                *pop_slot -= take;
                excess -= take;
            }
        }

        // Distribuir desempleo nacional proporcionalmente entre las fuerzas laborales regionales
        let total_workforce: u64 = reg_pops
            .iter()
            .map(|&p| ((p as f64) * 0.48).round().max(1.0) as u64)
            .sum();
        let target_total_unemp = (((total_workforce as f64) * (target_unemp_pct / 100.0)).round()
            as u64)
            .min(total_workforce);

        let mut unemp_weights: Vec<f64> = Vec::with_capacity(provs.len());
        for &rp in &reg_pops {
            let wf = ((rp as f64) * 0.48).max(1.0);
            let local_var = rng.random_range(0.78_f64..=1.24_f64);
            unemp_weights.push(wf * local_var);
        }
        let uw_sum: f64 = unemp_weights.iter().sum::<f64>().max(1.0);

        let mut reg_unemps: Vec<u64> = Vec::with_capacity(provs.len());
        let mut assigned_unemp = 0_u64;
        for (idx, uw) in unemp_weights.iter().enumerate() {
            let wf = ((reg_pops[idx] as f64) * 0.48).round().max(1.0) as u64;
            let val = (((target_total_unemp as f64) * (*uw / uw_sum)).floor() as u64).min(wf);
            assigned_unemp = assigned_unemp.saturating_add(val);
            reg_unemps.push(val);
        }
        if assigned_unemp < target_total_unemp {
            let mut rem = target_total_unemp - assigned_unemp;
            for idx in 0..provs.len() {
                if rem == 0 {
                    break;
                }
                let wf = ((reg_pops[idx] as f64) * 0.48).round().max(1.0) as u64;
                let room = wf.saturating_sub(reg_unemps[idx]);
                let add = room.min(rem);
                reg_unemps[idx] += add;
                rem -= add;
            }
        }

        for (idx, p) in provs.into_iter().enumerate() {
            state.regions.insert(
                p.mesh_name.clone(),
                RegionState {
                    mesh_name: p.mesh_name.clone(),
                    country_id: cid.clone(),
                    name: p.name.clone(),
                    population: reg_pops[idx],
                    unemployed: reg_unemps[idx],
                    stationed_recruited: 0,
                },
            );
        }
    }
}

/// Sincroniza la población regional y la variación mensual del desempleo con el país tras el paso mensual Lua.
pub fn sync_country_regions_after_month(
    state: &mut GameState,
    country_id: &str,
    prev_unemp_pct: f64,
) {
    let Some(country) = state.countries.get(country_id) else {
        return;
    };
    let target_pop = country.population;
    let unemp_delta_pct = country.economy.unemployment_pct - prev_unemp_pct;

    let keys: Vec<String> = state
        .regions
        .iter()
        .filter(|(_, r)| r.country_id == country_id)
        .map(|(k, _)| k.clone())
        .collect();
    if keys.is_empty() {
        return;
    }

    let current_pop_sum: u64 = keys.iter().filter_map(|k| state.regions.get(k)).map(|r| r.population).sum();
    if current_pop_sum > 0 && target_pop > 0 {
        let ratio = (target_pop as f64) / (current_pop_sum as f64);
        let mut new_sum = 0_u64;
        let mut largest_key = keys[0].clone();
        let mut largest_pop = 0_u64;

        for k in &keys {
            if let Some(reg) = state.regions.get_mut(k) {
                let scaled = ((reg.population as f64) * ratio).round().max(1.0) as u64;
                reg.population = scaled;
                new_sum = new_sum.saturating_add(scaled);
                if scaled >= largest_pop {
                    largest_pop = scaled;
                    largest_key = k.clone();
                }
            }
        }
        if let Some(largest) = state.regions.get_mut(&largest_key) {
            if new_sum < target_pop {
                largest.population = largest.population.saturating_add(target_pop - new_sum);
            } else if new_sum > target_pop {
                let excess = new_sum - target_pop;
                largest.population = largest.population.saturating_sub(excess).max(1);
            }
        }
    }

    adjust_country_regional_unemployment(state, country_id, unemp_delta_pct);
}

/// Aplica un delta en puntos porcentuales de desempleo sobre las regiones del país
/// conservando los recortes realizados por reclutamiento militar previo.
pub fn adjust_country_regional_unemployment(
    state: &mut GameState,
    country_id: &str,
    unemp_pct_delta: f64,
) {
    if unemp_pct_delta.abs() < 1e-6 {
        return;
    }
    for reg in state.regions.values_mut().filter(|r| r.country_id == country_id) {
        let wf = reg.workforce();
        let people_delta = ((wf as f64) * (unemp_pct_delta / 100.0)).round() as i64;
        let next_unemp = ((reg.unemployed as i64) + people_delta).clamp(0, wf as i64) as u64;
        reg.unemployed = next_unemp;
    }
}

/// Configura una partida al elegir país: despliega soldados/unidades predeterminadas
/// y forma empresas privadas aleatoriamente (solo en algunos países, nunca estáticas).
fn configure_campaign_for_country(
    mut state: GameState,
    graph: &ProvinceGraph,
    company_templates: &[CompanyCategoryTemplate],
    player_country_id: &str,
    seed: u64,
    res_tx: &Sender<SimResponse>,
) -> Result<GameState, String> {
    if !state.countries.contains_key(player_country_id) {
        return Err(format!("País seleccionado inválido: {player_country_id}"));
    }

    state.campaign_seed = seed;
    state.player_country_id = Some(player_country_id.to_string());
    state.selected_country_id = Some(player_country_id.to_string());
    state.paused = false;

    let country_name = state
        .countries
        .get(player_country_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| player_country_id.to_string());

    let _ = res_tx.send(SimResponse::CampaignLoadProgress {
        progress: 0.18,
        stage: "Designando líderes nacionales".to_string(),
        detail: format!("Generando Presidente y oposición aleatoria para {country_name} y países IA..."),
        units_configured: 0,
        companies_founded: 0,
    });

    // Asignar aleatoriamente Presidente, líderes de oposición y ternas de Ministros de Relaciones
    assign_random_leaders_to_countries(&mut state.countries, seed ^ 0x9911_2026);
    assign_initial_minister_pools(
        &mut state.countries,
        seed ^ 0x4D49_4E49_2026,
        &mut state.next_minister_id,
    );

    // Asegura nodos de respaldo en memoria para cualquier país sin malla cargada aún
    let mut working_graph = graph.clone();
    ensure_fallback_graph(&state.countries, &mut working_graph);
    setup_initial_regions(&mut state, &working_graph, seed);

    // 1. Configurar soldados predeterminados y número de unidades tácticas
    setup_initial_units(&mut state, &working_graph, seed);

    let units_count = state.units.len();
    let _ = res_tx.send(SimResponse::CampaignLoadProgress {
        progress: 0.56,
        stage: "Desplegando unidades militares".to_string(),
        detail: format!("Configurados {units_count} regimientos y guarniciones en Latinoamérica."),
        units_configured: units_count,
        companies_founded: 0,
    });

    // 2. Formar empresas iniciales aleatoriamente en algunos países (nunca en todos ni fijas)
    seed_dynamic_initial_companies(&mut state, company_templates, seed);

    let comp_count = state.companies.len();
    let _ = res_tx.send(SimResponse::CampaignLoadProgress {
        progress: 0.88,
        stage: "Constituyendo mercado empresarial".to_string(),
        detail: format!(
            "Fundadas {comp_count} empresas aleatorias en plazas económicas seleccionadas."
        ),
        units_configured: units_count,
        companies_founded: comp_count,
    });

    // 3. Seleccionar provincia y unidad inicial del jugador y recalcular balances
    if let Some(first_player_prov) = working_graph
        .values()
        .find(|p| p.country_id == player_country_id)
        .map(|p| p.mesh_name.clone())
    {
        state.selected_province_mesh = Some(first_player_prov);
    }
    state.selected_unit_id = state
        .units
        .iter()
        .find(|u| u.country_id == player_country_id)
        .map(|u| u.id);

    recompute_country_balances(&mut state.countries);
    state.sync_country_troop_totals();
    for country in state.countries.values_mut() {
        let monthly_def =
            (country.economy.gdp_usd_millions / 12.0) * (country.military.defense_budget_pct / 100.0);
        country.military.ai_military_budget_m = (monthly_def * 0.65).clamp(40.0, 420.0);
        country.military.last_ai_recruit_day = 0;
    }

    let player_units = state
        .units
        .iter()
        .filter(|u| u.country_id == player_country_id)
        .count();
    let player_troops = state
        .countries
        .get(player_country_id)
        .map(|c| c.military.active_troops)
        .unwrap_or(0);
    let pres_summary = state
        .countries
        .get(player_country_id)
        .and_then(|c| c.president())
        .map(|l| format!("{} ({} años, {})", l.full_name, l.age, l.nationality))
        .unwrap_or_else(|| "Mandatario designado".to_string());

    state.push_chronicle(
        player_country_id.to_string(),
        format!(
            "Asumes el liderazgo de {country_name} con {pres_summary}: {player_units} unidades ({player_troops} soldados) y {comp_count} empresas en Latam."
        ),
    );

    save_player_campaign_thumbnail(&state);

    let _ = res_tx.send(SimResponse::CampaignLoadProgress {
        progress: 1.0,
        stage: "Campaña lista".to_string(),
        detail: format!("Despliegue completo en {country_name}."),
        units_configured: units_count,
        companies_founded: comp_count,
    });

    Ok(state)
}

fn setup_initial_units(state: &mut GameState, graph: &ProvinceGraph, seed: u64) {
    state.units.clear();
    state.next_unit_id = 1;

    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xA11C_E001_55AA_9911);
    let diff_ai_factor = match state.difficulty {
        Difficulty::Normal => 1.0,
        Difficulty::Desafiante => 1.08,
        Difficulty::Realista => 1.15,
    };

    let country_ids: Vec<String> = state.countries.keys().cloned().collect();
    for cid in country_ids {
        let Some(country) = state.countries.get_mut(&cid) else {
            continue;
        };

        let is_player = state.player_country_id.as_deref() == Some(cid.as_str());
        let base_troops = country.military.active_troops.max(1_500);
        let jitter = rng.random_range(0.92_f64..=1.10_f64);
        let scaled_troops = if is_player {
            ((base_troops as f64) * jitter).round() as u64
        } else {
            ((base_troops as f64) * jitter * diff_ai_factor).round() as u64
        };
        let total_troops = ((scaled_troops / 500).max(3)) * 500;
        country.military.active_troops = total_troops;

        let mut provs: Vec<&ProvinceNode> =
            graph.values().filter(|p| p.country_id == cid).collect();
        provs.sort_by(|a, b| a.mesh_name.cmp(&b.mesh_name));

        let max_divs = if provs.is_empty() {
            1
        } else if total_troops >= 120_000 {
            4.min(provs.len())
        } else if total_troops >= 45_000 {
            3.min(provs.len())
        } else if total_troops >= 12_000 {
            2.min(provs.len())
        } else {
            1
        };

        let div_count = if max_divs > 1 && rng.random_bool(0.45) {
            (max_divs - 1).max(1)
        } else {
            max_divs
        };

        // Elegir provincias de despliegue de forma variada según la semilla
        let mut chosen_indices: Vec<usize> = (0..provs.len()).collect();
        for i in (1..chosen_indices.len()).rev() {
            let j = rng.random_range(0..=i);
            chosen_indices.swap(i, j);
        }

        let mut remaining = total_troops;
        for d in 0..div_count {
            let troops = if d + 1 == div_count {
                remaining
            } else {
                let min_share = 1_000_u64.min(remaining);
                let ideal = (remaining / ((div_count - d) as u64)).max(min_share);
                let varied = ((ideal as f64) * rng.random_range(0.80_f64..=1.20_f64)).round() as u64;
                let share = ((varied / 500).max(2) * 500).min(remaining.saturating_sub(1_000));
                remaining = remaining.saturating_sub(share);
                share
            };

            let (prov_mesh, world_pos) = if !provs.is_empty() {
                let p = provs[chosen_indices[d % chosen_indices.len()]];
                (p.mesh_name.clone(), p.world_pos)
            } else {
                (format!("PROV_{cid}_0_CAP"), [0.0, 0.4, 0.0])
            };

            let uid = state.next_unit_id;
            state.next_unit_id += 1;
            state.units.push(ArmyUnit {
                id: uid,
                name: format!("Cuerpo {} #{}", cid, d + 1),
                country_id: cid.clone(),
                troops,
                current_province: prov_mesh,
                target_province: None,
                route_queue: Vec::new(),
                march_progress: 0.0,
                world_pos,
                morale: rng.random_range(78.0..=90.0),
            });
        }
    }
}

fn seed_dynamic_initial_companies(
    state: &mut GameState,
    company_templates: &[CompanyCategoryTemplate],
    seed: u64,
) {
    state.companies.clear();
    state.next_company_id = 1;
    if company_templates.is_empty() || state.countries.is_empty() {
        return;
    }

    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xC04A_91E5_7788_2026);
    let mut ranked: Vec<(String, f64)> = state
        .countries
        .iter()
        .map(|(cid, c)| {
            let gdp_factor = (c.economy.gdp_usd_millions / 80_000.0).sqrt().clamp(0.4, 3.2);
            let macro_score = (c.stability * 0.55
                + (c.economy.gdp_growth_pct + 4.0).max(0.5) * 6.5
                + gdp_factor * 12.0
                - (c.economy.inflation_pct - 8.0).max(0.0) * 1.1)
                .max(8.0);
            let random_roll = rng.random_range(0.45_f64..=1.65_f64);
            (cid.clone(), macro_score * random_roll)
        })
        .collect();

    // Mezcla ponderada sin usar nunca una lista fija ni asignar a todos los países
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let target_countries = rng
        .random_range(5_usize..=10_usize)
        .min((state.countries.len() / 2).max(4))
        .min(ranked.len());

    for (cid, _) in ranked.into_iter().take(target_countries) {
        let count_in_country = if rng.random_bool(0.24) { 2 } else { 1 };
        for _ in 0..count_in_country {
            let cat_idx = rng.random_range(0..company_templates.len());
            if let Some(comp_idx) =
                try_found_company_in(state, company_templates, &cid, &mut rng, cat_idx, false)
            {
                seed_initial_company_history(&mut state.companies[comp_idx], &mut rng);
            }
        }
    }
}

fn seed_initial_company_history(company: &mut CompanyEntity, rng: &mut ChaCha8Rng) {
    let mut price = company.share_price;
    let mut history = Vec::with_capacity(8);
    history.push(price);
    for _ in 0..7 {
        let step = rng.random_range(-0.025_f64..=0.028_f64);
        price = (price * (1.0 + step)).clamp(2.5, 2_500.0);
        history.push(price);
    }
    company.prev_share_price = history[history.len().saturating_sub(2)];
    company.share_price = *history.last().unwrap_or(&price);
    company.price_history = history;
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

    // 6. Al cambiar de mes: ciclo global, reglas macroeconómicas/parlamentarias en Lua 5.4, elecciones y eventos
    if new_month {
        step_global_macro_cycle(state);

        let ids: Vec<String> = state.countries.keys().cloned().collect();
        for id in ids {
            let corp_gdp = state.country_company_gdp_bonus(&id);
            let corp_export = state.country_company_export_bonus(&id);
            let corp_jobs = state.country_company_jobs(&id);
            let trade_pacts_count = state.country_trade_pacts_count(&id);
            let at_war = state.country_at_war(&id);
            let global_cycle_snapshot = state.global_cycle.clone();

            let mut prev_unemp_pct: Option<f64> = None;
            if let Some(country) = state.countries.get_mut(&id) {
                prev_unemp_pct = Some(country.economy.unemployment_pct);
                // Preservamos la tesorería diaria para no duplicar el cobro mensual en Lua
                let daily_treasury = country.economy.treasury_usd_millions;
                let step_ctx = LuaStepContext {
                    corporate_gdp_bonus_m: corp_gdp,
                    corporate_export_bonus_m: corp_export,
                    corporate_jobs_total: corp_jobs,
                    trade_pacts_count,
                    at_war,
                    global_cycle: &global_cycle_snapshot,
                };
                lua.step_country_via_lua(country, &step_ctx)
                    .map_err(|e| format!("Error ejecutando reglas Lua para {id}: {e}"))?;
                country.economy.treasury_usd_millions = daily_treasury;

                // Recarga mensual del presupuesto militar disponible para reclutamiento de la IA
                let monthly_gdp = country.economy.gdp_usd_millions / 12.0;
                let defense_alloc = monthly_gdp * (country.military.defense_budget_pct / 100.0);
                let fiscal_factor = if country.economy.last_monthly_balance >= 0.0 {
                    1.0
                } else if country.economy.last_monthly_balance > -(monthly_gdp * 0.008) {
                    0.55
                } else {
                    0.20
                };
                let recruit_share = if at_war { 0.55 } else { 0.35 };
                let monthly_topup = defense_alloc * recruit_share * fiscal_factor;
                let budget_cap = (defense_alloc * if at_war { 3.5 } else { 2.5 }).max(140.0);
                country.military.ai_military_budget_m =
                    (country.military.ai_military_budget_m + monthly_topup).min(budget_cap);
            }
            if let Some(prev_u) = prev_unemp_pct {
                sync_country_regions_after_month(state, &id, prev_u);
            }
        }

        process_elections(state);

        // Archivar instantánea macroeconómica mensual y refrescar miniatura en caché
        for (cid, c) in &state.countries {
            state.pending_macro_archive.push(MacroHistorySnapshot {
                day: state.total_days,
                year: state.year,
                month: state.month,
                country_id: cid.clone(),
                gdp_usd_millions: c.economy.gdp_usd_millions,
                inflation_pct: c.economy.inflation_pct,
                stability: c.stability,
            });
        }
        save_player_campaign_thumbnail(state);

        // Evaluación contextual de eventos por causas reales (nunca % 4)
        if state.active_event.is_none() {
            if let Some(pid) = state.player_country_id.clone() {
                if let Some(player_c) = state.countries.get(&pid) {
                    let severe_trigger = player_c.stability < 42.0
                        || player_c.economy.inflation_pct > 15.0
                        || player_c.economy.country_risk_bps > 750.0;
                    let min_cooldown = if severe_trigger { 45 } else { 85 };
                    if state.total_days.saturating_sub(state.last_event_day) >= min_cooldown {
                        let company_count =
                            state.companies.iter().filter(|c| c.country_id == pid).count();
                        let dominant_sector = state
                            .companies
                            .iter()
                            .filter(|c| c.country_id == pid)
                            .max_by(|a, b| {
                                a.annual_gdp_contribution_m
                                    .partial_cmp(&b.annual_gdp_contribution_m)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|c| c.category.label())
                            .unwrap_or("Industria y Servicios");
                        let at_war = state.country_at_war(&pid);
                        let ev_ctx = LuaEventContext {
                            country: player_c,
                            company_count,
                            dominant_sector,
                            at_war,
                            global_cycle: &state.global_cycle,
                            last_event_id: state.last_event_id.as_deref(),
                            total_days: state.total_days,
                        };
                        if let Some(ev) = lua
                            .pick_contextual_event(&ev_ctx)
                            .map_err(|e| format!("Error generando evento Lua: {e}"))?
                        {
                            state.last_event_id = Some(ev.id.clone());
                            state.last_event_day = state.total_days;
                            state.active_event = Some(ev);
                        }
                    }
                }
            }
        }
    }

    // 7. Volcado periódico del buffer temporal de crónica, gráficas viejas e instantáneas a caché en disco
    if state.total_days % 10 == 0
        || state.pending_chronicle_archive.len() >= 8
        || state.pending_stock_archive.len() >= 32
        || state.pending_macro_archive.len() >= 21
    {
        flush_history_buffer_to_cache(state);
    }

    Ok(())
}

fn step_global_macro_cycle(state: &mut GameState) {
    let phase = (state.total_days as f64) / 180.0;
    let seed_offset = ((state.campaign_seed % 360) as f64).to_radians();
    let wave_fed = (phase * 0.55 + seed_offset).sin();
    let wave_comm = (phase * 0.42 + seed_offset * 1.7).cos();

    let mut rng = ChaCha8Rng::seed_from_u64(
        state
            .campaign_seed
            .wrapping_add(state.total_days.wrapping_mul(0x9E37_79B9)),
    );
    let fed_target = (4.10 + wave_fed * 1.45 + rng.random_range(-0.20_f64..=0.20_f64)).clamp(1.75, 6.25);
    let comm_target =
        (100.0 + wave_comm * 16.0 + rng.random_range(-2.5_f64..=2.5_f64)).clamp(76.0, 128.0);

    state.global_cycle.fed_rate_pct =
        (state.global_cycle.fed_rate_pct * 0.72 + fed_target * 0.28).clamp(1.75, 6.25);
    state.global_cycle.commodity_index =
        (state.global_cycle.commodity_index * 0.75 + comm_target * 0.25).clamp(76.0, 128.0);

    state.global_cycle.cycle_label = if state.global_cycle.commodity_index >= 107.0 {
        "Auge Materias Primas".to_string()
    } else if state.global_cycle.fed_rate_pct >= 5.0 {
        "Restricción Financiera".to_string()
    } else if state.global_cycle.fed_rate_pct <= 3.1 && state.global_cycle.commodity_index >= 98.0 {
        "Expansión Global".to_string()
    } else {
        "Equilibrio Global".to_string()
    };
}

fn save_player_campaign_thumbnail(state: &GameState) {
    let Some(pid) = state.player_country_id.as_deref() else {
        return;
    };
    let Some(c) = state.countries.get(pid) else {
        return;
    };
    let president_name = c
        .president()
        .map(|l| l.full_name.clone())
        .unwrap_or_else(|| "Mandatario".to_string());
    crate::cache::save_campaign_thumbnail(&CampaignThumbnail {
        campaign_seed: state.campaign_seed,
        country_id: c.id.clone(),
        country_name: c.name.clone(),
        president_name,
        date_label: state.date_label(),
        gdp_usd_millions: c.economy.gdp_usd_millions,
        active_troops: c.military.active_troops,
        companies_count: state.companies.len(),
    });
}

fn flush_history_buffer_to_cache(state: &mut GameState) {
    if state.pending_chronicle_archive.is_empty()
        && state.pending_stock_archive.is_empty()
        && state.pending_macro_archive.is_empty()
    {
        return;
    }
    let chronicle_batch = std::mem::take(&mut state.pending_chronicle_archive);
    let stock_batch = std::mem::take(&mut state.pending_stock_archive);
    let macro_batch = std::mem::take(&mut state.pending_macro_archive);
    crate::cache::append_campaign_history_batch(
        state.campaign_seed,
        &chronicle_batch,
        &stock_batch,
        &macro_batch,
    );
}

fn step_unit_movements(state: &mut GameState, graph: &ProvinceGraph) {
    let mut arrivals: Vec<(String, String, String)> = Vec::new();
    let mut country_map_updates: Vec<(String, String)> = Vec::new();

    let units = &mut state.units;
    let relations = &state.relations;
    let player_country_id = state.player_country_id.as_deref();

    for unit in units {
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

        let has_access = if dest_owner == unit.country_id {
            true
        } else {
            let rel = relations
                .get(&GameState::ordered_pair(&unit.country_id, &dest_owner))
                .cloned()
                .unwrap_or_default();
            rel.alliance || rel.at_war
        };
        if !has_access {
            unit.target_province = None;
            unit.route_queue.clear();
            unit.march_progress = 0.0;
            continue;
        }

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

            let at_war = if dest_owner != unit.country_id {
                relations
                    .get(&GameState::ordered_pair(&unit.country_id, &dest_owner))
                    .map(|r| r.at_war)
                    .unwrap_or(false)
            } else {
                false
            };
            if at_war {
                country_map_updates.push((unit.country_id.clone(), dest_name.clone()));
            }

            if !unit.route_queue.is_empty() {
                unit.target_province = Some(unit.route_queue.remove(0));
            } else {
                unit.target_province = None;
                if player_country_id == Some(unit.country_id.as_str()) {
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
    let day_seed = state
        .campaign_seed
        .wrapping_add(state.total_days.wrapping_mul(0x517C_C1B7_2722_0A95))
        ^ state.next_company_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ 0xA5A5_5A5A;
    let mut rng = ChaCha8Rng::seed_from_u64(day_seed);

    let mut pact_counts: BTreeMap<String, u32> = BTreeMap::new();
    for cid in state.countries.keys() {
        pact_counts.insert(cid.clone(), state.country_trade_pacts_count(cid));
    }
    let commodity_factor = (state.global_cycle.commodity_index - 100.0) / 100.0;

    // 1. Ciclo bidireccional diario: bolsa, salud financiera, expansión o recorte/quiebra
    let mut evicted_stocks: Vec<ArchivedStockSample> = Vec::new();
    let mut unemp_adjustments: Vec<(String, f64, f64)> = Vec::new();
    let mut bankrupt_indices: Vec<usize> = Vec::new();
    let current_day = state.total_days;

    for (idx, comp) in state.companies.iter_mut().enumerate() {
        let Some(c) = state.countries.get(&comp.country_id) else {
            continue;
        };
        comp.prev_share_price = comp.share_price;

        let real_rate = c.economy.interest_rate_pct - c.economy.inflation_pct;
        let growth_drift = (c.economy.gdp_growth_pct - 1.8) * 0.0020;
        let inflation_drag = (c.economy.inflation_pct - 6.0).max(0.0)
            * 0.00075
            * comp.inflation_sensitivity;
        let rate_effect = -((real_rate - 3.0).max(0.0) * 0.00075 * comp.rate_sensitivity);
        let stability_drift = (c.stability - 55.0) * 0.00018;
        let export_lift = comp.export_share * commodity_factor * 0.0045;
        let noise = rng.random_range(-0.015_f64..=0.015_f64);

        let pct_change = (growth_drift - inflation_drag + rate_effect + stability_drift + export_lift + noise)
            .clamp(-0.06, 0.06);
        comp.share_price = (comp.share_price * (1.0 + pct_change)).clamp(2.5, 2_500.0);

        comp.price_history.push(comp.share_price);
        if comp.price_history.len() > 24 {
            let old_price = comp.price_history.remove(0);
            evicted_stocks.push(ArchivedStockSample {
                company_id: comp.id,
                company_name: comp.name.clone(),
                day: current_day,
                share_price: old_price,
            });
        }

        // Evolución bidireccional de la salud financiera corporativa
        let pacts = pact_counts.get(&comp.country_id).copied().unwrap_or(0) as f64;
        let macro_health_delta = (c.economy.gdp_growth_pct - 1.6) * 0.22
            - (real_rate - 5.5).max(0.0) * 0.16 * comp.rate_sensitivity
            - (c.economy.inflation_pct - 10.0).max(0.0) * 0.08 * comp.inflation_sensitivity
            - (46.0 - c.stability).max(0.0) * 0.07
            - (c.economy.tax_rate_pct - 26.0).max(0.0) * 0.06
            + pacts * 0.05
            + comp.export_share * commodity_factor * 0.45;
        comp.financial_health = (comp.financial_health + macro_health_delta).clamp(0.0, 100.0);

        if comp.financial_health <= 10.0 {
            comp.distress_days += 1;
        } else if comp.financial_health >= 30.0 {
            comp.distress_days = comp.distress_days.saturating_sub(1);
        }

        if comp.distress_days >= 18 {
            bankrupt_indices.push(idx);
            continue;
        }

        // Recorte de personal por estrés o contratación orgánica en auge
        let workforce = ((c.population as f64) * 0.48).max(120_000.0);
        if comp.financial_health < 42.0 && current_day % 6 == 0 && comp.employees > 450 {
            let laid_off = ((comp.employees as f64) * 0.05).round().max(50.0) as u64;
            let actual_loss = laid_off.min(comp.employees.saturating_sub(350));
            if actual_loss > 0 {
                let gdp_per_job = comp.annual_gdp_contribution_m / (comp.employees.max(1) as f64);
                comp.employees -= actual_loss;
                comp.annual_gdp_contribution_m = (comp.employees as f64) * gdp_per_job;
                let unemp_inc = ((actual_loss as f64 / workforce) * 100.0 * 1.25).clamp(0.02, 0.65);
                unemp_adjustments.push((comp.country_id.clone(), unemp_inc, 0.0));
            }
        } else if comp.financial_health > 82.0
            && c.economy.gdp_growth_pct > 2.3
            && current_day % 12 == 0
            && comp.employees < 45_000
        {
            let hired = ((comp.employees as f64) * 0.025).round().max(25.0) as u64;
            let gdp_per_job = comp.annual_gdp_contribution_m / (comp.employees.max(1) as f64);
            comp.employees += hired;
            comp.annual_gdp_contribution_m = (comp.employees as f64) * gdp_per_job;
            let unemp_dec = -((hired as f64 / workforce) * 100.0 * 1.15).clamp(0.01, 0.45);
            unemp_adjustments.push((comp.country_id.clone(), unemp_dec, 0.0));
        }
    }
    state.pending_stock_archive.extend(evicted_stocks);

    // Procesar quiebras reales de empresas insolventes
    if !bankrupt_indices.is_empty() {
        let mut closed_logs: Vec<(String, String)> = Vec::new();
        for idx in bankrupt_indices.into_iter().rev() {
            let comp = state.companies.remove(idx);
            if let Some(c) = state.countries.get(&comp.country_id) {
                let workforce = ((c.population as f64) * 0.48).max(120_000.0);
                let unemp_inc = ((comp.employees as f64 / workforce) * 100.0 * 1.35).clamp(0.05, 1.2);
                unemp_adjustments.push((
                    comp.country_id.clone(),
                    unemp_inc,
                    -comp.annual_gdp_contribution_m * 0.12,
                ));
                closed_logs.push((
                    comp.country_id.clone(),
                    format!(
                        "Quiebra corporativa en {}: {} ({}) cierra tras perder {} empleos.",
                        c.name,
                        comp.name,
                        comp.category.label(),
                        comp.employees
                    ),
                ));
            }
        }
        for (cid, msg) in closed_logs {
            state.push_chronicle(cid, msg);
        }
    }

    for (cid, unemp_delta, gdp_delta) in unemp_adjustments {
        if let Some(c) = state.countries.get_mut(&cid) {
            let prev_u = c.economy.unemployment_pct;
            c.economy.unemployment_pct = (prev_u + unemp_delta).clamp(2.2, 34.0);
            let applied_delta = c.economy.unemployment_pct - prev_u;
            if gdp_delta != 0.0 {
                c.economy.gdp_usd_millions = (c.economy.gdp_usd_millions + gdp_delta).max(100.0);
            }
            adjust_country_regional_unemployment(state, &cid, applied_delta);
        }
    }

    // Sincronizar bono corporativo al PIB de cada país tras crecimientos o quiebras
    let country_ids: Vec<String> = state.countries.keys().cloned().collect();
    for cid in country_ids {
        let bonus = state.country_company_gdp_bonus(&cid);
        if let Some(c) = state.countries.get_mut(&cid) {
            c.economy.corporate_gdp_bonus_m = bonus;
        }
    }

    // 2. Cada 5 días de juego, evaluar dinámicamente el nacimiento de nuevas empresas (nunca estático)
    if state.total_days % 5 != 0 || templates.is_empty() || state.companies.len() >= 80 {
        return;
    }

    let spawn_prob = if state.companies.len() < 24 {
        0.74
    } else if state.companies.len() < 48 {
        0.52
    } else {
        0.30
    };
    if !rng.random_bool(spawn_prob) {
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
        if existing_count >= 7 {
            continue;
        }
        let real_rate = c.economy.interest_rate_pct - c.economy.inflation_pct;
        let score = (22.0
            + c.stability * 0.50
            + (c.economy.gdp_growth_pct + 3.0).max(0.2) * 7.0
            + c.economy.infra_investment_pct * 3.5
            - (real_rate - 4.5).max(0.0) * 2.2
            - (c.economy.inflation_pct - 10.0).max(0.0) * 1.1
            - (c.economy.tax_rate_pct - 24.0).max(0.0) * 1.3)
            / ((existing_count as f64) * 0.65 + 1.0);
        if score > 8.0 {
            weights.push((cid, score * rng.random_range(0.75_f64..=1.35_f64)));
        }
    }

    if weights.is_empty() {
        return;
    }

    let total_w: f64 = weights.iter().map(|(_, w)| *w).sum();
    let mut pick = rng.random_range(0.0_f64..total_w);
    let mut chosen_cid = weights[0].0.clone();
    for (cid, w) in weights {
        if pick <= w {
            chosen_cid = cid;
            break;
        }
        pick -= w;
    }

    let cat_idx = rng.random_range(0..templates.len());
    let _ = try_found_company_in(state, templates, &chosen_cid, &mut rng, cat_idx, true);
}

fn default_export_share_for(category: CompanyCategory, rng: &mut ChaCha8Rng) -> f64 {
    let (lo, hi) = match category {
        CompanyCategory::Agroindustria => (0.42, 0.72),
        CompanyCategory::Energia => (0.38, 0.68),
        CompanyCategory::Tecnologia => (0.35, 0.65),
        CompanyCategory::Manufactura => (0.30, 0.58),
        CompanyCategory::Logistica => (0.24, 0.48),
        CompanyCategory::Finanzas => (0.12, 0.30),
        CompanyCategory::Salud => (0.10, 0.25),
    };
    rng.random_range(lo..=hi)
}

fn try_found_company_in(
    state: &mut GameState,
    templates: &[CompanyCategoryTemplate],
    country_id: &str,
    rng: &mut ChaCha8Rng,
    cat_idx: usize,
    log_chronicle: bool,
) -> Option<usize> {
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
    let export_share = default_export_share_for(tmpl.category, rng);
    let initial_health = rng.random_range(70.0_f64..=88.0_f64);

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
        financial_health: initial_health,
        export_share,
        distress_days: 0,
    };
    state.companies.push(company);
    let inserted_idx = state.companies.len() - 1;

    // Impacto inmediato y lógico en empleo (baja desempleo) y PIB del país
    let mut applied_unemp_delta = 0.0_f64;
    if let Some(c) = state.countries.get_mut(country_id) {
        let workforce = ((c.population as f64) * 0.48).max(120_000.0);
        let unemp_reduction = ((employees as f64 / workforce) * 100.0 * 1.4).clamp(0.05, 1.4);
        let prev_u = c.economy.unemployment_pct;
        c.economy.unemployment_pct = (prev_u - unemp_reduction).max(2.2);
        applied_unemp_delta = c.economy.unemployment_pct - prev_u;
        c.economy.corporate_gdp_bonus_m += annual_gdp_contribution_m;
        c.economy.gdp_usd_millions += annual_gdp_contribution_m * 0.25;
        c.economy.gdp_growth_pct = (c.economy.gdp_growth_pct + 0.08).min(11.5);
        c.recompute_fiscal_snapshot();
    }
    adjust_country_regional_unemployment(state, country_id, applied_unemp_delta);

    if log_chronicle {
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
    }
    Some(inserted_idx)
}
