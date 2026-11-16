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
    'game/src/ui/screens/companies_screen.rs',
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
  assert.equal(yml.includes('github.run_id'), false, 'No debe incluir github.run_id en la clave de caché o multiplicará cachés en cada ejecución');
  assert.ok(yml.includes('actions: write'), 'android_apk.yml necesita actions: write para sobrescribir caché previa');
  assert.ok(yml.includes('gh cache delete'), 'android_apk.yml debe limpiar la caché anterior antes de guardar una nueva');
  assert.ok(yml.includes('gh release download'), 'Debe descargar mapa 3D y provincias desde Release');
  assert.ok(yml.includes('actions/cache/restore@v5'), 'Debe restaurar caché');
  assert.ok(yml.includes('actions/cache/save@v5'), 'Debe guardar caché obligatoriamente');
  assert.ok(yml.includes('if: always()'), 'La caché debe guardarse incluso si falla un paso posterior');
  assert.ok(yml.includes('assembleDebug'), 'Debe compilar APK Debug');
  assert.ok(yml.includes('arm64-v8a'), 'Debe incluir build 64 bits');
  assert.ok(yml.includes('armeabi-v7a'), 'Debe incluir build 32 bits');
  assert.ok(yml.includes('HorizonteLatam-debug-${{ matrix.arch_label }}-${{ matrix.abi }}'), 'Debe subir cada arquitectura por separado');

  const cleanYml = fs.readFileSync(path.resolve('.github/workflows/clean_cache.yml'), 'utf8');
  assert.ok(cleanYml.includes('workflow_dispatch:'), 'clean_cache.yml debe ser exclusivamente manual');
  assert.equal(/^\s*(push|pull_request|schedule):/m.test(cleanYml), false, 'clean_cache.yml nunca debe activarse automáticamente');
  assert.ok(cleanYml.includes('actions: write'), 'clean_cache.yml debe tener permiso actions: write');
  assert.ok(cleanYml.includes('gh cache delete --all'), 'clean_cache.yml debe poder limpiar todas las cachés');
});

test('Existen los 6 archivos de documentación en la raíz del proyecto', () => {
  for (const doc of ['README.md', 'ROADMAP.md', 'STRUCTURE.md', 'AI_CONTEXT.md', 'AGENTS.md', 'README_AI.md']) {
    assert.ok(fs.existsSync(path.resolve(doc)), `Falta ${doc} en la raíz`);
  }
});

test('Android oculta barras del sistema y desactiva edge-to-edge intrusivo en API 35 sin Kotlin', () => {
  const manifest = fs.readFileSync(path.resolve('game/android/app/src/main/AndroidManifest.xml'), 'utf8');
  const stylesBase = fs.readFileSync(path.resolve('game/android/app/src/main/res/values/styles.xml'), 'utf8');
  const stylesV28 = fs.readFileSync(path.resolve('game/android/app/src/main/res/values-v28/styles.xml'), 'utf8');
  const stylesV35 = fs.readFileSync(path.resolve('game/android/app/src/main/res/values-v35/styles.xml'), 'utf8');
  const libRs = fs.readFileSync(path.resolve('game/src/lib.rs'), 'utf8');

  assert.ok(manifest.includes('android:theme="@style/HorizonteTheme"'), 'AndroidManifest debe aplicar @style/HorizonteTheme');
  assert.ok(stylesBase.includes('android:windowFullscreen">true'), 'Falta windowFullscreen=true');
  assert.ok(stylesV28.includes('android:windowLayoutInDisplayCutoutMode">never'), 'Falta cutout mode seguro');
  assert.ok(
    stylesV35.includes('android:windowOptOutEdgeToEdgeEnforcement">true'),
    'Falta windowOptOutEdgeToEdgeEnforcement=true para evitar que los 3 botones tapen la UI en Android 15',
  );
  assert.ok(libRs.includes('with_fullscreen(true)'), 'android_main debe activar viewport fullscreen');
});

