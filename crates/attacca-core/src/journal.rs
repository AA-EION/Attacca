//! Recuperación tras cierre inesperado (requisito 9 de robustez).
//!
//! Toda operación que escriba varios archivos deja constancia de su inicio y de
//! su conclusión. Al arrancar, las operaciones sin conclusión se presentan a la
//! persona para completarlas o revertirlas.
//!
//! El diario es una caché de recuperación, no una fuente de verdad: su pérdida
//! no invalida el repositorio, cuyo estado sigue estando en los manifiestos.

use crate::clock;
use crate::error::{Error, Result};
use crate::fsx::atomic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const JOURNAL_FILE: &str = ".attacca-journal";

/// Operación registrada en el diario.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Operation {
    pub id: String,
    /// Clave de la operación: `emitir_paquete`, `derivar_proyecto`,
    /// `conmutar_replica`, `ingerir_paquete`, `archivar_proyecto`.
    pub kind: String,
    pub started: String,
    pub project_uid: Option<String>,
    /// Rutas creadas durante la operación, para poder revertirla.
    pub artifacts: Vec<PathBuf>,
    pub detail: Value,
}

/// Diario de operaciones en curso.
pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn at(dir: &Path) -> Journal {
        Journal {
            path: dir.join(JOURNAL_FILE),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Registra el inicio de una operación.
    pub fn begin(
        &self,
        kind: &str,
        project_uid: Option<&str>,
        artifacts: &[PathBuf],
        detail: Value,
    ) -> Result<Operation> {
        let op = Operation {
            id: crate::ids::new_uid(),
            kind: kind.to_string(),
            started: clock::now_rfc3339(),
            project_uid: project_uid.map(str::to_string),
            artifacts: artifacts.to_vec(),
            detail,
        };
        let mut abiertas = self.open_operations()?;
        abiertas.push(op.clone());
        self.write(&abiertas)?;
        Ok(op)
    }

    /// Registra la conclusión de una operación.
    pub fn commit(&self, id: &str) -> Result<()> {
        let abiertas: Vec<Operation> = self
            .open_operations()?
            .into_iter()
            .filter(|o| o.id != id)
            .collect();
        self.write(&abiertas)
    }

    /// Operaciones sin conclusión registrada.
    pub fn open_operations(&self) -> Result<Vec<Operation>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let datos = std::fs::read(&self.path).map_err(|e| Error::io(&self.path, e))?;
        // Un diario ilegible no debe impedir arrancar: se descarta.
        Ok(serde_json::from_slice(&datos).unwrap_or_default())
    }

    fn write(&self, ops: &[Operation]) -> Result<()> {
        if ops.is_empty() {
            if self.path.exists() {
                std::fs::remove_file(&self.path).map_err(|e| Error::io(&self.path, e))?;
            }
            return Ok(());
        }
        let datos = serde_json::to_vec(ops)
            .map_err(|e| Error::io(&self.path, std::io::Error::other(e.to_string())))?;
        atomic::write(&self.path, &datos)
    }

    /// Revierte una operación suprimiendo lo que dejó a medias.
    ///
    /// Solo se suprimen las rutas que la propia operación creó. Nada del
    /// material anterior se toca.
    pub fn revert(&self, op: &Operation) -> Result<usize> {
        let mut suprimidos = 0usize;
        for artefacto in &op.artifacts {
            if artefacto.is_file() {
                let _ = crate::fsx::readonly::set_file_readonly(artefacto, false);
                if std::fs::remove_file(artefacto).is_ok() {
                    suprimidos += 1;
                }
            } else if artefacto.is_dir() {
                let _ = crate::fsx::readonly::set_tree_readonly(artefacto, false, &[]);
                if std::fs::remove_dir_all(artefacto).is_ok() {
                    suprimidos += 1;
                }
            }
        }
        self.commit(&op.id)?;
        Ok(suprimidos)
    }
}

/// Situación detectada al arrancar.
#[derive(Clone, Debug, Default)]
pub struct Recovery {
    /// Operaciones sin conclusión registrada.
    pub incomplete: Vec<Operation>,
    /// Archivos temporales de escritura atómica abandonados.
    pub abandoned_temps: Vec<PathBuf>,
}

impl Recovery {
    pub fn is_clean(&self) -> bool {
        self.incomplete.is_empty() && self.abandoned_temps.is_empty()
    }
}

