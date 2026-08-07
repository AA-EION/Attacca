//! Núcleo normativo de Attacca. Implementa STAVE 2.0.
//!
//! # Principio rector
//!
//! El apartado 42 de la norma prevalece sobre cualquier otra disposición: el
//! árbol de archivos y los manifiestos son la fuente de verdad. Todo lo que este
//! módulo mantiene en memoria o en la caché de índice es derivado y debe poder
//! reconstruirse recorriendo el árbol. Si se borra la caché, no se pierde nada.
//!
//! # Clase de conformidad
//!
//! Attacca declara la clase `M` (Gestora) de la Tabla 36. La declaración
//! completa figura en `docs/CONFORMIDAD.md` y se expone mediante
//! [`conformance::declaration`].

pub mod clock;
pub mod conformance;
pub mod custody;
pub mod doc;
pub mod error;
pub mod eventlog;
pub mod fsx;
pub mod ids;
pub mod index;
pub mod integrity;
pub mod journal;
pub mod manifest;
pub mod naming;
pub mod nonconformity;
pub mod package;
pub mod project;
pub mod release;
pub mod replica;
pub mod repo;
pub mod stage;
pub mod validate;

pub use error::{Error, Result};

/// Nombre de la implementación, para el campo `written_by` (apartado 44.1).
pub const IMPL_NAME: &str = "Attacca";

/// Versión de la implementación.
pub const IMPL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versión de la norma aplicada.
pub const STAVE_VERSION: &str = "2.0";

/// Valor del campo `written_by` de los manifiestos (apartado 44.1, segundo
/// guion): toda implementación debe registrar su nombre y su versión al
/// modificar un proyecto.
pub fn written_by() -> String {
    format!("{IMPL_NAME} {IMPL_VERSION}")
}

/// Prefijo de las extensiones propias (apartado 44.1, quinto guion).
///
/// Todo campo que Attacca introduzca fuera del Anexo B lleva este prefijo y está
/// documentado en `docs/EXTENSIONES.md`.
pub const EXT_PREFIX: &str = "x_attacca_";
