//! Manifiestos (Anexo B).
//!
//! Un manifiesto se representa siempre como el documento completo leído del
//! disco, no como una estructura fija. Los accesores tipados leen y escriben
//! campos concretos sin descartar el resto: es lo que permite que un manifiesto
//! escrito por otra implementación, o por una versión posterior de la norma,
//! sobreviva a una reescritura de Attacca (apartado 44.1).

pub mod custody_lock;
pub mod exchange;
pub mod project;
pub mod receipt;
pub mod release;
pub mod replica_hold;
pub mod volume;

use crate::doc::{canonical_order, emit, parse, ArtifactKind, Map};
use crate::error::{Error, Result};
use crate::fsx::atomic;
use std::path::{Path, PathBuf};

/// Nombre del manifiesto de proyecto (apartado 7.1).
pub const PROJECT_FILE: &str = "PROJECT.yaml";
/// Nombre del manifiesto de release (apartado 8.4).
pub const RELEASE_FILE: &str = "RELEASE.yaml";
/// Nombre del manifiesto de intercambio (apartado 32.1).
pub const EXCHANGE_FILE: &str = "EXCHANGE.yaml";
/// Nombre del marcador de custodia (apartado 14.3.3).
pub const CUSTODY_LOCK_FILE: &str = "CUSTODY.lock";
/// Nombre del marcador de réplica (apartado 6.6.5).
pub const REPLICA_HOLD_FILE: &str = "REPLICA.hold";
/// Nombre del descriptor de volumen (Anexo B.5).
pub const VOLUME_FILE: &str = "VOLUME.yaml";
/// Nombre del manifiesto de integridad (apartado 17.1).
pub const INTEGRITY_FILE: &str = "MANIFEST.sha256";

/// Manifiesto cargado desde el disco, con su ruta de origen.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub path: PathBuf,
    pub doc: Map,
    kind: ArtifactKind,
}

impl Manifest {
    /// Crea un manifiesto en memoria, todavía sin escribir.
    pub fn new(path: impl Into<PathBuf>, kind: ArtifactKind) -> Self {
        Self {
            path: path.into(),
            doc: Map::new(),
            kind,
        }
    }

    /// Lee un manifiesto del disco.
    pub fn load(path: impl AsRef<Path>, kind: ArtifactKind) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        let doc = parse(&text).map_err(|e| Error::Manifest {
            path: path.to_path_buf(),
            detail: e.0,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            doc,
            kind,
        })
    }

    pub fn kind(&self) -> ArtifactKind {
        self.kind
    }

    /// Serializa el manifiesto en su forma canónica.
    ///
    /// Ordena las claves conocidas, conserva las desconocidas y registra la
    /// implementación en `stave.written_by` conforme al apartado 44.1.
    pub fn to_string_canonical(&self) -> String {
        let mut doc = self.doc.clone();
        stamp_written_by(&mut doc);
        canonical_order(&mut doc, self.kind);
        emit(&doc)
    }

    /// Escribe el manifiesto de forma atómica.
    pub fn save(&mut self) -> Result<()> {
        stamp_written_by(&mut self.doc);
        canonical_order(&mut self.doc, self.kind);
        let text = emit(&self.doc);
        atomic::write_str(&self.path, &text)
    }

    /// Escribe el manifiesto sin registrar la implementación.
    ///
    /// Se emplea al copiar un manifiesto ajeno sin modificar su contenido: el
    /// apartado 44.1 exige registrar la implementación al *modificar* un
    /// proyecto, no al trasladarlo intacto.
    pub fn save_verbatim(&mut self) -> Result<()> {
        canonical_order(&mut self.doc, self.kind);
        let text = emit(&self.doc);
        atomic::write_str(&self.path, &text)
    }
}

/// Registra el nombre y la versión de la implementación (apartado 44.1).
pub fn stamp_written_by(doc: &mut Map) {
    let block = doc.ensure_map("stave");
    if !block.contains_key("version") {
        block.set("version", crate::doc::Node::str(crate::STAVE_VERSION));
    }
    block.set("written_by", crate::doc::Node::str(crate::written_by()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Node;

    #[test]
    fn conserva_los_campos_no_comprendidos_al_reescribir() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join(PROJECT_FILE);
        // Manifiesto escrito por otra implementación, con campos que Attacca no
        // conoce, en el nivel superior y anidados.
        std::fs::write(
            &ruta,
            "stave:\n  version: \"2.0\"\n  level: B\n  written_by: OtraHerramienta 3.1\n  x_otra_marca: azul\nuid: 01J9ZQ8F3K7N2VYB4T6XM0RSAE\nid: 2026-08-06_Tema_ORIG\nx_otra_bloque:\n  campo: valor\n  lista:\n    - 1\n    - 2\naudio:\n  sample_rate: 48000\n  x_otra_latencia: 512\n",
        )
        .unwrap();

        let mut m = Manifest::load(&ruta, ArtifactKind::Project).unwrap();
        m.doc.set("title", Node::str("Tema"));
        m.save().unwrap();

        let releido = std::fs::read_to_string(&ruta).unwrap();
        assert!(releido.contains("x_otra_marca: azul"));
        assert!(releido.contains("x_otra_bloque:"));
        assert!(releido.contains("x_otra_latencia: 512"));
        assert!(releido.contains("- 1"));
        // La implementación se registra al modificar.
        assert!(releido.contains(&format!("written_by: {}", crate::written_by())));
    }

    #[test]
    fn la_escritura_repetida_produce_los_mismos_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join(PROJECT_FILE);
        let mut m = Manifest::new(&ruta, ArtifactKind::Project);
        m.doc.set("uid", Node::str("01J9ZQ8F3K7N2VYB4T6XM0RSAE"));
        m.doc.set("id", Node::str("2026-08-06_Tema_ORIG"));
        m.doc.set("tempo", Node::Null);
        m.save().unwrap();
        let primera = std::fs::read_to_string(&ruta).unwrap();

        let mut m2 = Manifest::load(&ruta, ArtifactKind::Project).unwrap();
        m2.save().unwrap();
        let segunda = std::fs::read_to_string(&ruta).unwrap();
        assert_eq!(primera, segunda);
    }

    #[test]
    fn save_verbatim_no_altera_la_implementacion_declarada() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join(PROJECT_FILE);
        let mut m = Manifest::new(&ruta, ArtifactKind::Project);
        m.doc.ensure_map("stave").set("written_by", Node::str("OtraHerramienta 3.1"));
        m.save_verbatim().unwrap();
        let texto = std::fs::read_to_string(&ruta).unwrap();
        assert!(texto.contains("written_by: OtraHerramienta 3.1"));
    }
}
