--[[
  Propósito del archivo (Lua 5.4):
  Reglas económicas, monetarias, cambiarias y políticas realistas ejecutadas cada mes.
  Implementa Regla de Taylor frente a tasa neutral, Curva de Phillips (brecha de empleo/PIB),
  pass-through del tipo de cambio, prima de riesgo soberano, recaudación corporativa y
  gobernabilidad parlamentaria conectada a coaliciones del Congreso.
--]]

local rules = {}

local function clamp(x, lo, hi)
  if x < lo then return lo end
  if x > hi then return hi end
  return x
end

-- Calcula la variación mensual realista de PIB, inflación, tipo de cambio, prima de riesgo y cuentas fiscales
function rules.calculate_monthly_economy(econ)
  local prev_inf = econ.inflation_pct or 3.5
  local target_inf = econ.inflation_target_pct or 3.0
  local unemp = econ.unemployment_pct or 6.5
  local nairu = 6.2
  local fed_rate = econ.global_fed_rate_pct or 4.25
  local commodity_idx = econ.commodity_index or 100.0
  local trade_pacts = econ.trade_pacts_count or 0
  local active_troops = econ.active_troops or 0
  local at_war = econ.at_war or false

  -- 1. Prima de riesgo país (en puntos básicos) según deuda, reservas y estabilidad institucional
  local treasury_months = (econ.treasury_usd_millions or 0.0) / math.max(50.0, (econ.gdp_usd_millions / 12.0) * 0.20)
  local debt_stress = math.max(0.0, (econ.debt_to_gdp_pct or 45.0) - 55.0) * 11.5
  local inst_stress = math.max(0.0, 62.0 - (econ.stability or 60.0)) * 9.0
  local reserve_stress = math.max(0.0, 3.0 - treasury_months) * 85.0
  local war_stress = at_war and 260.0 or 0.0
  local country_risk_bps = clamp(120.0 + debt_stress + inst_stress + reserve_stress + war_stress, 85.0, 2600.0)

  -- 2. Crecimiento potencial estructural y brecha de producto (Curva de Phillips + Regla de Taylor)
  local corp_bonus_m = econ.corporate_gdp_bonus_m or 0.0
  local corp_export_m = econ.corporate_export_bonus_m or 0.0
  local corp_ratio_boost = math.min(2.1, (corp_bonus_m / math.max(1200.0, econ.gdp_usd_millions)) * 28.0)
  local infra_structural = math.min(2.4, (econ.infra_investment_pct or 3.5) * 0.24)
  local pact_boost = math.min(0.85, trade_pacts * 0.14)
  local potential_growth = clamp(1.8 + infra_structural + corp_ratio_boost + pact_boost, 1.5, 6.2)

  -- Tasa real neutral ajustada por riesgo país (~2.25% + riesgo financiero)
  local real_rate = (econ.interest_rate_pct or 6.0) - prev_inf
  local neutral_real_rate = 2.25 + (country_risk_bps / 1400.0)
  local monetary_stance = real_rate - neutral_real_rate -- >0 restrictivo, <0 expansivo

  -- Crecimiento mensual anualizado del PIB
  local tax_drag = math.max(0.0, ((econ.tax_rate_pct or 20.0) - 21.0) * 0.095)
  local tax_stimulus = math.max(0.0, (19.0 - (econ.tax_rate_pct or 20.0)) * 0.06)
  local monetary_drag = monetary_stance * 0.22
  local stability_factor = ((econ.stability or 60.0) - 52.0) * 0.022
  local commodity_cycle = ((commodity_idx - 100.0) / 100.0) * 1.4
  local war_drag = at_war and 1.35 or 0.0

  -- Suavizado realista hacia el nuevo ritmo de crecimiento (evita saltos bruscos de un mes a otro)
  local target_gdp_growth = clamp(
    potential_growth + tax_stimulus + stability_factor + commodity_cycle - tax_drag - monetary_drag - war_drag,
    -8.5,
    11.5
  )
  local new_gdp_growth = clamp(
    (econ.gdp_growth_pct or potential_growth) * 0.65 + target_gdp_growth * 0.35,
    -8.5,
    11.5
  )

  local monthly_factor = 1.0 + (new_gdp_growth / 100.0) / 12.0
  local new_gdp = econ.gdp_usd_millions * monthly_factor
  local monthly_gdp = new_gdp / 12.0

  -- 3. Balanza comercial mensual (exportaciones empresariales + ciclo de commodities + tratados - importaciones)
  local base_trade_ratio = (commodity_idx - 100.0) * 0.0005 + trade_pacts * 0.0012 - math.max(0.0, new_gdp_growth - 3.0) * 0.0016
  local monthly_corp_exports = (corp_export_m / 12.0) * (commodity_idx / 100.0)
  local trade_balance_m = (monthly_gdp * base_trade_ratio) + monthly_corp_exports

  -- 4. Tipo de cambio frente al USD y pass-through inflacionario (monedas dolarizadas permanecen fijas)
  local new_exchange = econ.exchange_rate_usd
  local fx_pct_change = 0.0
  if econ.currency_code ~= "USD" and econ.currency_code ~= "PAB" then
    local external_rate_req = fed_rate + (country_risk_bps / 100.0) * 0.55
    local carry_diff = ((econ.interest_rate_pct or 6.0) - external_rate_req) / 100.0
    local inf_diff = (prev_inf - 2.4) / 100.0
    local trade_support = clamp(trade_balance_m / math.max(100.0, monthly_gdp), -0.08, 0.08)

    local fx_pressure = clamp((inf_diff / 12.0) - (carry_diff / 18.0) - (trade_support * 0.14), -0.025, 0.045)
    local next_fx = math.max(0.5, econ.exchange_rate_usd * (1.0 + fx_pressure))
    fx_pct_change = (next_fx - econ.exchange_rate_usd) / math.max(0.5, econ.exchange_rate_usd)
    new_exchange = next_fx
  end

  -- 5. Ecuación de inflación realista:
  --    - Inercia + anclaje a la meta del Banco Central (evita caer absurdamente a 0.4%)
  --    - Curva de Phillips (brecha de empleo NAIRU y brecha de producto)
  --    - Política monetaria (Regla de Taylor respecto a tasa neutral, amortiguada cerca de la meta)
  --    - Pass-through cambiario y dominancia fiscal por déficit/deuda alta
  local output_gap = new_gdp_growth - potential_growth
  local labor_gap = clamp(nairu - unemp, -4.0, 3.5) -- empleo muy bajo empuja salarios arriba
  local phillips_pressure = output_gap * 0.055 + labor_gap * 0.045

  local inflation_excess = prev_inf - target_inf
  local taylor_damping = 0.0
  if monetary_stance > 0.0 then
    -- Tasa real restrictiva frena el exceso de inflación sobre la meta, sin aplastar hacia cero
    local anchor_scale = clamp((prev_inf - (target_inf * 0.55)) / 4.0, 0.0, 1.4)
    taylor_damping = -monetary_stance * 0.042 * anchor_scale
  else
    -- Tasas reales por debajo de la tasa neutral estimulan demanda e inflación
    taylor_damping = -monetary_stance * 0.055
  end

  local anchor_pull = (target_inf - prev_inf) * 0.018
  local fx_passthrough = clamp(fx_pct_change * 100.0 * 0.24, -0.20, 0.65)
  local fiscal_pressure = 0.0
  if (econ.last_monthly_balance or 0.0) < 0.0 and (econ.debt_to_gdp_pct or 40.0) > 65.0 then
    fiscal_pressure = clamp((math.abs(econ.last_monthly_balance) / math.max(80.0, monthly_gdp)) * 1.15, 0.0, 0.45)
  end

  local inflation_delta = anchor_pull + phillips_pressure + taylor_damping + fx_passthrough + fiscal_pressure
  local new_inflation = clamp(prev_inf + inflation_delta, -0.4, 210.0)

  -- 6. Presupuesto fiscal mensual con recaudación corporativa real y costo de deuda vinculado al riesgo país
  local troop_upkeep_m = (active_troops / 1000.0) * 1.35
  local corp_tax_revenue_m = (corp_bonus_m / 12.0) * ((econ.tax_rate_pct or 20.0) / 100.0) * 0.42
  local tax_revenue = monthly_gdp * ((econ.tax_rate_pct or 20.0) / 100.0) + corp_tax_revenue_m
  local defense_cost = monthly_gdp * ((econ.defense_budget_pct or 1.0) / 100.0)
  local infra_cost = monthly_gdp * ((econ.infra_investment_pct or 3.5) / 100.0)
  local social_cost = monthly_gdp * ((econ.social_investment_pct or 4.5) / 100.0)

  local monthly_debt_yield = (0.0032 + (country_risk_bps / 10000.0) * 0.085)
  local debt_service = (new_gdp * ((econ.debt_to_gdp_pct or 45.0) / 100.0)) * monthly_debt_yield
  local net_balance = tax_revenue - (defense_cost + troop_upkeep_m + infra_cost + social_cost + debt_service)

  local new_treasury = econ.treasury_usd_millions + net_balance
  local new_debt_pct = econ.debt_to_gdp_pct
  if new_treasury < 0.0 then
    local deficit_pct = (math.abs(new_treasury) / math.max(1.0, new_gdp)) * 100.0
    new_debt_pct = math.min(240.0, new_debt_pct + deficit_pct)
    new_treasury = 0.0
  elseif net_balance > 0.0 and new_debt_pct > 10.0 then
    local amort = math.min(0.25, (net_balance / math.max(10.0, new_gdp)) * 18.0)
    new_debt_pct = math.max(5.0, new_debt_pct - amort)
  end

  -- 7. Dinámica laboral (Ley de Okun + empresas + inversión pública)
  local corp_jobs = econ.corporate_jobs_total or 0
  local workforce = math.max(100000.0, (econ.population or 10000000.0) * 0.48)
  local corp_job_effect = math.min(2.2, (corp_jobs / workforce) * 28.0)
  local okun_delta = (potential_growth * 0.72 - new_gdp_growth) * 0.055
  local unemp_delta = okun_delta
    + math.max(0.0, (econ.tax_rate_pct or 20.0) - 25.0) * 0.015
    - (econ.infra_investment_pct or 3.5) * 0.018
    - corp_job_effect * 0.025
  local new_unemployment = clamp(unemp + unemp_delta, 2.2, 34.0)

  return {
    gdp_usd_millions = new_gdp,
    gdp_growth_pct = new_gdp_growth,
    potential_gdp_growth_pct = potential_growth,
    inflation_pct = new_inflation,
    exchange_rate_usd = new_exchange,
    treasury_usd_millions = new_treasury,
    debt_to_gdp_pct = new_debt_pct,
    monthly_net_balance = net_balance,
    troop_upkeep_m = troop_upkeep_m,
    unemployment_pct = new_unemployment,
    trade_balance_usd_millions = trade_balance_m,
    country_risk_bps = country_risk_bps,
    corporate_tax_revenue_m = corp_tax_revenue_m
  }
