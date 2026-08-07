//! Réplicas y volumen activo (apartado 6.6).
//!
//! De todas las réplicas de un proyecto, exactamente una es la activa. Solo la
//! activa se modifica. Este requisito opera dentro de la organización y es
//! independiente de la custodia editorial, que opera entre organizaciones.

use crate::clock;
use crate::custody::CustodyState;
use crate::doc::{Map, Node};
use crate::error::{Error, Result};
use crate::fsx;
use crate::manifest::project::ProjectManifest;
use crate::manifest::replica_hold::{self, ReplicaHold};
use std::path::Path;

/// Estados de réplica (Tabla 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaState {
    Active,
    Standby,
    Disconnected,
    Divergent,
}

impl ReplicaState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReplicaState::Active => "activa",
            ReplicaState::Standby => "en_espera",
            ReplicaState::Disconnected => "desconectada",
            ReplicaState::Divergent => "divergente",
        }
    }

    pub fn parse(s: &str) -> Option<ReplicaState> {
        Some(match s {
            "activa" => ReplicaState::Active,
            "en_espera" => ReplicaState::Standby,
            "desconectada" => ReplicaState::Disconnected,
            "divergente" => ReplicaState::Divergent,
            _ => return None,
        })
    }

    /// Solo la réplica activa admite escritura (Tabla 6).
    pub fn allows_write(&self) -> bool {
        matches!(self, ReplicaState::Active)
    }

    /// El marcador `REPLICA.hold` existe en toda réplica no activa
    /// (apartado 6.6.5).
    pub fn requires_hold_marker(&self) -> bool {
        !self.allows_write()
    }
}

/// Una réplica declarada en el manifiesto.
#[derive(Clone, Debug, PartialEq)]
pub struct Replica {
    /// Identificador persistente del volumen.
    pub volume: String,
    /// Ruta relativa a la raíz del volumen.
    pub path: String,
    pub state: ReplicaState,
    pub last_synced: Option<String>,
}

impl Replica {
    pub fn from_node(node: &Node) -> Option<Replica> {
        let m = node.as_map()?;
        Some(Replica {
            volume: m.get("volume")?.present_str()?.to_string(),
            path: m
                .get("path")
                .and_then(|n| n.present_str())
                .unwrap_or_default()
                .to_string(),
            state: ReplicaState::parse(m.get("state")?.as_str()?)?,
            last_synced: m
                .get("last_synced")
                .and_then(|n| n.present_str())
                .map(str::to_string),
        })
    }

    pub fn to_node(&self) -> Node {
        Node::map(vec![
            ("volume", Node::str(&self.volume)),
            ("path", Node::str(&self.path)),
            ("state", Node::str(self.state.as_str())),
            ("last_synced", Node::opt_str(self.last_synced.as_deref())),
        ])
    }
}

/// Lee el inventario de réplicas de un manifiesto.
pub fn replicas_of(doc: &Map) -> Vec<Replica> {
    doc.at("replication.replicas")
        .and_then(|n| n.as_seq())
        .map(|s| s.iter().filter_map(Replica::from_node).collect())
        .unwrap_or_default()
}

/// Escribe el inventario de réplicas en un manifiesto.
pub fn set_replicas(doc: &mut Map, replicas: &[Replica]) {
    let nodes: Vec<Node> = replicas.iter().map(Replica::to_node).collect();
    doc.ensure_map("replication")
        .set("replicas", Node::Seq(nodes));
}

