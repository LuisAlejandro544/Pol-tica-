--[[
  Propósito del archivo (Lua 5.4):
  Generador de eventos geopolíticos, económicos, empresariales y parlamentarios
  disparados por condiciones reales del país y el ciclo global (nunca rotación % 4).
--]]

local events = {}

local function build_catalog(ctx)
  local sector = ctx.dominant_sector or "Tecnología e Industria"
  return {
    INFLATION_SPIRAL = {
      id = "INFLATION_SPIRAL",
      title = "Presión Inflacionaria y Demanda Salarial",
      description = string.format(
        "La inflación (%.1f%%) erosiona el poder de compra y los gremios exigen ajuste de tarifas e ingresos.",
        ctx.inflation_pct or 12.0
      ),
      options = {
        {
          label = "Anclar expectativas con respaldo del Banco Central",
          treasury_delta_pct = -0.012,
          gdp_growth_delta = -0.25,
          inflation_delta = -1.6,
          stability_delta = 1.8
        },
        {
          label = "Acordar banda salarial gradual con empresarios",
          treasury_delta_pct = -0.006,
          gdp_growth_delta = 0.15,
          inflation_delta = 0.7,
          stability_delta = 3.0
        }
      }
    },
    FX_CAPITAL_FLIGHT = {
      id = "FX_CAPITAL_FLIGHT",
      title = "Tensión Cambiaria y Salida de Capitales",
      description = string.format(
        "El riesgo país (%.0f pb) y las tasas externas (Fed %.2f%%) presionan el tipo de cambio.",
        ctx.country_risk_bps or 500.0,
        ctx.fed_rate_pct or 4.25
      ),
      options = {
        {
          label = "Subir tasa de referencia e intervenir en mercado",
          treasury_delta_pct = -0.018,
          gdp_growth_delta = -0.35,
          inflation_delta = -1.1,
          stability_delta = 1.2
        },
        {
          label = "Permitir flotación cambiaria y proteger exportadores",
          treasury_delta_pct = 0.008,
          gdp_growth_delta = 0.30,
          inflation_delta = 1.3,
          stability_delta = -1.5
        }
      }
    },
    CONGRESS_GRIDLOCK = {
      id = "CONGRESS_GRIDLOCK",
      title = "Bloqueo en la Mesa Circular del Congreso",
      description = string.format(
        "La coalición oficialista suma solo %.1f%% de los escaños y la oposición bloquea leyes presupuestarias.",
        ctx.legislative_support_pct or 40.0
      ),
      options = {
        {
          label = "Conceder partidas regionales y pactar gobernabilidad",
          treasury_delta_pct = -0.015,
          gdp_growth_delta = 0.25,
          inflation_delta = 0.15,
          stability_delta = 5.0
        },
        {
          label = "Gobernar por decreto ejecutivo de emergencia",
          treasury_delta_pct = 0.012,
          gdp_growth_delta = -0.20,
          inflation_delta = -0.1,
          stability_delta = -4.0
        }
      }
    },
    LABOR_STRIKE = {
      id = "LABOR_STRIKE",
      title = "Paro Nacional y Reclamo de Empleo",
      description = string.format(
        "Con %.1f%% de desempleo y estabilidad en %.0f%%, sindicatos convocan marchas en las capitales.",
        ctx.unemployment_pct or 10.5,
        ctx.stability or 45.0
      ),
      options = {
        {
          label = "Plan de obra pública intensivo en mano de obra",
          treasury_delta_pct = -0.022,
          gdp_growth_delta = 0.45,
          inflation_delta = 0.35,
          stability_delta = 4.2
        },
        {
          label = "Mantener disciplina fiscal y mesa técnica",
          treasury_delta_pct = 0.005,
          gdp_growth_delta = -0.25,
          inflation_delta = -0.35,
          stability_delta = -2.8
        }
      }
    },
    DEBT_DOWNGRADE = {
      id = "DEBT_DOWNGRADE",
      title = "Alerta de Calificadoras sobre Deuda Pública",
      description = string.format(
        "La deuda alcanza %.1f%% del PIB; acreedores internacionales exigen señal de solvencia fiscal.",
        ctx.debt_to_gdp_pct or 85.0
      ),
      options = {
        {
          label = "Plan plurianual de superávit primario",
          treasury_delta_pct = 0.025,
          gdp_growth_delta = -0.35,
          inflation_delta = -0.4,
          stability_delta = -1.5
        },
        {
          label = "Canje de bonos con banca y empresas locales",
          treasury_delta_pct = -0.008,
          gdp_growth_delta = 0.20,
          inflation_delta = 0.45,
          stability_delta = 1.6
        }
      }
    },
    SECTOR_EXPANSION = {
      id = "SECTOR_EXPANSION",
      title = "Auge Exportador en " .. sector,
      description = string.format(
        "Las %d empresas privadas nacionales y la demanda externa abren nuevos contratos de exportación.",
        ctx.company_count or 1
      ),
      options = {
        {
          label = "Cofinanciar logística portuaria y aduanera",
          treasury_delta_pct = -0.012,
          gdp_growth_delta = 0.65,
          inflation_delta = 0.15,
          stability_delta = 2.4
        },
        {
          label = "Cobrar regalía extraordinaria para reservas",
          treasury_delta_pct = 0.032,
          gdp_growth_delta = 0.20,
          inflation_delta = -0.25,
          stability_delta = 1.2
        }
      }
    },
    CORPORATE_CREDIT_CRUNCH = {
      id = "CORPORATE_CREDIT_CRUNCH",
      title = "Encarecimiento del Crédito Corporativo",
      description = string.format(
        "La tasa de referencia (%.1f%%) encarece el financiamiento de las empresas privadas y frena inversión.",
        ctx.interest_rate_pct or 15.0
      ),
      options = {
        {
          label = "Línea de crédito de fomento productivo",
          treasury_delta_pct = -0.015,
          gdp_growth_delta = 0.45,
          inflation_delta = 0.35,
          stability_delta = 2.0
        },
        {
          label = "Priorizar control estricto de precios",
          treasury_delta_pct = 0.0,
          gdp_growth_delta = -0.30,
          inflation_delta = -0.85,
          stability_delta = -0.8
        }
      }
    },
    DEFLATIONARY_SLACK = {
      id = "DEFLATIONARY_SLACK",
      title = "Enfriamiento Económico y Baja Demanda",
      description = string.format(
        "La inflación cayó a %.1f%% con crecimiento débil (%.1f%%); el consumo interno necesita impulso.",
        ctx.inflation_pct or 1.0,
        ctx.gdp_growth_pct or 1.0
      ),
      options = {
        {
          label = "Recortar cargas y acelerar licitaciones públicas",
          treasury_delta_pct = -0.018,
          gdp_growth_delta = 0.70,
          inflation_delta = 0.60,
          stability_delta = 2.5
        },
        {
          label = "Sostener ahorro fiscal precautorio",
          treasury_delta_pct = 0.018,
          gdp_growth_delta = -0.15,
          inflation_delta = -0.10,
          stability_delta = -1.0
        }
      }
    },
    COMMODITY_BOOM = {
      id = "COMMODITY_BOOM",
      title = "Ciclo Alcista Global de Materias Primas",
      description = string.format(
        "El índice mundial de commodities sube a %.0f puntos, mejorando los términos de intercambio en Latam.",
        ctx.commodity_index or 108.0
      ),
      options = {
        {
          label = "Crear fondo de estabilización contracíclico",
          treasury_delta_pct = 0.035,
          gdp_growth_delta = 0.30,
          inflation_delta = -0.30,
          stability_delta = 2.2
        },
        {
          label = "Acelerar inversión pública e infraestructura",
          treasury_delta_pct = -0.010,
          gdp_growth_delta = 0.75,
          inflation_delta = 0.35,
          stability_delta = 2.6
        }
      }
    },
    REGIONAL_TRADE_SUMMIT = {
      id = "REGIONAL_TRADE_SUMMIT",
      title = "Cumbre Comercial y Corredores de Integración",
      description = "Naciones vecinas proponen reducir barreras aduaneras y agilizar cadenas de suministro.",
      options = {
        {
          label = "Firmar acuerdo de facilitación comercial",
          treasury_delta_pct = 0.015,
          gdp_growth_delta = 0.50,
          inflation_delta = -0.25,
          stability_delta = 2.0
        },
        {
          label = "Negociar salvaguardas para industria local",
          treasury_delta_pct = 0.008,
          gdp_growth_delta = 0.10,
          inflation_delta = 0.25,
          stability_delta = 1.4
        }
      }
    }
  }
