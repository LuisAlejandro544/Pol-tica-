# Contexto Técnico para IA (`AI_CONTEXT.md`)

Este archivo resume el estado exacto, las decisiones arquitectónicas y las restricciones reales del proyecto para cualquier asistente o desarrollador que retome el trabajo.

---

## 1. Propósito del proyecto y entorno
- **Juego:** *Horizonte Latam*, juego móvil de gran estrategia geopolítica en **tiempo real** y **formato horizontal (`sensorLandscape`)**.
- **Stack nativo:** **Rust (edición 2024)** para toda la interfaz gráfica, motor 3D/2D e IA local paralela, combinado con **Lua 5.4** (`mlua` con feature `vendored`) para datos de naciones, provincias, categorías empresariales, reglas económicas y eventos.
- **Host web de compilación:** `/src/App.tsx` es intencionalmente un simple `"Hello World"` en React/Vite para que el entorno preview web compile sin errores mientras todo el juego móvil vive en `/game/`. **No reemplazar el Hello World web por UI del juego a menos que el usuario lo pida.**
- **Entorno del usuario:** Trabaja únicamente desde un **teléfono móvil (sin PC)** y distribuirá el APK fuera de Google Play (por ejemplo en **Uptodown**).

## 2. Estado actual del motor
- **Interfaz actual (`game/src/ui/`):** Usa `eframe` / `egui` (`0.32`) con arquitectura multipantalla (`MainMenu`, `CountrySelect`, `WorldMap`, `Economy`, `Companies`, `Politics`, `Diplomacy`, `Military`, `Settings`) y popups modales (`EventPopup`, `Popup País`).
- **Visor 3D operativo (`wgpu 25` + `egui-wgpu 0.32` + `gltf 1.4`):** `game/src/map/glb_loader.rs` y `game/src/map/renderer_3d.rs` cargan `latinoamerica_3d.glb` junto a `scripts/provinces_latam.lua`, permitiendo pan/zoom/órbita táctil, 4 capas de mapa, raycasting 3D exacto por provincia y país, proyección de unidades militares (`ArmyUnit`) en 3D y selección táctil de marcha.
- **Simulación continua en tiempo real (`engine/mod.rs`):** El bucle ya no es por turnos; avanza día a día según la velocidad (`1x`, `2x`, `3x`) en `SimulationWorker` (`crossbeam-channel`) con flujo diario de tesorería, movimiento interpolado de soldados por el grafo de estados (`ProvinceGraph`), cotización diaria de acciones y cierre macroeconómico mensual en Lua 5.4 + IA paralela (`rayon`).
- **Militar y mantenimiento:** Jugador e IA reclutan unidades en estados concretos pagando costo inicial y sufriendo descuento mensual continuo en ingresos (`troop_upkeep_m` en `game_rules.lua`), además de poder mover regimientos por estados propios, aliados o en guerra.
- **Empresas, bolsa y desempleo (`companies_latam.lua` + `companies_screen.rs`):** De forma aleatoria y ponderada por estabilidad/crecimiento nacen empresas privadas en distintos países (no en todos a la vez) con categoría, producto vendido, empleos creados (reduciendo `unemployment_pct`), aporte al PIB y precio de acción vivo con gráfica histórica.
- **Seguridad 32/64 bits:** Los modelos en `game/src/models/mod.rs` usan `u64`, `i64`, `u32` y `f64` explícitos (nunca `usize` para población o PIB) para evitar desbordamientos en 32 bits (`armeabi-v7a`) mientras ambos targets (`arm64-v8a` y `armeabi-v7a`) se compilan por separado.

## 3. Pipeline Blender + GeoJSON + Lua 5.4 + APK Manual
- **Assets pesados excluidos de Git:** `latinoamerica_3d.glb` y `provinces_latam.lua` están en `.gitignore` para no inflar el repositorio y se distribuyen vía **GitHub Releases**.
- **Script Blender (`game/tools/blender/build_latam_map.py`):** Descarga GeoJSON de Dominio Público (Natural Earth Admin-1 provincias y ciudades), proyecta Latinoamérica sin distorsión latitudinal, extruye cada provincia (`PROV_<ISO2>_<IDX>_<CODE>`) con relieve andino y coloca capitales (`CAP_<ISO2>`, `PCAP_*`), exportando `latinoamerica_3d.glb` y `provinces_latam.lua`.
- **Script Lua (`game/tools/blender/sync_lua_metadata.lua`):** Ejecutado por `lua5.4` antes y después de Blender para leer colores/países de `countries_latam.lua` y validar el `provinces_latam.lua` resultante.
- **GitHub Action Blender (`/.github/workflows/build_blender_latam_map.yml`):** Configurado **estrictamente con `on: workflow_dispatch:`** con opción condicional `publish_release` + `release_version`. Nunca se dispara en `push` ni `pull_request`.
- **GitHub Action APK Debug (`/.github/workflows/android_apk.yml`):** Configurado **estrictamente con `on: workflow_dispatch:`**; descarga `.glb` y `.lua` desde GitHub Releases, restaura y guarda obligatoriamente la caché (`if: always()`) y genera dos artefactos separados: 64 bits (`arm64-v8a`) y 32 bits (`armeabi-v7a`).

## 4. Restricciones críticas
1. **Cero Kotlin / Cero Google Play:** La app Android arranca vía `NativeActivity` (`hasCode="false"`). No añadir código Kotlin ni SDKs de Google Play.
2. **Licencias permisivas únicamente:** No usar ni recomendar librerías/datos GPL, AGPL, LGPL ni MPL (por ejemplo, evitar `resvg`/`usvg`). Solo `MIT`, `Apache-2.0`, `BSD`, `Zlib` o Dominio Público.
3. **Sin marcas protegidas en nombres de archivos o código:** Evitar referencias comerciales con derechos de autor.
4. **Comentarios explicativos:** Todo archivo de código nuevo o editado debe incluir un encabezado explicando su lógica y propósito.
5. **No tocar el diseño visual ni lógica existente sin petición explícita del usuario.**
