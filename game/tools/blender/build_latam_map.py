"""
Propósito del archivo (Blender Python `bpy` + `bmesh`):
Descarga datos geográficos de Dominio Público (Natural Earth Admin-1 Estados/Provincias
y Ciudades/Capitales en formato GeoJSON), los filtra para las 21 naciones de Latinoamérica
definidas en `countries_latam.lua`, construye un mapa 3D detallado en Blender con:
  - Mallas extruidas independientes por cada provincia (`PROV_<ISO2>_<N>`)
  - Colores y materiales coordinados desde Lua con variación tonal por provincia
  - Relieve sutil simulado en zonas andinas/montañosas y base oceánica
  - Marcadores 3D y nodos `Empty` para capitales nacionales y provinciales (`CAP_*`)
  - Metadatos glTF (`extras`) listos para `gltf` + `wgpu` en Rust
Exporta el modelo binario `latinoamerica_3d.glb` y genera `provinces_latam.lua`.
"""

import argparse
import json
import math
import os
import re
import sys
import urllib.request

import bpy
import bmesh
from mathutils import Vector

# Fuentes GeoJSON de Dominio Público (Natural Earth Vector - sin requerir atribución)
URL_ADMIN1_GEOJSON = (
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
    "ne_10m_admin_1_states_provinces.geojson"
)
URL_CITIES_GEOJSON = (
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
    "ne_10m_populated_places_simple.geojson"
)

# Centro geográfico de referencia para proyectar Latinoamérica sin distorsión excesiva
REF_LON = -72.0
REF_LAT = -12.0
SCALE_XY = 1.0  # 1 grado ~ 1 unidad Blender (ajustado en X por cos(REF_LAT))
COS_LAT = math.cos(math.radians(REF_LAT))


def parse_args():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    else:
        argv = []

    parser = argparse.ArgumentParser(description="Generador 3D de Latinoamérica en Blender")
    parser.add_argument("--countries-json", required=True, help="JSON exportado desde countries_latam.lua")
    parser.add_argument("--out-glb", required=True, help="Ruta de salida .glb")
    parser.add_argument("--out-lua", required=True, help="Ruta de salida provinces_latam.lua")
    parser.add_argument("--cache-dir", default="/tmp/latam_geojson_cache", help="Directorio caché GeoJSON")
    parser.add_argument("--simplify-tol", type=float, default=0.05, help="Tolerancia Douglas-Peucker en grados")
    parser.add_argument("--extrude-height", type=float, default=0.45, help="Altura base de extrusión 3D")
    return parser.parse_args(argv)


