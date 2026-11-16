# Contexto Técnico para IA (`AI_CONTEXT.md`)

Este archivo resume el estado exacto, las decisiones arquitectónicas y las restricciones reales del proyecto para cualquier asistente o desarrollador que retome el trabajo.

---

## 1. Propósito del proyecto y entorno
- **Juego:** *Horizonte Latam*, juego móvil de gran estrategia geopolítica en **formato horizontal (`sensorLandscape`)**.
- **Stack nativo:** **Rust (edición 2024)** para toda la interfaz gráfica, motor 3D/2D e IA local paralela, combinado con **Lua 5.4** (`mlua` con feature `vendored`) para datos de naciones, provincias, reglas económicas y eventos.
- **Host web de compilación:** `/src/App.tsx` es intencionalmente un simple `"Hello World"` en React/Vite para que el entorno preview web compile sin errores mientras todo el juego móvil vive en `/game/`. **No reemplazar el Hello World web por UI del juego a menos que el usuario lo pida.**
- **Entorno del usuario:** Trabaja únicamente desde un **teléfono móvil (sin PC)** y distribuirá el APK fuera de Google Play (por ejemplo en **Uptodown**).

## 2. Estado actual del motor
- **Interfaz actual (`game/src/ui/`):** Usa `eframe` / `egui` (`0.31`) con arquitectura multipantalla (`MainMenu`, `CountrySelect`, `WorldMap`, `Economy`, `Politics`, `Diplomacy`, `Military`, `Settings`) y popups modales (`EventPopup`, `Popup País`).
- **Preparación 3D (`game/Cargo.toml`):** Ya están declaradas `wgpu = "24"`, `egui-wgpu = "0.31"`, `egui-winit = "0.31"` y `gltf = "1.4"` para migrar del mapa SVG 2D (`latinoamerica.svg`) al mapa 3D generado en Blender (`latinoamerica_3d.glb`).
- **Hilos y concurrencia:** La simulación mensual (fórmulas Lua 5.4 + IA local paralela con `rayon`) corre en un hilo separado (`SimulationWorker` en `game/src/engine/mod.rs`) usando `crossbeam-channel`, manteniendo el hilo de interfaz libre a 60 FPS.
- **Seguridad 32/64 bits:** Los modelos en `game/src/models/mod.rs` usan `u64`, `i64` y `f64` explícitos (nunca `usize` para población o PIB) para evitar desbordamientos en 32 bits (`armeabi-v7a`) mientras se evalúa pasar a 64 bits (`arm64-v8a`) como target principal por complejidad 3D/RAM.

## 3. Pipeline Blender + GeoJSON + Lua 5.4
- **Script Blender (`game/tools/blender/build_latam_map.py`):** Descarga GeoJSON de Dominio Público (Natural Earth Admin-1 provincias y ciudades), proyecta Latinoamérica sin distorsión latitudinal, extruye cada provincia (`PROV_<ISO2>_<IDX>_<CODE>`) con relieve andino y coloca capitales (`CAP_<ISO2>`, `PCAP_*`), exportando `latinoamerica_3d.glb` y `provinces_latam.lua`.
- **Script Lua (`game/tools/blender/sync_lua_metadata.lua`):** Ejecutado por `lua5.4` antes y después de Blender para leer colores/países de `countries_latam.lua` y validar el `provinces_latam.lua` resultante.
- **GitHub Action (`/.github/workflows/build_blender_latam_map.yml`):** Configurado **estrictamente con `on: workflow_dispatch:`**. Nunca debe dispararse en `push` ni `pull_request`.

## 4. Restricciones críticas
1. **Cero Kotlin / Cero Google Play:** La app Android arranca vía `NativeActivity` (`hasCode="false"`). No añadir código Kotlin ni SDKs de Google Play.
2. **Licencias permisivas únicamente:** No usar ni recomendar librerías/datos GPL, AGPL, LGPL ni MPL (por ejemplo, evitar `resvg`/`usvg`). Solo `MIT`, `Apache-2.0`, `BSD`, `Zlib` o Dominio Público.
3. **Sin marcas protegidas en nombres de archivos o código:** Evitar referencias comerciales con derechos de autor.
4. **Comentarios explicativos:** Todo archivo de código nuevo o editado debe incluir un encabezado explicando su lógica y propósito.
5. **No tocar el diseño visual ni lógica existente sin petición explícita del usuario.**
