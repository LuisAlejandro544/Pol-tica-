# README_AI — Guía Rápida de Arranque para Asistentes

Si eres un modelo o agente que acaba de abrir este repositorio, lee esto antes de modificar cualquier archivo.

---

## Mapa mental rápido

| Qué | Dónde | Para qué |
| :--- | :--- | :--- |
| Host web AI Studio | `/src/App.tsx` | Debe mostrar **solo `Hello World`** para que el contenedor Vite compile limpio. |
| Motor Rust + UI | `/game/src/` | Juego móvil horizontal en tiempo real en Rust (`egui`/`eframe` + `wgpu` + `gltf`), caché nativa (`cache/mod.rs`), líderes aleatorios (`engine/leaders.rs`), pantalla de carga (`loading_screen.rs`), reclutamiento en mapa, mesa circular política e IA paralela (`rayon`). |
| Scripts Lua 5.4 | `/game/assets/scripts/` | Datos de países, provincias, empresas/sectores (`companies_latam.lua`), reglas económicas (`game_rules.lua`) y eventos (`events_latam.lua`). |
| Pipeline Blender 3D | `/game/tools/blender/` | Convierte GeoJSON Natural Earth a `latinoamerica_3d.glb` + `provinces_latam.lua` usando `bpy` y `lua5.4`. |
| Action Blender manual | `/.github/workflows/build_blender_latam_map.yml` | **Solo manual (`workflow_dispatch`)**. No agregar triggers automáticos. |
| Empaquetado Android | `/game/android/` | APK puro sin Kotlin (`NativeActivity`), listo para Uptodown. |
| Pruebas rápidas | `/game/tests/verify_assets.test.ts` | Ejecutar con `npx --yes tsx --test game/tests/verify_assets.test.ts`. |

---

## Lista de verificación antes de editar
1. **¿El cambio afecta a Rust/Lua o al Action?** Recuerda que el contenedor web de AI Studio no tiene `rustc`, `cargo` ni `blender` instalados: valida sintaxis, dependencias y pruebas en `/game/tests/verify_assets.test.ts`, y valida la web con `npm run build` / `npm run lint`.
2. **¿Vas a agregar un crate a `game/Cargo.toml`?** Verifica que sea `MIT` o `Apache-2.0` y compatible con `egui = "0.32"` y `wgpu = "25"`.
3. **¿Vas a agregar una nueva sección visual?** Crea su propia pantalla bajo `/game/src/ui/screens/` y su botón en `render_bottom_nav` (o un popup si es una vista contextual pequeña).
4. **¿Estás respetando el diseño previo?** No cambies colores, iconos ni lógica estable sin orden directa del usuario.
