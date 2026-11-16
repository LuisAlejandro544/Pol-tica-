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

## Fase 3 — Renderizador 3D Nativo (`wgpu` + `gltf`) y Nivel Provincial (En curso)
- [x] Reemplazar el backend `eframe/glow` por `eframe` con `wgpu` (`egui-wgpu`) sobre Vulkan/GLES en Android.
- [x] Cargar `latinoamerica_3d.glb` con `gltf`, renderizar relieve andino, iluminación direccional, fronteras, ríos y capitales.
- [x] Implementar selección táctil 3D (*raycasting* Möller-Trumbore con `glam`) sobre las mallas `PROV_*`.
- [x] Migrar el bucle de turnos a **simulación continua en tiempo real** (`1x` / `2x` / `3x` / pausa) con flujo diario de caja.
- [x] Construir grafo de adyacencia provincial (`ProvinceGraph`) para reclutar y mover unidades militares (`ArmyUnit`) por los estados en tiempo real, con impacto directo de mantenimiento en ingresos, y reclutamiento directo desde el popup de la región seleccionada en el mapa.
- [x] Implementar pantalla de carga (`CampaignLoading`) tras elegir país para configurar unidades/soldados predeterminados y fundar empresas privadas aleatorias (sin lista fija estática ni en todos los países), más generación continua de nuevas empresas durante la partida.
- [x] Incorporar mesa circular del Congreso en `Política` con sillas pintadas según los escaños de cada partido y generación aleatoria de Presidente y líderes de oposición (nombre, edad y nacionalidad) para jugador e IA.
- [x] Configurar caché nativa (`src/cache/mod.rs` con `postcard` + `lz4_flex`) para pre-cocinado binario del mapa 3D y `ProvinceGraph`, `wgpu::PipelineCache`, bytecode precompilado de Lua 5.4 (`provinces_latam`, `countries_latam`, `game_rules`) y buffer temporal de Crónica, gráficas históricas y miniaturas.
- [ ] Expandir el modelo de datos en Rust/Lua para gestionar infraestructura e impuestos específicos a nivel de **provincia**.
- [ ] Evaluar migración de arquitectura principal exclusiva a **64 bits (`arm64-v8a`)** conforme crezca la complejidad de mallas 3D y memoria.

## Fase 4 — Profundidad Estratégica y Militar
- [ ] Combate táctico automático entre divisiones enemigas al coincidir en provincias disputadas, logística y suministro.
- [ ] Árbol de Enfoque nacional/regional en Lua 5.4 con pantalla dedicada y efectos progresivos.
- [ ] Sistema profundo de coaliciones parlamentarias, mociones de censura, proyectos de ley votados sobre la mesa circular, reemplazo generacional de líderes y bancos centrales.
- [ ] Guardado de partidas completo (`savegame`) aprovechando `postcard` + `lz4_flex` para campañas largas de varias décadas.
- [ ] Publicación de APK estable en tiendas de terceros (Uptodown).
