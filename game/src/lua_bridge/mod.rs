//! Propósito del archivo:
//! Puente seguro entre Rust y Lua 5.4 (`mlua` con Lua 5.4 compilado estáticamente).
//! Carga los países iniciales desde `scripts/countries_latam.lua`, las provincias y
//! capitales desde `scripts/provinces_latam.lua`, ejecuta las fórmulas macroeconómicas
//! y parlamentarias de `scripts/game_rules.lua` (Taylor, Curva de Phillips, Congreso)
//! y evalúa eventos contextuales por causas reales en `scripts/events_latam.lua`.

use crate::map::svg_loader::ASSETS_DIR;
use crate::models::{
    AiPersonality, CapitalMarkerData, CompanyCategory, CompanyCategoryTemplate, CountryState,
    EconomyState, EventOption, GameEvent, GlobalMacroCycle, LeaderProfile, MilitaryState,
    PoliticalParty, ProvinceData,
};
use mlua::{ChunkMode, Function, Lua, Table, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LuaBridgeError {
    #[error("Error de máquina virtual Lua 5.4: {0}")]
    Lua(#[from] mlua::Error),
    #[error("No se encontró el script incrustado: {0}")]
    MissingScript(String),
}

pub struct LuaStepContext<'a> {
    pub corporate_gdp_bonus_m: f64,
    pub corporate_export_bonus_m: f64,
    pub corporate_jobs_total: u64,
    pub trade_pacts_count: u32,
    pub at_war: bool,
    pub global_cycle: &'a GlobalMacroCycle,
}

pub struct LuaEventContext<'a> {
    pub country: &'a CountryState,
    pub company_count: usize,
    pub dominant_sector: &'a str,
    pub at_war: bool,
    pub global_cycle: &'a GlobalMacroCycle,
    pub last_event_id: Option<&'a str>,
    pub total_days: u64,
}

pub struct LuaEngine {
    lua: Lua,
    rules_bytecode: OnceLock<Vec<u8>>,
    events_bytecode: OnceLock<Vec<u8>>,
}

impl LuaEngine {
    /// Inicializa Lua 5.4 en modo seguro sin acceso peligroso al sistema de archivos externo.
    pub fn new() -> Result<Self, LuaBridgeError> {
        let lua = Lua::new();
        Ok(Self {
            lua,
            rules_bytecode: OnceLock::new(),
            events_bytecode: OnceLock::new(),
        })
    }

