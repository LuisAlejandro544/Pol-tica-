//! Propósito del archivo:
//! Módulo contenedor de cartografía vectorial y carga del mapa SVG de Latinoamérica.

pub mod svg_loader;

pub use svg_loader::{CountryMeshData, LatamVectorMap};
