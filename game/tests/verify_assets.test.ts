/**
 * Propósito del archivo:
 * Suite de pruebas automatizadas (node:test) para validar la integridad estructural
 * del mapa SVG de respaldo, los scripts Lua 5.4, el visor 3D (wgpu + gltf),
 * las reglas de .gitignore y los workflows manuales de Blender (Release) y APK Debug (32/64-bit).
 */

import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const EXPECTED_IDS = [
  'MX', 'BZ', 'GT', 'SV', 'HN', 'NI', 'CR', 'PA',
  'CU', 'HT', 'DO', 'CO', 'VE', 'EC', 'PE', 'BR',
  'BO', 'PY', 'UY', 'CL', 'AR',
];

test('latinoamerica.svg contiene las 21 naciones latinoamericanas con rutas válidas', () => {
  const svgPath = path.resolve('game/assets/maps/latinoamerica.svg');
  const svg = fs.readFileSync(svgPath, 'utf8');

  assert.ok(svg.includes('viewBox="0 0 1000 1200"'), 'Falta viewBox esperado en SVG');

  for (const id of EXPECTED_IDS) {
    const idPattern = new RegExp(`<path\\s+id="${id}"[^>]*d="M\\s+[^"]+Z"`, 'm');
    assert.match(svg, idPattern, `Falta o es inválido el polígono SVG del país ${id}`);
  }
});

test('countries_latam.lua define población, moneda, PIB y partidos políticos para los 21 países', () => {
  const luaPath = path.resolve('game/assets/scripts/countries_latam.lua');
  const lua = fs.readFileSync(luaPath, 'utf8');

  for (const id of EXPECTED_IDS) {
    assert.ok(lua.includes(`id = "${id}"`), `Falta país ${id} en countries_latam.lua`);
  }

  assert.ok(lua.includes('population ='), 'Falta campo population');
  assert.ok(lua.includes('currency_code ='), 'Falta campo currency_code');
  assert.ok(lua.includes('gdp_usd_millions ='), 'Falta campo gdp_usd_millions');
  assert.ok(lua.includes('political_parties ='), 'Falta campo political_parties');
});

test('Cargo.toml usa Rust 2024, Lua 5.4 vendored, wgpu 3D y solo licencias permisivas', () => {
  const cargoPath = path.resolve('game/Cargo.toml');
  const cargo = fs.readFileSync(cargoPath, 'utf8');

  assert.ok(cargo.includes('edition = "2024"'));
  assert.ok(cargo.includes('lua54'));
  assert.ok(cargo.includes('vendored'));
  assert.ok(cargo.includes('eframe = { version = "0.32"'));
  assert.ok(cargo.includes('wgpu = "25"'));
  assert.ok(cargo.includes('mlua = { version = "0.11"'));
  assert.ok(cargo.includes('roxmltree = "0.21"'));
  assert.ok(cargo.includes('svgtypes = "0.16"'));
  assert.ok(cargo.includes('rand = { version = "0.10"'));
  assert.ok(cargo.includes('glam ='));
  assert.ok(cargo.includes('bytemuck ='));
  assert.ok(cargo.includes('gltf = "1.4"'));
  assert.ok(cargo.includes('android-activity'));
  assert.equal(/wgpu\s*=\s*\{[^}]*features\s*=\s*\[[^\]]*("gles"|"vulkan")/.test(cargo), false, 'wgpu no requiere features manuales gles/vulkan en Cargo.toml');
  assert.equal(/resvg|usvg|gpl|agpl/i.test(cargo), false, 'No debe incluir crates copyleft');
});

