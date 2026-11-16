# Estructura del Proyecto (`STRUCTURE.md`)

Descripción exacta de carpetas y responsabilidades de cada módulo en el repositorio.

---

## Árbol de directorios

```text
/
├── .github/workflows/
│   └── build_blender_latam_map.yml   # GitHub Action SOLO MANUAL para generar mapa 3D (.glb) y provincias (.lua)
├── src/
│   ├── App.tsx                       # Host web mínimo "Hello World" para compilación en entorno AI Studio
│   ├── index.css                     # Estilos base Tailwind del host web
│   └── main.tsx                      # Arranque React
├── game/                             # Proyecto real del juego móvil (Rust + Lua 5.4 + Android)
│   ├── Cargo.toml                    # Dependencias Rust (MIT/Apache-2.0): egui, wgpu, gltf, mlua (Lua 5.4), rayon
│   ├── rust-toolchain.toml           # Toolchain Rust estable + targets Android (arm64-v8a, armeabi-v7a, x86_64)
│   ├── .cargo/config.toml            # Flags de enlazado NDK (16KB page size en 64-bit, NEON en 32-bit)
│   ├── android/                      # Empaquetado APK puro sin Kotlin (NativeActivity)
│   │   ├── settings.gradle.kts
│   │   └── app/
│   │       ├── build.gradle.kts      # Ejecuta `cargo ndk` antes de empaquetar jniLibs
│   │       └── src/main/
│   │           └── AndroidManifest.xml # Fuerza sensorLandscape y hasCode="false"
│   ├── assets/
│   │   ├── maps/
│   │   │   ├── latinoamerica.svg     # Mapa vectorial 2D de 21 naciones
│   │   │   └── latinoamerica_3d.glb  # (Generado por Action manual) Mapa 3D Blender con provincias y capitales
│   │   └── scripts/
│   │       ├── countries_latam.lua   # Catálogo de 21 países (población, moneda, PIB, partidos, vecinos)
│   │       ├── provinces_latam.lua   # (Generado por Action manual) Catálogo de provincias y capitales
│   │       ├── game_rules.lua        # Fórmulas mensuales de PIB, inflación, moneda y aprobación
│   │       └── events_latam.lua      # Generador de eventos geopolíticos para popups
│   ├── tools/
│   │   └── blender/
│   │       ├── build_latam_map.py    # Script Blender (bpy/bmesh) que convierte GeoJSON a .glb + .lua
│   │       └── sync_lua_metadata.lua # Script Lua 5.4 para exportar países a JSON y validar provincias
│   ├── tests/
│   │   └── verify_assets.test.ts     # Pruebas automatizadas (node:test) de integridad de SVG, Lua y Cargo.toml
│   └── src/
│       ├── lib.rs                    # Entrada android_main(AndroidApp) y run_desktop()
│       ├── main.rs                   # Binario de escritorio
│       ├── models/mod.rs             # Structs y enums (CountryState, EconomyState, PoliticalParty, GameState)
│       ├── map/
│       │   ├── mod.rs
│       │   └── svg_loader.rs         # Lector SVG + teselación lyon + hit-test táctil
│       ├── lua_bridge/mod.rs         # VM Lua 5.4 (mlua) para datos, turnos y eventos
│       ├── ai/mod.rs                 # IA local paralela en Rust (rayon) con personalidades y elecciones
│       ├── engine/mod.rs             # Hilo secundario SimulationWorker (crossbeam-channel)
│       └── ui/
│           ├── mod.rs                # Router multipantalla (HorizonteLatamApp) y bucle de ticks
│           ├── theme.rs              # Paleta táctica, espaciado táctil y formateadores
│           ├── components/
│           │   ├── chrome.rs         # HUD superior, navegación inferior y popup de eventos
│           │   └── map_canvas.rs     # Lienzo interactivo pan/zoom del mapa SVG
│           └── screens/
│               ├── main_menu.rs
│               ├── country_select.rs
│               ├── map_screen.rs
│               ├── economy_screen.rs
│               ├── politics_screen.rs
│               ├── diplomacy_screen.rs
│               ├── military_screen.rs
│               └── settings_screen.rs
├── README.md
├── ROADMAP.md
├── STRUCTURE.md
├── AI_CONTEXT.md
├── AGENTS.md
└── README_AI.md
```