    fn read_script(path: &str) -> Result<&'static str, LuaBridgeError> {
        ASSETS_DIR
            .get_file(path)
            .and_then(|f| f.contents_utf8())
            .ok_or_else(|| LuaBridgeError::MissingScript(path.to_string()))
    }

    fn read_optional_script_owned(asset_path: &str, disk_fallbacks: &[&str]) -> Option<String> {
        if let Some(text) = ASSETS_DIR.get_file(asset_path).and_then(|f| f.contents_utf8()) {
            return Some(text.to_string());
        }
        #[cfg(not(target_os = "android"))]
        for path in disk_fallbacks {
            if let Ok(text) = std::fs::read_to_string(path) {
                return Some(text);
            }
        }
        let _ = disk_fallbacks;
        None
    }

    /// Lee los bytes crudos de `scripts/provinces_latam.lua` para calcular la huella de caché del mapa.
    pub fn read_provinces_lua_bytes() -> Vec<u8> {
        Self::read_optional_script_owned(
            "scripts/provinces_latam.lua",
            &[
                "scripts/provinces_latam.lua",
                "../scripts/provinces_latam.lua",
                "game/assets/scripts/provinces_latam.lua",
            ],
        )
        .map(String::into_bytes)
        .unwrap_or_default()
    }

    fn load_or_compile_bytecode(&self, cache_name: &str, src: &str) -> Result<Vec<u8>, LuaBridgeError> {
        let fingerprint = crate::cache::compute_lua_fingerprint(cache_name, src);
        if let Some(cached) = crate::cache::load_lua_bytecode(cache_name, fingerprint) {
            if self
                .lua
                .load(&cached)
                .set_mode(ChunkMode::Binary)
                .into_function()
                .is_ok()
            {
                return Ok(cached);
            }
        }

        let func = self.lua.load(src).into_function()?;
        let bytecode = func.dump(true);
        crate::cache::save_lua_bytecode(cache_name, fingerprint, &bytecode);
        Ok(bytecode)
    }

    fn eval_cached_table(&self, cache_name: &str, src: &str) -> Result<Table, LuaBridgeError> {
        let bytecode = self.load_or_compile_bytecode(cache_name, src)?;
        match self
            .lua
            .load(&bytecode)
            .set_mode(ChunkMode::Binary)
            .eval::<Table>()
        {
            Ok(tbl) => Ok(tbl),
            Err(_) => {
                // Respaldo: recompila si el binario en disco quedara corrupto
                let func = self.lua.load(src).into_function()?;
                let fresh = func.dump(true);
                let fp = crate::cache::compute_lua_fingerprint(cache_name, src);
                crate::cache::save_lua_bytecode(cache_name, fp, &fresh);
                Ok(func.call(())?)
            }
        }
    }

    /// Carga los 21 países latinoamericanos definidos en `scripts/countries_latam.lua` usando caché de bytecode.
    pub fn load_initial_countries(&self) -> Result<BTreeMap<String, CountryState>, LuaBridgeError> {
        let src = Self::read_script("scripts/countries_latam.lua")?;
        let table = self.eval_cached_table("countries_latam", src)?;
        let mut countries = BTreeMap::new();

        for (idx, pair) in table.sequence_values::<Table>().enumerate() {
            let row = pair?;
            let id: String = row.get("id")?;
            let color_tbl: Table = row.get("color")?;
            let color = [
                color_tbl.get::<u8>(1)?,
                color_tbl.get::<u8>(2)?,
                color_tbl.get::<u8>(3)?,
            ];

            let mut parties = Vec::new();
            let parties_tbl: Table = row.get("political_parties")?;
            for p_item in parties_tbl.sequence_values::<Table>() {
                let pt = p_item?;
                parties.push(PoliticalParty {
                    name: pt.get("name")?,
                    acronym: pt.get("acronym")?,
                    ideology: pt.get("ideology")?,
                    seats_pct: pt.get("seats_pct")?,
                    approval_pct: pt.get("approval_pct")?,
                    is_ruling: pt.get("is_ruling")?,
                    leader: LeaderProfile::default(),
                });
            }

            let mut neighbors = Vec::new();
            let neighbors_tbl: Table = row.get("neighbors")?;
            for n_item in neighbors_tbl.sequence_values::<String>() {
                neighbors.push(n_item?);
            }

            let ai_personality = match idx % 4 {
                0 => AiPersonality::Economista,
                1 => AiPersonality::Nacionalista,
                2 => AiPersonality::Reformista,
                _ => AiPersonality::Diplomatico,
            };

            let active_troops: u64 = row.get("active_troops")?;
            let gdp_usd_millions: f64 = row.get("gdp_usd_millions")?;
            let defense_budget_pct: f64 = row.get("defense_budget_pct")?;
            let inflation_pct: f64 = row.get("inflation_pct")?;
            let gdp_growth_pct: f64 = row.get("gdp_growth_pct")?;
            let debt_to_gdp_pct: f64 = row.get("debt_to_gdp_pct")?;
            let stability: f64 = row.get("stability")?;
            let initial_unemp =
                (6.4 + inflation_pct * 0.12 - gdp_growth_pct * 0.35).clamp(3.8, 14.5);
            let initial_upkeep = (active_troops as f64 / 1_000.0) * 1.35;
            let initial_risk = (130.0
                + (debt_to_gdp_pct - 50.0).max(0.0) * 9.5
                + (62.0 - stability).max(0.0) * 8.0)
                .clamp(95.0, 1800.0);
            let ruling_seats = parties
                .iter()
                .find(|p| p.is_ruling)
                .map(|p| p.seats_pct)
                .unwrap_or(45.0);

            let mut state = CountryState {
                id: id.clone(),
                name: row.get("name")?,
                capital: row.get("capital")?,
                region: row.get("region")?,
                color,
                population: row.get("population")?,
                pop_growth_pct: row.get("pop_growth_pct")?,
                urban_pct: row.get("urban_pct")?,
                stability,
                economy: EconomyState {
                    currency_code: row.get("currency_code")?,
                    currency_name: row.get("currency_name")?,
                    exchange_rate_usd: row.get("exchange_rate_usd")?,
                    inflation_pct,
                    gdp_usd_millions,
                    gdp_growth_pct,
                    debt_to_gdp_pct,
                    treasury_usd_millions: row.get("treasury_usd_millions")?,
                    tax_rate_pct: row.get("tax_rate_pct")?,
                    interest_rate_pct: row.get("interest_rate_pct")?,
                    infra_investment_pct: 3.5,
                    social_investment_pct: 4.5,
                    last_monthly_balance: 0.0,
                    troop_upkeep_usd_millions: initial_upkeep,
                    unemployment_pct: initial_unemp,
                    corporate_gdp_bonus_m: 0.0,
                    potential_gdp_growth_pct: (gdp_growth_pct + 0.35).clamp(2.2, 5.2),
                    inflation_target_pct: if inflation_pct >= 12.0 { 6.0 } else { 3.0 },
                    trade_balance_usd_millions: 0.0,
                    country_risk_bps: initial_risk,
                    corporate_tax_revenue_m: 0.0,
                    monthly_tax_revenue_m: 0.0,
                    monthly_defense_cost_m: 0.0,
                    monthly_infra_cost_m: 0.0,
                    monthly_social_cost_m: 0.0,
                    monthly_debt_service_m: 0.0,
                },
                military: MilitaryState {
                    active_troops,
                    defense_budget_pct,
                    readiness: 72.0,
                    ai_military_budget_m: (((gdp_usd_millions / 12.0)
                        * (defense_budget_pct / 100.0))
                        * 0.65)
                        .clamp(40.0, 420.0),
                    last_ai_recruit_day: 0,
                },
                political_parties: parties,
                neighbors,
                ai_personality,
                months_to_election: 24 + ((idx as u32 * 7) % 24),
                coalition_parties: Vec::new(),
                legislative_support_pct: ruling_seats,
                political_capital: 48.0,
                foreign_minister: None,
                minister_candidates: Vec::new(),
            };
            state.recompute_fiscal_snapshot();
            countries.insert(id, state);
        }

        Ok(countries)
    }

    /// Carga las categorías y productos empresariales desde `scripts/companies_latam.lua`.
    pub fn load_company_catalog(&self) -> Result<Vec<CompanyCategoryTemplate>, LuaBridgeError> {
        let src = Self::read_script("scripts/companies_latam.lua")?;
        let root: Table = self.lua.load(src).eval()?;
        let cats_tbl: Table = root.get("categories")?;
        let mut templates = Vec::new();

        for item in cats_tbl.sequence_values::<Table>() {
            let ct = item?;
            let id: String = ct.get("id")?;
            let mut prefixes = Vec::new();
            for p in ct.get::<Table>("prefixes")?.sequence_values::<String>() {
                prefixes.push(p?);
            }
            let mut suffixes = Vec::new();
            for s in ct.get::<Table>("suffixes")?.sequence_values::<String>() {
                suffixes.push(s?);
            }
            let mut products = Vec::new();
            for pr in ct.get::<Table>("products")?.sequence_values::<String>() {
                products.push(pr?);
            }

            templates.push(CompanyCategoryTemplate {
                category: CompanyCategory::from_lua_id(&id),
                id,
                prefixes,
                suffixes,
                products,
                base_price_min: ct.get("base_price_min")?,
                base_price_max: ct.get("base_price_max")?,
                jobs_min: ct.get("jobs_min")?,
                jobs_max: ct.get("jobs_max")?,
                gdp_per_job_m: ct.get("gdp_per_job_m")?,
                inflation_sensitivity: ct.get("inflation_sensitivity")?,
                rate_sensitivity: ct.get("rate_sensitivity")?,
            });
        }

        Ok(templates)
    }

    /// Carga el catálogo de provincias y capitales generado por Blender desde `scripts/provinces_latam.lua`.
    pub fn load_provinces_and_capitals(
        &self,
    ) -> Result<(BTreeMap<String, ProvinceData>, Vec<CapitalMarkerData>), LuaBridgeError> {
        let Some(src) = Self::read_optional_script_owned(
            "scripts/provinces_latam.lua",
            &[
                "scripts/provinces_latam.lua",
                "../scripts/provinces_latam.lua",
                "game/assets/scripts/provinces_latam.lua",
            ],
        ) else {
            return Ok((BTreeMap::new(), Vec::new()));
        };

        let root = self.eval_cached_table("provinces_latam", &src)?;
        let mut provinces = BTreeMap::new();
        if let Ok(prov_tbl) = root.get::<Table>("provinces") {
            for item in prov_tbl.sequence_values::<Table>() {
                let pt = item?;
                let mesh_name: String = pt.get("mesh_name")?;
                provinces.insert(
                    mesh_name.clone(),
                    ProvinceData {
                        id: pt.get("id")?,
                        mesh_name,
                        country_id: pt.get("country_id")?,
                        name: pt.get("name")?,
                        admin_type: pt.get("admin_type")?,
                        terrain_type: pt.get("terrain_type")?,
                        elevation_avg: pt.get("elevation_avg")?,
                        area_weight: pt.get("area_weight")?,
                    },
                );
            }
        }

        let mut capitals = Vec::new();
        if let Ok(cap_tbl) = root.get::<Table>("capitals") {
            for item in cap_tbl.sequence_values::<Table>() {
                let ct = item?;
                capitals.push(CapitalMarkerData {
                    node_name: ct.get("node_name")?,
                    country_id: ct.get("country_id")?,
                    name: ct.get("name")?,
                    is_national: ct.get("is_national")?,
                    pop_max: ct.get("pop_max")?,
                });
            }
        }

        Ok((provinces, capitals))
    }

    /// Ejecuta un paso económico, cambiario y parlamentario mensual usando `scripts/game_rules.lua`.
    pub fn step_country_via_lua(
        &self,
        country: &mut CountryState,
        ctx: &LuaStepContext<'_>,
    ) -> Result<(), LuaBridgeError> {
        let rules_bytecode = if let Some(bc) = self.rules_bytecode.get() {
            bc
        } else {
            let rules_src = Self::read_script("scripts/game_rules.lua")?;
            let compiled = self.load_or_compile_bytecode("game_rules", rules_src)?;
            let _ = self.rules_bytecode.set(compiled);
            self.rules_bytecode
                .get()
                .expect("rules_bytecode inicializado")
        };
        let rules_tbl: Table = self
            .lua
            .load(rules_bytecode.as_slice())
            .set_mode(ChunkMode::Binary)
            .eval()?;

        // 1. Cálculo macroeconómico realista en Lua 5.4
        let econ_fn: Function = rules_tbl.get("calculate_monthly_economy")?;
        let econ_in = self.lua.create_table()?;
        econ_in.set("currency_code", country.economy.currency_code.as_str())?;
        econ_in.set("exchange_rate_usd", country.economy.exchange_rate_usd)?;
        econ_in.set("inflation_pct", country.economy.inflation_pct)?;
        econ_in.set("inflation_target_pct", country.economy.inflation_target_pct)?;
        econ_in.set("gdp_usd_millions", country.economy.gdp_usd_millions)?;
        econ_in.set("gdp_growth_pct", country.economy.gdp_growth_pct)?;
        econ_in.set("debt_to_gdp_pct", country.economy.debt_to_gdp_pct)?;
        econ_in.set("treasury_usd_millions", country.economy.treasury_usd_millions)?;
        econ_in.set("tax_rate_pct", country.economy.tax_rate_pct)?;
        econ_in.set("interest_rate_pct", country.economy.interest_rate_pct)?;
        econ_in.set("defense_budget_pct", country.military.defense_budget_pct)?;
        econ_in.set("infra_investment_pct", country.economy.infra_investment_pct)?;
        econ_in.set("social_investment_pct", country.economy.social_investment_pct)?;
        econ_in.set("stability", country.stability)?;
        econ_in.set("active_troops", country.military.active_troops)?;
        econ_in.set("unemployment_pct", country.economy.unemployment_pct)?;
        econ_in.set("corporate_gdp_bonus_m", ctx.corporate_gdp_bonus_m)?;
        econ_in.set("corporate_export_bonus_m", ctx.corporate_export_bonus_m)?;
        econ_in.set("corporate_jobs_total", ctx.corporate_jobs_total)?;
        econ_in.set("population", country.population)?;
        econ_in.set("global_fed_rate_pct", ctx.global_cycle.fed_rate_pct)?;
        econ_in.set("commodity_index", ctx.global_cycle.commodity_index)?;
        econ_in.set("trade_pacts_count", ctx.trade_pacts_count)?;
        econ_in.set("at_war", ctx.at_war)?;
        econ_in.set("last_monthly_balance", country.economy.last_monthly_balance)?;

        let econ_out: Table = econ_fn.call(econ_in)?;
        country.economy.gdp_usd_millions = econ_out.get("gdp_usd_millions")?;
        country.economy.gdp_growth_pct = econ_out.get("gdp_growth_pct")?;
        country.economy.potential_gdp_growth_pct = econ_out.get("potential_gdp_growth_pct")?;
        country.economy.inflation_pct = econ_out.get("inflation_pct")?;
        country.economy.exchange_rate_usd = econ_out.get("exchange_rate_usd")?;
        country.economy.treasury_usd_millions = econ_out.get("treasury_usd_millions")?;
        country.economy.debt_to_gdp_pct = econ_out.get("debt_to_gdp_pct")?;
        country.economy.last_monthly_balance = econ_out.get("monthly_net_balance")?;
        country.economy.monthly_tax_revenue_m = econ_out.get("monthly_tax_revenue_m")?;
        country.economy.monthly_defense_cost_m = econ_out.get("monthly_defense_cost_m")?;
        country.economy.monthly_infra_cost_m = econ_out.get("monthly_infra_cost_m")?;
        country.economy.monthly_social_cost_m = econ_out.get("monthly_social_cost_m")?;
        country.economy.monthly_debt_service_m = econ_out.get("monthly_debt_service_m")?;
        country.economy.troop_upkeep_usd_millions = econ_out.get("troop_upkeep_m")?;
        country.economy.unemployment_pct = econ_out.get("unemployment_pct")?;
        country.economy.trade_balance_usd_millions = econ_out.get("trade_balance_usd_millions")?;
        country.economy.country_risk_bps = econ_out.get("country_risk_bps")?;
        country.economy.corporate_tax_revenue_m = econ_out.get("corporate_tax_revenue_m")?;
        country.economy.corporate_gdp_bonus_m = ctx.corporate_gdp_bonus_m;

        // 2. Gobernabilidad del Congreso (mesa circular), estabilidad y dinámica viva de oposición
        country.legislative_support_pct = country.compute_legislative_support_pct();
        let ruling_approval = country
            .ruling_party()
            .map(|p| p.approval_pct)
            .unwrap_or(45.0);

        let pol_fn: Function = rules_tbl.get("calculate_political_shift")?;
        let pol_in = self.lua.create_table()?;
        pol_in.set("gdp_growth_pct", country.economy.gdp_growth_pct)?;
        pol_in.set("inflation_pct", country.economy.inflation_pct)?;
        pol_in.set("social_investment_pct", country.economy.social_investment_pct)?;
        pol_in.set("tax_rate_pct", country.economy.tax_rate_pct)?;
        pol_in.set("ruling_approval_pct", ruling_approval)?;
        pol_in.set("stability", country.stability)?;
        pol_in.set("unemployment_pct", country.economy.unemployment_pct)?;
        pol_in.set("country_risk_bps", country.economy.country_risk_bps)?;
        pol_in.set("legislative_support_pct", country.legislative_support_pct)?;
        pol_in.set("political_capital", country.political_capital)?;
        pol_in.set("at_war", ctx.at_war)?;

        let pol_out: Table = pol_fn.call(pol_in)?;
        let new_ruling_approval: f64 = pol_out.get("ruling_approval_pct")?;
        country.stability = pol_out.get("stability")?;
        country.political_capital = pol_out.get("political_capital")?;

        let left_delta: f64 = pol_out.get("left_opposition_delta")?;
        let right_delta: f64 = pol_out.get("right_opposition_delta")?;
        let center_delta: f64 = pol_out.get("center_opposition_delta")?;

        update_party_approvals(
            &mut country.political_parties,
            new_ruling_approval,
            left_delta,
            right_delta,
            center_delta,
        );

        // Crecimiento demográfico mensual
        let monthly_pop_factor = 1.0 + (country.pop_growth_pct / 100.0) / 12.0;
        country.population = ((country.population as f64) * monthly_pop_factor).round() as u64;

        Ok(())
    }

    /// Evalúa un evento contextual por causas reales desde `scripts/events_latam.lua`.
    pub fn pick_contextual_event(
        &self,
        ctx: &LuaEventContext<'_>,
    ) -> Result<Option<GameEvent>, LuaBridgeError> {
        let events_bytecode = if let Some(bc) = self.events_bytecode.get() {
            bc
        } else {
            let src = Self::read_script("scripts/events_latam.lua")?;
            let compiled = self.load_or_compile_bytecode("events_latam", src)?;
            let _ = self.events_bytecode.set(compiled);
            self.events_bytecode
                .get()
                .expect("events_bytecode inicializado")
        };

        let tbl: Table = self
            .lua
            .load(events_bytecode.as_slice())
            .set_mode(ChunkMode::Binary)
            .eval()?;
        let pick_fn: Function = tbl.get("pick_contextual_event")?;

        let lua_ctx = self.lua.create_table()?;
        lua_ctx.set("inflation_pct", ctx.country.economy.inflation_pct)?;
        lua_ctx.set("gdp_growth_pct", ctx.country.economy.gdp_growth_pct)?;
        lua_ctx.set("unemployment_pct", ctx.country.economy.unemployment_pct)?;
        lua_ctx.set("debt_to_gdp_pct", ctx.country.economy.debt_to_gdp_pct)?;
        lua_ctx.set("interest_rate_pct", ctx.country.economy.interest_rate_pct)?;
        lua_ctx.set("country_risk_bps", ctx.country.economy.country_risk_bps)?;
        lua_ctx.set(
            "trade_balance_usd_millions",
            ctx.country.economy.trade_balance_usd_millions,
        )?;
        lua_ctx.set("stability", ctx.country.stability)?;
        lua_ctx.set(
            "legislative_support_pct",
            ctx.country.legislative_support_pct,
        )?;
        lua_ctx.set("company_count", ctx.company_count)?;
        lua_ctx.set("dominant_sector", ctx.dominant_sector)?;
        lua_ctx.set("at_war", ctx.at_war)?;
        lua_ctx.set("commodity_index", ctx.global_cycle.commodity_index)?;
        lua_ctx.set("fed_rate_pct", ctx.global_cycle.fed_rate_pct)?;
        lua_ctx.set("last_event_id", ctx.last_event_id.unwrap_or(""))?;
        lua_ctx.set("total_days", ctx.total_days)?;

        let val: Value = pick_fn.call(lua_ctx)?;
        let Value::Table(ev_tbl) = val else {
            return Ok(None);
        };

        let opts_tbl: Table = ev_tbl.get("options")?;
        let mut options = Vec::new();
        for item in opts_tbl.sequence_values::<Table>() {
            let ot = item?;
            options.push(EventOption {
                label: ot.get("label")?,
                treasury_delta_pct: ot.get("treasury_delta_pct")?,
                gdp_growth_delta: ot.get("gdp_growth_delta")?,
                inflation_delta: ot.get("inflation_delta")?,
                stability_delta: ot.get("stability_delta")?,
            });
        }

        Ok(Some(GameEvent {
            id: ev_tbl.get("id")?,
            title: ev_tbl.get("title")?,
            description: ev_tbl.get("description")?,
            options,
        }))
    }
}

