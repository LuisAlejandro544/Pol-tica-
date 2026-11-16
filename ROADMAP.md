# Hoja de Ruta (Roadmap) — Horizonte Latam

Este documento describe la evolución planificada del proyecto desde el prototipo táctil SVG hasta un juego complejo de gran estrategia en 3D para Android.

---

## Fase 1 — Cimientos del Motor y Mapa Base (Completado)
- [x] Arquitectura modular en Rust (`edition = "2024"`) con `crossbeam-channel` + `rayon` fuera del hilo de UI.
- [x] Integración de Lua 5.4 embebido (`mlua` con feature `vendored`).
- [x] Mapa vectorial `latinoamerica.svg` de 21 naciones con teselación GPU (`lyon_tessellation`) y detección táctil.
- [x] Datos iniciales de Población, Moneda y Tipo de Cambio, PIB y Partidos Políticos por nación.
- [x] IA local en Rust con personalidades estratégicas y elecciones parlamentarias periódicas.
- [x] Interfaz horizontal multipantalla (`MainMenu`, `CountrySelect`, `WorldMap`, `Economy`, `Politics`, `Diplomacy`, `Military`, `Settings`) + popups modales.
- [x] Empaquetado Android puro (`NativeActivity`, sin Kotlin, sin Google Play).

## Fase 2 — Pipeline Cartográfico 3D en Blender + Preparación `wgpu` (En curso)
- [x] Script Python para Blender (`build_latam_map.py`) que convierte GeoJSON de Dominio Público (Natural Earth Admin-1) en mallas 3D de provincias y capitales (`latinoamerica_3d.glb`).
- [x] Sincronización y validación con Lua 5.4 (`sync_lua_metadata.lua` → `provinces_latam.lua`).
- [x] GitHub Action estrictamente manual (`workflow_dispatch`) para generar el `.glb` en la nube sin necesitar PC.
- [x] Dependencias base 3D agregadas en `Cargo.toml` (`wgpu`, `egui-wgpu`, `egui-winit`, `gltf`).

## Fase 3 — Renderizador 3D Nativo (`wgpu` + `gltf`) y Nivel Provincial
- [ ] Reemplazar el backend `eframe/glow` por un ciclo propio `winit` + `wgpu` + `egui-wgpu` sobre Vulkan en Android.
- [ ] Cargar `latinoamerica_3d.glb` con `gltf`, renderizar relieve andino, iluminación direccional, fronteras y pines de capitales.
- [ ] Implementar selección táctil 3D (*raycasting* con `glam` / `parry3d`) sobre las mallas `PROV_*`.
- [ ] Expandir el modelo de datos en Rust/Lua para gestionar economía, población e infraestructura a nivel de **provincia**.
- [ ] Evaluar migración de arquitectura principal exclusiva a **64 bits (`arm64-v8a`)** conforme crezca la complejidad de mallas 3D y memoria.

## Fase 4 — Profundidad Estratégica y Militar
- [ ] Grafo de adyacencia provincial (`petgraph` / `pathfinding`) para movimiento de ejércitos, logística y rutas comerciales.
- [ ] Sistema profundo de coaliciones parlamentarias, mociones de censura, proyectos de ley y bancos centrales.
- [ ] Guardado binario comprimido (`postcard` + `lz4_flex`) para campañas largas de varias décadas.
- [ ] Publicación de APK estable en tiendas de terceros (Uptodown).