def download_if_needed(url: str, dest_path: str):
    if os.path.exists(dest_path) and os.path.getsize(dest_path) > 1024:
        print(f"[Cache] Usando {dest_path}")
        return
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    print(f"[Descarga] Obteniendo GeoJSON desde {url} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "HorizonteLatam-MapBuilder/1.0"})
    with urllib.request.urlopen(req, timeout=120) as resp, open(dest_path, "wb") as out:
        out.write(resp.read())
    print(f"[Descarga] Guardado en {dest_path}")


def project_lon_lat(lon: float, lat: float) -> tuple[float, float]:
    """Proyección equirectangular centrada en Latinoamérica con corrección de aspecto."""
    x = (lon - REF_LON) * COS_LAT * SCALE_XY
    y = (lat - REF_LAT) * SCALE_XY
    return (x, y)


def douglas_peucker(pts: list[tuple[float, float]], tol: float) -> list[tuple[float, float]]:
    """Simplifica anillos GeoJSON muy densos preservando la silueta de cada provincia."""
    if len(pts) < 4 or tol <= 0.0:
        return pts

    def perp_dist(p, a, b):
        dx = b[0] - a[0]
        dy = b[1] - a[1]
        if dx == 0.0 and dy == 0.0:
            return math.hypot(p[0] - a[0], p[1] - a[1])
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
        t = max(0.0, min(1.0, t))
        proj = (a[0] + t * dx, a[1] + t * dy)
        return math.hypot(p[0] - proj[0], p[1] - proj[1])

    stack = [(0, len(pts) - 1)]
    keep = [False] * len(pts)
    keep[0] = True
    keep[-1] = True

    while stack:
        start, end = stack.pop()
        max_d = 0.0
        index = start
        for i in range(start + 1, end):
            d = perp_dist(pts[i], pts[start], pts[end])
            if d > max_d:
                max_d = d
                index = i
        if max_d > tol:
            keep[index] = True
            stack.append((start, index))
            stack.append((index, end))

    simplified = [p for p, k in zip(pts, keep) if k]
    return simplified if len(simplified) >= 3 else pts[:3]


def sanitize_ident(text: str) -> str:
    clean = re.sub(r"[^A-Za-z0-9]+", "_", text or "").strip("_").upper()
    return clean[:24] if clean else "PROV"


def estimate_andean_elevation(lon: float, lat: float) -> float:
    """Agrega relieve sutil a la franja andina y mesetas altas para dar volumen táctico en 3D."""
    # Eje aproximado de la Cordillera de los Andes desde Venezuela/Colombia hasta Tierra del Fuego
    andes_lon = -70.0 + max(0.0, (lat + 18.0)) * 0.14 if lat < 10.0 else -99.0
    dist_andes = abs(lon - andes_lon)
    ridge = max(0.0, 1.0 - dist_andes / 4.2) * 0.55 if -55.0 <= lat <= 11.0 else 0.0

    # Eje montañoso de Sierra Madre en México
    mexico_ridge = 0.0
    if 15.0 <= lat <= 28.0 and -106.0 <= lon <= -95.0:
        mexico_ridge = 0.28

    return ridge + mexico_ridge


def clear_blender_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def get_or_create_collection(name: str, parent=None):
    if name in bpy.data.collections:
        return bpy.data.collections[name]
    col = bpy.data.collections.new(name)
    if parent is None:
        bpy.context.scene.collection.children.link(col)
    else:
        parent.children.link(col)
    return col


def make_material(name: str, rgb_255: list[int], variation: float, roughness: float = 0.72):
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        r = max(0.02, min(1.0, (rgb_255[0] / 255.0) * variation))
        g = max(0.02, min(1.0, (rgb_255[1] / 255.0) * variation))
        b = max(0.02, min(1.0, (rgb_255[2] / 255.0) * variation))
        bsdf.inputs["Base Color"].default_value = (r, g, b, 1.0)
        bsdf.inputs["Roughness"].default_value = roughness
    return mat


def extract_polygon_rings(geometry: dict) -> list[list[tuple[float, float]]]:
    gtype = geometry.get("type")
    coords = geometry.get("coordinates", [])
    rings = []
    if gtype == "Polygon" and coords:
        rings.append([(c[0], c[1]) for c in coords[0]])
    elif gtype == "MultiPolygon":
        for poly in coords:
            if poly and poly[0]:
                rings.append([(c[0], c[1]) for c in poly[0]])
    return rings


def polygon_signed_area(pts: list[tuple[float, float]]) -> float:
    area = 0.0
    n = len(pts)
    for i in range(n):
        x1, y1 = pts[i]
        x2, y2 = pts[(i + 1) % n]
        area += x1 * y2 - x2 * y1
    return area * 0.5


def build_province_mesh(
    mesh_name: str,
    rings_lon_lat: list[list[tuple[float, float]]],
    simplify_tol: float,
    base_extrude: float,
    material,
    collection,
    extras: dict,
):
    bm = bmesh.new()
    all_centroids = []
    total_area = 0.0
    max_z = base_extrude

    # Ordenar islas por área descendente y conservar hasta las 6 mayores por provincia
    rings_sorted = sorted(rings_lon_lat, key=lambda r: abs(polygon_signed_area(r)), reverse=True)[:6]

    for ring in rings_sorted:
        if len(ring) < 3:
            continue
        # Eliminar punto de cierre duplicado si existe
        if ring[0] == ring[-1]:
            ring = ring[:-1]
        simplified = douglas_peucker(ring, simplify_tol)
        if len(simplified) < 3:
            continue

        # Calcular centro de esta isla y elevación andina
        avg_lon = sum(p[0] for p in simplified) / len(simplified)
        avg_lat = sum(p[1] for p in simplified) / len(simplified)
        relief = estimate_andean_elevation(avg_lon, avg_lat)
        top_z = base_extrude + relief
        max_z = max(max_z, top_z)

        proj_pts = [project_lon_lat(lon, lat) for (lon, lat) in simplified]
        area = abs(polygon_signed_area(proj_pts))
        if area < 0.002:
            continue
        total_area += area
        cx = sum(p[0] for p in proj_pts) / len(proj_pts)
        cy = sum(p[1] for p in proj_pts) / len(proj_pts)
        all_centroids.append((cx, cy, area))

        # Crear vértices inferiores y formar cara base para extrusión limpia
        bottom_verts = []
        for x, y in proj_pts:
            v = bm.verts.new((x, y, 0.0))
            bottom_verts.append(v)

        bm.verts.ensure_lookup_table()
        try:
            face = bm.faces.new(bottom_verts)
        except ValueError:
            continue

        # Triangular cara base y extruir hacia +Z (en Blender Z es arriba; glTF exporta Y-up automáticamente)
        res = bmesh.ops.extrude_face_region(bm, geom=[face])
        extruded_verts = [ele for ele in res["geom"] if isinstance(ele, bmesh.types.BMVert)]
        for v in extruded_verts:
            v.co.z = top_z

    if len(bm.verts) < 3 or not all_centroids:
        bm.free()
        return None

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bmesh.ops.triangulate(bm, faces=bm.faces)

    mesh = bpy.data.meshes.new(mesh_name)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(mesh_name, mesh)
    obj.data.materials.append(material)
    for k, v in extras.items():
        obj[k] = v
    collection.objects.link(obj)

    # Centro ponderado por área de islas
    w_sum = sum(c[2] for c in all_centroids)
    final_cx = sum(c[0] * c[2] for c in all_centroids) / w_sum
    final_cy = sum(c[1] * c[2] for c in all_centroids) / w_sum

    return {
        "center": (final_cx, final_cy, max_z),
        "area": total_area,
    }


def create_ocean_plane(collection):
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=8, y_segments=8, size=75.0)
    for v in bm.verts:
        v.co.z = -0.05
    mesh = bpy.data.meshes.new("OCEAN_BASE_MESH")
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new("OCEAN_BASE", mesh)
    mat = make_material("MAT_OCEAN", [11, 28, 48], 1.0, roughness=0.35)
    obj.data.materials.append(mat)
    collection.objects.link(obj)


