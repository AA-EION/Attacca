//! Marcador de réplica `REPLICA.hold` (Anexo B.8 y apartado 6.6.5).
//!
//! Toda réplica cuyo estado no sea activa lo declara en texto plano, en la raíz
//! del proyecto. Cuando el sistema de archivos no puede sostener el atributo de
//! solo lectura, este marcador es lo único que advierte a la persona usuaria; el
//! apartado 6.6.2 lo exige precisamente para ese caso.

use crate::error::{Error, Result};
use crate::fsx::atomic;
use crate::replica::ReplicaState;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicaHold {
    pub state: ReplicaState,
    /// Etiqueta legible del volumen que aloja la réplica activa.
    pub active_label: String,
    /// Identificador persistente de ese volumen.
    pub active_uuid: String,
    /// Última sincronización, conforme a RFC 3339.
    pub last_synced: String,
}

impl ReplicaHold {
    pub fn render(&self) -> String {
        format!(
            "STAVE REPLICA HOLD\n\
             \n\
             estado:             {}\n\
             replica activa en:  {} (uuid {})\n\
             ultima sync:        {}\n\
             \n\
             Esta copia no debe modificarse. Los cambios hechos aqui no se propagan\n\
             y obligan a una reconciliacion manual.\n\
             El manifiesto PROJECT.yaml prevalece sobre este archivo.\n",
            self.state.as_str(),
            self.active_label,
            self.active_uuid,
            self.last_synced,
        )
    }

    pub fn parse(text: &str) -> Result<ReplicaHold> {
        let mut fields = std::collections::HashMap::new();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let value = v.trim();
                if !value.is_empty() {
                    fields.insert(k.trim().to_string(), value.to_string());
                }
            }
        }
        let get = |k: &str| fields.get(k).cloned().unwrap_or_default();
        let estado = get("estado");
        let state = ReplicaState::parse(&estado).ok_or_else(|| {
            Error::input(format!(
                "El marcador de réplica declara el estado «{estado}», ajeno a la Tabla 6. El marcador no se ha interpretado. Los estados admisibles son activa, en_espera, desconectada y divergente."
            ))
        })?;
        let activa = get("replica activa en");
        let (label, uuid) = match activa.rsplit_once("(uuid ") {
            Some((l, u)) => (
                l.trim().to_string(),
                u.trim_end_matches(')').trim().to_string(),
            ),
            None => (activa.clone(), String::new()),
        };
        Ok(ReplicaHold {
            state,
            active_label: label,
            active_uuid: uuid,
            last_synced: get("ultima sync"),
        })
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        atomic::write_str(path, &self.render())
    }
}

/// Escribe el marcador en la raíz de una réplica no activa.
pub fn write_marker(project_root: &Path, hold: &ReplicaHold) -> Result<()> {
    hold.write(&project_root.join(super::REPLICA_HOLD_FILE))
}

/// Suprime el marcador. Se invoca cuando la réplica pasa a activa
/// (apartado 6.6.5, segundo párrafo).
pub fn remove_marker(project_root: &Path) -> Result<()> {
    let path = project_root.join(super::REPLICA_HOLD_FILE);
    if !path.exists() {
        return Ok(());
    }
    let _ = crate::fsx::readonly::set_file_readonly(&path, false);
    std::fs::remove_file(&path).map_err(|e| Error::io(&path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marcador() -> ReplicaHold {
        ReplicaHold {
            state: ReplicaState::Standby,
            active_label: "Estudio - Portable 01".into(),
            active_uuid: "8f3c1a20-0000-4000-8000-000000000001".into(),
            last_synced: "2026-08-06T17:20:00-05:00".into(),
        }
    }

    #[test]
    fn reproduce_el_formato_del_anexo_b_8() {
        let t = marcador().render();
        assert!(t.starts_with("STAVE REPLICA HOLD\n"));
        assert!(t.contains("estado:             en_espera"));
        assert!(t.contains("Estudio - Portable 01 (uuid 8f3c1a20-0000-4000-8000-000000000001)"));
        assert!(t.contains("El manifiesto PROJECT.yaml prevalece sobre este archivo."));
    }

    #[test]
    fn la_lectura_recupera_lo_escrito() {
        let o = marcador();
        assert_eq!(ReplicaHold::parse(&o.render()).unwrap(), o);
    }

    #[test]
    fn rechaza_un_estado_ajeno_a_la_tabla_6() {
        assert!(ReplicaHold::parse("estado: espejo\n").is_err());
    }

    #[test]
    fn se_escribe_y_se_suprime_al_activar() {
        let dir = tempfile::tempdir().unwrap();
        write_marker(dir.path(), &marcador()).unwrap();
        assert!(dir.path().join(super::super::REPLICA_HOLD_FILE).exists());
        remove_marker(dir.path()).unwrap();
        assert!(!dir.path().join(super::super::REPLICA_HOLD_FILE).exists());
    }
}