/// Comprueba el invariante del apartado 6.6.1: exactamente una réplica activa.
///
/// Un proyecto cuya custodia no sea propia no tiene réplica activa; en ese caso
/// la ausencia no es un incumplimiento.
pub fn check_single_active(replicas: &[Replica], custody: CustodyState) -> Result<()> {
    let activas = replicas
        .iter()
        .filter(|r| r.state == ReplicaState::Active)
        .count();
    if !custody.allows_write() {
        return if activas == 0 {
            Ok(())
        } else {
            Err(Error::requirement(
                "6.6.1",
                format!("El proyecto declara {activas} réplicas activas y su custodia es {}. Un proyecto cuya custodia no es propia no tiene réplica activa.", custody.as_str()),
            ))
        };
    }
    match activas {
        1 => Ok(()),
        0 => Err(Error::requirement(
            "6.6.1",
            "El proyecto no declara ninguna réplica activa. La escritura queda bloqueada. Conmutar una réplica a activa.",
        )),
        n => Err(Error::requirement(
            "6.6.1",
            format!("El proyecto declara {n} réplicas activas y solo puede haber una. La escritura queda bloqueada. Conmutar una sola réplica a activa."),
        )),
    }
}

/// Comprueba que se escriba únicamente en la réplica activa.
///
/// Esta comprobación precede a toda modificación de proyecto. La regla no admite
/// excepción: no se escribe en una réplica que no sea la activa bajo ninguna
/// circunstancia.
pub fn require_active_replica(project: &ProjectManifest, this_volume: &str) -> Result<()> {
    let Some(activo) = project.active_volume() else {
        // Un proyecto sin inventario de réplicas es una copia única. Su volumen
        // es implícitamente el activo.
        return Ok(());
    };
    if activo == this_volume {
        return Ok(());
    }
    let replicas = replicas_of(project.doc());
    let etiqueta = replicas
        .iter()
        .find(|r| r.volume == activo)
        .map(|r| r.path.clone())
        .unwrap_or_else(|| activo.to_string());
    Err(Error::requirement(
        "6.6.2",
        format!("La réplica activa reside en el volumen {activo} ({etiqueta}) y esta copia reside en {this_volume}. No se ha escrito nada. Conmutar la réplica activa antes de trabajar aquí."),
    ))
}

/// Informe de la conmutación de réplica activa (apartado 6.6.3).
#[derive(Clone, Debug)]
pub struct SwitchReport {
    pub from_volume: String,
    pub to_volume: String,
    pub files_verified: usize,
    pub switched_at: String,
}