def create_capital_marker(
    node_name: str,
    pos_xyz: tuple[float, float, float],
    is_national: bool,
    material,
    collection,
    extras: dict,
):
    bm = bmesh.new()
    radius = 0.22 if is_national else 0.12
    height = 0.35 if is_national else 0.18
    bmesh.ops.create_cone(
        bm,
        cap_ends=True,
        cap_tris=True,
        segments=8,
        radius1=radius,
        radius2=radius * 0.25,
        depth=height,
    )
    mesh = bpy.data.meshes.new(f"{node_name}_MESH")
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(node_name, mesh)
    obj.location = Vector((pos_xyz[0], pos_xyz[1], pos_xyz[2] + height * 0.5))
    obj.data.materials.append(material)
    for k, v in extras.items():
        obj[k] = v
    collection.objects.link(obj)


def lua_escape(text: str) -> str:
    return (
        str(text)
        .replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", " ")
        .replace("\r", "")
    )


def write_provinces_lua(out_lua_path: str, provinces_records: list[dict], capitals_records: list[dict]):
    os.makedirs(os.path.dirname(out_lua_path), exist_ok=True)
    lines = [
        "--[[",
        "  Propósito del archivo (Lua 5.4):",
        "  Catálogo autogenerado por el pipeline Blender + GeoJSON (Natural Earth Dominio Público).",
        "  Conecta cada provincia y capital del modelo 3D (`latinoamerica_3d.glb`) con la",
        "  lógica de simulación geopolítica en Rust.",
        "--]]",
        "",
        "local data = {",
        "  provinces = {",
    ]

    for p in provinces_records:
        lines.append(
            f'    {{ id = "{lua_escape(p["id"])}", '
            f'mesh_name = "{lua_escape(p["mesh_name"])}", '
            f'country_id = "{lua_escape(p["country_id"])}", '
            f'name = "{lua_escape(p["name"])}", '
            f'admin_type = "{lua_escape(p["admin_type"])}", '
            f'area_weight = {p["area_weight"]:.4f}, '
            f'center = {{ x = {p["cx"]:.4f}, y = {p["cy"]:.4f}, z = {p["cz"]:.4f} }} }},'
        )

    lines.append("  },")
    lines.append("  capitals = {")

    for c in capitals_records:
        nat_flag = "true" if c["is_national"] else "false"
        lines.append(
            f'    {{ node_name = "{lua_escape(c["node_name"])}", '
            f'country_id = "{lua_escape(c["country_id"])}", '
            f'name = "{lua_escape(c["name"])}", '
            f'is_national = {nat_flag}, '
            f'pop_max = {int(c["pop_max"])}, '
            f'pos = {{ x = {c["x"]:.4f}, y = {c["y"]:.4f}, z = {c["z"]:.4f} }} }},'
        )

    lines.append("  }")
    lines.append("}")
    lines.append("")
    lines.append("return data")
    lines.append("")

    with open(out_lua_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"[Lua] Generado {out_lua_path} con {len(provinces_records)} provincias y {len(capitals_records)} capitales.")


