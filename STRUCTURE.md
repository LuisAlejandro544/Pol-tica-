# Estructura del Proyecto (`STRUCTURE.md`)

Descripción exacta de carpetas y responsabilidades de cada módulo en el repositorio.

---

## Árbol de directorios

```text
/
├── .github/workflows/
│   ├── android_apk.yml               # GitHub Action SOLO MANUAL para compilar APKs Debug (64-bit y 32-bit) desde Releases
│   └── build_blender_latam_map.yml   # GitHub Action SOLO MANUAL para generar mapa 3D (.glb), provincias (.lua) y Release opcional
├── src/
│   ├── App.tsx                       # Host web mínimo "Hello World" para compilación en entorno AI Studio
│   ├── index.css                     # Estilos base Tailwind del host web
│   └── main.tsx                      # Arranque React
├── game/                             # Proyecto real del juego móvil (Rust + Lua 5.4 + Android)
│   ├── Cargo.toml                    # Dependencias Rust (MIT/Apache-2.0): egui, wgpu, gltf, mlua (Lua 5.4), rayon, postcard, lz4_flex
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
│   │       ├── companies_latam.lua   # Catálogo de sectores, productos, empleos y parámetros bursátiles
│   │       ├── provinces_latam.lua   # (Generado por Action manual) Catálogo de provincias y capitales
│   │       ├── game_rules.lua        # Fórmulas de PIB, inflación, desempleo y costo de mantenimiento militar
│   │       └── events_latam.lua      # Generador de eventos geopolíticos para popups
│   ├── tools/
│   │   └── blender/
│   │       ├── build_latam_map.py    # Script Blender (bpy/bmesh) que convierte GeoJSON a .glb + .lua
│   │       └── sync_lua_metadata.lua # Script Lua 5.4 para exportar países a JSON y validar provincias
│   ├── tests/
│   │   └── verify_assets.test.ts     # Pruebas automatizadas (node:test) de integridad de SVG, Lua, caché, líderes y UI
│   └── src/
│       ├── lib.rs                    # Entrada android_main(AndroidApp) (con cacheDir + PIPELINE_CACHE) y run_desktop()
│       ├── main.rs                   # Binario de escritorio
│       ├── models/mod.rs             # Structs y enums (CountryState, LeaderProfile, ArmyUnit, CompanyEntity, ProvinceNode, GameState)
│       ├── cache/mod.rs              # Caché nativa (mapa 3D/grafo, shaders wgpu, bytecode Lua 5.4, crónica/gráficas/miniaturas)
│       ├── map/
│       │   ├── mod.rs
│       │   ├── glb_loader.rs         # Lector glTF/GLB + raycasting 3D + grafo provincial (`build_province_graph`)
│       │   ├── renderer_3d.rs        # Renderizador 3D `wgpu` + `egui-wgpu` con `PipelineCache` persistente
│       │   └── svg_loader.rs         # Incrustación `assets` (`include_dir!`) y lector SVG
│       ├── lua_bridge/mod.rs         # VM Lua 5.4 (mlua) con carga/guardado de bytecode precompilado
│       ├── ai/mod.rs                 # IA local paralela (rayon) con reclutamiento, rutas entre estados, elecciones y diplomacia
│       ├── engine/
│       │   ├── mod.rs                # Simulación continua en tiempo real (días, tropas, bolsa, empresas y volcado a caché)
│       │   └── leaders.rs            # Generador aleatorio de Presidente y líderes de oposición (nombre, edad, nacionalidad)
│       └── ui/
│           ├── mod.rs                # Router multipantalla (HorizonteLatamApp) y acumulador de tiempo real
│           ├── theme.rs              # Paleta táctica, marcos flotantes con sombra, íconos vectoriales y biseles
│           ├── components/
│           │   ├── chrome.rs         # HUD superior y dock inferior (fijo en gestión, flotante sobre el mapa)
│           │   └── map_canvas.rs     # Lienzo 3D interactivo borde a borde con fichas militares separadas
│           └── screens/
│               ├── main_menu.rs
│               ├── country_select.rs
│               ├── loading_screen.rs # Pantalla de carga que configura unidades iniciales y empresas aleatorias al elegir país
│               ├── map_screen.rs     # Visor 3D 100% pantalla completa + reclutamiento directo en región propia y crónica plegable
│               ├── economy_screen.rs
│               ├── companies_screen.rs # Bolsa regional, empresas dinámicas, productos, empleos y gráficas
│               ├── politics_screen.rs  # Mesa circular del Congreso con sillas pintadas por partido, tarjetas y decretos
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
