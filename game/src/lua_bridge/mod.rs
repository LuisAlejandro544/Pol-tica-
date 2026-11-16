//! Propósito del archivo:
//! Puente seguro entre Rust y Lua 5.4 (`mlua` con Lua 5.4 compilado estáticamente).
//! Carga los países iniciales desde `scripts/countries_latam.lua`, las provincias y
//! capitales desde `scripts/provinces_latam.lua`, ejecuta las fórmulas mensuales de
//! `scripts/game_rules.lua` y genera eventos desde `scripts/events_latam.lua`.
//! Vive en el hilo secundario de simulación para nunca bloquear el renderizado de la UI.

use crate::map::svg_loader::ASSETS_DIR;
use crate::models::{
    AiPersonality, CapitalMarkerData, CompanyCategory, CompanyCategoryTemplate, CountryState,
    EconomyState, EventOption, GameEvent, MilitaryState, PoliticalParty, ProvinceData,
};
use mlua::{Function, Lua, Table};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LuaBridgeError {
    #[error("Error de máquina virtual Lua 5.4: {0}")]
    Lua(#[from] mlua::Error),
    #[error("No se encontró el script incrustado: {0}")]
    MissingScript(String),
}

pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    /// Inicializa Lua 5.4 en modo seguro sin acceso peligroso al sistema de archivos externo.
    pub fn new() -> Result<Self, LuaBridgeError> {
        let lua = Lua::new();
        Ok(Self { lua })
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

    /// Carga los 21 países latinoamericanos definidos en `scripts/countries_latam.lua`.
    pub fn load_initial_countries(&self) -> Result<BTreeMap<String, CountryState>, LuaBridgeError> {
        let src = Self::read_script("scripts/countries_latam.lua")?;
        let table: Table = self.lua.load(src).eval()?;
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
            let inflation_pct: f64 = row.get("inflation_pct")?;
            let gdp_growth_pct: f64 = row.get("gdp_growth_pct")?;
            let initial_unemp = (6.4 + inflation_pct * 0.12 - gdp_growth_pct * 0.35).clamp(3.8, 14.5);
            let initial_upkeep = (active_troops as f64 / 1_000.0) * 1.35;

            let state = CountryState {
                id: id.clone(),
                name: row.get("name")?,
                capital: row.get("capital")?,
                region: row.get("region")?,
                color,
                population: row.get("population")?,
                pop_growth_pct: row.get("pop_growth_pct")?,
                urban_pct: row.get("urban_pct")?,
                stability: row.get("stability")?,
                economy: EconomyState {
                    currency_code: row.get("currency_code")?,
                    currency_name: row.get("currency_name")?,
                    exchange_rate_usd: row.get("exchange_rate_usd")?,
                    inflation_pct,
                    gdp_usd_millions: row.get("gdp_usd_millions")?,
                    gdp_growth_pct,
                    debt_to_gdp_pct: row.get("debt_to_gdp_pct")?,
                    treasury_usd_millions: row.get("treasury_usd_millions")?,
                    tax_rate_pct: row.get("tax_rate_pct")?,
                    interest_rate_pct: row.get("interest_rate_pct")?,
                    infra_investment_pct: 3.5,
                    social_investment_pct: 4.5,
                    last_monthly_balance: 0.0,
                    troop_upkeep_usd_millions: initial_upkeep,
                    unemployment_pct: initial_unemp,
                    corporate_gdp_bonus_m: 0.0,
                },
                military: MilitaryState {
                    active_troops,
                    defense_budget_pct: row.get("defense_budget_pct")?,
                    readiness: 72.0,
                },
                political_parties: parties,
                neighbors,
                ai_personality,
                months_to_election: 24 + ((idx as u32 * 7) % 24),
            };
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
    /// Si el archivo aún no existe, devuelve colecciones vacías para mantener compatibilidad.
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

        let root: Table = self.lua.load(&src).eval()?;
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

    /// Ejecuta un paso económico y político mensual para un país usando `scripts/game_rules.lua`.
    pub fn step_country_via_lua(
        &self,
        country: &mut CountryState,
        corporate_gdp_bonus_m: f64,
        corporate_jobs_total: u64,
    ) -> Result<(), LuaBridgeError> {
        let rules_src = Self::read_script("scripts/game_rules.lua")?;
        let rules_tbl: Table = self.lua.load(rules_src).eval()?;

        // 1. Cálculo macroeconómico en Lua 5.4 (incluye mantenimiento militar, empresas y desempleo)
        let econ_fn: Function = rules_tbl.get("calculate_monthly_economy")?;
        let econ_in = self.lua.create_table()?;
        econ_in.set("currency_code", country.economy.currency_code.as_str())?;
        econ_in.set("exchange_rate_usd", country.economy.exchange_rate_usd)?;
        econ_in.set("inflation_pct", country.economy.inflation_pct)?;
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
        econ_in.set("corporate_gdp_bonus_m", corporate_gdp_bonus_m)?;
        econ_in.set("corporate_jobs_total", corporate_jobs_total)?;
        econ_in.set("population", country.population)?;

        let econ_out: Table = econ_fn.call(econ_in)?;
        country.economy.gdp_usd_millions = econ_out.get("gdp_usd_millions")?;
        country.economy.gdp_growth_pct = econ_out.get("gdp_growth_pct")?;
        country.economy.inflation_pct = econ_out.get("inflation_pct")?;
        country.economy.exchange_rate_usd = econ_out.get("exchange_rate_usd")?;
        country.economy.treasury_usd_millions = econ_out.get("treasury_usd_millions")?;
        country.economy.debt_to_gdp_pct = econ_out.get("debt_to_gdp_pct")?;
        country.economy.last_monthly_balance = econ_out.get("monthly_net_balance")?;
        country.economy.troop_upkeep_usd_millions = econ_out.get("troop_upkeep_m")?;
        country.economy.unemployment_pct = econ_out.get("unemployment_pct")?;
        country.economy.corporate_gdp_bonus_m = corporate_gdp_bonus_m;

        // 2. Cálculo de aprobación política y estabilidad en Lua 5.4
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

        let pol_out: Table = pol_fn.call(pol_in)?;
        let new_ruling_approval: f64 = pol_out.get("ruling_approval_pct")?;
        country.stability = pol_out.get("stability")?;

        if let Some(ruling) = country.political_parties.iter_mut().find(|p| p.is_ruling) {
            ruling.approval_pct = new_ruling_approval;
        }

        // Crecimiento demográfico mensual
        let monthly_pop_factor = 1.0 + (country.pop_growth_pct / 100.0) / 12.0;
        country.population = ((country.population as f64) * monthly_pop_factor).round() as u64;

        Ok(())
    }

    /// Selecciona un evento dinámico desde `scripts/events_latam.lua`.
    pub fn pick_event(&self, turn: u64, roll: u64) -> Result<GameEvent, LuaBridgeError> {
        let src = Self::read_script("scripts/events_latam.lua")?;
        let tbl: Table = self.lua.load(src).eval()?;
        let pick_fn: Function = tbl.get("pick_event")?;
        let ev_tbl: Table = pick_fn.call((turn, roll))?;

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

        Ok(GameEvent {
            id: ev_tbl.get("id")?,
            title: ev_tbl.get("title")?,
            description: ev_tbl.get("description")?,
            options,
        })
    }
}
