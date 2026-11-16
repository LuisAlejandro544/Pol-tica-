//! Propósito del archivo:
//! Motor de Inteligencia Artificial local 100% escrito en Rust para todas las naciones
//! rivales de Latinoamérica. Evalúa indicadores macroeconómicos, riesgo de inflación,
//! presión parlamentaria, personalidad geopolítica y relaciones con países vecinos
//! usando paralelismo con `rayon` sobre hilos de fondo (sin bloquear la UI).

use crate::models::{AiPersonality, CountryState, Difficulty, GameState};
use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

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
pub fn evaluate_ai_countries(state: &GameState) -> Vec<AiDecision> {
    let player_id = state.player_country_id.as_deref();
    let difficulty_mult = match state.difficulty {
        Difficulty::Normal => 1.0,
        Difficulty::Desafiante => 1.15,
        Difficulty::Realista => 1.3,
    };

    // Recolectamos IDs de países IA para procesarlos en paralelo sin mutar el mapa compartido
    let ai_ids: Vec<&String> = state
        .countries
        .keys()
        .filter(|id| Some(id.as_str()) != player_id)
        .collect();

    ai_ids
        .par_iter()
        .filter_map(|&country_id| {
            let country = state.countries.get(country_id)?;
            Some(decide_for_country(country, state, difficulty_mult))
        })
        .collect()
}

fn decide_for_country(
    country: &CountryState,
    state: &GameState,
    difficulty_mult: f64,
) -> AiDecision {
    // Semilla determinista por turno + país para resultados reproducibles
    let mut seed_val: u64 = state.turn.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in country.id.bytes() {
        seed_val = seed_val.rotate_left(7) ^ (b as u64);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed_val);

    let mut tax = country.economy.tax_rate_pct;
    let mut interest = country.economy.interest_rate_pct;
    let mut infra = country.economy.infra_investment_pct;
    let mut social = country.economy.social_investment_pct;
    let mut defense = country.economy.defense_budget_pct;
    let mut recruit_troops = 0_u64;
    let mut log_message = None;

    // 1. Política monetaria: control de inflación y estabilidad cambiaria
    if country.economy.inflation_pct > 10.0 {
        interest = (interest + 0.75 * difficulty_mult).min(45.0);
    } else if country.economy.inflation_pct < 3.0 && country.economy.gdp_growth_pct < 2.2 {
        interest = (interest - 0.35).max(2.0);
    }

    // 2. Política fiscal y gasto según personalidad de la IA
    match country.ai_personality {
        AiPersonality::Economista => {
            if country.economy.debt_to_gdp_pct > 75.0 {
                tax = (tax + 0.5).min(32.0);
                infra = (infra + 0.25).min(6.0);
            } else {
                infra = (infra + 0.4).min(7.5);
            }
        }
        AiPersonality::Nacionalista => {
            defense = (defense + 0.25 * difficulty_mult).min(4.5);
            if country.economy.treasury_usd_millions > 1_800.0 && rng.random_bool(0.25) {
                recruit_troops = ((country.population / 20_000).clamp(500, 4_000)) as u64;
            }
        }
        AiPersonality::Reformista => {
            if country.stability < 62.0 {
                social = (social + 0.45).min(8.5);
            }
            if country.economy.gdp_growth_pct < 2.5 {
                tax = (tax - 0.3).max(12.0);
            }
        }
        AiPersonality::Diplomatico => {
            social = (social + 0.2).min(7.0);
            infra = (infra + 0.25).min(6.5);
        }
    }

    // 3. Evaluación diplomática hacia países vecinos
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
            } else if !rel.alliance && rel.opinion > 65.0 && country.ai_personality == AiPersonality::Diplomatico {
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
        diplomacy_target,
        log_message,
    }
}

/// Aplica las decisiones calculadas en paralelo al `GameState` y procesa elecciones democráticas.
pub fn apply_ai_decisions(state: &mut GameState, decisions: Vec<AiDecision>) {
    for d in decisions {
        let Some(country) = state.countries.get_mut(&d.country_id) else {
            continue;
        };
        country.economy.tax_rate_pct = d.new_tax_rate;
        country.economy.interest_rate_pct = d.new_interest_rate;
        country.economy.infra_investment_pct = d.new_infra_pct;
        country.economy.social_investment_pct = d.new_social_pct;
        country.economy.defense_budget_pct = d.new_defense_pct;

        if d.recruit_troops > 0 && country.economy.treasury_usd_millions > 300.0 {
            country.military.active_troops += d.recruit_troops;
            country.economy.treasury_usd_millions -= 120.0;
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
                country.months_to_election = 48; // Ciclo electoral de 4 años

                // Reasignar escaños proporcionalmente a la popularidad actual
                let total_approval: f64 = country
                    .political_parties
                    .iter()
                    .map(|p| p.approval_pct.max(5.0))
                    .sum();

                let mut best_idx = 0usize;
                let mut best_seats = -1.0_f64;

                for (idx, party) in country.political_parties.iter_mut().enumerate() {
                    party.seats_pct = ((party.approval_pct.max(5.0) / total_approval) * 100.0 * 10.0).round() / 10.0;
                    party.is_ruling = false;
                    if party.seats_pct > best_seats {
                        best_seats = party.seats_pct;
                        best_idx = idx;
                    }
                }

                country.political_parties[best_idx].is_ruling = true;
                let winner = &country.political_parties[best_idx];
                election_msg = Some(format!(
                    "Elecciones en {}: {} ({}) gana con {:.1}% del Congreso.",
                    country.name, winner.name, winner.acronym, winner.seats_pct
                ));
            }
        }
        if let Some(msg) = election_msg {
            state.push_chronicle(id, msg);
        }
    }
}
