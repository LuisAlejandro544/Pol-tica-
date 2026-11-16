/**
 * Propósito del archivo:
 * Suite de pruebas automatizadas (node:test) para validar la integridad estructural
 * del mapa SVG de Latinoamérica, los scripts Lua 5.4 de países y reglas,
 * el manifiesto Rust Cargo.toml y la configuración Android 32/64-bit sin Kotlin.
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

test('Cargo.toml usa Rust 2024, Lua 5.4 vendored y solo licencias permisivas', () => {
  const cargoPath = path.resolve('game/Cargo.toml');
  const cargo = fs.readFileSync(cargoPath, 'utf8');

  assert.ok(cargo.includes('edition = "2024"'));
  assert.ok(cargo.includes('lua54'));
  assert.ok(cargo.includes('vendored'));
  assert.ok(cargo.includes('eframe'));
  assert.ok(cargo.includes('android-activity'));
  assert.equal(/resvg|usvg|gpl|agpl/i.test(cargo), false, 'No debe incluir crates copyleft');
});

test('Configuración Android soporta 32-bit, 64-bit, horizontal y cero Kotlin', () => {
  const gradle = fs.readFileSync(path.resolve('game/android/app/build.gradle.kts'), 'utf8');
  const manifest = fs.readFileSync(path.resolve('game/android/app/src/main/AndroidManifest.xml'), 'utf8');

  assert.ok(gradle.includes('armeabi-v7a'), 'Falta ABI 32-bit armeabi-v7a');
  assert.ok(gradle.includes('arm64-v8a'), 'Falta ABI 64-bit arm64-v8a');
  assert.ok(gradle.includes('cargo'), 'Gradle debe ejecutar cargo-ndk');
  assert.ok(manifest.includes('android:screenOrientation="sensorLandscape"'), 'Debe forzar formato horizontal');
  assert.ok(manifest.includes('android:hasCode="false"'), 'NativeActivity puro sin Kotlin');
});

test('Cargo.toml incluye preparación 3D (wgpu, egui-wgpu, egui-winit, gltf)', () => {
  const cargo = fs.readFileSync(path.resolve('game/Cargo.toml'), 'utf8');
  assert.ok(cargo.includes('wgpu = "24"'), 'Falta wgpu');
  assert.ok(cargo.includes('egui-wgpu = "0.31"'), 'Falta egui-wgpu');
  assert.ok(cargo.includes('egui-winit ='), 'Falta egui-winit');
  assert.ok(cargo.includes('gltf = "1.4"'), 'Falta gltf');
});

test('GitHub Action de Blender es estrictamente manual (solo workflow_dispatch) y usa Lua 5.4', () => {
  const yml = fs.readFileSync(path.resolve('.github/workflows/build_blender_latam_map.yml'), 'utf8');
  assert.ok(yml.includes('workflow_dispatch:'), 'Debe tener workflow_dispatch');
  assert.equal(/^\s*(push|pull_request|schedule):/m.test(yml), false, 'No debe activarse automáticamente con push o PR');
  assert.ok(yml.includes('lua5.4'), 'Debe ejecutar Lua 5.4 en el Action');
  assert.ok(yml.includes('--python-exit-code 1'), 'Blender debe propagar errores de Python');
  assert.ok(yml.includes('build_latam_map.py'), 'Debe ejecutar el script Blender Python');

  const py = fs.readFileSync(path.resolve('game/tools/blender/build_latam_map.py'), 'utf8');
  assert.ok(py.includes('Matrix.Translation'), 'Debe usar mathutils.Matrix en create_cone');
  assert.ok(py.includes('ne_10m_admin_1_states_provinces.geojson'), 'Debe usar GeoJSON Admin-1 de provincias');
  assert.ok(py.includes('ne_10m_rivers_lake_centerlines.geojson'), 'Debe incluir ríos principales');
  assert.ok(py.includes('srgb_channel_to_linear'), 'Debe convertir colores sRGB a Linear');
  assert.ok(py.includes('BORDERS_INTERNATIONAL'), 'Debe generar fronteras internacionales');
  assert.ok(py.includes('BORDERS_PROVINCIAL'), 'Debe generar bordes provinciales');
  assert.ok(py.includes('MAT_OCEAN_BATHYMETRY'), 'Debe incluir batimetría oceánica');
  assert.ok(py.includes('elevation_at_lon_lat'), 'Debe esculpir relieve andino continuo');
  assert.ok(py.includes('add_subdivided_draped_segment'), 'Debe subdividir ríos y fronteras sobre el relieve');
  assert.ok(py.includes('f.smooth = f.normal.z > 0.35'), 'Debe suavizar caras superiores del terreno');
  assert.ok(py.includes('MAT_COUNTRY_{iso2}'), 'Debe compartir materiales por país para reducir draw calls');
  assert.ok(py.includes('export_scene.gltf'), 'Debe exportar .glb desde Blender');

  const luaSync = fs.readFileSync(path.resolve('game/tools/blender/sync_lua_metadata.lua'), 'utf8');
  assert.ok(luaSync.includes('export-countries'), 'Falta modo export-countries en Lua');
  assert.ok(luaSync.includes('validate-provinces'), 'Falta modo validate-provinces en Lua');
});

test('Existen los 6 archivos de documentación en la raíz del proyecto', () => {
  for (const doc of ['README.md', 'ROADMAP.md', 'STRUCTURE.md', 'AI_CONTEXT.md', 'AGENTS.md', 'README_AI.md']) {
    assert.ok(fs.existsSync(path.resolve(doc)), `Falta ${doc} en la raíz`);
  }
});