/// Detecta operaciones a medias al arrancar.
pub fn detect(repo_root: &Path, journal_dir: &Path) -> Result<Recovery> {
    Ok(Recovery {
        incomplete: Journal::at(journal_dir).open_operations()?,
        abandoned_temps: atomic::find_abandoned(repo_root),
    })
}

/// Suprime los temporales abandonados.
///
/// Un temporal de escritura atómica nunca llegó a ser el archivo definitivo: su
/// supresión no pierde nada. El destino conserva el contenido anterior íntegro.
pub fn clear_abandoned_temps(recovery: &Recovery) -> usize {
    recovery
        .abandoned_temps
        .iter()
        .filter(|p| std::fs::remove_file(p).is_ok())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn una_operacion_concluida_no_queda_abierta() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::at(dir.path());
        let op = j.begin("emitir_paquete", Some("UID"), &[], json!({})).unwrap();
        assert_eq!(j.open_operations().unwrap().len(), 1);
        j.commit(&op.id).unwrap();
        assert!(j.open_operations().unwrap().is_empty());
        assert!(!j.path().exists(), "el diario vacío se suprime");
    }

    #[test]
    fn una_operacion_interrumpida_se_detecta_al_arrancar() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::at(dir.path());
        j.begin("derivar_proyecto", Some("UID"), &[], json!({"reason": "x"}))
            .unwrap();
        // El proceso termina de forma inesperada: no hay commit.
        let r = detect(dir.path(), dir.path()).unwrap();
        assert!(!r.is_clean());
        assert_eq!(r.incomplete.len(), 1);
        assert_eq!(r.incomplete[0].kind, "derivar_proyecto");
    }

    #[test]
    fn revertir_suprime_solo_lo_que_la_operacion_creo() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::at(dir.path());

        let anterior = dir.path().join("material-anterior.wav");
        std::fs::write(&anterior, b"no debe tocarse").unwrap();
        let creado_dir = dir.path().join("derivado");
        std::fs::create_dir_all(&creado_dir).unwrap();
        std::fs::write(creado_dir.join("a.wav"), b"a medias").unwrap();
        let creado_file = dir.path().join("paquete.stave");
        std::fs::write(&creado_file, b"truncado").unwrap();

        let op = j
            .begin("derivar_proyecto", None, &[creado_dir.clone(), creado_file.clone()], json!({}))
            .unwrap();
        let n = j.revert(&op).unwrap();

        assert_eq!(n, 2);
        assert!(!creado_dir.exists());
        assert!(!creado_file.exists());
        assert!(anterior.exists(), "el material anterior sigue intacto");
        assert!(j.open_operations().unwrap().is_empty());
    }

    #[test]
    fn detecta_y_suprime_temporales_abandonados() {
        let dir = tempfile::tempdir().unwrap();
        let huerfano = dir
            .path()
            .join(format!(".PROJECT.yaml.1.0{}", atomic::TEMP_SUFFIX));
        std::fs::write(&huerfano, b"escritura a medias").unwrap();
        let definitivo = dir.path().join("PROJECT.yaml");
        std::fs::write(&definitivo, b"uid: A\n").unwrap();

        let r = detect(dir.path(), dir.path()).unwrap();
        assert_eq!(r.abandoned_temps.len(), 1);
        assert_eq!(clear_abandoned_temps(&r), 1);
        assert!(!huerfano.exists());
        // El destino conserva su contenido anterior íntegro.
        assert_eq!(std::fs::read_to_string(&definitivo).unwrap(), "uid: A\n");
    }

    #[test]
    fn un_diario_ilegible_no_impide_arrancar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(JOURNAL_FILE), b"esto no es JSON").unwrap();
        let r = detect(dir.path(), dir.path()).unwrap();
        assert!(r.incomplete.is_empty());
    }

    #[test]
    fn el_diario_admite_varias_operaciones_simultaneas() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::at(dir.path());
        let a = j.begin("emitir_paquete", Some("U1"), &[], json!({})).unwrap();
        let b = j.begin("conmutar_replica", Some("U2"), &[], json!({})).unwrap();
        assert_eq!(j.open_operations().unwrap().len(), 2);
        j.commit(&a.id).unwrap();
        let abiertas = j.open_operations().unwrap();
        assert_eq!(abiertas.len(), 1);
        assert_eq!(abiertas[0].id, b.id);
    }
}
