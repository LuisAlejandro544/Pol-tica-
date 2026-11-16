//! Propósito del archivo:
//! Módulo contenedor de cartografía: visor 3D glTF (`latinoamerica_3d.glb` + `wgpu`)
//! y cargador vectorial SVG de respaldo (`latinoamerica.svg`).

pub mod glb_loader;
pub mod renderer_3d;
pub mod svg_loader;

pub use glb_loader::{Latam3dScene, MeshKind, SceneMesh3D, Vertex3D};
pub use renderer_3d::{Map3dCamera, Map3dPaintCallback};
pub use svg_loader::{CountryMeshData, LatamVectorMap};
