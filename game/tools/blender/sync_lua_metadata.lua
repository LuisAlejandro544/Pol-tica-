--[[
  Propósito del archivo (Lua 5.4):
  Puente ejecutado dentro del Action manual de GitHub para sincronizar datos entre
  el catálogo de países en Lua (`countries_latam.lua`), el script de Blender en Python
  y el catálogo generado de provincias (`provinces_latam.lua`).

  Modos de uso desde CLI:
    lua5.4 sync_lua_metadata.lua export-countries <countries_latam.lua> <out_countries.json>
    lua5.4 sync_lua_metadata.lua validate-provinces <provinces_latam.lua>
--]]

local mode = arg[1]

local function json_escape(str)
  str = tostring(str or "")
  str = str:gsub("\\", "\\\\")
           :gsub('"', '\\"')
           :gsub("\n", "\\n")
           :gsub("\r", "\\r")
           :gsub("\t", "\\t")
  return '"' .. str .. '"'
end

if mode == "export-countries" then
  local countries_path = assert(arg[2], "Falta ruta a countries_latam.lua")
  local out_json_path = assert(arg[3], "Falta ruta de salida JSON")

  local countries = assert(dofile(countries_path), "No se pudo cargar countries_latam.lua")
  local parts = { "{\n" }

  for i, c in ipairs(countries) do
    local col = c.color or { 100, 140, 180 }
    local comma = (i < #countries) and "," or ""
    local line = string.format(
      '  %s: {"name": %s, "capital": %s, "region": %s, "color": [%d, %d, %d]}%s\n',
      json_escape(c.id),
      json_escape(c.name),
      json_escape(c.capital),
      json_escape(c.region),
      col[1], col[2], col[3],
      comma
    )
    table.insert(parts, line)
  end
  table.insert(parts, "}\n")

  local f = assert(io.open(out_json_path, "w"), "No se pudo abrir salida JSON")
  f:write(table.concat(parts))
  f:close()
  print(string.format("[Lua 5.4] Exportados %d países a %s", #countries, out_json_path))

elseif mode == "validate-provinces" then
  local provinces_path = assert(arg[2], "Falta ruta a provinces_latam.lua")
  local data = assert(dofile(provinces_path), "Error de sintaxis en provinces_latam.lua")

  assert(type(data.provinces) == "table", "Falta tabla data.provinces")
  assert(type(data.capitals) == "table", "Falta tabla data.capitals")
  assert(#data.provinces > 0, "data.provinces está vacía")

  local seen_mesh = {}
  local by_country = {}
  for idx, prov in ipairs(data.provinces) do
    assert(type(prov.id) == "string" and #prov.id > 0, "Provincia sin id en índice " .. idx)
    assert(type(prov.mesh_name) == "string" and #prov.mesh_name > 0, "Provincia sin mesh_name: " .. prov.id)
    assert(type(prov.country_id) == "string" and #prov.country_id == 2, "country_id inválido: " .. prov.id)
    assert(type(prov.name) == "string" and #prov.name > 0, "Provincia sin nombre: " .. prov.id)
    assert(type(prov.terrain_type) == "string" and #prov.terrain_type > 0, "Provincia sin terrain_type: " .. prov.id)
    assert(not seen_mesh[prov.mesh_name], "mesh_name duplicado: " .. prov.mesh_name)
    seen_mesh[prov.mesh_name] = true
    by_country[prov.country_id] = (by_country[prov.country_id] or 0) + 1
  end

  local country_count = 0
  for cid, cnt in pairs(by_country) do
    country_count = country_count + 1
    print(string.format("  - %s: %d provincias", cid, cnt))
  end

  print(string.format(
    "[Lua 5.4] Validación exitosa: %d provincias en %d países y %d capitales/ciudades.",
    #data.provinces, country_count, #data.capitals
  ))
else
  io.stderr:write("Uso: lua5.4 sync_lua_metadata.lua <export-countries|validate-provinces> ...\n")
  os.exit(1)
end