test('UI móvil: scroll táctil habilitado, sin colisiones de ScrollArea ID y columnas verticales explícitas', () => {
  const theme = fs.readFileSync(path.resolve('game/src/ui/theme.rs'), 'utf8');
  assert.ok(theme.includes('selectable_labels = false'), 'Debe desactivar selectable_labels para permitir deslizar sobre textos');

  const scrollFiles = [
    'game/src/ui/components/chrome.rs',
    'game/src/ui/screens/main_menu.rs',
    'game/src/ui/screens/country_select.rs',
    'game/src/ui/screens/companies_screen.rs',
    'game/src/ui/screens/map_screen.rs',
    'game/src/ui/screens/economy_screen.rs',
    'game/src/ui/screens/politics_screen.rs',
    'game/src/ui/screens/diplomacy_screen.rs',
    'game/src/ui/screens/military_screen.rs',
    'game/src/ui/screens/settings_screen.rs',
  ];

  const seenSalts = new Set<string>();
  for (const file of scrollFiles) {
    const src = fs.readFileSync(path.resolve(file), 'utf8');
    const scrollCount = (src.match(/ScrollArea::(vertical|horizontal)/g) || []).length;
    const salts = [...src.matchAll(/\.id_salt\("([^"]+)"\)/g)].map((m) => m[1]);
    assert.equal(
      salts.length,
      scrollCount,
      `Cada ScrollArea en ${file} debe tener su propio .id_salt(...) para evitar "First use of ScrollArea ID"`,
    );
    for (const s of salts) {
      assert.equal(seenSalts.has(s), false, `id_salt duplicado: ${s}`);
      seenSalts.add(s);
    }

    // Comprobación básica de balanceo de llaves/paréntesis en archivos editados
    let braces = 0;
    let parens = 0;
    for (const ch of src) {
      if (ch === '{') braces++;
      else if (ch === '}') braces--;
      else if (ch === '(') parens++;
      else if (ch === ')') parens--;
    }
    assert.equal(braces, 0, `Llaves desbalanceadas en ${file}`);
    assert.equal(parens, 0, `Paréntesis desbalanceados en ${file}`);
  }

  const countrySelect = fs.readFileSync(path.resolve('game/src/ui/screens/country_select.rs'), 'utf8');
  const mapScreen = fs.readFileSync(path.resolve('game/src/ui/screens/map_screen.rs'), 'utf8');
  assert.ok(countrySelect.includes('Layout::top_down'), 'country_select debe usar Layout::top_down en sus columnas');
  assert.ok(mapScreen.includes('Layout::top_down'), 'map_screen debe usar Layout::top_down en sus columnas');
});

test('Visor 3D puro compatible con Mali-G52 (Vulkan, write_buffer, Float32x2, Depth24Plus y cero fallback SVG)', () => {
  const libRs = fs.readFileSync(path.resolve('game/src/lib.rs'), 'utf8');
  const uiMod = fs.readFileSync(path.resolve('game/src/ui/mod.rs'), 'utf8');
  const renderer3d = fs.readFileSync(path.resolve('game/src/map/renderer_3d.rs'), 'utf8');
  const glbLoader = fs.readFileSync(path.resolve('game/src/map/glb_loader.rs'), 'utf8');
  const mapCanvas = fs.readFileSync(path.resolve('game/src/ui/components/map_canvas.rs'), 'utf8');
  const settings = fs.readFileSync(path.resolve('game/src/ui/screens/settings_screen.rs'), 'utf8');

  assert.ok(libRs.includes('Backends::VULKAN'), 'lib.rs debe configurar Vulkan en wgpu_options');
  assert.ok(libRs.includes('adapter.limits()'), 'lib.rs debe solicitar adapter.limits()');
  assert.ok(uiMod.includes('frame.wgpu_render_state()'), 'ui/mod.rs debe sincronizar target_format en cada frame');
  assert.equal(uiMod.includes('LatamVectorMap'), false, 'ui/mod.rs no debe usar fallback SVG 2D');
  assert.equal(mapCanvas.includes('render_latam_map_2d'), false, 'map_canvas.rs no debe tener fallback SVG 2D');
  assert.equal(settings.includes('Mapa 2D (SVG'), false, 'settings_screen.rs no debe ofrecer modo 2D fallback');
  assert.ok(renderer3d.includes('sync_target_format'), 'renderer_3d.rs debe reconstruir blit_pipeline si cambia target_format');
  assert.ok(renderer3d.includes('queue.write_buffer'), 'renderer_3d.rs debe subir buffers con queue.write_buffer en lugar de mapped_at_creation');
  assert.ok(renderer3d.includes('Depth24Plus'), 'renderer_3d.rs debe usar Depth24Plus compatible con GPU móvil');
  assert.ok(renderer3d.includes('VertexFormat::Float32x2'), 'renderer_3d.rs debe usar Float32x2 en meta');
  assert.equal(renderer3d.includes('positions[vid]'), false, 'SHADER_BLIT_WGSL no debe usar indexado dinámico en array local');
  assert.ok(glbLoader.includes('buffers.get(buffer.index()).map(|v| &**v)'), 'glb_loader.rs debe convertir Option<&Data> a Option<&[u8]>');
  assert.ok(glbLoader.includes('pm.positions.get('), 'glb_loader.rs debe usar indexado seguro con .get()');
  assert.ok(glbLoader.includes('raw_normals.get('), 'glb_loader.rs debe proteger lectura de normales');
  assert.ok(mapCanvas.includes('rect.is_positive()'), 'map_canvas.rs debe validar rect antes de crear PaintCallback');

  for (const file of [
    'game/src/lib.rs',
    'game/src/ui/mod.rs',
    'game/src/map/renderer_3d.rs',
    'game/src/map/glb_loader.rs',
  ]) {
    const src = fs.readFileSync(path.resolve(file), 'utf8');
    let braces = 0;
    let parens = 0;
    for (const ch of src) {
      if (ch === '{') braces++;
      else if (ch === '}') braces--;
      else if (ch === '(') parens++;
      else if (ch === ')') parens--;
    }
    assert.equal(braces, 0, `Llaves desbalanceadas en ${file}`);
    assert.equal(parens, 0, `Paréntesis desbalanceados en ${file}`);
  }
});

test('Shader WGSL evita palabra reservada meta y la UI táctica incluye escudos, botones físicos y margen derecho seguro', () => {
  const renderer3d = fs.readFileSync(path.resolve('game/src/map/renderer_3d.rs'), 'utf8');
  const theme = fs.readFileSync(path.resolve('game/src/ui/theme.rs'), 'utf8');
  const uiMod = fs.readFileSync(path.resolve('game/src/ui/mod.rs'), 'utf8');
  const chrome = fs.readFileSync(path.resolve('game/src/ui/components/chrome.rs'), 'utf8');

  // Asegura que SHADER_3D_WGSL no declare `@location(3) meta:`
  const shaderSection = renderer3d.slice(renderer3d.indexOf('const SHADER_3D_WGSL'));
  assert.equal(/\bmeta\s*:/.test(shaderSection), false, 'WGSL prohíbe `meta` como nombre de atributo');
  assert.ok(shaderSection.includes('@location(3) slot_ids: vec2<f32>'), 'Debe usar slot_ids en WGSL');

  assert.ok(theme.includes('draw_country_shield'), 'Debe incluir escudo heráldico procedural');
  assert.ok(theme.includes('game_button'), 'Debe incluir botones táctiles mecanizados');
  assert.ok(theme.includes('layout_no_wrap'), 'game_button debe medir texto con layout_no_wrap para evitar recortes');
  assert.ok(theme.includes('SAFE_RIGHT_PAD'), 'Debe definir margen derecho seguro para barra Android');
  assert.ok(uiMod.includes('SAFE_RIGHT_PAD'), 'ui/mod.rs debe aplicar SAFE_RIGHT_PAD');
  assert.ok(uiMod.includes('.exact_height(54.0)'), 'top_hud_panel debe fijar altura exacta para no tapar el CentralPanel');
  assert.ok(uiMod.includes('.exact_height(50.0)'), 'bottom_nav_panel debe fijar altura exacta');
  assert.ok(chrome.includes('draw_country_shield'), 'Top HUD debe mostrar escudo del país');
  assert.ok(chrome.includes('TextWrapMode::Extend'), 'Top HUD debe evitar saltos de línea en métricas y fecha');
  assert.ok(chrome.includes('.max_height(36.0)'), 'top_hud_scroll debe limitar su altura máxima');
});

test('Simulación en tiempo real, reclutamiento/movimiento de tropas por estados y empresas/bolsa/desempleo', () => {
  const companiesLua = fs.readFileSync(path.resolve('game/assets/scripts/companies_latam.lua'), 'utf8');
  const rulesLua = fs.readFileSync(path.resolve('game/assets/scripts/game_rules.lua'), 'utf8');
  const modelsRs = fs.readFileSync(path.resolve('game/src/models/mod.rs'), 'utf8');
  const engineRs = fs.readFileSync(path.resolve('game/src/engine/mod.rs'), 'utf8');
  const aiRs = fs.readFileSync(path.resolve('game/src/ai/mod.rs'), 'utf8');
  const chromeRs = fs.readFileSync(path.resolve('game/src/ui/components/chrome.rs'), 'utf8');
  const mapCanvasRs = fs.readFileSync(path.resolve('game/src/ui/components/map_canvas.rs'), 'utf8');
  const militaryRs = fs.readFileSync(path.resolve('game/src/ui/screens/military_screen.rs'), 'utf8');
  const companiesRs = fs.readFileSync(path.resolve('game/src/ui/screens/companies_screen.rs'), 'utf8');
  const uiModRs = fs.readFileSync(path.resolve('game/src/ui/mod.rs'), 'utf8');

  // Catálogo empresarial Lua
  for (const catId of ['TECH', 'ENERGY', 'AGRO', 'MANUF', 'LOGISTICS', 'FINANCE', 'HEALTH']) {
    assert.ok(companiesLua.includes(`id = "${catId}"`), `Falta categoría ${catId} en companies_latam.lua`);
  }
  assert.ok(companiesLua.includes('products ='), 'Faltan productos vendidos en companies_latam.lua');
  assert.ok(companiesLua.includes('base_price_min ='), 'Falta rango de precio de acción');
  assert.ok(companiesLua.includes('gdp_per_job_m ='), 'Falta aporte al PIB por empleo');

  // Reglas económicas Lua: mantenimiento de tropas, empresas y desempleo
  assert.ok(rulesLua.includes('troop_upkeep_m'), 'game_rules.lua debe calcular mantenimiento de soldados');
  assert.ok(rulesLua.includes('corporate_gdp_bonus_m'), 'game_rules.lua debe incluir aporte empresarial al PIB');
  assert.ok(rulesLua.includes('unemployment_pct'), 'game_rules.lua debe calcular desempleo');

  // Tiempo real en lugar de turnos
  assert.equal(chromeRs.includes('+1 MES'), false, 'La barra superior ya no debe ser por turnos (+1 MES)');
  assert.equal(chromeRs.includes('TURNO {}'), false, 'La barra superior debe mostrar días en tiempo real');
  assert.ok(chromeRs.includes('EN VIVO'), 'Top HUD debe indicar simulación EN VIVO');
  assert.ok(chromeRs.includes('Empresas'), 'Navegación inferior debe incluir pantalla Empresas');
  assert.ok(chromeRs.includes('DESEMPLEO'), 'Top HUD debe mostrar tasa de desempleo');
  assert.ok(uiModRs.includes('maybe_advance_realtime'), 'ui/mod.rs debe avanzar en tiempo real');
  assert.ok(engineRs.includes('advance_single_day'), 'engine/mod.rs debe simular días en tiempo real');

  // Reclutamiento, descuento de ingresos y marcha por estados
  assert.ok(modelsRs.includes('struct ArmyUnit'), 'models/mod.rs debe definir ArmyUnit');
  assert.ok(modelsRs.includes('struct ProvinceNode'), 'models/mod.rs debe definir ProvinceNode');
  assert.ok(modelsRs.includes('struct CompanyEntity'), 'models/mod.rs debe definir CompanyEntity');
  assert.ok(engineRs.includes('step_unit_movements'), 'engine/mod.rs debe mover tropas entre estados');
  assert.ok(engineRs.includes('step_companies_and_stocks'), 'engine/mod.rs debe fundar empresas y actualizar acciones');
  assert.ok(aiRs.includes('find_province_route'), 'ai/mod.rs debe calcular rutas entre estados');
  assert.ok(aiRs.includes('move_unit'), 'IA debe mover unidades entre estados');
  assert.ok(mapCanvasRs.includes('project_world_to_rect'), 'map_canvas.rs debe proyectar unidades 3D en pantalla');
  assert.ok(militaryRs.includes('RecruitSoldiers'), 'military_screen.rs debe reclutar soldados por estado');
  assert.ok(militaryRs.includes('OrderUnitMove'), 'military_screen.rs debe permitir ordenar marcha entre estados');
  assert.ok(companiesRs.startsWith('//! Propósito del archivo:'), 'companies_screen.rs debe tener encabezado explicativo');
  assert.ok(companiesRs.includes('draw_stock_sparkline'), 'companies_screen.rs debe dibujar gráfica de acciones');
  assert.equal(uiModRs.includes('TurnCompleted'), false, 'ui/mod.rs no debe usar variante inexistente TurnCompleted');
  assert.ok(uiModRs.includes('SimResponse::TickCompleted'), 'ui/mod.rs debe manejar SimResponse::TickCompleted');
  assert.equal(engineRs.includes('state.get_relation(&unit.country_id'), false, 'step_unit_movements no debe prestar state entero mientras itera &mut state.units');
});


