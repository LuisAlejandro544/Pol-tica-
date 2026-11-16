//! Propósito del archivo:
//! Puente seguro entre Rust y Lua 5.4 (`mlua` con Lua 5.4 compilado estáticamente).
//! Carga los países iniciales desde `scripts/countries_latam.lua`, ejecuta las fórmulas
//! mensuales de `scripts/game_rules.lua` y genera eventos desde `scripts/events_latam.lua`.
//! Vive en el hilo secundario de simulación para nunca bloquear el renderizado de la UI.

use crate::map::svg_loader::ASSETS_DIR;
use crate::models::{
    AiPersonality, CountryState, EconomyState, EventOption, GameEvent, MilitaryState,
    PoliticalParty,
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
                    inflation_pct: row.get("inflation_pct")?,
                    gdp_usd_millions: row.get("gdp_usd_millions")?,
                    gdp_growth_pct: row.get("gdp_growth_pct")?,
                    debt_to_gdp_pct: row.get("debt_to_gdp_pct")?,
                    treasury_usd_millions: row.get("treasury_usd_millions")?,
                    tax_rate_pct: row.get("tax_rate_pct")?,
                    interest_rate_pct: row.get("interest_rate_pct")?,
                    infra_investment_pct: 3.5,
                    social_investment_pct: 4.5,
                    last_monthly_balance: 0.0,
                },
                military: MilitaryState {
                    active_troops: row.get("active_troops")?,
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

    /// Ejecuta un paso económico y político mensual para un país usando `scripts/game_rules.lua`.
    pub fn step_country_via_lua(&self, country: &mut CountryState) -> Result<(), LuaBridgeError> {
        let rules_src = Self::read_script("scripts/game_rules.lua")?;
        let rules_tbl: Table = self.lua.load(rules_src).eval()?;

        // 1. Cálculo macroeconómico en Lua 5.4
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

        let econ_out: Table = econ_fn.call(econ_in)?;
        country.economy.gdp_usd_millions = econ_out.get("gdp_usd_millions")?;
        country.economy.gdp_growth_pct = econ_out.get("gdp_growth_pct")?;
        country.economy.inflation_pct = econ_out.get("inflation_pct")?;
        country.economy.exchange_rate_usd = econ_out.get("exchange_rate_usd")?;
        country.economy.treasury_usd_millions = econ_out.get("treasury_usd_millions")?;
        country.economy.debt_to_gdp_pct = econ_out.get("debt_to_gdp_pct")?;
        country.economy.last_monthly_balance = econ_out.get("monthly_net_balance")?;

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