def main():
    args = parse_args()
    with open(args.countries_json, "r", encoding="utf-8") as f:
        countries_meta = json.load(f)

    target_iso2 = set(countries_meta.keys())

    admin1_path = os.path.join(args.cache_dir, "ne_10m_admin_1_states_provinces.geojson")
    cities_path = os.path.join(args.cache_dir, "ne_10m_populated_places_simple.geojson")
    download_if_needed(URL_ADMIN1_GEOJSON, admin1_path)
    download_if_needed(URL_CITIES_GEOJSON, cities_path)

    with open(admin1_path, "r", encoding="utf-8") as f:
        admin1_geo = json.load(f)
    with open(cities_path, "r", encoding="utf-8") as f:
        cities_geo = json.load(f)

    clear_blender_scene()

    col_world = get_or_create_collection("LATAM_WORLD")
    col_provs = get_or_create_collection("PROVINCES", col_world)
    col_caps = get_or_create_collection("CAPITALS", col_world)
    create_ocean_plane(col_world)

    mat_nat_cap = make_material("MAT_CAP_NATIONAL", [245, 200, 75], 1.0, roughness=0.25)
    mat_prov_cap = make_material("MAT_CAP_PROVINCIAL", [220, 232, 245], 1.0, roughness=0.35)

    provinces_records = []
    country_counters: dict[str, int] = {}

    for feat in admin1_geo.get("features", []):
        props = feat.get("properties", {})
        iso2 = props.get("iso_a2") or ""
        if iso2 not in target_iso2:
            continue

        geom = feat.get("geometry")
        if not geom:
            continue

        rings = extract_polygon_rings(geom)
        if not rings:
            continue

        idx = country_counters.get(iso2, 0) + 1
        country_counters[iso2] = idx

        prov_name = props.get("name") or props.get("name_es") or f"Provincia {idx}"
        admin_type = props.get("type_en") or props.get("type") or "Province"
        code_local = props.get("iso_3166_2") or f"{iso2}-{idx:02d}"
        prov_id = sanitize_ident(code_local)
        mesh_name = f"PROV_{iso2}_{idx:03d}_{prov_id}"

        base_rgb = countries_meta[iso2]["color"]
        variation = 0.86 + ((idx % 5) * 0.07)
        mat_prov = make_material(f"MAT_{mesh_name}", base_rgb, variation, roughness=0.72)

        built = build_province_mesh(
            mesh_name=mesh_name,
            rings_lon_lat=rings,
            simplify_tol=args.simplify_tol,
            base_extrude=args.extrude_height,
            material=mat_prov,
            collection=col_provs,
            extras={
                "country_id": iso2,
                "province_id": prov_id,
                "province_name": prov_name,
                "admin_type": admin_type,
            },
        )
        if built:
            cx, cy, cz = built["center"]
            provinces_records.append(
                {
                    "id": prov_id,
                    "mesh_name": mesh_name,
                    "country_id": iso2,
                    "name": prov_name,
                    "admin_type": admin_type,
                    "area_weight": built["area"],
                    "cx": cx,
                    "cy": cy,
                    "cz": cz,
                }
            )

    # Capitales nacionales y provinciales
    capitals_records = []
    cap_counters: dict[str, int] = {}
    for feat in cities_geo.get("features", []):
        props = feat.get("properties", {})
        iso2 = props.get("iso_a2") or ""
        if iso2 not in target_iso2:
            continue

        fclass = props.get("featurecla") or ""
        if fclass not in ("Admin-0 capital", "Admin-1 capital"):
            continue

        coords = feat.get("geometry", {}).get("coordinates")
        if not coords or len(coords) < 2:
            continue

        lon, lat = float(coords[0]), float(coords[1])
        x, y = project_lon_lat(lon, lat)
        z = args.extrude_height + estimate_andean_elevation(lon, lat)
        is_nat = fclass == "Admin-0 capital"
        city_name = props.get("name") or "Capital"
        pop_max = props.get("pop_max") or 0

        cap_idx = cap_counters.get(iso2, 0) + 1
        cap_counters[iso2] = cap_idx
        node_name = f"CAP_{iso2}" if is_nat else f"PCAP_{iso2}_{cap_idx:02d}_{sanitize_ident(city_name)}"

        create_capital_marker(
            node_name=node_name,
            pos_xyz=(x, y, z),
            is_national=is_nat,
            material=mat_nat_cap if is_nat else mat_prov_cap,
            collection=col_caps,
            extras={
                "country_id": iso2,
                "city_name": city_name,
                "is_national_capital": is_nat,
                "pop_max": int(pop_max),
            },
        )

        capitals_records.append(
            {
                "node_name": node_name,
                "country_id": iso2,
                "name": city_name,
                "is_national": is_nat,
                "pop_max": pop_max,
                "x": x,
                "y": y,
                "z": z,
            }
        )

    os.makedirs(os.path.dirname(args.out_glb), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=args.out_glb,
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        export_extras=True,
        export_materials="EXPORT",
    )
    print(f"[Blender] Exportado modelo 3D a {args.out_glb}")

    write_provinces_lua(args.out_lua, provinces_records, capitals_records)


if __name__ == "__main__":
    main()
