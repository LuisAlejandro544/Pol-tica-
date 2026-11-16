# Reglas para Agentes e Inteligencia Artificial (`AGENTS.md`)

Instrucciones operativas obligatorias para cualquier agente de codificación que trabaje en este repositorio.

---

## Reglas de arquitectura y lenguaje
1. **Rust para UI y lógica pesada + Lua 5.4 para datos y reglas:**
   - No introducir Kotlin ni Java para lógica o UI móvil.
   - No implementar fallbacks en Kotlin cuando algo corresponda a Rust o Lua.
   - Usar siempre librerías reales y completas (`mlua`, `wgpu`, `egui-wgpu`, `gltf`, `rayon`, `crossbeam-channel`, etc.).
2. **Licencias permitidas:**
   - Solo se permiten dependencias y fuentes de datos con licencias permisivas (`MIT`, `Apache-2.0`, `BSD`, `Zlib`, `CC0` / Dominio Público) que **no** obliguen a abrir código fuente ni exigir créditos intrusivos.
   - **Prohibido:** `GPL`, `AGPL`, `LGPL`, `MPL-2.0`.
3. **Uso de hilos y concurrencia:**
   - Nunca ejecutar turnos de IA, cálculos masivos ni evaluaciones Lua pesados en el hilo principal de renderizado UI. Canalizar todo por `SimulationWorker` (`crossbeam-channel`) y paralelismo `rayon`.
4. **Empaquetado Android (`gradlew` / `build.gradle.kts`):**
   - La compilación Rust (`cargo ndk`) debe permanecer integrada en Gradle; jamás saltársela.
   - Mantener `android:screenOrientation="sensorLandscape"` y configuración para distribución APK directa (sin Google Play).
   - No usar `persist.sys.*` bajo ninguna circunstancia.

## Reglas de diseño UI y código
5. **Diseño multipantalla no minimalista:**
   - Mantener pantallas separadas con botones claros de navegación; usar popups para detalles secundarios o eventos.
   - No amontonar toda la interfaz en una sola vista.
   - No alterar iconos, paleta ni diseño existente a menos que el usuario lo pida explícitamente.
6. **Modularidad y documentación en código:**
   - Todo archivo debe empezar con un comentario explicando para qué sirve su lógica.
   - No modificar lógica funcional que ya funciona si no es estrictamente necesario.
7. **GitHub Actions de Blender:**
   - `/.github/workflows/build_blender_latam_map.yml` debe conservar **únicamente** el disparador manual `workflow_dispatch`. Nunca agregar `push` o `pull_request` a ese flujo.
8. **Host web (`/src/`):**
   - Conservar `/src/App.tsx` como `"Hello World"` mientras el usuario no pida lo contrario.
   - Si existe `commit_message.txt`, mantener su texto en español y no actualizarlo salvo petición expresa.
