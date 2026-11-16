# Horizonte Latam — Juego de Estrategia Geopolítica (Rust + Lua 5.4)

Videojuego móvil de gran estrategia en **formato horizontal (`sensorLandscape`)** ambientado en Latinoamérica, construido íntegramente en **Rust** (interfaz gráfica nativa, renderizado, simulación paralela e IA local) junto con **Lua 5.4** (datos de países, provincias, reglas económicas y eventos) y un pipeline de generación de mapas 3D en **Blender + Python + GeoJSON**.

> **Nota sobre el entorno web:** La raíz Vite/React (`/src/App.tsx`) contiene únicamente un host mínimo `"Hello World"` para que el entorno web compile sin errores mientras el juego nativo reside en `/game/`.

---

## Características principales

- **Cero Kotlin en la app:** Entrada nativa Android mediante `android.app.NativeActivity` (`android-activity`) y `hasCode="false"`.
- **Caché nativa modular en Rust (`postcard` + `lz4_flex`):** Utiliza el directorio nativo `cache/` de Android para acelerar arranques y ahorrar RAM (32 y 64 bits) mediante 4 subsistemas:
  1. **Pre-cocinado binario del Mapa 3D y `ProvinceGraph`:** Evita re-parsear `latinoamerica_3d.glb`, `provinces_latam.lua` y la malla de adyacencia en cada inicio.
  2. **Caché de pipelines y shaders `wgpu`:** Persiste `wgpu::PipelineCache` en Vulkan/GLES según la clave del adaptador GPU móvil.
  3. **Bytecode precompilado Lua 5.4:** Compila y reutiliza bytecode binario (`ChunkMode::Binary`) para `provinces_latam.lua`, `countries_latam.lua` y `game_rules.lua`.
  4. **Buffer temporal de Crónica, gráficas históricas y miniaturas:** Archiva entradas antiguas de Crónica, muestras viejas de acciones bursátiles, instantáneas macroeconómicas y miniaturas ligeras de campaña.
- **Pantalla de carga y semilla dinámica de campaña:** Al elegir un país, una pantalla dedicada (`CampaignLoading`) prepara en segundo plano los líderes nacionales aleatorios, los regimientos/soldados predeterminados y funda empresas aleatorias en un subconjunto variable de países (nunca fijas ni en todos a la vez).
- **Líderes políticos aleatorios (Jugador e IA):** Genera dinámicamente el nombre completo, la edad y la nacionalidad tanto de tu Presidente y líderes de oposición como de los 20 países rivales controlados por la IA, integrados con la mesa circular del Congreso y las elecciones.
- **Simulación continua en tiempo real:** Calendario diario (`1x`, `2x`, `3x` y pausa), caja diaria en vivo, marcha interpolada de tropas sobre el mapa 3D y cierres macroeconómicos mensuales.
- **Interfaz y motor pesado en Rust:** `egui` / `eframe` + `wgpu 25` + `gltf 1.4` con arquitectura multipantalla táctil (`Carga`, `Mapa`, `Economía`, `Empresas`, `Política`, `Diplomacia`, `Defensa`, `Opciones`).
- **Reclutamiento directo por región y movimiento militar:** Al tocar cualquier región propia en el mapa 3D puedes reclutar soldados directamente allí mismo (con costo inicial y mantenimiento mensual continuo), además de gestionarlo en `Defensa` y mover regimientos por el grafo de provincias.
- **Empresas aleatorias en vivo, bolsa y desempleo:** Catálogo Lua (`companies_latam.lua`) con nacimiento dinámico continuo de nuevas compañías (semilla única por campaña), sectores, productos vendidos, precios de acción en vivo con gráfica histórica, creación de empleo y aporte al PIB.
- **Mesa circular del Congreso (`Política`):** Hemiciclo circular con 50 sillas pintadas proporcionalmente según los escaños de cada partido político, preparando el terreno para votaciones de leyes y coaliciones.
- **Scripting en Lua 5.4 (`mlua` vendored):** Catálogo de las 21 naciones latinoamericanas, fórmulas macroeconómicas, inflación, moneda nacional, desempleo, mantenimiento militar, partidos políticos y eventos dinámicos.
- **IA Local paralela en Rust (`rayon` + `crossbeam-channel`):** Evalúa a las naciones rivales en un hilo secundario sin bloquear la fluidez de la interfaz táctil.
- **Mapa 3D Blender (`.glb`) y Pipeline GeoJSON:** Generador 3D en Blender (`/game/tools/blender/build_latam_map.py`) usando GeoJSON de Dominio Público (Natural Earth Admin-1 provincias y capitales), sincronizado con Lua 5.4 (`/game/tools/blender/sync_lua_metadata.lua`).
- **Distribución directa APK (sin Google Play):** Configurado para generar APK instalable para tiendas de terceros como Uptodown.

---

## Cómo generar el mapa 3D de Blender (desde el teléfono)

El flujo `/.github/workflows/build_blender_latam_map.yml` se ejecuta **únicamente de forma manual (`workflow_dispatch`)** para no consumir recursos en commits o pull requests:

1. Entra en tu repositorio de GitHub desde el navegador del teléfono.
2. Ve a la pestaña **Actions** → **Generar Mapa 3D Blender (Manual)**.
3. Pulsa **Run workflow**, ajusta `simplify_tolerance` (por defecto `0.05`) y marca `commit_assets` si deseas guardar `latinoamerica_3d.glb` y `provinces_latam.lua` directamente en el repositorio.

---

## Compilación de pruebas y entorno

### Validar assets y reglas (Node / TypeScript)
```bash
npx --yes tsx --test game/tests/verify_assets.test.ts
```

### Compilar el motor Rust (requiere Rust 1.85+ y Android NDK)
```bash
cd game
cargo check
cargo ndk -t arm64-v8a -t armeabi-v7a --platform 24 -o android/app/src/main/jniLibs build --release
```

### Compilar APK vía Gradle
```bash
cd game/android
gradle assembleRelease
```