/// Actualiza la aprobación del gobierno y redistribuye la intención de voto entre la oposición
/// según su afinidad ideológica con la coyuntura (desempleo, inflación, minoría parlamentaria).
fn update_party_approvals(
    parties: &mut [PoliticalParty],
    new_ruling_approval: f64,
    left_delta: f64,
    right_delta: f64,
    center_delta: f64,
) {
    if parties.is_empty() {
        return;
    }
    let clamped_ruling = new_ruling_approval.clamp(8.0, 86.0);
    for party in parties.iter_mut() {
        if party.is_ruling {
            party.approval_pct = clamped_ruling;
        } else {
            let lower = party.ideology.to_lowercase();
            let swing = if lower.contains("izquierda")
                || lower.contains("progres")
                || lower.contains("ecolog")
                || lower.contains("social")
            {
                left_delta
            } else if lower.contains("derecha")
                || lower.contains("conserv")
                || lower.contains("liberal")
            {
                right_delta
            } else {
                center_delta
            };
            party.approval_pct = (party.approval_pct + swing * 0.35).clamp(4.0, 75.0);
        }
    }

    // Rebalanceo suave de la oposición para que la suma total se mantenga cerca de 100%
    let opp_sum: f64 = parties
        .iter()
        .filter(|p| !p.is_ruling)
        .map(|p| p.approval_pct)
        .sum();
    let target_opp = (100.0 - clamped_ruling).max(12.0);
    if opp_sum > 0.1 {
        let ratio = target_opp / opp_sum;
        for party in parties.iter_mut().filter(|p| !p.is_ruling) {
            party.approval_pct = (party.approval_pct * (0.65 + 0.35 * ratio)).clamp(4.0, 75.0);
        }
    }
}
