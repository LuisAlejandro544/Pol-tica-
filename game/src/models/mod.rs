//! Propósito del archivo:
//! Define todas las estructuras de datos del juego en tiempo real (países, economía,
//! moneda, PIB, desempleo, empresas y precios de acciones, soldados/unidades por estado,
//! partidos políticos, diplomacia, pantallas y popups).
//! Se usan tipos de tamaño explícito (`u64`, `i64`, `f64`, `u32`) para garantizar
//! exactitud numérica idéntica en arquitecturas Android de 32 bits y 64 bits.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Pantallas principales del juego horizontal (arquitectura multipantalla modular).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenRoute {
    MainMenu,
    CountrySelect,
    CampaignLoading,
    WorldMap,
    Economy,
    Companies,
    Politics,
    Diplomacy,
    Military,
    Settings,
}

/// Modos de visualización cromática sobre el mapa 3D de Latinoamérica.
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

/// Perfil de líder político (Presidente o líder de partido de oposición).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaderProfile {
    pub full_name: String,
    pub age: u8,
    pub nationality: String,
}

impl Default for LeaderProfile {
    fn default() -> Self {
        Self {
            full_name: "Por designar".to_string(),
            age: 48,
            nationality: "Latinoamericana".to_string(),
        }
    }
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
    #[serde(default)]
    pub leader: LeaderProfile,
}

/// Estado macroeconómico, monetario, laboral y fiscal de una nación.
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
    pub troop_upkeep_usd_millions: f64,
    pub unemployment_pct: f64,
    pub corporate_gdp_bonus_m: f64,
}

/// Estado militar y defensa nacional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilitaryState {
    pub active_troops: u64,
    pub defense_budget_pct: f64,
    pub readiness: f64,
}

/// Unidad o regimiento desplegado en un estado/provincia y desplazable en tiempo real.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmyUnit {
    pub id: u64,
    pub name: String,
    pub country_id: String,
    pub troops: u64,
    pub current_province: String,
    pub target_province: Option<String>,
    pub route_queue: Vec<String>,
    pub march_progress: f32,
    pub world_pos: [f32; 3],
    pub morale: f64,
}

/// Categorías sectoriales para empresas fundadas dinámicamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanyCategory {
    Tecnologia,
    Energia,
    Agroindustria,
    Manufactura,
    Logistica,
    Finanzas,
    Salud,
}

impl CompanyCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tecnologia => "Tecnología",
            Self::Energia => "Energía",
            Self::Agroindustria => "Agroindustria",
            Self::Manufactura => "Manufactura",
            Self::Logistica => "Logística",
            Self::Finanzas => "Finanzas",
            Self::Salud => "Salud",
        }
    }

    pub fn from_lua_id(id: &str) -> Self {
        match id {
            "TECH" => Self::Tecnologia,
            "ENERGY" => Self::Energia,
            "AGRO" => Self::Agroindustria,
            "MANUF" => Self::Manufactura,
            "LOGISTICS" => Self::Logistica,
            "FINANCE" => Self::Finanzas,
            _ => Self::Salud,
        }
    }
}

/// Plantilla de sector corporativo cargada desde `scripts/companies_latam.lua`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyCategoryTemplate {
    pub id: String,
    pub category: CompanyCategory,
    pub prefixes: Vec<String>,
    pub suffixes: Vec<String>,
    pub products: Vec<String>,
    pub base_price_min: f64,
    pub base_price_max: f64,
    pub jobs_min: u64,
    pub jobs_max: u64,
    pub gdp_per_job_m: f64,
    pub inflation_sensitivity: f64,
    pub rate_sensitivity: f64,
}

/// Empresa privada fundada aleatoriamente en algún país latinoamericano.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyEntity {
    pub id: u64,
    pub name: String,
    pub country_id: String,
    pub category: CompanyCategory,
    pub sells: String,
    pub founded_label: String,
    pub employees: u64,
    pub share_price: f64,
    pub prev_share_price: f64,
    pub price_history: Vec<f64>,
    pub annual_gdp_contribution_m: f64,
    pub inflation_sensitivity: f64,
    pub rate_sensitivity: f64,
}

impl CompanyEntity {
    pub fn share_delta_pct(&self) -> f64 {
        if self.prev_share_price <= 0.01 {
            return 0.0;
        }
        ((self.share_price - self.prev_share_price) / self.prev_share_price) * 100.0
    }
}

/// Nodo topológico de estado/provincia con posición 3D y adyacencia para movimiento de tropas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvinceNode {
    pub mesh_name: String,
    pub country_id: String,
    pub name: String,
    pub world_pos: [f32; 3],
    pub terrain_type: String,
    pub elevation_avg: f64,
    pub neighbors: Vec<String>,
}