end

-- Calcula aprobación ciudadana, estabilidad institucional, capital político y deriva de partidos opositores
function rules.calculate_political_shift(pol)
  local growth = pol.gdp_growth_pct or 2.0
  local inflation = pol.inflation_pct or 4.0
  local social = pol.social_investment_pct or 4.5
  local tax = pol.tax_rate_pct or 20.0
  local unemp = pol.unemployment_pct or 6.5
  local risk_bps = pol.country_risk_bps or 200.0
  local seats_support = pol.legislative_support_pct or 50.0
  local at_war = pol.at_war or false
  local prev_approval = pol.ruling_approval_pct or 45.0
  local prev_stability = pol.stability or 65.0
  local prev_cap = pol.political_capital or 45.0

  -- 1. Popularidad del gobierno: responde al poder adquisitivo real, empleo y red social
  local real_income_feel = growth - math.max(0.0, inflation - 4.5) * 0.38
  local social_cushion = (social - 3.6) * 0.35
  local tax_friction = math.max(0.0, tax - 23.5) * 0.22
  local unemp_pain = math.max(0.0, unemp - 7.2) * 0.24
  local approval_delta = real_income_feel * 0.40 + social_cushion - tax_friction - unemp_pain

  -- Reversión suave a la media electoral para evitar extremos irrealistas
  local mean_reversion = (46.0 - prev_approval) * 0.015
  local new_ruling_approval = clamp(prev_approval + approval_delta * 0.28 + mean_reversion, 10.0, 88.0)

  -- 2. Estabilidad Institucional: NO cae porque aprobación < 50%.
  --    Depende de gobernabilidad en el Congreso (mesa circular), costo de vida, empleo, gasto social y riesgo.
  local congress_gov = 0.0
  if seats_support >= 50.0 then
    congress_gov = math.min(0.24, (seats_support - 50.0) * 0.014 + 0.06)
  else
    -- Minoría parlamentaria desgasta lentamente la estabilidad hasta que se negocie coalición
    congress_gov = -math.min(0.28, (50.0 - seats_support) * 0.012)
  end

  local purchasing_power = clamp((growth * 0.14) - math.max(0.0, inflation - 6.0) * 0.055, -0.55, 0.35)
  local social_peace = clamp((social - 3.0) * 0.075 - math.max(0.0, unemp - 8.5) * 0.055, -0.45, 0.32)
  local risk_drag = math.max(0.0, risk_bps - 650.0) * 0.00035
  local war_penalty = at_war and 0.35 or 0.0
  -- Aprobación solo penaliza si cae en crisis de legitimidad (<33%) o premia si supera 56%
  local legitimacy = 0.0
  if prev_approval < 33.0 then
    legitimacy = -(33.0 - prev_approval) * 0.018
  elseif prev_approval > 56.0 then
    legitimacy = (prev_approval - 56.0) * 0.012
  end

  -- Equilibrio institucional base suave para evitar derrumbes artificiales en países sanos
  local stab_delta = congress_gov + purchasing_power + social_peace + legitimacy - risk_drag - war_penalty
  local new_stability = clamp(prev_stability + stab_delta, 12.0, 98.0)

  -- 3. Capital político mensual generado por mayoría en la mesa circular y estabilidad
  local cap_gain = (seats_support >= 50.0 and 3.5 or 1.4) + (new_stability >= 60.0 and 1.2 or -0.6)
  local new_political_capital = clamp(prev_cap + cap_gain, 0.0, 100.0)

  -- 4. Señales ideológicas para actualizar la intención de voto de los partidos de oposición
  local left_swing = math.max(0.0, unemp - 7.5) * 0.14 + math.max(0.0, 3.5 - social) * 0.25 - math.max(0.0, tax - 26.0) * 0.08
  local right_swing = math.max(0.0, inflation - 6.0) * 0.12 + math.max(0.0, tax - 23.0) * 0.18 - math.max(0.0, unemp - 10.0) * 0.05
  local center_swing = math.max(0.0, 52.0 - seats_support) * 0.04 + (new_stability < 52.0 and 0.12 or -0.04)

  return {
    ruling_approval_pct = new_ruling_approval,
    stability = new_stability,
    political_capital = new_political_capital,
    left_opposition_delta = left_swing,
    right_opposition_delta = right_swing,
    center_opposition_delta = center_swing
  }
end

return rules
