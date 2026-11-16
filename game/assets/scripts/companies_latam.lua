--[[
  Propósito del archivo (Lua 5.4):
  Catálogo de sectores corporativos, productos vendidos, generación de empleo,
  aporte al PIB y parámetros bursátiles para el sistema de fundación aleatoria
  de empresas en Latinoamérica. Utiliza nombres ficticios libres de marcas registradas.
--]]

local companies = {}

companies.categories = {
  {
    id = "TECH",
    name = "Tecnología",
    prefixes = { "Nube", "Sistemas", "Cóndor", "Soluciones", "Red", "Andes" },
    suffixes = { "Digital S.A.", "Tech Latam", "Software Corp.", "Datos S.A.", "Cloud" },
    products = {
      "Plataformas de pagos digitales y nube empresarial",
      "Software logístico y ciberseguridad corporativa",
      "Infraestructura de telecomunicaciones y fibra óptica",
      "Servicios de inteligencia operativa y comercio electrónico"
    },
    base_price_min = 24.0,
    base_price_max = 68.0,
    jobs_min = 3200,
    jobs_max = 18000,
    gdp_per_job_m = 0.042,
    inflation_sensitivity = 0.35,
    rate_sensitivity = 0.45
  },
  {
    id = "ENERGY",
    name = "Energía",
    prefixes = { "Energía", "Litio", "Viento", "Solar", "Hidro", "Altiplano" },
    suffixes = { "del Sur S.A.", "Renovable", "Austral", "Energética", "Recursos S.A." },
    products = {
      "Extracción de litio y ensamblaje de baterías",
      "Generación eléctrica solar y eólica regional",
      "Redes de transmisión eléctrica de alta tensión",
      "Biocombustibles industriales y gas natural licuado"
    },
    base_price_min = 30.0,
    base_price_max = 82.0,
    jobs_min = 4500,
    jobs_max = 22000,
    gdp_per_job_m = 0.055,
    inflation_sensitivity = 0.22,
    rate_sensitivity = 0.30
  },
  {
    id = "AGRO",
    name = "Agroindustria",
    prefixes = { "AgroPampa", "Cosecha", "Valle", "Tierra", "Campos", "Trópico" },
    suffixes = { "Export S.A.", "Alimentos", "Agroindustrial", "del Pacífico", "Granos S.A." },
    products = {
      "Procesamiento de granos, soja y proteína vegetal",
      "Exportación de café de altura, cacao y frutales",
      "Lácteos refrigerados y cadena de frío nacional",
      "Fertilizantes orgánicos y semillas mejoradas"
    },
    base_price_min = 16.0,
    base_price_max = 46.0,
    jobs_min = 8500,
    jobs_max = 36000,
    gdp_per_job_m = 0.024,
    inflation_sensitivity = 0.18,
    rate_sensitivity = 0.20
  },
  {
    id = "MANUF",
    name = "Manufactura",
    prefixes = { "Industrias", "Aceros", "Motores", "Fábrica", "Metalúrgica", "Progreso" },
    suffixes = { "Norte S.A.", "Industrial", "del Plata", "Componentes", "Manufacturas" },
    products = {
      "Autopartes eléctricas y ensamblaje de vehículos",
      "Maquinaria agrícola y equipos de construcción",
      "Acero laminado y materiales estructurales",
      "Electrodomésticos y electrónica de consumo"
    },
    base_price_min = 20.0,
    base_price_max = 58.0,
    jobs_min = 7000,
    jobs_max = 32000,
    gdp_per_job_m = 0.032,
    inflation_sensitivity = 0.30,
    rate_sensitivity = 0.38
  },
  {
    id = "LOGISTICS",
    name = "Logística",
    prefixes = { "Puertos", "Ferro", "Ruta", "Carga", "Navegación", "Corredor" },
    suffixes = { "Continental", "Marítima S.A.", "Express", "Logística", "Interoceánica" },
    products = {
      "Operación portuaria y terminales de contenedores",
      "Transporte ferroviario de carga pesada",
      "Distribución nacional de paquetería y última milla",
      "Almacenes aduaneros y transporte aéreo regional"
    },
    base_price_min = 18.0,
    base_price_max = 50.0,
    jobs_min = 5500,
    jobs_max = 26000,
    gdp_per_job_m = 0.028,
    inflation_sensitivity = 0.26,
    rate_sensitivity = 0.25
  },
  {
    id = "FINANCE",
    name = "Finanzas",
    prefixes = { "Banco", "Crédito", "Capital", "Seguros", "Fideicomiso", "Mercantil" },
    suffixes = { "Austral S.A.", "de Fomento", "Metropolitano", "Financiera", "Holdings" },
    products = {
      "Crédito productivo para pequeñas y medianas industrias",
      "Seguros comerciales, hipotecas y fondos de retiro",
      "Banca digital y cámara de compensación interbancaria",
      "Mercado de bonos corporativos y divisas"
    },
    base_price_min = 26.0,
    base_price_max = 75.0,
    jobs_min = 3800,
    jobs_max = 19000,
    gdp_per_job_m = 0.048,
    inflation_sensitivity = 0.40,
    rate_sensitivity = -0.18
  },
  {
    id = "HEALTH",
    name = "Salud",
    prefixes = { "BioSur", "Laboratorios", "Farma", "Genoma", "Clínica", "Vida" },
    suffixes = { "Médica S.A.", "Biotec", "Farma", "del Caribe", "Salud Corp." },
    products = {
      "Fármacos genéricos y suministros hospitalarios",
      "Reactivos de diagnóstico clínico y vacunas regionales",
      "Equipos médicos y telemedicina hospitalaria",
      "Bioinsumos farmacéuticos y laboratorios de análisis"
    },
    base_price_min = 22.0,
    base_price_max = 64.0,
    jobs_min = 4200,
    jobs_max = 21000,
    gdp_per_job_m = 0.038,
    inflation_sensitivity = 0.20,
    rate_sensitivity = 0.22
  }
}

return companies
