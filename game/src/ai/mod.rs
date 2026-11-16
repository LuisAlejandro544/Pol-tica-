//! Propósito del archivo:
//! Motor de Inteligencia Artificial local 100% escrito en Rust para todas las naciones
//! rivales de Latinoamérica. Evalúa indicadores macroeconómicos, riesgo de inflación,
//! presión parlamentaria, presupuesto militar mensual dedicado, tope de tropas en paz/guerra,
//! enfriamiento de reclutamiento sin spam de unidades, maniobras entre estados/provincias
//! y relaciones diplomáticas usando paralelismo con `rayon` en hilos de fondo.

use crate::models::{
    AiPersonality, ArmyUnit, CountryState, Difficulty, GameState, ProvinceGraph,
};
use rand::{rngs::ChaCha8Rng, RngExt, SeedableRng};
use rayon::prelude::*;
use std::collections::VecDeque;

/// Decisión emitida por la IA local para aplicarse al estado global tras el cálculo paralelo.
#[derive(Debug, Clone)]
pub struct AiDecision {
    pub country_id: String,
    pub new_tax_rate: f64,
    pub new_interest_rate: f64,
    pub new_infra_pct: f64,
    pub new_social_pct: f64,
    pub new_defense_pct: f64,
    pub recruit_troops: u64,
    pub recruit_province: Option<String>,
    pub move_unit: Option<(u64, String)>,
    pub diplomacy_target: Option<(String, AiDiplomaticIntent)>,
    pub log_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDiplomaticIntent {
    ProposeTradePact,
    ImproveRelations,
    ProposeAlliance,
    DenounceRival,
}

/// Evalúa en paralelo todas las naciones controladas por la IA utilizando `rayon`.
pub fn evaluate_ai_countries(state: &GameState, graph: &ProvinceGraph) -> Vec<AiDecision> {
    let player_id = state.player_country_id.as_deref();
    let difficulty_mult = match state.difficulty {
        Difficulty::Normal => 1.0,
        Difficulty::Desafiante => 1.15,
        Difficulty::Realista => 1.3,
    };

    let ai_ids: Vec<&String> = state
        .countries
        .keys()
        .filter(|id| Some(id.as_str()) != player_id)
        .collect();

    ai_ids
        .par_iter()
        .filter_map(|&country_id| {
            let country = state.countries.get(country_id)?;
            Some(decide_for_country(country, state, graph, difficulty_mult))
        })
        .collect()
}

/// Reserva mínima de tesorería (M USD) proporcional al PIB mensual que la IA debe conservar.
pub fn compute_ai_min_treasury_reserve(country: &CountryState) -> f64 {
    let monthly_gdp = country.economy.gdp_usd_millions / 12.0;
    (monthly_gdp * 0.45).clamp(150.0, 3_500.0)
}

/// Calcula el tope objetivo de tropas sostenibles para una nación IA según población,
/// capacidad fiscal (límite de mantenimiento), personalidad y estado de paz/guerra.
pub fn compute_ai_target_troops(
    country: &CountryState,
    state: &GameState,
    difficulty_mult: f64,
) -> u64 {
    let at_war = state.country_at_war(&country.id);
    let has_hostile_neighbor = country.neighbors.iter().any(|nb| {
        let rel = state.get_relation(&country.id, nb);
        !rel.alliance && !rel.non_aggression_pact && rel.opinion < -35.0
    });

    let pop_cap = (country.population as f64) * 0.0024;
    let monthly_gdp = (country.economy.gdp_usd_millions / 12.0).max(50.0);
    let monthly_tax_rev = monthly_gdp * (country.economy.tax_rate_pct / 100.0);
    let max_upkeep_share = if at_war {
        0.28
    } else if has_hostile_neighbor {
        0.20
    } else {
        0.16
    };
    let econ_troop_cap = ((monthly_tax_rev * max_upkeep_share) / 1.35) * 1_000.0;
    let base_cap = pop_cap.min(econ_troop_cap * 1.15).max(1_500.0);

    let personality_mult = match country.ai_personality {
        AiPersonality::Economista => 0.80,
        AiPersonality::Nacionalista => 1.18,
        AiPersonality::Reformista => 0.95,
        AiPersonality::Diplomatico => 0.85,
    };
    let geo_mult = if at_war {
        1.55
    } else if has_hostile_neighbor {
        1.18
    } else {
        1.0
    };

    let raw = base_cap * personality_mult * geo_mult * difficulty_mult;
    ((raw / 500.0).floor() as u64).max(3) * 500
}

fn decide_for_country(
    country: &CountryState,
    state: &GameState,
    graph: &ProvinceGraph,
    difficulty_mult: f64,
) -> AiDecision {
    let mut seed_val: u64 = state.total_days.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in country.id.bytes() {
        seed_val = seed_val.rotate_left(7) ^ (b as u64);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed_val);

    let mut tax = country.economy.tax_rate_pct;
    let mut interest = country.economy.interest_rate_pct;
    let mut infra = country.economy.infra_investment_pct;
    let mut social = country.economy.social_investment_pct;
    let mut defense = country.military.defense_budget_pct;
    let mut recruit_troops = 0_u64;
    let mut recruit_province = None;
    let mut move_unit = None;
    let mut log_message = None;

    // 1. Política monetaria estilo Regla de Taylor frente a tasa neutral y riesgo país
    let inf_target = country.economy.inflation_target_pct.max(2.5);
    let neutral_real = 2.25 + (country.economy.country_risk_bps / 1400.0);
    let taylor_rate = (inf_target
        + neutral_real
        + (country.economy.inflation_pct - inf_target) * 1.35
        + (country.economy.gdp_growth_pct - country.economy.potential_gdp_growth_pct) * 0.45)
        .clamp(2.0, 48.0);
    let rate_step = (taylor_rate - interest).clamp(-0.65, 0.75 * difficulty_mult);
    interest = (interest + rate_step).clamp(2.0, 48.0);

    // 2. Política fiscal, presupuesto militar dedicado, tope paz/guerra y límite de mantenimiento
    let own_provinces: Vec<&String> = graph
        .iter()
        .filter(|(_, node)| node.country_id == country.id)
        .map(|(mesh, _)| mesh)
        .collect();

    let at_war = state.country_at_war(&country.id);
    let has_hostile_neighbor = country.neighbors.iter().any(|nb| {
        let rel = state.get_relation(&country.id, nb);
        !rel.alliance && !rel.non_aggression_pact && rel.opinion < -35.0
    });

    let monthly_gdp = (country.economy.gdp_usd_millions / 12.0).max(50.0);
    let monthly_tax_rev = monthly_gdp * (country.economy.tax_rate_pct / 100.0);
    let target_troops = compute_ai_target_troops(country, state, difficulty_mult);
    let troop_deficit = target_troops.saturating_sub(country.military.active_troops);

    let cooldown_days = if at_war { 18 } else { 30 };
    let cooldown_ready =
        state.total_days.saturating_sub(country.military.last_ai_recruit_day) >= cooldown_days;

    let min_reserve = compute_ai_min_treasury_reserve(country);
    let max_upkeep_share = if at_war { 0.28 } else { 0.16 };
    let max_upkeep_budget = monthly_tax_rev * max_upkeep_share;
    let fiscal_healthy = country.economy.last_monthly_balance > -(monthly_gdp * 0.008);

    // Ajuste prudente del presupuesto de defensa según situación fiscal y necesidad militar
    if !at_war && (country.military.active_troops >= target_troops || !fiscal_healthy) {
        defense = (defense - 0.10).max(0.5);
    }

    let desired_batch = match country.ai_personality {
        AiPersonality::Economista => {
            if country.economy.debt_to_gdp_pct > 75.0 {
                tax = (tax + 0.5).min(32.0);
                infra = (infra + 0.25).min(6.0);
            } else {
                infra = (infra + 0.4).min(7.5);
            }
            if at_war || (troop_deficit >= 1_500 && rng.random_bool(0.35)) {
                1_500
            } else {
                0
            }
        }
        AiPersonality::Nacionalista => {
            if (at_war || has_hostile_neighbor || troop_deficit > 2_000) && fiscal_healthy {
                defense = (defense + 0.15 * difficulty_mult).min(3.8);
            }
            if troop_deficit >= 1_000 && (at_war || rng.random_bool(0.55)) {
                let scaled = ((country.population / 22_000).clamp(1_000, 3_000) / 500) * 500;
                if at_war {
                    scaled.max(1_500)
                } else {
                    scaled.min(2_000)
                }
            } else {
                0
            }
        }
        AiPersonality::Reformista => {
            if country.stability < 62.0 {
                social = (social + 0.45).min(8.5);
            }
            if country.economy.gdp_growth_pct < 2.5 {
                tax = (tax - 0.3).max(12.0);
            }
            if troop_deficit >= 1_000 && (at_war || rng.random_bool(0.40)) {
                1_000
            } else {
                0
            }
        }
        AiPersonality::Diplomatico => {
            social = (social + 0.2).min(7.0);
            infra = (infra + 0.25).min(6.5);
            if troop_deficit >= 1_000 && (at_war || rng.random_bool(0.35)) {
                1_000
            } else {
                0
            }
        }
    };

    let candidate_batch = (desired_batch.min(troop_deficit) / 500) * 500;
    if cooldown_ready && candidate_batch >= 500 && fiscal_healthy {
        let batch_cost_m = (candidate_batch as f64 / 1_000.0) * 70.0;
        let projected_upkeep =
            ((country.military.active_troops + candidate_batch) as f64 / 1_000.0) * 1.35;

        if country.military.ai_military_budget_m >= batch_cost_m
            && country.economy.treasury_usd_millions >= min_reserve + batch_cost_m
            && projected_upkeep <= max_upkeep_budget
        {
            recruit_troops = candidate_batch;
        }
    }

    if recruit_troops > 0 && !own_provinces.is_empty() {
        let idx = rng.random_range(0..own_provinces.len());
        recruit_province = Some(own_provinces[idx].clone());
    }

    // 3. Movimiento estratégico de unidades ociosas entre estados del país (o rivales en guerra)
    let idle_units: Vec<&ArmyUnit> = state
        .units
        .iter()
        .filter(|u| u.country_id == country.id && u.target_province.is_none())
        .collect();

    if !idle_units.is_empty() && !own_provinces.is_empty() && rng.random_bool(0.42) {
        let u = idle_units[rng.random_range(0..idle_units.len())];
        let war_targets: Vec<String> = graph
            .iter()
            .filter(|(_, node)| {
                node.country_id != country.id
                    && state.get_relation(&country.id, &node.country_id).at_war
            })
            .map(|(mesh, _)| mesh.clone())
            .collect();

        if !war_targets.is_empty() {
            let t = war_targets[rng.random_range(0..war_targets.len())].clone();
            if t != u.current_province {
                move_unit = Some((u.id, t));
            }
        } else if own_provinces.len() > 1 {
            let dest = own_provinces[rng.random_range(0..own_provinces.len())].clone();
            if dest != u.current_province {
                move_unit = Some((u.id, dest));
            }
        }
    }

    // 4. Evaluación diplomática hacia países vecinos
    let mut diplomacy_target = None;
    if !country.neighbors.is_empty() && rng.random_bool(0.32) {
        let idx = rng.random_range(0..country.neighbors.len());
        let neighbor_id = &country.neighbors[idx];
        let rel = state.get_relation(&country.id, neighbor_id);

        if let Some(neighbor) = state.countries.get(neighbor_id) {
            let my_power = country.military_power_score();
            let their_power = neighbor.military_power_score();

            if !rel.trade_pact && rel.opinion > 25.0 {
                diplomacy_target = Some((neighbor_id.clone(), AiDiplomaticIntent::ProposeTradePact));
                log_message = Some(format!(
                    "{} propone tratado comercial a {}.",
                    country.name, neighbor.name
                ));
            } else if !rel.alliance
                && rel.opinion > 65.0
                && country.ai_personality == AiPersonality::Diplomatico
            {
                diplomacy_target = Some((neighbor_id.clone(), AiDiplomaticIntent::ProposeAlliance));
                log_message = Some(format!(
                    "{} busca alianza estratégica con {}.",
                    country.name, neighbor.name
                ));
            } else if rel.opinion < -35.0
                && country.ai_personality == AiPersonality::Nacionalista
                && my_power > their_power * 0.9
            {
                diplomacy_target = Some((neighbor_id.clone(), AiDiplomaticIntent::DenounceRival));
                log_message = Some(format!(
                    "{} emite nota diplomática firme hacia {}.",
                    country.name, neighbor.name
                ));
            } else if rel.opinion < 50.0 {
                diplomacy_target = Some((neighbor_id.clone(), AiDiplomaticIntent::ImproveRelations));
            }
        }
    }

    AiDecision {
        country_id: country.id.clone(),
        new_tax_rate: tax,
        new_interest_rate: interest,
        new_infra_pct: infra,
        new_social_pct: social,
        new_defense_pct: defense,
        recruit_troops,
        recruit_province,
        move_unit,
        diplomacy_target,
        log_message,
    }
}

/// Calcula la ruta de provincias desde `from` hasta `to` respetando acceso propio, aliado o en guerra.
pub fn find_province_route(
    graph: &ProvinceGraph,
    state: &GameState,
    country_id: &str,
    from: &str,
    to: &str,
) -> Vec<String> {
    if from == to {
        return Vec::new();
    }
    if !graph.contains_key(from) || !graph.contains_key(to) {
        return Vec::new();
    }

    let can_enter = |target_owner: &str| -> bool {
        if target_owner == country_id {
            return true;
        }
        let rel = state.get_relation(country_id, target_owner);
        rel.alliance || rel.at_war
    };

    if let Some(dest_node) = graph.get(to) {
        if !can_enter(&dest_node.country_id) {
            return Vec::new();
        }
    }

    let mut queue = VecDeque::new();
    let mut visited = std::collections::BTreeSet::new();
    let mut parent: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    queue.push_back(from.to_string());
    visited.insert(from.to_string());

    let mut found = false;
    while let Some(cur) = queue.pop_front() {
        if cur == to {
            found = true;
            break;
        }
        if let Some(node) = graph.get(&cur) {
            for nb in &node.neighbors {
                if visited.contains(nb) {
                    continue;
                }
                if let Some(nb_node) = graph.get(nb) {
                    if can_enter(&nb_node.country_id) {
                        visited.insert(nb.clone());
                        parent.insert(nb.clone(), cur.clone());
                        queue.push_back(nb.clone());
                    }
                }
            }
        }
    }

    if found {
        let mut path = Vec::new();
        let mut step = to.to_string();
        while step != from {
            path.push(step.clone());
            if let Some(prev) = parent.get(&step) {
                step = prev.clone();
            } else {
                break;
            }
        }
        path.reverse();
        return path;
    }

    // Respaldo aéreo/anfibio dentro del mismo país o aliados si la geometría no toca
    vec![to.to_string()]
}

/// Aplica las decisiones calculadas en paralelo al `GameState`.
pub fn apply_ai_decisions(
    state: &mut GameState,
    graph: &ProvinceGraph,
    decisions: Vec<AiDecision>,
) {
    for d in decisions {
        let mut recruited_info: Option<(String, u64, String)> = None;
        let at_war = state.country_at_war(&d.country_id);
        let total_days = state.total_days;

        if let Some(country) = state.countries.get_mut(&d.country_id) {
            country.economy.tax_rate_pct = d.new_tax_rate;
            country.economy.interest_rate_pct = d.new_interest_rate;
            country.economy.infra_investment_pct = d.new_infra_pct;
            country.economy.social_investment_pct = d.new_social_pct;
            country.military.defense_budget_pct = d.new_defense_pct;

            // Si la nación IA gobierna en minoría y dispone de capital político, negocia coalición
            if country.legislative_support_pct < 50.0 && country.political_capital >= 22.0 {
                if let Some(partner_acronym) = country
                    .political_parties
                    .iter()
                    .filter(|p| {
                        !p.is_ruling
                            && !country.coalition_parties.iter().any(|ac| ac == &p.acronym)
                    })
                    .max_by(|a, b| {
                        a.seats_pct
                            .partial_cmp(&b.seats_pct)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|p| p.acronym.clone())
                {
                    country.coalition_parties.push(partner_acronym);
                    country.political_capital = (country.political_capital - 20.0).max(0.0);
                    country.legislative_support_pct = country.compute_legislative_support_pct();
                }
            }

            let cooldown_days = if at_war { 18 } else { 30 };
            let min_reserve = compute_ai_min_treasury_reserve(country);
            let cost_m = (d.recruit_troops as f64 / 1_000.0) * 70.0;
            if d.recruit_troops > 0
                && total_days.saturating_sub(country.military.last_ai_recruit_day) >= cooldown_days
                && country.military.ai_military_budget_m >= cost_m
                && country.economy.treasury_usd_millions >= cost_m + min_reserve
            {
                country.military.ai_military_budget_m =
                    (country.military.ai_military_budget_m - cost_m).max(0.0);
                country.military.last_ai_recruit_day = total_days;
                country.economy.treasury_usd_millions -= cost_m;
                country.military.active_troops += d.recruit_troops;
                country.economy.troop_upkeep_usd_millions = country.estimated_troop_upkeep_m();
                if let Some(prov) = d.recruit_province.clone() {
                    recruited_info = Some((prov, d.recruit_troops, country.name.clone()));
                }
            }
        }

        if let Some((prov_mesh, troops, cname)) = recruited_info {
            let own_provs_count = graph
                .values()
                .filter(|p| p.country_id == d.country_id)
                .count();
            let max_ai_divisions = own_provs_count.clamp(2, 6);
            let existing_divs = state
                .units
                .iter()
                .filter(|u| u.country_id == d.country_id)
                .count();

            // Evita spam de mini-unidades: refuerza divisiones existentes antes de crear una nueva
            let reinforce_idx = state
                .units
                .iter()
                .enumerate()
                .filter(|(_, u)| u.country_id == d.country_id)
                .filter(|(_, u)| existing_divs >= max_ai_divisions || u.troops < 18_000)
                .min_by_key(|(_, u)| {
                    let same_prov_penalty = if u.current_province == prov_mesh { 0 } else { 1 };
                    (same_prov_penalty, u.troops)
                })
                .map(|(i, _)| i);

            if let Some(idx) = reinforce_idx {
                let (unit_name, prov_id) = {
                    let u = &mut state.units[idx];
                    u.troops += troops;
                    u.morale = ((u.morale + 85.0) * 0.5).clamp(40.0, 95.0);
                    (u.name.clone(), u.current_province.clone())
                };
                let prov_label = graph
                    .get(&prov_id)
                    .map(|p| p.name.clone())
                    .unwrap_or(prov_id);
                state.push_chronicle(
                    d.country_id.clone(),
                    format!(
                        "{cname} refuerza {unit_name} con {troops} soldados en {prov_label} (cargo a presupuesto militar)."
                    ),
                );
            } else {
                let world_pos = graph
                    .get(&prov_mesh)
                    .map(|p| p.world_pos)
                    .unwrap_or([0.0, 0.5, 0.0]);
                let prov_label = graph
                    .get(&prov_mesh)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| prov_mesh.clone());

                let uid = state.next_unit_id;
                state.next_unit_id += 1;
                state.units.push(ArmyUnit {
                    id: uid,
                    name: format!("División {} #{uid}", d.country_id),
                    country_id: d.country_id.clone(),
                    troops,
                    current_province: prov_mesh,
                    target_province: None,
                    route_queue: Vec::new(),
                    march_progress: 0.0,
                    world_pos,
                    morale: 85.0,
                });
                state.push_chronicle(
                    d.country_id.clone(),
                    format!("{cname} recluta {troops} soldados en {prov_label}."),
                );
            }
        }

        if let Some((unit_id, dest_prov)) = d.move_unit {
            let route = if let Some(u) = state.units.iter().find(|u| u.id == unit_id) {
                find_province_route(graph, state, &d.country_id, &u.current_province, &dest_prov)
            } else {
                Vec::new()
            };
            if !route.is_empty() {
                if let Some(u) = state.units.iter_mut().find(|u| u.id == unit_id) {
                    let next_hop = route[0].clone();
                    u.target_province = Some(next_hop);
                    u.route_queue = route.into_iter().skip(1).collect();
                    u.march_progress = 0.0;
                }
            }
        }

        if let Some((target_id, intent)) = d.diplomacy_target {
            let key = GameState::ordered_pair(&d.country_id, &target_id);
            let rel = state.relations.entry(key).or_default();
            match intent {
                AiDiplomaticIntent::ProposeTradePact => {
                    rel.trade_pact = true;
                    rel.opinion = (rel.opinion + 10.0).min(100.0);
                }
                AiDiplomaticIntent::ImproveRelations => {
                    rel.opinion = (rel.opinion + 5.0).min(100.0);
                }
                AiDiplomaticIntent::ProposeAlliance => {
                    rel.alliance = true;
                    rel.opinion = (rel.opinion + 15.0).min(100.0);
                }
                AiDiplomaticIntent::DenounceRival => {
                    rel.opinion = (rel.opinion - 12.0).max(-100.0);
                }
            }
        }

        if let Some(msg) = d.log_message {
            state.push_chronicle(d.country_id, msg);
        }
    }

    state.sync_country_troop_totals();
}

/// Simula elecciones periódicas cuando `months_to_election` llega a 0.
pub fn process_elections(state: &mut GameState) {
    let ids: Vec<String> = state.countries.keys().cloned().collect();
    for id in ids {
        let mut election_msg: Option<String> = None;
        if let Some(country) = state.countries.get_mut(&id) {
            if country.months_to_election > 0 {
                country.months_to_election -= 1;
            }
            if country.months_to_election == 0 && !country.political_parties.is_empty() {
                country.months_to_election = 48;

                let total_approval: f64 = country
                    .political_parties
                    .iter()
                    .map(|p| p.approval_pct.max(5.0))
                    .sum();

                let mut best_idx = 0usize;
                let mut best_seats = -1.0_f64;

                for (idx, party) in country.political_parties.iter_mut().enumerate() {
                    party.seats_pct =
                        ((party.approval_pct.max(5.0) / total_approval) * 100.0 * 10.0).round()
                            / 10.0;
                    party.is_ruling = false;
                    if party.seats_pct > best_seats {
                        best_seats = party.seats_pct;
                        best_idx = idx;
                    }
                }

                country.political_parties[best_idx].is_ruling = true;
                country.coalition_parties.clear();
                country.legislative_support_pct = country.compute_legislative_support_pct();
                if country.political_parties[best_idx].leader.age > 74 {
                    let used: Vec<String> = country
                        .political_parties
                        .iter()
                        .map(|p| p.leader.full_name.clone())
                        .collect();
                    let new_leader = crate::engine::leaders::generate_successor_leader(
                        &id,
                        true,
                        state.campaign_seed.wrapping_add(state.total_days),
                        &used,
                    );
                    country.political_parties[best_idx].leader = new_leader;
                }
                let winner = &country.political_parties[best_idx];
                election_msg = Some(format!(
                    "Elecciones en {}: {} ({}) gana con {:.1}% del Congreso; preside {} ({} años, {}).",
                    country.name,
                    winner.name,
                    winner.acronym,
                    winner.seats_pct,
                    winner.leader.full_name,
                    winner.leader.age,
                    winner.leader.nationality
                ));
            }
        }
        if let Some(msg) = election_msg {
            state.push_chronicle(id, msg);
        }
    }
}
