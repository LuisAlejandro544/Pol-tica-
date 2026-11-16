--[[
  Propósito del archivo (Lua 5.4):
  Reglas económicas, fiscales, monetarias y políticas ejecutadas en cada turno mensual.
  Rust invoca estas funciones desde un hilo secundario (SimulationWorker) para mantener
  la fluidez de la interfaz móvil a 60 FPS sin congelar el hilo principal.
--]]

local rules = {}

-- Calcula la variación mensual de PIB, inflación, tipo de cambio y tesorería nacional
function rules.calculate_monthly_economy(econ)
  -- Efecto de la tasa de interés real sobre la inflación y el crecimiento
  local real_rate = econ.interest_rate_pct - econ.inflation_pct
  local inflation_delta = -real_rate * 0.045 + (econ.tax_rate_pct < 15.0 and 0.12 or -0.04)
  local new_inflation = math.max(0.4, math.min(180.0, econ.inflation_pct + inflation_delta))

  -- El crecimiento del PIB responde a inversión en infraestructura, estabilidad y carga fiscal
  local tax_drag = math.max(0.0, (econ.tax_rate_pct - 20.0) * 0.08)
  local rate_drag = math.max(0.0, real_rate * 0.05)
  local infra_boost = econ.infra_investment_pct * 0.14
  local stability_factor = (econ.stability - 50.0) * 0.025
  local new_gdp_growth = math.max(-8.0, math.min(12.0,
    2.4 + infra_boost + stability_factor - tax_drag - rate_drag
  ))

  -- Crecimiento compuesto mensual del PIB nominal (en millones de USD)
  local monthly_factor = 1.0 + (new_gdp_growth / 100.0) / 12.0
  local new_gdp = econ.gdp_usd_millions * monthly_factor

  -- Tipo de cambio frente al USD (monedas dolarizadas permanecen fijas en 1.0)
  local new_exchange = econ.exchange_rate_usd
  if econ.currency_code ~= "USD" and econ.currency_code ~= "PAB" then
    local fx_pressure = math.max(-0.015, (new_inflation - 2.5) / 1200.0)
    new_exchange = math.max(0.5, econ.exchange_rate_usd * (1.0 + fx_pressure))
  end

  -- Recaudación fiscal mensual menos gasto de defensa, infraestructura y servicio de deuda
  local monthly_gdp = new_gdp / 12.0
  local tax_revenue = monthly_gdp * (econ.tax_rate_pct / 100.0)
  local defense_cost = monthly_gdp * (econ.defense_budget_pct / 100.0)
  local infra_cost = monthly_gdp * (econ.infra_investment_pct / 100.0)
  local social_cost = monthly_gdp * (econ.social_investment_pct / 100.0)
  local debt_service = (new_gdp * (econ.debt_to_gdp_pct / 100.0)) * 0.0045
  local net_balance = tax_revenue - (defense_cost + infra_cost + social_cost + debt_service)

  local new_treasury = econ.treasury_usd_millions + net_balance
  local new_debt_pct = econ.debt_to_gdp_pct
  if new_treasury < 0.0 then
    -- Emisión de deuda automática para cubrir déficit de caja
    local deficit_pct = (math.abs(new_treasury) / math.max(1.0, new_gdp)) * 100.0
    new_debt_pct = math.min(240.0, new_debt_pct + deficit_pct)
    new_treasury = 0.0
  elseif net_balance > 0.0 and new_debt_pct > 10.0 then
    new_debt_pct = math.max(5.0, new_debt_pct - 0.08)
  end

  return {
    gdp_usd_millions = new_gdp,
    gdp_growth_pct = new_gdp_growth,
    inflation_pct = new_inflation,
    exchange_rate_usd = new_exchange,
    treasury_usd_millions = new_treasury,
    debt_to_gdp_pct = new_debt_pct,
    monthly_net_balance = net_balance
  }
end

-- Calcula el cambio mensual en estabilidad y popularidad del partido gobernante
function rules.calculate_political_shift(pol)
  local growth_bonus = pol.gdp_growth_pct * 0.25
  local inflation_penalty = math.max(0.0, pol.inflation_pct - 5.0) * 0.12
  local social_bonus = pol.social_investment_pct * 0.18
  local tax_penalty = math.max(0.0, pol.tax_rate_pct - 24.0) * 0.15

  local approval_delta = growth_bonus + social_bonus - inflation_penalty - tax_penalty
  local new_ruling_approval = math.max(8.0, math.min(92.0, pol.ruling_approval_pct + approval_delta * 0.25))
  local new_stability = math.max(10.0, math.min(98.0,
    pol.stability + (new_ruling_approval - 50.0) * 0.03 - inflation_penalty * 0.2
  ))

  return {
    ruling_approval_pct = new_ruling_approval,
    stability = new_stability
  }
end

return rules