test('Correcciones Rust: AI MilitaryState, U16 GLB colors, Window max_width, Stroke _f32 y sin flags ARM inestables', () => {
  const ai = fs.readFileSync(path.resolve('game/src/ai/mod.rs'), 'utf8');
  const glbLoader = fs.readFileSync(path.resolve('game/src/map/glb_loader.rs'), 'utf8');
  const chrome = fs.readFileSync(path.resolve('game/src/ui/components/chrome.rs'), 'utf8');
  const mapScreen = fs.readFileSync(path.resolve('game/src/ui/screens/map_screen.rs'), 'utf8');
  const cargoCfg = fs.readFileSync(path.resolve('game/.cargo/config.toml'), 'utf8');
  const renderer3d = fs.readFileSync(path.resolve('game/src/map/renderer_3d.rs'), 'utf8');

  assert.equal(ai.includes('economy.defense_budget_pct'), false, 'defense_budget_pct pertenece a MilitaryState');
  assert.ok(ai.includes('country.military.defense_budget_pct'));
  assert.ok(ai.includes('RngExt'), 'rand 0.10 usa RngExt');
  assert.ok(glbLoader.includes('ReadColors::RgbU16') && glbLoader.includes('ReadColors::RgbaU16'));
  assert.equal(chrome.includes('.fixed_width('), false, 'egui::Window no tiene .fixed_width()');
  assert.equal(mapScreen.includes('.fixed_width('), false, 'egui::Window no tiene .fixed_width()');
  assert.equal(renderer3d.includes('depth_slice:'), false, 'wgpu 25 RenderPassColorAttachment no tiene depth_slice');
  assert.equal(/\+neon|\+vfp3/.test(cargoCfg), false, 'No usar -C target-feature=+neon,+vfp3 inestable en Rust estable');

  // Verifica que ningún archivo UI use literales flotantes sin sufijo _f32 en Stroke::new(...)
  const uiFiles = [
    'game/src/ui/theme.rs',
    'game/src/ui/components/chrome.rs',
    'game/src/ui/components/map_canvas.rs',
    'game/src/ui/screens/country_select.rs',
    'game/src/ui/screens/diplomacy_screen.rs',
    'game/src/ui/screens/economy_screen.rs',
    'game/src/ui/screens/main_menu.rs',
    'game/src/ui/screens/map_screen.rs',
    'game/src/ui/screens/military_screen.rs',
    'game/src/ui/screens/politics_screen.rs',
    'game/src/ui/screens/settings_screen.rs',
  ];
  for (const file of uiFiles) {
    const src = fs.readFileSync(path.resolve(file), 'utf8');
    assert.equal(/Stroke::new\(\s*\d+\.\d+\s*,/.test(src), false, `Falta sufijo _f32 en Stroke::new en ${file}`);
  }
});

test('Visor 3D Rust (glb_loader + renderer_3d + map_canvas) está integrado y documentado', () => {
  const glbLoader = fs.readFileSync(path.resolve('game/src/map/glb_loader.rs'), 'utf8');
  const renderer3d = fs.readFileSync(path.resolve('game/src/map/renderer_3d.rs'), 'utf8');
  const canvas = fs.readFileSync(path.resolve('game/src/ui/components/map_canvas.rs'), 'utf8');

  assert.ok(glbLoader.startsWith('//! Propósito del archivo:'), 'glb_loader.rs debe tener encabezado explicativo');
  assert.ok(renderer3d.startsWith('//! Propósito del archivo:'), 'renderer_3d.rs debe tener encabezado explicativo');
  assert.ok(glbLoader.includes('pick_province_at_ray'), 'Falta raycasting 3D por provincia');
  assert.ok(renderer3d.includes('CallbackTrait'), 'Falta integración egui_wgpu::CallbackTrait');
  assert.ok(canvas.includes('new_paint_callback'), 'Falta PaintCallback 3D en map_canvas.rs');
});

test('Configuración Android soporta 32-bit, 64-bit, horizontal, cero Kotlin y selección por ABI', () => {
  const gradle = fs.readFileSync(path.resolve('game/android/app/build.gradle.kts'), 'utf8');
  const manifest = fs.readFileSync(path.resolve('game/android/app/src/main/AndroidManifest.xml'), 'utf8');

  assert.ok(gradle.includes('armeabi-v7a'), 'Falta ABI 32-bit armeabi-v7a');
  assert.ok(gradle.includes('arm64-v8a'), 'Falta ABI 64-bit arm64-v8a');
  assert.ok(gradle.includes('rustAbi'), 'build.gradle.kts debe soportar filtro -PrustAbi');
  assert.ok(gradle.includes('cargo'), 'Gradle debe ejecutar cargo-ndk');
  assert.ok(
    gradle.includes('"--platform"') || gradle.includes('"-P"'),
    'build.gradle.kts debe usar --platform o -P (en cargo-ndk 4.x -p significa --package)',
  );
  assert.equal(gradle.includes('"-p", "24"'), false, 'No usar -p 24 con cargo-ndk 4.x');
  assert.ok(manifest.includes('android:screenOrientation="sensorLandscape"'), 'Debe forzar formato horizontal');
  assert.ok(manifest.includes('android:hasCode="false"'), 'NativeActivity puro sin Kotlin');
});

test('.gitignore excluye latinoamerica_3d.glb y provinces_latam.lua', () => {
  const gitignore = fs.readFileSync(path.resolve('.gitignore'), 'utf8');
  assert.ok(gitignore.includes('maps/latinoamerica_3d.glb'));
  assert.ok(gitignore.includes('scripts/provinces_latam.lua'));
  assert.ok(gitignore.includes('game/assets/maps/latinoamerica_3d.glb'));
  assert.ok(gitignore.includes('game/assets/scripts/provinces_latam.lua'));
});

test('GitHub Action de Blender es manual (workflow_dispatch) y tiene opción condicional de Release', () => {
  const yml = fs.readFileSync(path.resolve('.github/workflows/build_blender_latam_map.yml'), 'utf8');
  assert.ok(yml.includes('workflow_dispatch:'), 'Debe tener workflow_dispatch');
  assert.equal(/^\s*(push|pull_request|schedule):/m.test(yml), false, 'No debe activarse automáticamente con push o PR');
  assert.ok(yml.includes('publish_release:'), 'Falta opción publish_release');
  assert.ok(yml.includes('release_version:'), 'Falta input release_version');
  assert.ok(yml.includes('if: ${{ inputs.publish_release }}'), 'El paso de Release debe ser condicional');
  assert.ok(yml.includes('gh release'), 'Debe publicar en GitHub Release');
});

test('GitHub Action de APK Debug está en /.github/workflows/, es manual, descarga Release, cachea y separa 64-bit y 32-bit', () => {
  assert.equal(
    fs.existsSync(path.resolve('game/.github/workflows/android_apk.yml')),
    false,
    'No debe quedar copia duplicada en game/.github/workflows/',
  );
  const yml = fs.readFileSync(path.resolve('.github/workflows/android_apk.yml'), 'utf8');
  assert.ok(yml.includes('workflow_dispatch:'), 'Debe ser activable manualmente');
  assert.equal(/^\s*(push|pull_request|schedule):/m.test(yml), false, 'Nunca debe activarse con commits/push');
  assert.ok(yml.includes('gh release download'), 'Debe descargar mapa 3D y provincias desde Release');
  assert.ok(yml.includes('actions/cache/restore@v5'), 'Debe restaurar caché');
  assert.ok(yml.includes('actions/cache/save@v5'), 'Debe guardar caché obligatoriamente');
  assert.ok(yml.includes('if: always()'), 'La caché debe guardarse incluso si falla un paso posterior');
  assert.ok(yml.includes('assembleDebug'), 'Debe compilar APK Debug');
  assert.ok(yml.includes('arm64-v8a'), 'Debe incluir build 64 bits');
  assert.ok(yml.includes('armeabi-v7a'), 'Debe incluir build 32 bits');
  assert.ok(yml.includes('HorizonteLatam-debug-${{ matrix.arch_label }}-${{ matrix.abi }}'), 'Debe subir cada arquitectura por separado');
});

test('Existen los 6 archivos de documentación en la raíz del proyecto', () => {
  for (const doc of ['README.md', 'ROADMAP.md', 'STRUCTURE.md', 'AI_CONTEXT.md', 'AGENTS.md', 'README_AI.md']) {
    assert.ok(fs.existsSync(path.resolve(doc)), `Falta ${doc} en la raíz`);
  }
});