/// Conmuta la réplica activa conforme a la secuencia del apartado 6.6.3.
///
/// La secuencia es: comprobar descriptores abiertos, sincronizar desde la
/// activa, verificar integridad, conmutar en un solo acto, aplicar solo lectura
/// a la anterior y registrar. Entre la conmutación y el bloqueo no se admite
/// escritura sobre ninguna de las dos réplicas.
pub fn switch_active(
    project: &mut ProjectManifest,
    from_root: &Path,
    to_root: &Path,
    to_volume: &str,
    to_volume_label: &str,
    enforces_readonly: bool,
) -> Result<SwitchReport> {
    let from_volume = project
        .active_volume()
        .map(str::to_string)
        .unwrap_or_default();

    // Paso 1: ningún archivo abierto para escritura por otro proceso.
    let deteccion = fsx::openfiles::open_for_write(from_root);
    if !deteccion.files().is_empty() {
        return Err(Error::OpenFiles(deteccion.files().to_vec()));
    }

    // Paso 2: sincronizar desde la activa hacia el destino (apartado 6.6.2,
    // cuarto guion: siempre en ese sentido).
    let entradas = fsx::walk::conserved_files(from_root);
    fsx::space::ensure_available(to_root, fsx::walk::total_bytes(&entradas))?;
    // El destino puede estar bloqueado por una conmutación anterior.
    if to_root.exists() {
        let _ = fsx::readonly::set_tree_readonly(to_root, false, &[]);
    }
    fsx::copy_tree(from_root, to_root, &[])?;

    // Paso 3: verificar la integridad del destino frente al origen.
    let esperado = crate::integrity::compute(from_root)?;
    let verificacion = crate::integrity::verify_against(to_root, &esperado)?;
    if !verificacion.is_ok() {
        return Err(Error::Integrity {
            checked: verificacion.checked,
            failed: verificacion.failed_paths(),
        });
    }

    // Paso 4: conmutar en un solo acto. El manifiesto se reescribe una vez, con
    // ambos estados ya modificados.
    let ahora = clock::now_rfc3339();
    let mut replicas = replicas_of(project.doc());
    for r in replicas.iter_mut() {
        if r.volume == to_volume {
            r.state = ReplicaState::Active;
            r.last_synced = Some(ahora.clone());
        } else if r.state == ReplicaState::Active {
            r.state = ReplicaState::Standby;
            r.last_synced = Some(ahora.clone());
        }
    }
    if !replicas.iter().any(|r| r.volume == to_volume) {
        replicas.push(Replica {
            volume: to_volume.to_string(),
            path: fsx::walk::relative_slash(to_root, to_root).unwrap_or_default(),
            state: ReplicaState::Active,
            last_synced: Some(ahora.clone()),
        });
    }
    set_replicas(project.doc_mut(), &replicas);
    project
        .doc_mut()
        .ensure_map("replication")
        .set("active_volume", Node::str(to_volume));

    // El manifiesto del destino es el que pasa a ser autoritativo.
    let destino_manifest = to_root.join(crate::manifest::PROJECT_FILE);
    let mut copia = ProjectManifest::new(&destino_manifest);
    copia.doc_mut().clone_from(project.doc());
    copia.save()?;
    project.save()?;

    // Paso 5: aplicar el régimen de solo lectura a la réplica anterior.
    replica_hold::remove_marker(to_root)?;
    if enforces_readonly {
        fsx::readonly::set_tree_readonly(from_root, true, &[crate::manifest::REPLICA_HOLD_FILE])?;
    }
    replica_hold::write_marker(
        from_root,
        &ReplicaHold {
            state: ReplicaState::Standby,
            active_label: to_volume_label.to_string(),
            active_uuid: to_volume.to_string(),
            last_synced: ahora.clone(),
        },
    )?;

    Ok(SwitchReport {
        from_volume,
        to_volume: to_volume.to_string(),
        files_verified: verificacion.checked,
        switched_at: ahora,
    })
}

/// Resultado de la reconciliación al reconectar un volumen (apartado 6.6.4).
#[derive(Clone, Debug, PartialEq)]
pub enum Reconciliation {
    /// Sin cambios posteriores: pasa a `en_espera` y se sincroniza desde la
    /// activa.
    Synchronizable,
    /// Contiene cambios posteriores a su última sincronización: se marca
    /// divergente y se abre una no conformidad. No se sobrescribe de forma
    /// automática.
    Divergent { changed: Vec<String> },
}

