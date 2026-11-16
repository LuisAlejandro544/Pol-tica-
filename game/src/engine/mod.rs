//! Propósito del archivo:
//! Orquestador de simulación en segundo plano (`SimulationWorker`).
//! Utiliza un hilo dedicado con `crossbeam-channel` para ejecutar los turnos mensuales
//! (fórmulas Lua 5.4 + IA local paralela en Rust con `rayon`), evitando cualquier bloqueo
//! en el hilo principal de interfaz en dispositivos móviles.

use crate::ai::{apply_ai_decisions, evaluate_ai_countries, process_elections};
use crate::lua_bridge::LuaEngine;
use crate::models::{Difficulty, GameState};
use crossbeam_channel::{Receiver, Sender};
use std::collections::BTreeMap;
use std::thread;

/// Comandos que la interfaz envía de forma asíncrona al trabajador de simulación.
pub enum SimCommand {
    AdvanceTurn(Box<GameState>),
    NewCampaign {
        difficulty: Difficulty,
    },
}

/// Respuestas enviadas de regreso al hilo de interfaz cuando concluye el cálculo.
pub enum SimResponse {
    TurnCompleted(Box<GameState>),
    CampaignReady(Box<GameState>),
    Error(String),
}

pub struct SimulationWorker {
    pub tx: Sender<SimCommand>,
    pub rx: Receiver<SimResponse>,
    pub is_busy: bool,
}

impl SimulationWorker {
    /// Arranca el hilo secundario dueño de la máquina virtual Lua 5.4.
    pub fn spawn() -> Self {
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

                while let Ok(cmd) = cmd_rx.recv() {
                    match cmd {
                        SimCommand::NewCampaign { difficulty } => {
                            match build_initial_state(&lua_engine, difficulty) {
                                Ok(state) => {
                                    let _ = res_tx.send(SimResponse::CampaignReady(Box::new(state)));
                                }
                                Err(e) => {
                                    let _ = res_tx.send(SimResponse::Error(e));
                                }
                            }
                        }
                        SimCommand::AdvanceTurn(mut state) => {
                            match run_single_turn(&lua_engine, &mut state) {
                                Ok(()) => {
                                    let _ = res_tx.send(SimResponse::TurnCompleted(state));
                                }
                                Err(e) => {
                                    let _ = res_tx.send(SimResponse::Error(e));
                                }
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

    pub fn request_new_campaign(&mut self, difficulty: Difficulty) {
        self.is_busy = true;
        let _ = self.tx.send(SimCommand::NewCampaign { difficulty });
    }

    pub fn request_advance_turn(&mut self, state: &GameState) {
        if self.is_busy {
            return;
        }
        self.is_busy = true;
        let _ = self.tx.send(SimCommand::AdvanceTurn(Box::new(state.clone())));
    }
}

fn build_initial_state(lua: &LuaEngine, difficulty: Difficulty) -> Result<GameState, String> {
    let countries = lua
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

    let mut state = GameState {
        turn: 1,
        year: 2026,
        month: 1,
        paused: true,
        speed_level: 1,
        difficulty,
        player_country_id: None,
        selected_country_id: Some("MX".to_string()),
        countries,
        relations,
        active_event: None,
        chronicle: Vec::new(),
    };
    state.push_chronicle("LATAM", "Campaña geopolítica iniciada. Elige una nación para gobernar.");
    Ok(state)
}

fn run_single_turn(lua: &LuaEngine, state: &mut GameState) -> Result<(), String> {
    // 1. Avanzar calendario mensual
    state.turn += 1;
    if state.month >= 12 {
        state.month = 1;
        state.year += 1;
    } else {
        state.month += 1;
    }

    // 2. La IA local en Rust evalúa en paralelo todas las naciones no jugadoras
    let ai_decisions = evaluate_ai_countries(state);
    apply_ai_decisions(state, ai_decisions);

    // 3. Ejecutar reglas económicas y políticas de Lua 5.4 para cada país
    let ids: Vec<String> = state.countries.keys().cloned().collect();
    for id in ids {
        if let Some(country) = state.countries.get_mut(&id) {
            lua.step_country_via_lua(country)
                .map_err(|e| format!("Error ejecutando turno Lua para {id}: {e}"))?;
        }
    }

    // 4. Procesar ciclos electorales en el Congreso
    process_elections(state);

    // 5. Cada 4 turnos, disparar un evento estratégico desde Lua 5.4 para el jugador
    if state.active_event.is_none() && state.player_country_id.is_some() && state.turn % 4 == 0 {
        let ev = lua
            .pick_event(state.turn, state.countries.len() as u64)
            .map_err(|e| format!("Error generando evento Lua: {e}"))?;
        state.active_event = Some(ev);
    }

    Ok(())
}