end

-- Evalúa causas macro, cambiarias, corporativas y parlamentarias reales en vez de módulo % 4
function events.pick_contextual_event(ctx)
  local catalog = build_catalog(ctx)
  local last_id = ctx.last_event_id or ""
  local candidates = {}

  local function push_if(cond, id, weight)
    if cond and id ~= last_id and catalog[id] then
      table.insert(candidates, { id = id, weight = weight })
    end
  end

  push_if((ctx.inflation_pct or 0.0) >= 11.0, "INFLATION_SPIRAL", 35)
  push_if(
    (ctx.country_risk_bps or 200.0) >= 480.0
      or ((ctx.trade_balance_usd_millions or 0.0) < -50.0 and (ctx.fed_rate_pct or 4.0) >= 4.5),
    "FX_CAPITAL_FLIGHT",
    32
  )
  push_if((ctx.legislative_support_pct or 55.0) < 48.0, "CONGRESS_GRIDLOCK", 28)
  push_if((ctx.unemployment_pct or 6.0) >= 10.2 or (ctx.stability or 70.0) < 46.0, "LABOR_STRIKE", 30)
  push_if((ctx.debt_to_gdp_pct or 45.0) >= 82.0, "DEBT_DOWNGRADE", 26)
  push_if((ctx.interest_rate_pct or 7.0) >= 14.0 and (ctx.company_count or 0) >= 1, "CORPORATE_CREDIT_CRUNCH", 22)
  push_if((ctx.inflation_pct or 4.0) <= 1.25 and (ctx.gdp_growth_pct or 2.0) <= 1.6, "DEFLATIONARY_SLACK", 22)
  push_if((ctx.company_count or 0) >= 1 and (ctx.gdp_growth_pct or 2.0) >= 2.1, "SECTOR_EXPANSION", 20)
  push_if((ctx.commodity_index or 100.0) >= 105.0, "COMMODITY_BOOM", 20)
  push_if((ctx.stability or 60.0) >= 56.0, "REGIONAL_TRADE_SUMMIT", 14)

  if #candidates == 0 then
    return nil
  end

  local total_w = 0
  for _, c in ipairs(candidates) do
    total_w = total_w + c.weight
  end
  local roll = ((ctx.total_days or 1) * 1103515245 + math.floor((ctx.stability or 50.0) * 97)) % total_w
  local acc = 0
  for _, c in ipairs(candidates) do
    acc = acc + c.weight
    if roll < acc then
      return catalog[c.id]
    end
  end
  return catalog[candidates[1].id]
end

return events