/// Compara una réplica reconectada con la activa mediante sus manifiestos de
/// integridad (apartado 6.6.4).
pub fn reconcile(active_root: &Path, reconnected_root: &Path) -> Result<Reconciliation> {
    let activa = crate::integrity::compute(active_root)?;
    let reconectada = crate::integrity::compute(reconnected_root)?;

    let mut cambiados = Vec::new();
    for (path, digest) in reconectada.entries() {
        match activa.digest_of(path) {
            // Un archivo presente en ambas con resumen distinto es un cambio no
            // presente en la activa.
            Some(d) if d != digest => cambiados.push(path.to_string()),
            // Un archivo que solo existe en la reconectada también lo es.
            None => cambiados.push(path.to_string()),
            _ => {}
        }
    }
    cambiados.sort();

    Ok(if cambiados.is_empty() {
        Reconciliation::Synchronizable
    } else {
        Reconciliation::Divergent { changed: cambiados }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn replica(vol: &str, state: ReplicaState) -> Replica {
        Replica {
            volume: vol.into(),
            path: "20_PROJECTS/1_ACTIVE/T".into(),
            state,
            last_synced: Some("2026-08-06T10:00:00-05:00".into()),
        }
    }

    #[test]
    fn los_estados_usan_el_vocabulario_de_la_tabla_6() {
        assert_eq!(ReplicaState::Active.as_str(), "activa");
        assert_eq!(ReplicaState::Standby.as_str(), "en_espera");
        assert_eq!(ReplicaState::Disconnected.as_str(), "desconectada");
        assert_eq!(ReplicaState::Divergent.as_str(), "divergente");
    }

    #[test]
    fn exige_exactamente_una_replica_activa() {
        let una = vec![replica("v1", ReplicaState::Active), replica("v2", ReplicaState::Standby)];
        assert!(check_single_active(&una, CustodyState::Own).is_ok());

        let dos = vec![replica("v1", ReplicaState::Active), replica("v2", ReplicaState::Active)];
        assert!(check_single_active(&dos, CustodyState::Own).is_err());

        let ninguna = vec![replica("v1", ReplicaState::Standby)];
        assert!(check_single_active(&ninguna, CustodyState::Own).is_err());
    }

    #[test]
    fn un_proyecto_cedido_no_tiene_replica_activa() {
        let ninguna = vec![replica("v1", ReplicaState::Standby)];
        assert!(check_single_active(&ninguna, CustodyState::Ceded).is_ok());
        let una = vec![replica("v1", ReplicaState::Active)];
        assert!(check_single_active(&una, CustodyState::Ceded).is_err());
    }

    #[test]
    fn el_inventario_sobrevive_a_una_ida_y_vuelta() {
        let mut doc = Map::new();
        let originales = vec![replica("v1", ReplicaState::Active), replica("v2", ReplicaState::Divergent)];
        set_replicas(&mut doc, &originales);
        assert_eq!(replicas_of(&doc), originales);
    }

    #[test]
    fn bloquea_la_escritura_en_una_replica_que_no_es_la_activa() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        p.doc_mut()
            .ensure_map("replication")
            .set("active_volume", Node::str("vol-portatil"));
        set_replicas(p.doc_mut(), &[replica("vol-portatil", ReplicaState::Active)]);

        assert!(require_active_replica(&p, "vol-portatil").is_ok());
        let e = require_active_replica(&p, "vol-local").unwrap_err();
        assert_eq!(e.clause(), Some("6.6.2"));
        assert!(e.to_string().contains("No se ha escrito nada"));
    }

    #[test]
    fn una_copia_unica_sin_inventario_admite_escritura() {
        let dir = tempfile::tempdir().unwrap();
        let p = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        assert!(require_active_replica(&p, "cualquiera").is_ok());
    }

    #[test]
    fn detecta_la_divergencia_al_reconectar() {
        let activa = tempfile::tempdir().unwrap();
        let otra = tempfile::tempdir().unwrap();
        fs::write(activa.path().join("a.wav"), b"contenido").unwrap();
        fs::write(otra.path().join("a.wav"), b"contenido").unwrap();
        assert_eq!(
            reconcile(activa.path(), otra.path()).unwrap(),
            Reconciliation::Synchronizable
        );

        // La réplica reconectada recibe un cambio que la activa no tiene.
        fs::write(otra.path().join("a.wav"), b"contenido modificado").unwrap();
        match reconcile(activa.path(), otra.path()).unwrap() {
            Reconciliation::Divergent { changed } => assert_eq!(changed, vec!["a.wav"]),
            other => panic!("se esperaba divergencia, se obtuvo {other:?}"),
        }

        // Un archivo nuevo solo en la reconectada también es divergencia.
        fs::write(otra.path().join("a.wav"), b"contenido").unwrap();
        fs::write(otra.path().join("b.wav"), b"nuevo").unwrap();
        match reconcile(activa.path(), otra.path()).unwrap() {
            Reconciliation::Divergent { changed } => assert_eq!(changed, vec!["b.wav"]),
            other => panic!("se esperaba divergencia, se obtuvo {other:?}"),
        }
    }
}
