"""
Propósito del archivo (Blender Python `bpy` + `bmesh`):
Generador 3D táctico de Latinoamérica para Horizonte Latam.
Descarga datos GeoJSON de Dominio Público (Natural Earth Admin-1 provincias,
ciudades/capitales y ríos principales) y construye un modelo `.glb` listo para `wgpu` + `gltf`:
  1. Colores corregidos con conversión real sRGB -> Linear RGB y materiales PBR mate tácticos.
  2. Bordes nítidos diferenciados: fronteras internacionales, bordes provinciales y costas.
  3. Océano dinámico con batimetría costera (Vertex Colors), cuadrícula cartográfica (Ecuador/Trópicos) y marco.
  4. Relieve topográfico continuo por vértice (Cordillera de los Andes, Altiplano, Sierra Madre, Mesetas)
     y red de ríos principales (Amazonas, Paraná, Orinoco, Magdalena, Río Grande, etc.).
  5. Marcadores 3D visibles para capitales nacionales (torre dorada) y provinciales (pin plateado).
Exporta `latinoamerica_3d.glb` y el catálogo `provinces_latam.lua`.
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
from mathutils import Matrix, Vector

# Fuentes GeoJSON de Dominio Público (Natural Earth Vector - sin requerir atribución)
URL_ADMIN1_GEOJSON = (
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
    "ne_10m_admin_1_states_provinces.geojson"
)
URL_CITIES_GEOJSON = (
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
    "ne_10m_populated_places_simple.geojson"
)
URL_RIVERS_GEOJSON = (
    "https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/"
    "ne_10m_rivers_lake_centerlines.geojson"
)

# Centro geográfico y límites de Latinoamérica
REF_LON = -72.0
REF_LAT = -12.0
COS_LAT = math.cos(math.radians(REF_LAT))
LATAM_LON_MIN, LATAM_LON_MAX = -119.0, -33.0
LATAM_LAT_MIN, LATAM_LAT_MAX = -56.5, 33.5


def parse_args():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    else:
        argv = []

    parser = argparse.ArgumentParser(description="Generador 3D táctico de Latinoamérica en Blender")
    parser.add_argument("--countries-json", required=True, help="JSON exportado desde countries_latam.lua")
    parser.add_argument("--out-glb", required=True, help="Ruta de salida .glb")
    parser.add_argument("--out-lua", required=True, help="Ruta de salida provinces_latam.lua")
    parser.add_argument("--cache-dir", default="/tmp/latam_geojson_cache", help="Directorio caché GeoJSON")
    parser.add_argument("--simplify-tol", type=float, default=0.04, help="Tolerancia Douglas-Peucker en grados")
    parser.add_argument("--extrude-height", type=float, default=0.40, help="Altura base de extrusión 3D")
    return parser.parse_args(argv)


def download_if_needed(url: str, dest_path: str):
    if os.path.exists(dest_path) and os.path.getsize(dest_path) > 1024:
        print(f"[Cache] Usando {dest_path}")
        return
    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
    print(f"[Descarga] Obteniendo GeoJSON desde {url} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "HorizonteLatam-MapBuilder/2.0"})
    with urllib.request.urlopen(req, timeout=120) as resp, open(dest_path, "wb") as out:
        out.write(resp.read())
    print(f"[Descarga] Guardado en {dest_path}")


def srgb_channel_to_linear(c_0_255: float) -> float:
    """Conversión estándar IEC 61966-2-1 de sRGB (0..255) a Linear RGB (0..1) para Blender."""
    u = max(0.0, min(1.0, c_0_255 / 255.0))
    if u <= 0.04045:
        return u / 12.92
    return ((u + 0.055) / 1.055) ** 2.4


def srgb_to_linear_rgba(rgb_255: list[int], scale: float = 1.0) -> tuple[float, float, float, float]:
    r = srgb_channel_to_linear(rgb_255[0] * scale)
    g = srgb_channel_to_linear(rgb_255[1] * scale)
    b = srgb_channel_to_linear(rgb_255[2] * scale)
    return (r, g, b, 1.0)


def project_lon_lat(lon: float, lat: float) -> tuple[float, float]:
    """Proyección equirectangular centrada en Latinoamérica con corrección de aspecto."""
    x = (lon - REF_LON) * COS_LAT
    y = lat - REF_LAT
    return (x, y)


def unproject_xy(x: float, y: float) -> tuple[float, float]:
    """Inversa exacta de `project_lon_lat` para calcular relieve real por vértice."""
    lon = (x / COS_LAT) + REF_LON
    lat = y + REF_LAT
    return (lon, lat)


def elevation_at_lon_lat(lon: float, lat: float) -> float:
    """
    Relieve topográfico continuo de Latinoamérica en unidades Blender:
    - Cordillera de los Andes (curva realista de Venezuela/Colombia -> Perú -> Altiplano -> Chile/Argentina)
    - Escudo Guayanés y Meseta Brasileña
    - Sierra Madre Oriental/Occidental y cordillera centroamericana
    Al ser determinista por (lon, lat), provincias vecinas comparten exactamente la misma altura en su frontera.
    """
    elev = 0.0

    # 1. Cordillera de los Andes (-55.5° hasta +11.5°)
    if -56.0 <= lat <= 12.0:
        if lat < -18.0:
            # Tramo sur/central Chile-Argentina-Bolivia: de -71° en Patagonia a -68° en Altiplano
            spine_lon = -71.5 + ((lat + 55.0) / 37.0) * 3.8
            width = 3.2 if lat > -26.0 else 2.2
        elif lat < -5.0:
            # Tramo Altiplano -> Perú: gira hacia el noroeste (-68° a -77.5°)
            t = (lat + 18.0) / 13.0
            spine_lon = -67.7 - t * 9.5
            width = 4.6 - t * 1.4
        else:
            # Ecuador -> Colombia -> Mérida (Venezuela)
            t = (lat + 5.0) / 17.0
            spine_lon = -77.2 + t * 4.5
            width = 2.8

        dist = abs(lon - spine_lon)
        if dist < width * 1.6:
            ridge = math.exp(-(dist * dist) / (2.0 * (width * 0.55) ** 2))
            # Ondulación natural de picos y valles andinos
            peaks = 0.78 + 0.22 * math.sin(lat * 1.35 + lon * 0.9) * math.cos(lon * 1.7)
            elev = max(elev, ridge * 1.65 * peaks)

    # 2. Meseta Brasileña / Planalto Central y Serra do Mar
    if -28.0 <= lat <= -4.0 and -52.0 <= lon <= -38.0:
        d_br = math.hypot((lon + 45.0) / 7.5, (lat + 15.0) / 9.5)
        if d_br < 1.2:
            elev = max(elev, (1.2 - d_br) * 0.42)

    # 3. Escudo Guayanés / Sur de Venezuela
    if 1.0 <= lat <= 7.5 and -67.0 <= lon <= -59.0:
        d_gu = math.hypot((lon + 63.0) / 4.5, (lat + 4.0) / 4.0)
        if d_gu < 1.3:
            elev = max(elev, (1.3 - d_gu) * 0.48)

    # 4. Sierra Madre y Eje Neovolcánico (México) + Cordillera Centroamericana
    if 8.0 <= lat <= 31.0 and -111.0 <= lon <= -82.0:
        # Eje aproximado desde Sonora (-109, 29) hasta Costa Rica (-83.5, 9.5)
        t = max(0.0, min(1.0, (30.0 - lat) / 21.0))
        mex_spine = -107.0 + t * 23.0
        dist_m = abs(lon - mex_spine)
        if dist_m < 3.6:
            ridge_m = math.exp(-(dist_m * dist_m) / (2.0 * 1.6 ** 2))
            elev = max(elev, ridge_m * 0.85)

    return elev


def classify_terrain(avg_elev: float) -> str:
    if avg_elev >= 0.75:
        return "Alta Montaña"
    if avg_elev >= 0.30:
        return "Meseta / Sierra"
    return "Llanura / Costa"


def douglas_peucker(pts: list[tuple[float, float]], tol: float) -> list[tuple[float, float]]:
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


def dedupe_projected_ring(pts: list[tuple[float, float]], min_dist: float = 0.02) -> list[tuple[float, float]]:
    """Elimina vértices casi coincidentes antes de triangular para evitar triángulos de área cero."""
    cleaned = []
    for p in pts:
        if not cleaned or math.hypot(p[0] - cleaned[-1][0], p[1] - cleaned[-1][1]) >= min_dist:
            cleaned.append(p)
    if len(cleaned) >= 2 and math.hypot(cleaned[0][0] - cleaned[-1][0], cleaned[0][1] - cleaned[-1][1]) < min_dist:
        cleaned.pop()
    return cleaned


def sanitize_ident(text: str) -> str:
    clean = re.sub(r"[^A-Za-z0-9]+", "_", text or "").strip("_").upper()
    return clean[:24] if clean else "PROV"


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


def set_bsdf_specular(bsdf, value: float):
    if "Specular IOR Level" in bsdf.inputs:
        bsdf.inputs["Specular IOR Level"].default_value = value
    elif "Specular" in bsdf.inputs:
        bsdf.inputs["Specular"].default_value = value


def make_material(
    name: str,
    rgb_255: list[int],
    color_scale: float = 1.0,
    roughness: float = 0.86,
    specular: float = 0.12,
    emission_strength: float = 0.0,
):
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf:
        lin_rgba = srgb_to_linear_rgba(rgb_255, color_scale)
        bsdf.inputs["Base Color"].default_value = lin_rgba
        bsdf.inputs["Roughness"].default_value = roughness
        set_bsdf_specular(bsdf, specular)
        if emission_strength > 0.0 and "Emission Color" in bsdf.inputs:
            bsdf.inputs["Emission Color"].default_value = lin_rgba
            bsdf.inputs["Emission Strength"].default_value = emission_strength
    return mat


def make_vertex_color_material(name: str, roughness: float = 0.55, specular: float = 0.22):
    """Material conectado al atributo `Color` de vértices para la batimetría oceánica."""
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes.get("Principled BSDF")
    vcol = nt.nodes.new("ShaderNodeVertexColor")
    vcol.layer_name = "Color"
    if bsdf:
        nt.links.new(vcol.outputs["Color"], bsdf.inputs["Base Color"])
        bsdf.inputs["Roughness"].default_value = roughness
        set_bsdf_specular(bsdf, specular)
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
    border_segments_out: list,
    land_points_out: list,
    country_id: str,
):
    bm = bmesh.new()
    all_centroids = []
    total_area = 0.0
    elev_samples = []

    rings_sorted = sorted(rings_lon_lat, key=lambda r: abs(polygon_signed_area(r)), reverse=True)[:6]

    for ring in rings_sorted:
        if len(ring) < 3:
            continue
        if ring[0] == ring[-1]:
            ring = ring[:-1]
        simplified = douglas_peucker(ring, simplify_tol)
        proj_raw = [project_lon_lat(lon, lat) for (lon, lat) in simplified]
        proj_pts = dedupe_projected_ring(proj_raw, min_dist=0.025)
        if len(proj_pts) < 3:
            continue

        signed_a = polygon_signed_area(proj_pts)
        area = abs(signed_a)
        if area < 0.004:
            continue
        # Asegurar orientación antihoraria consistente para normales limpias
        if signed_a < 0.0:
            proj_pts.reverse()

        total_area += area
        cx = sum(p[0] for p in proj_pts) / len(proj_pts)
        cy = sum(p[1] for p in proj_pts) / len(proj_pts)
        all_centroids.append((cx, cy, area))

        # Guardar puntos para batimetría y segmentos para bordes nítidos
        for i in range(len(proj_pts)):
            p0 = proj_pts[i]
            p1 = proj_pts[(i + 1) % len(proj_pts)]
            land_points_out.append(p0)
            border_segments_out.append((country_id, p0, p1))

        # Crear cara base en z=0
        bottom_verts = [bm.verts.new((x, y, 0.0)) for (x, y) in proj_pts]
        bm.verts.ensure_lookup_table()
        try:
            face = bm.faces.new(bottom_verts)
        except ValueError:
            continue

        # Extruir hacia arriba para formar bloque sólido cerrado
        res = bmesh.ops.extrude_face_region(bm, geom=[face])
        extruded_verts = [ele for ele in res["geom"] if isinstance(ele, bmesh.types.BMVert)]
        for v in extruded_verts:
            v.co.z = base_extrude

    if len(bm.verts) < 3 or not all_centroids:
        bm.free()
        return None

    # Triangular primero para poder subdividir triángulos largos y esculpir relieve continuo
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")

    # Subdividir aristas superiores largas (> 1.4 unidades) para que los Andes tengan curvas suaves
    for _pass in range(2):
        long_top_edges = [
            e
            for e in bm.edges
            if e.verts[0].co.z > 0.05
            and e.verts[1].co.z > 0.05
            and (e.verts[0].co - e.verts[1].co).length > 1.45
        ]
        if not long_top_edges:
            break
        bmesh.ops.subdivide_edges(bm, edges=long_top_edges, cuts=1)
        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")

    # Aplicar relieve topográfico continuo a cada vértice superior según su (lon, lat)
    for v in bm.verts:
        if v.co.z > 0.05:
            lon, lat = unproject_xy(v.co.x, v.co.y)
            rel = elevation_at_lon_lat(lon, lat)
            v.co.z = base_extrude + rel
            elev_samples.append(rel)

    # Limpiar aristas/triángulos degenerados de área cero
    bmesh.ops.dissolve_degenerate(bm, dist=0.002, edges=list(bm.edges))
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))

    mesh = bpy.data.meshes.new(mesh_name)
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(mesh_name, mesh)
    obj.data.materials.append(material)
    for k, v in extras.items():
        obj[k] = v
    collection.objects.link(obj)

    w_sum = sum(c[2] for c in all_centroids)
    final_cx = sum(c[0] * c[2] for c in all_centroids) / w_sum
    final_cy = sum(c[1] * c[2] for c in all_centroids) / w_sum
    c_lon, c_lat = unproject_xy(final_cx, final_cy)
    c_z = base_extrude + elevation_at_lon_lat(c_lon, c_lat)
    avg_rel = sum(elev_samples) / max(1, len(elev_samples))

    return {
        "center": (final_cx, final_cy, c_z),
        "area": total_area,
        "avg_elev": avg_rel,
        "terrain_type": classify_terrain(avg_rel),
    }


def add_ribbon_segment_to_bmesh(
    bm: bmesh.types.BMesh,
    p0: tuple[float, float],
    p1: tuple[float, float],
    width: float,
    z0: float,
    z1: float,
):
    """Agrega un segmento de cinta 3D plana/inclinada entre p0 y p1 con ancho `width`."""
    dx = p1[0] - p0[0]
    dy = p1[1] - p0[1]
    length = math.hypot(dx, dy)
    if length < 1e-4:
        return
    nx = -dy / length * (width * 0.5)
    ny = dx / length * (width * 0.5)

    v0 = bm.verts.new((p0[0] - nx, p0[1] - ny, z0))
    v1 = bm.verts.new((p0[0] + nx, p0[1] + ny, z0))
    v2 = bm.verts.new((p1[0] + nx, p1[1] + ny, z1))
    v3 = bm.verts.new((p1[0] - nx, p1[1] - ny, z1))
    try:
        bm.faces.new((v0, v1, v2, v3))
    except ValueError:
        pass


def build_borders_layers(
    border_segments: list[tuple[str, tuple[float, float], tuple[float, float]]],
    base_extrude: float,
    collection,
):
    """
    Clasifica cada segmento fronterizo mediante hash espacial:
    - Frontera Internacional (dos países distintos vecinos): cinta gruesa oscura
    - Frontera Provincial (mismo país, dos provincias vecinas): cinta fina gris oscuro
    - Costa / Límite marítimo: trazo nítido oscuro
    """
    cell_size = 0.16
    spatial: dict[tuple[int, int], list[tuple[str, tuple[float, float]]]] = {}
    for cid, p0, p1 in border_segments:
        mx = (p0[0] + p1[0]) * 0.5
        my = (p0[1] + p1[1]) * 0.5
        key = (int(math.floor(mx / cell_size)), int(math.floor(my / cell_size)))
        spatial.setdefault(key, []).append((cid, (mx, my)))

    bm_intl = bmesh.new()
    bm_prov = bmesh.new()
    bm_coast = bmesh.new()

    for cid, p0, p1 in border_segments:
        mx = (p0[0] + p1[0]) * 0.5
        my = (p0[1] + p1[1]) * 0.5
        gx = int(math.floor(mx / cell_size))
        gy = int(math.floor(my / cell_size))

        other_country_near = False
        same_country_other_edge = 0
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for n_cid, (nmx, nmy) in spatial.get((gx + dx, gy + dy), ()):
                    if math.hypot(mx - nmx, my - nmy) < 0.14:
                        if n_cid != cid:
                            other_country_near = True
                        else:
                            same_country_other_edge += 1

        lon0, lat0 = unproject_xy(p0[0], p0[1])
        lon1, lat1 = unproject_xy(p1[0], p1[1])
        e0 = base_extrude + elevation_at_lon_lat(lon0, lat0)
        e1 = base_extrude + elevation_at_lon_lat(lon1, lat1)

        if other_country_near:
            add_ribbon_segment_to_bmesh(bm_intl, p0, p1, width=0.13, z0=e0 + 0.032, z1=e1 + 0.032)
        elif same_country_other_edge >= 2:
            add_ribbon_segment_to_bmesh(bm_prov, p0, p1, width=0.055, z0=e0 + 0.020, z1=e1 + 0.020)
        else:
            add_ribbon_segment_to_bmesh(bm_coast, p0, p1, width=0.085, z0=e0 + 0.026, z1=e1 + 0.026)

    def finalize_border(bm, obj_name, mat_name, rgb, width_rough=0.92):
        if not bm.faces:
            bm.free()
            return
        bmesh.ops.triangulate(bm, faces=list(bm.faces))
        mesh = bpy.data.meshes.new(f"{obj_name}_MESH")
        bm.to_mesh(mesh)
        bm.free()
        obj = bpy.data.objects.new(obj_name, mesh)
        obj.data.materials.append(make_material(mat_name, rgb, 1.0, roughness=width_rough, specular=0.05))
        collection.objects.link(obj)

    finalize_border(bm_intl, "BORDERS_INTERNATIONAL", "MAT_BORDER_INTL", [18, 18, 22])
    finalize_border(bm_prov, "BORDERS_PROVINCIAL", "MAT_BORDER_PROV", [42, 50, 62])
    finalize_border(bm_coast, "BORDERS_COASTLINE", "MAT_BORDER_COAST", [14, 24, 38])


def build_rivers_layer(rivers_path: str, base_extrude: float, collection):
    """Construye los principales ríos latinoamericanos sobre el relieve 3D."""
    if not os.path.exists(rivers_path):
        return
    with open(rivers_path, "r", encoding="utf-8") as f:
        rivers_geo = json.load(f)

    bm = bmesh.new()
    river_count = 0

    for feat in rivers_geo.get("features", []):
        props = feat.get("properties", {})
        scalerank = props.get("scalerank")
        if scalerank is None or float(scalerank) > 5.0:
            continue

        geom = feat.get("geometry") or {}
        gtype = geom.get("type")
        coords = geom.get("coordinates") or []

        lines = []
        if gtype == "LineString":
            lines.append(coords)
        elif gtype == "MultiLineString":
            lines.extend(coords)

        width = 0.105 if float(scalerank) <= 2.0 else (0.07 if float(scalerank) <= 4.0 else 0.048)

        for line in lines:
            if len(line) < 2:
                continue
            # Filtrar dentro del rectángulo de Latinoamérica
            pts_ll = [(float(c[0]), float(c[1])) for c in line]
            if not any(LATAM_LON_MIN <= lon <= LATAM_LON_MAX and LATAM_LAT_MIN <= lat <= LATAM_LAT_MAX for lon, lat in pts_ll):
                continue

            pts_ll = douglas_peucker(pts_ll, 0.04)
            river_count += 1
            for i in range(len(pts_ll) - 1):
                lon0, lat0 = pts_ll[i]
                lon1, lat1 = pts_ll[i + 1]
                if not (LATAM_LON_MIN <= lon0 <= LATAM_LON_MAX and LATAM_LAT_MIN <= lat0 <= LATAM_LAT_MAX):
                    continue
                p0 = project_lon_lat(lon0, lat0)
                p1 = project_lon_lat(lon1, lat1)
                z0 = base_extrude + elevation_at_lon_lat(lon0, lat0) + 0.028
                z1 = base_extrude + elevation_at_lon_lat(lon1, lat1) + 0.028
                add_ribbon_segment_to_bmesh(bm, p0, p1, width=width, z0=z0, z1=z1)

    if bm.faces:
        bmesh.ops.triangulate(bm, faces=list(bm.faces))
        mesh = bpy.data.meshes.new("RIVERS_LATAM_MESH")
        bm.to_mesh(mesh)
        obj = bpy.data.objects.new("RIVERS_LATAM", mesh)
        obj.data.materials.append(
            make_material("MAT_RIVERS", [44, 156, 232], 1.0, roughness=0.32, specular=0.35)
        )
        collection.objects.link(obj)
    bm.free()
    print(f"[Ríos] Trazados {river_count} tramos fluviales principales.")


def create_bathymetric_ocean_and_grid(land_points: list[tuple[float, float]], collection):
    """
    Crea un océano ajustado al marco de Latinoamérica con batimetría costera mediante
    colores por vértice (`COLOR_0`), cuadrícula cartográfica (meridianos/paralelos/Ecuador/Trópicos)
    y marco exterior táctico.
    """
    min_x, min_y = project_lon_lat(LATAM_LON_MIN, LATAM_LAT_MIN)
    max_x, max_y = project_lon_lat(LATAM_LON_MAX, LATAM_LAT_MAX)

    # Indexar puntos costeros para estimar distancia rápida a costa
    cell = 1.2
    land_hash: dict[tuple[int, int], list[tuple[float, float]]] = {}
    for lx, ly in land_points:
        key = (int(math.floor(lx / cell)), int(math.floor(ly / cell)))
        land_hash.setdefault(key, []).append((lx, ly))

    def dist_to_land(x: float, y: float) -> float:
        gx = int(math.floor(x / cell))
        gy = int(math.floor(y / cell))
        best = 99.0
        for dx in range(-4, 5):
            for dy in range(-4, 5):
                for lx, ly in land_hash.get((gx + dx, gy + dy), ()):
                    d = math.hypot(x - lx, y - ly)
                    if d < best:
                        best = d
        return best

    nx, ny = 100, 100
    bm = bmesh.new()
    grid_verts = []
    for iy in range(ny + 1):
        row = []
        ty = iy / ny
        y = min_y + ty * (max_y - min_y)
        for ix in range(nx + 1):
            tx = ix / nx
            x = min_x + tx * (max_x - min_x)
            row.append(bm.verts.new((x, y, -0.02)))
        grid_verts.append(row)

    for iy in range(ny):
        for ix in range(nx):
            bm.faces.new((
                grid_verts[iy][ix],
                grid_verts[iy][ix + 1],
                grid_verts[iy + 1][ix + 1],
                grid_verts[iy + 1][ix],
            ))

    color_layer = bm.loops.layers.color.new("Color")
    shallow = srgb_to_linear_rgba([28, 104, 148])
    mid = srgb_to_linear_rgba([14, 54, 88])
    deep = srgb_to_linear_rgba([7, 24, 44])

    def lerp_rgba(a, b, t):
        t = max(0.0, min(1.0, t))
        return (
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            1.0,
        )

    # Precomputar color por vértice (actualizar índices antes de indexar)
    bm.verts.index_update()
    vert_colors = {}
    for v in bm.verts:
        d = dist_to_land(v.co.x, v.co.y)
        if d < 1.5:
            col = lerp_rgba(shallow, mid, d / 1.5)
        elif d < 5.0:
            col = lerp_rgba(mid, deep, (d - 1.5) / 3.5)
        else:
            col = deep
        vert_colors[v.index] = col

    for f in bm.faces:
        for loop in f.loops:
            loop[color_layer] = vert_colors[loop.vert.index]

    bmesh.ops.triangulate(bm, faces=list(bm.faces))
    ocean_mesh = bpy.data.meshes.new("OCEAN_BASE_MESH")
    bm.to_mesh(ocean_mesh)
    bm.free()

    ocean_obj = bpy.data.objects.new("OCEAN_BASE", ocean_mesh)
    ocean_obj.data.materials.append(make_vertex_color_material("MAT_OCEAN_BATHYMETRY"))
    collection.objects.link(ocean_obj)

    # Cuadrícula cartográfica (meridianos y paralelos cada 10°, más Ecuador y Trópicos)
    bm_grid = bmesh.new()
    bm_tropics = bmesh.new()

    for lon in range(-110, -30, 10):
        p0 = project_lon_lat(float(lon), LATAM_LAT_MIN)
        p1 = project_lon_lat(float(lon), LATAM_LAT_MAX)
        add_ribbon_segment_to_bmesh(bm_grid, p0, p1, width=0.06, z0=0.002, z1=0.002)

    for lat in range(-50, 35, 10):
        p0 = project_lon_lat(LATAM_LON_MIN, float(lat))
        p1 = project_lon_lat(LATAM_LON_MAX, float(lat))
        target_bm = bm_tropics if lat == 0 else bm_grid
        w = 0.11 if lat == 0 else 0.06
        add_ribbon_segment_to_bmesh(target_bm, p0, p1, width=w, z0=0.004, z1=0.004)

    for tropic_lat in (-23.44, 23.44):
        p0 = project_lon_lat(LATAM_LON_MIN, tropic_lat)
        p1 = project_lon_lat(LATAM_LON_MAX, tropic_lat)
        add_ribbon_segment_to_bmesh(bm_tropics, p0, p1, width=0.08, z0=0.004, z1=0.004)

    # Marco perimetral cartográfico en latón oscuro
    corners = [(min_x, min_y), (max_x, min_y), (max_x, max_y), (min_x, max_y)]
    for i in range(4):
        add_ribbon_segment_to_bmesh(bm_tropics, corners[i], corners[(i + 1) % 4], width=0.35, z0=0.02, z1=0.02)

    if bm_grid.faces:
        bmesh.ops.triangulate(bm_grid, faces=list(bm_grid.faces))
        gm = bpy.data.meshes.new("MAP_GRID_LINES_MESH")
        bm_grid.to_mesh(gm)
        go = bpy.data.objects.new("MAP_GRID_LINES", gm)
        go.data.materials.append(make_material("MAT_MAP_GRID", [38, 86, 128], 1.0, roughness=0.8, specular=0.1))
        collection.objects.link(go)
    bm_grid.free()

    if bm_tropics.faces:
        bmesh.ops.triangulate(bm_tropics, faces=list(bm_tropics.faces))
        tm = bpy.data.meshes.new("MAP_TROPICS_FRAME_MESH")
        bm_tropics.to_mesh(tm)
        to = bpy.data.objects.new("MAP_TROPICS_FRAME", tm)
        to.data.materials.append(make_material("MAT_TROPICS_BRASS", [186, 148, 72], 1.0, roughness=0.55, specular=0.25))
        collection.objects.link(to)
    bm_tropics.free()


def create_capital_marker(
    node_name: str,
    pos_xyz: tuple[float, float, float],
    is_national: bool,
    material,
    collection,
    extras: dict,
):
    """
    Crea marcadores 3D claramente visibles desde vista continental:
    - Capitales nacionales: pedestal octagonal + aguja obelisco dorada alta.
    - Capitales provinciales: disco base + pin táctico plateado/cian.
    """
    bm = bmesh.new()
    if is_national:
        # Base pedestal
        bmesh.ops.create_cone(
            bm,
            cap_ends=True,
            cap_tris=True,
            segments=8,
            radius1=0.46,
            radius2=0.38,
            depth=0.16,
            matrix=Matrix.Translation((0.0, 0.0, 0.08)),
        )
        # Torre obelisco
        bmesh.ops.create_cone(
            bm,
            cap_ends=True,
            cap_tris=True,
            segments=6,
            radius1=0.24,
            radius2=0.04,
            depth=1.05,
            matrix=Matrix.Translation((0.0, 0.0, 0.68)),
        )
    else:
        bmesh.ops.create_cone(
            bm,
            cap_ends=True,
            cap_tris=True,
            segments=8,
            radius1=0.22,
            radius2=0.18,
            depth=0.08,
            matrix=Matrix.Translation((0.0, 0.0, 0.04)),
        )
        bmesh.ops.create_cone(
            bm,
            cap_ends=True,
            cap_tris=True,
            segments=6,
            radius1=0.12,
            radius2=0.02,
            depth=0.45,
            matrix=Matrix.Translation((0.0, 0.0, 0.30)),
        )

    mesh = bpy.data.meshes.new(f"{node_name}_MESH")
    bm.to_mesh(mesh)
    bm.free()

    obj = bpy.data.objects.new(node_name, mesh)
    obj.location = Vector((pos_xyz[0], pos_xyz[1], pos_xyz[2]))
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
        "  Conecta cada provincia, su tipo de terreno y cada capital del modelo 3D (`latinoamerica_3d.glb`)",
        "  con la lógica de simulación geopolítica en Rust.",
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
            f'terrain_type = "{lua_escape(p["terrain_type"])}", '
            f'elevation_avg = {p["avg_elev"]:.3f}, '
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
    rivers_path = os.path.join(args.cache_dir, "ne_10m_rivers_lake_centerlines.geojson")
    download_if_needed(URL_ADMIN1_GEOJSON, admin1_path)
    download_if_needed(URL_CITIES_GEOJSON, cities_path)
    download_if_needed(URL_RIVERS_GEOJSON, rivers_path)

    with open(admin1_path, "r", encoding="utf-8") as f:
        admin1_geo = json.load(f)
    with open(cities_path, "r", encoding="utf-8") as f:
        cities_geo = json.load(f)

    clear_blender_scene()

    col_world = get_or_create_collection("LATAM_WORLD")
    col_provs = get_or_create_collection("PROVINCES", col_world)
    col_borders = get_or_create_collection("BORDERS_AND_RIVERS", col_world)
    col_caps = get_or_create_collection("CAPITALS", col_world)

    mat_nat_cap = make_material(
        "MAT_CAP_NATIONAL",
        [255, 200, 55],
        color_scale=1.0,
        roughness=0.28,
        specular=0.5,
        emission_strength=0.35,
    )
    mat_prov_cap = make_material(
        "MAT_CAP_PROVINCIAL",
        [210, 238, 255],
        color_scale=1.0,
        roughness=0.35,
        specular=0.4,
        emission_strength=0.18,
    )

    provinces_records = []
    country_counters: dict[str, int] = {}
    border_segments = []
    land_points = []

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
        # Variación sutil estrictamente <= 1.0 para que ningún color claro se queme a blanco
        variation = 0.80 + ((idx % 5) * 0.045)
        mat_prov = make_material(
            f"MAT_{mesh_name}",
            base_rgb,
            color_scale=variation,
            roughness=0.88,
            specular=0.10,
        )

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
            border_segments_out=border_segments,
            land_points_out=land_points,
            country_id=iso2,
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
                    "terrain_type": built["terrain_type"],
                    "avg_elev": built["avg_elev"],
                    "area_weight": built["area"],
                    "cx": cx,
                    "cy": cy,
                    "cz": cz,
                }
            )

    # Construir océano batimétrico, cuadrícula cartográfica, bordes nítidos y ríos principales
    create_bathymetric_ocean_and_grid(land_points, col_world)
    build_borders_layers(border_segments, args.extrude_height, col_borders)
    build_rivers_layer(rivers_path, args.extrude_height, col_borders)

    # Capitales nacionales y provinciales sobre la elevación real del terreno
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
        z = args.extrude_height + elevation_at_lon_lat(lon, lat)
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
