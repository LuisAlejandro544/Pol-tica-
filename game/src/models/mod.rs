//! Propósito del archivo:
//! Define todas las estructuras de datos del juego (países, economía, moneda,
//! PIB, partidos políticos, diplomacia, fuerzas armadas, pantallas y popups).
//! Se usan tipos de tamaño explícito (`u64`, `i64`, `f64`) para garantizar
//! exactitud numérica idéntica en arquitecturas Android de 32 bits y 64 bits.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Pantallas principales del juego horizontal (arquitectura multipantalla modular).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenRoute {
    MainMenu,
    CountrySelect,
    WorldMap,
    Economy,
    Politics,
    Diplomacy,
    Military,
    Settings,
}

/// Modos de visualización cromática sobre el mapa SVG de Latinoamérica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapMode {
    Political,
    GdpPerCapita,
    Inflation,
    Diplomacy,
}

/// Dificultad de la IA local en Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Normal,
    Desafiante,
    Realista,
}

/// Partido político dentro del congreso o asamblea de un país.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalParty {
    pub name: String,
    pub acronym: String,
    pub ideology: String,
    pub seats_pct: f64,
    pub approval_pct: f64,
    pub is_ruling: bool,
}

/// Estado macroeconómico y monetario de una nación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub currency_code: String,
    pub currency_name: String,
    pub exchange_rate_usd: f64,
    pub inflation_pct: f64,
    pub gdp_usd_millions: f64,
    pub gdp_growth_pct: f64,
    pub debt_to_gdp_pct: f64,
    pub treasury_usd_millions: f64,
    pub tax_rate_pct: f64,
    pub interest_rate_pct: f64,
    pub infra_investment_pct: f64,
    pub social_investment_pct: f64,
    pub last_monthly_balance: f64,
}

/// Estado militar y defensa nacional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryState {
    pub active_troops: u64,
    pub defense_budget_pct: f64,
    pub readiness: f64,
}

/// Perfil estratégico de la IA local para naciones no controladas por el jugador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiPersonality {
    Economista,
    Nacionalista,
    Reformista,
    Diplomatico,
}

/// Entidad completa de un país latinoamericano.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryState {
    pub id: String,
    pub name: String,
    pub capital: String,
    pub region: String,
    pub color: [u8; 3],
    pub population: u64,
    pub pop_growth_pct: f64,
    pub urban_pct: f64,
    pub stability: f64,
    pub economy: EconomyState,
    pub military: MilitaryState,
    pub political_parties: Vec<PoliticalParty>,
    pub neighbors: Vec<String>,
    pub ai_personality: AiPersonality,
    pub months_to_election: u32,
}

impl CountryState {
    /// PIB per cápita anual estimado en dólares estadounidenses.
    pub fn gdp_per_capita_usd(&self) -> f64 {
        if self.population == 0 {
            return 0.0;
        }
        (self.economy.gdp_usd_millions * 1_000_000.0) / (self.population as f64)
    }

    /// Devuelve el partido gobernante actual, si existe.
    pub fn ruling_party(&self) -> Option<&PoliticalParty> {
        self.political_parties.iter().find(|p| p.is_ruling)
    }

    /// Índice compuesto de poder militar para comparativas diplomáticas e IA.
    pub fn military_power_score(&self) -> f64 {
        let troops_k = (self.military.active_troops as f64) / 1_000.0;
        let gdp_factor = (self.economy.gdp_usd_millions / 100_000.0).sqrt().max(0.4);
        troops_k * (0.5 + self.military.readiness / 100.0) * gdp_factor
    }
}

/// Estado diplomático bilateral entre dos países (clave ordenada `(min_id, max_id)`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiplomaticRelation {
    pub opinion: f64, // -100.0 a +100.0
    pub trade_pact: bool,
    pub alliance: bool,
    pub at_war: bool,
}

impl Default for DiplomaticRelation {
    fn default() -> Self {
        Self {
            opinion: 15.0,
            trade_pact: false,
            alliance: false,
            at_war: false,
        }
    }
}

/// Opción interactiva dentro de un popup de evento geopolítico.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOption {
    pub label: String,
    pub treasury_delta_pct: f64,
    pub gdp_growth_delta: f64,
    pub inflation_delta: f64,
    pub stability_delta: f64,
}

/// Evento dinámico generado desde Lua 5.4 para mostrarse en un popup modal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub options: Vec<EventOption>,
}

/// Registro reciente de acciones tomadas por la IA local o el jugador.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleEntry {
    pub turn: u64,
    pub date_label: String,
    pub country_id: String,
    pub summary: String,
}

/// Estado global completo de la partida en memoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub turn: u64,
    pub year: i32,
    pub month: u8,
    pub paused: bool,
    pub speed_level: u8, // 1 = normal, 2 = rápido, 3 = muy rápido
    pub difficulty: Difficulty,
    pub player_country_id: Option<String>,
    pub selected_country_id: Option<String>,
    pub countries: BTreeMap<String, CountryState>,
    pub relations: BTreeMap<(String, String), DiplomaticRelation>,
    pub active_event: Option<GameEvent>,
    pub chronicle: Vec<ChronicleEntry>,
}

impl GameState {
    pub fn ordered_pair(a: &str, b: &str) -> (String, String) {
        if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }

    pub fn get_relation(&self, a: &str, b: &str) -> DiplomaticRelation {
        if a == b {
            return DiplomaticRelation {
                opinion: 100.0,
                trade_pact: true,
                alliance: true,
                at_war: false,
            };
        }
        self.relations
            .get(&Self::ordered_pair(a, b))
            .cloned()
            .unwrap_or_default()
    }

    pub fn date_label(&self) -> String {
        let month_name = match self.month {
            1 => "Ene",
            2 => "Feb",
            3 => "Mar",
            4 => "Abr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Ago",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            _ => "Dic",
        };
        format!("{month_name} {}", self.year)
    }

    pub fn push_chronicle(&mut self, country_id: impl Into<String>, summary: impl Into<String>) {
        let entry = ChronicleEntry {
            turn: self.turn,
            date_label: self.date_label(),
            country_id: country_id.into(),
            summary: summary.into(),
        };
        self.chronicle.insert(0, entry);
        if self.chronicle.len() > 60 {
            self.chronicle.truncate(60);
        }
    }
}
