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
- **Ministro de Relaciones Exteriores asignable (`Diplomacia`):** El jugador debe elegir y asignar a su Canciller entre candidatos con estadísticas aleatorias (bono a mejora de relaciones y probabilidades de aceptación para **Acuerdo de Paz**, **Paz Blanca**, **Alianza** y **Pacto de No Agresión**). Todas sus misiones se ejecutan **de forma instantánea (0 días)**.
- **Simulación continua en tiempo real:** Calendario diario (`1x`, `2x`, `3x` y pausa), caja diaria en vivo, marcha interpolada de tropas sobre el mapa 3D y cierres macroeconómicos mensuales.
- **Presupuesto fiscal detallado y transparente (`Economía` + HUD):** Muestra de forma explícita los ingresos fiscales brutos (recaudación nacional + impuestos corporativos), el desglose de egresos mensuales (`Defensa`, `Tropas`, `Deuda`, `Infraestructura`, `Social`), el balance neto mensual/diario y la actualización inmediata al ajustar impuestos, gasto o presupuesto militar.
- **Interfaz y motor pesado en Rust:** `egui` / `eframe` + `wgpu 25` + `gltf 1.4` con arquitectura multipantalla táctil (`Carga`, `Mapa`, `Economía`, `Empresas`, `Política`, `Diplomacia`, `Defensa`, `Opciones`).
- **Población, desempleo regional y reclutamiento libre (`Mapa` + `Defensa`):** Cada región tiene su propia población exacta (junto a su `%` del total nacional) y su cantidad exacta de desempleados (`%` de tasa de desempleo y `%` de la población). Tanto en el mapa 3D como en `Defensa` puedes elegir libremente cualquier cantidad de tropas (`DragValue`, deslizador, pasos rápidos o `Máx`) convirtiendo desempleados regionales en soldados, con tope estricto según la población y los desempleados disponibles de esa región.
- **Empresas aleatorias en vivo, ciclo bidireccional y desempleo:** Catálogo Lua (`companies_latam.lua`) con nacimiento dinámico continuo de nuevas compañías, exportaciones que aportan divisas e impuestos corporativos, salud financiera, ampliación de plantilla en bonanza o recortes/quiebras durante recesiones o tasas prohibitivas, impactando en los desempleados regionales.
- **Mesa circular del Congreso y coaliciones (`Política`):** Hemiciclo circular con 50 sillas pintadas por partido, control real de quórum mayoría/minoría, capital político, pactos de coalición con líderes de oposición y deriva ideológica viva de los partidos según inflación, desempleo y presión fiscal.
- **Macroeconomía realista en Lua 5.4 (`game_rules.lua`):** Regla de Taylor frente a tasa neutral, Curva de Phillips (brecha de producto y desempleo NAIRU), *pass-through* cambiario, prima de riesgo soberano, servicio de deuda calibrado, balanza comercial y ciclo global compartido (Fed + commodities).
- **Eventos contextuales por causas reales (`events_latam.lua`):** Disparados por inflación, fuga de capitales, minoría en el Congreso, paros laborales, rebaja de deuda o auges sectoriales (en reemplazo de rotaciones cíclicas fijas), con impactos fiscales escalados al PIB mensual.
- **IA Local paralela en Rust (`rayon` + `crossbeam-channel`):** Evalúa política monetaria estilo Taylor, coaliciones parlamentarias, presupuesto militar dedicado (`ai_military_budget_m`), tope de tropas según paz o guerra, límite fiscal de mantenimiento, reclutamiento sujeto al mismo cupo de población y desempleados regionales que el jugador, enfriamiento de reclutamiento (sin spam de divisiones) y maniobras militares en un hilo secundario sin bloquear la interfaz táctil.
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
