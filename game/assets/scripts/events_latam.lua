--[[
  Propósito del archivo (Lua 5.4):
  Generador de eventos geopolíticos, económicos y parlamentarios para Latinoamérica.
  Devuelve eventos dinámicos con 2 o 3 decisiones estratégicas para mostrar en un
  popup modal dentro del mapa estratégico o al cambiar de turno.
--]]

local events = {}

local catalog = {
  {
    id = "COMMODITY_BOOM",
    title = "Auge Regional de Materias Primas",
    description = "La demanda mundial de litio, cobre, soja y energía impulsa las exportaciones latinoamericanas.",
    options = {
      {
        label = "Invertir excedentes en infraestructura",
        treasury_delta_pct = -0.02,
        gdp_growth_delta = 0.65,
        inflation_delta = 0.1,
        stability_delta = 2.5
      },
      {
        label = "Reducir deuda pública y fortalecer reservas",
        treasury_delta_pct = 0.05,
        gdp_growth_delta = 0.2,
        inflation_delta = -0.4,
        stability_delta = 1.0
      }
    }
  },
  {
    id = "CURRENCY_PRESSURE",
    title = "Presión Cambiaria en Mercados Emergentes",
    description = "Los capitales externos muestran volatilidad y presionan la cotización de la moneda nacional frente al dólar.",
    options = {
      {
        label = "Subir tasas de interés del Banco Central",
        treasury_delta_pct = -0.01,
        gdp_growth_delta = -0.35,
        inflation_delta = -1.2,
        stability_delta = -1.0
      },
      {
        label = "Mantener estímulo productivo y tolerar volatilidad",
        treasury_delta_pct = 0.0,
        gdp_growth_delta = 0.35,
        inflation_delta = 1.4,
        stability_delta = 0.5
      }
    }
  },
  {
    id = "PARLIAMENT_REFORM",
    title = "Debate de Reforma Fiscal en el Congreso",
    description = "Los partidos de oposición exigen negociar partidas presupuestarias para aprobar el presupuesto anual.",
    options = {
      {
        label = "Negociar pacto multipartidista moderado",
        treasury_delta_pct = -0.015,
        gdp_growth_delta = 0.25,
        inflation_delta = 0.0,
        stability_delta = 4.5
      },
      {
        label = "Imponer decreto presidencial de austeridad",
        treasury_delta_pct = 0.03,
        gdp_growth_delta = -0.15,
        inflation_delta = -0.3,
        stability_delta = -5.0
      }
    }
  },
  {
    id = "REGIONAL_TRADE_SUMMIT",
    title = "Cumbre de Integración Latinoamericana",
    description = "Vecinos estratégicos proponen reducir aranceles aduaneros y crear corredores logísticos compartidos.",
    options = {
      {
        label = "Firmar tratado de libre comercio regional",
        treasury_delta_pct = 0.02,
        gdp_growth_delta = 0.55,
        inflation_delta = -0.25,
        stability_delta = 2.0
      },
      {
        label = "Proteger industria nacional con aranceles",
        treasury_delta_pct = 0.01,
        gdp_growth_delta = -0.1,
        inflation_delta = 0.45,
        stability_delta = 1.5
      }
    }
  }
}

function events.pick_event(turn_number, roll_index)
  local idx = ((turn_number + roll_index) % #catalog) + 1
  return catalog[idx]
end

return events