pub type ProvinceGraph = BTreeMap<String, ProvinceNode>;

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

    /// Devuelve el perfil del Presidente en ejercicio (líder del partido gobernante).
    pub fn president(&self) -> Option<&LeaderProfile> {
        self.ruling_party().map(|p| &p.leader)
    }

    /// Devuelve los líderes de los partidos de oposición ordenados tal como figuran en el Congreso.
    pub fn opposition_leaders(&self) -> Vec<(&PoliticalParty, &LeaderProfile)> {
        self.political_parties
            .iter()
            .filter(|p| !p.is_ruling)
            .map(|p| (p, &p.leader))
            .collect()
    }

    /// Costo mensual de mantenimiento por soldados activos en millones de USD.
    pub fn estimated_troop_upkeep_m(&self) -> f64 {
        (self.military.active_troops as f64 / 1_000.0) * 1.35
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

/// Metadatos de una provincia del modelo 3D (`provinces_latam.lua` + `latinoamerica_3d.glb`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvinceData {
    pub id: String,
    pub mesh_name: String,
    pub country_id: String,
    pub name: String,
    pub admin_type: String,
    pub terrain_type: String,
    pub elevation_avg: f64,
    pub area_weight: f64,
}

/// Marcador de capital nacional o provincial generado por Blender (`CAP_*` / `PCAP_*`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalMarkerData {
    pub node_name: String,
    pub country_id: String,
    pub name: String,
    pub is_national: bool,
    pub pop_max: u64,
}

/// Registro reciente de acciones tomadas por la IA local, empresas o el jugador.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleEntry {
    pub day: u64,
    pub date_label: String,
    pub country_id: String,
    pub summary: String,
}

/// Muestra histórica antigua de cotización bursátil movida al buffer de caché en disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedStockSample {
    pub company_id: u64,
    pub company_name: String,
    pub day: u64,
    pub share_price: f64,
}

/// Instantánea mensual macroeconómica archivada en el buffer temporal de caché.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroHistorySnapshot {
    pub day: u64,
    pub year: i32,
    pub month: u8,
    pub country_id: String,
    pub gdp_usd_millions: f64,
    pub inflation_pct: f64,
    pub stability: f64,
}

/// Miniatura ligera de campaña guardada en caché para vistas previas y resumen rápido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignThumbnail {
    pub campaign_seed: u64,
    pub country_id: String,
    pub country_name: String,
    pub president_name: String,
    pub date_label: String,
    pub gdp_usd_millions: f64,
    pub active_troops: u64,
    pub companies_count: usize,
}

/// Acciones emitidas desde la interfaz que deben preservarse aunque el hilo de simulación esté ocupado.
#[derive(Debug, Clone)]
pub enum PlayerAction {
    RecruitSoldiers {
        country_id: String,
        province_mesh: String,
        troops: u64,
        cost_m: f64,
    },
    OrderUnitMove {
        unit_id: u64,
        target_province: String,
    },
}

/// Estado global completo de la partida en memoria (simulación continua en tiempo real).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub campaign_seed: u64,
    pub total_days: u64,
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub paused: bool,
    pub speed_level: u8, // 1 = normal, 2 = rápido, 3 = muy rápido
    pub difficulty: Difficulty,
    pub player_country_id: Option<String>,
    pub selected_country_id: Option<String>,
    pub selected_province_mesh: Option<String>,
    pub selected_unit_id: Option<u64>,
    pub countries: BTreeMap<String, CountryState>,
    pub relations: BTreeMap<(String, String), DiplomaticRelation>,
    pub units: Vec<ArmyUnit>,
    pub next_unit_id: u64,
    pub companies: Vec<CompanyEntity>,
    pub next_company_id: u64,
    pub active_event: Option<GameEvent>,
    pub chronicle: Vec<ChronicleEntry>,
    #[serde(skip)]
    pub pending_chronicle_archive: Vec<ChronicleEntry>,
    #[serde(skip)]
    pub pending_stock_archive: Vec<ArchivedStockSample>,
    #[serde(skip)]
    pub pending_macro_archive: Vec<MacroHistorySnapshot>,
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
        format!("{:02} {month_name} {}", self.day, self.year)
    }

    pub fn push_chronicle(&mut self, country_id: impl Into<String>, summary: impl Into<String>) {
        let entry = ChronicleEntry {
            day: self.total_days,
            date_label: self.date_label(),
            country_id: country_id.into(),
            summary: summary.into(),
        };
        self.chronicle.insert(0, entry);
        while self.chronicle.len() > 70 {
            if let Some(evicted) = self.chronicle.pop() {
                self.pending_chronicle_archive.push(evicted);
            }
        }
    }

    pub fn country_company_jobs(&self, country_id: &str) -> u64 {
        self.companies
            .iter()
            .filter(|c| c.country_id == country_id)
            .map(|c| c.employees)
            .sum()
    }

    pub fn country_company_gdp_bonus(&self, country_id: &str) -> f64 {
        self.companies
            .iter()
            .filter(|c| c.country_id == country_id)
            .map(|c| c.annual_gdp_contribution_m)
            .sum()
    }

    pub fn sync_country_troop_totals(&mut self) {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        for unit in &self.units {
            *counts.entry(unit.country_id.clone()).or_insert(0) += unit.troops;
        }
        for (cid, country) in &mut self.countries {
            if let Some(&sum) = counts.get(cid) {
                country.military.active_troops = sum;
            }
            country.economy.troop_upkeep_usd_millions = country.estimated_troop_upkeep_m();
        }
    }
}
