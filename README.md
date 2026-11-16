# Horizonte Latam — Juego de Estrategia Geopolítica (Rust + Lua 5.4)

Videojuego móvil de gran estrategia en **formato horizontal (`sensorLandscape`)** ambientado en Latinoamérica, construido íntegramente en **Rust** (interfaz gráfica nativa, renderizado, simulación paralela e IA local) junto con **Lua 5.4** (datos de países, provincias, reglas económicas y eventos) y un pipeline de generación de mapas 3D en **Blender + Python + GeoJSON**.

> **Nota sobre el entorno web:** La raíz Vite/React (`/src/App.tsx`) contiene únicamente un host mínimo `"Hello World"` para que el entorno web compile sin errores mientras el juego nativo reside en `/game/`.

---

## Características principales

- **Cero Kotlin en la app:** Entrada nativa Android mediante `android.app.NativeActivity` (`android-activity`) y `hasCode="false"`.
- **Interfaz y motor pesado en Rust:** `egui` / `eframe` preparando transición gráfica 3D hacia `wgpu` + `egui-wgpu` + `egui-winit` + `gltf`.
- **Scripting en Lua 5.4 (`mlua` vendored):** Catálogo de las 21 naciones latinoamericanas, fórmulas macroeconómicas, inflación, moneda nacional, PIB, partidos políticos y eventos dinámicos.
- **IA Local paralela en Rust (`rayon` + `crossbeam-channel`):** Evalúa a las naciones rivales en un hilo secundario sin bloquear la fluidez de la interfaz táctil.
- **Mapa vectorial SVG + Pipeline 3D Blender (`.glb`):**
  - Mapa base actual en `/game/assets/maps/latinoamerica.svg` con 21 países.
  - Generador 3D en Blender (`/game/tools/blender/build_latam_map.py`) usando GeoJSON de Dominio Público (Natural Earth Admin-1 provincias y capitales), sincronizado con Lua 5.4 (`/game/tools/blender/sync_lua_metadata.lua`).
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
cargo ndk -t arm64-v8a -t armeabi-v7a -p 24 -o android/app/src/main/jniLibs build --release
```

### Compilar APK vía Gradle
```bash
cd game/android
gradle assembleRelease
```
