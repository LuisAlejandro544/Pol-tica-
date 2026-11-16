//! Propósito del archivo:
//! Lee `assets/maps/latinoamerica.svg` en tiempo de ejecución desde los assets incrustados,
//! interpreta cada `<path>` mediante `roxmltree` y `svgtypes`, tesela los polígonos con
//! `lyon_tessellation` (licencia MIT/Apache-2.0) para dibujarlos en GPU con `egui`,
//! y calcula detección de toques (point-in-polygon) para seleccionar países en móvil.

use egui::Pos2;
use include_dir::{include_dir, Dir};
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
    path::Path as LyonPath,
};
use roxmltree::Document;
use std::collections::BTreeMap;
use svgtypes::{PathParser, PathSegment};

/// Assets incrustados dentro del binario compilado (.so en Android).
pub static ASSETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

#[derive(Debug, Clone)]
pub struct CountryMeshData {
    pub id: String,
    pub display_name: String,
    pub outline: Vec<Pos2>,
    pub triangles_vertices: Vec<Pos2>,
    pub triangles_indices: Vec<u32>,
    pub centroid: Pos2,
    pub bbox_min: Pos2,
    pub bbox_max: Pos2,
}

#[derive(Debug, Clone)]
pub struct LatamVectorMap {
    pub width: f32,
    pub height: f32,
    pub countries: BTreeMap<String, CountryMeshData>,
}

impl LatamVectorMap {
    /// Carga y tesela el mapa vectorial `maps/latinoamerica.svg`.
    pub fn load_embedded() -> Result<Self, String> {
        let svg_file = ASSETS_DIR
            .get_file("maps/latinoamerica.svg")
            .ok_or_else(|| "No se encontró assets/maps/latinoamerica.svg".to_string())?;
        let svg_text = svg_file
            .contents_utf8()
            .ok_or_else(|| "El archivo latinoamerica.svg no es UTF-8 válido".to_string())?;

        Self::from_svg_str(svg_text)
    }

    pub fn from_svg_str(svg_text: &str) -> Result<Self, String> {
        let doc = Document::parse(svg_text).map_err(|e| format!("Error XML en SVG: {e}"))?;
        let mut countries = BTreeMap::new();
        let mut tessellator = FillTessellator::new();

        for node in doc.descendants() {
            if node.tag_name().name() != "path" {
                continue;
            }
            let Some(id) = node.attribute("id") else {
                continue;
            };
            let Some(d) = node.attribute("d") else {
                continue;
            };
            // Ignora elementos decorativos si existieran
            if id == "OCEAN" || id == "GRID" {
                continue;
            }

            let display_name = node.attribute("data-name").unwrap_or(id).to_string();
            let points = parse_polygon_points(d)?;
            if points.len() < 3 {
                continue;
            }

            let (triangles_vertices, triangles_indices) =
                tessellate_polygon(&points, &mut tessellator)?;
            let (bbox_min, bbox_max, centroid) = compute_bounds_and_centroid(&points);

            countries.insert(
                id.to_string(),
                CountryMeshData {
                    id: id.to_string(),
                    display_name,
                    outline: points,
                    triangles_vertices,
                    triangles_indices,
                    centroid,
                    bbox_min,
                    bbox_max,
                },
            );
        }

        Ok(Self {
            width: 1000.0,
            height: 1200.0,
            countries,
        })
    }

    /// Determina qué país fue tocado en coordenadas del espacio SVG (0..1000 x 0..1200).
    pub fn hit_test(&self, svg_pos: Pos2) -> Option<&str> {
        for (id, mesh) in &self.countries {
            if svg_pos.x < mesh.bbox_min.x
                || svg_pos.x > mesh.bbox_max.x
                || svg_pos.y < mesh.bbox_min.y
                || svg_pos.y > mesh.bbox_max.y
            {
                continue;
            }
            if point_in_polygon(svg_pos, &mesh.outline) {
                return Some(id.as_str());
            }
        }
        None
    }
}

fn parse_polygon_points(d: &str) -> Result<Vec<Pos2>, String> {
    let mut pts = Vec::new();
    let mut cursor = Pos2::ZERO;

    for seg in PathParser::from(d) {
        let seg = seg.map_err(|e| format!("Error parseando comando SVG path: {e}"))?;
        match seg {
            PathSegment::MoveTo { abs, x, y } => {
                cursor = if abs {
                    Pos2::new(x as f32, y as f32)
                } else {
                    Pos2::new(cursor.x + x as f32, cursor.y + y as f32)
                };
                pts.push(cursor);
            }
            PathSegment::LineTo { abs, x, y } => {
                cursor = if abs {
                    Pos2::new(x as f32, y as f32)
                } else {
                    Pos2::new(cursor.x + x as f32, cursor.y + y as f32)
                };
                pts.push(cursor);
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                cursor.x = if abs { x as f32 } else { cursor.x + x as f32 };
                pts.push(cursor);
            }
            PathSegment::VerticalLineTo { abs, y } => {
                cursor.y = if abs { y as f32 } else { cursor.y + y as f32 };
                pts.push(cursor);
            }
            PathSegment::ClosePath { .. } => {}
            _ => {}
        }
    }

    Ok(pts)
}

fn tessellate_polygon(
    points: &[Pos2],
    tessellator: &mut FillTessellator,
) -> Result<(Vec<Pos2>, Vec<u32>), String> {
    let mut builder = LyonPath::builder();
    builder.begin(lyon_tessellation::math::point(points[0].x, points[0].y));
    for p in &points[1..] {
        builder.line_to(lyon_tessellation::math::point(p.x, p.y));
    }
    builder.close();
    let lyon_path = builder.build();

    let mut buffers: VertexBuffers<Pos2, u32> = VertexBuffers::new();
    tessellator
        .tessellate_path(
            &lyon_path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
                let pos = v.position();
                Pos2::new(pos.x, pos.y)
            }),
        )
        .map_err(|e| format!("Error de teselación lyon: {e:?}"))?;

    Ok((buffers.vertices, buffers.indices))
}

fn compute_bounds_and_centroid(points: &[Pos2]) -> (Pos2, Pos2, Pos2) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut sum_x = 0.0_f32;
    let mut sum_y = 0.0_f32;

    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
        sum_x += p.x;
        sum_y += p.y;
    }

    let n = points.len().max(1) as f32;
    (
        Pos2::new(min_x, min_y),
        Pos2::new(max_x, max_y),
        Pos2::new(sum_x / n, sum_y / n),
    )
}

/// Algoritmo ray-casting estándar para detección precisa de toques dentro de polígonos cóncavos.
fn point_in_polygon(p: Pos2, poly: &[Pos2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        let intersect = ((pi.y > p.y) != (pj.y > p.y))
            && (p.x < (pj.x - pi.x) * (p.y - pi.y) / ((pj.y - pi.y) + 1e-6) + pi.x);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}
