//! Registro de eventos (apartado 22 y Anexo B.6).
//!
//! Una entrada por línea, texto estructurado conforme a RFC 8259, solo anexado y
//! encadenado por resumen criptográfico. El encadenamiento no impide la
//! manipulación del registro; hace detectable la supresión de cualquier entrada
//! intermedia, que es la propiedad exigible a un registro auditable.

use crate::clock;
use crate::error::{Error, Result};
use crate::integrity::digest_bytes;
use serde_json::{json, Map as JsonMap, Value};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Ruta del registro dentro del repositorio.
pub const LOG_RELATIVE: &str = "00_SYSTEM/log/events.jsonl";

/// Nombres normativos de los eventos (Tabla B.1).
pub mod event {
    pub const PACKAGE_BUILT: &str = "exchange.package.built";
    pub const PACKAGE_EMITTED: &str = "exchange.package.emitted";
    pub const KEY_SENT: &str = "exchange.key.sent";
    pub const PACKAGE_RECEIVED: &str = "exchange.package.received";
    pub const PACKAGE_VERIFIED: &str = "exchange.package.verified";
    pub const RECEIPT_ISSUED: &str = "exchange.receipt.issued";
    pub const RECEIPT_RECEIVED: &str = "exchange.receipt.received";
    pub const PACKAGE_INGESTED: &str = "exchange.package.ingested";
    pub const PACKAGE_REJECTED: &str = "exchange.package.rejected";
    pub const PACKAGE_SUPERSEDED: &str = "exchange.package.superseded";
    pub const REVOCATION_ISSUED: &str = "exchange.revocation.issued";
    pub const MATERIAL_DESTROYED: &str = "exchange.material.destroyed";

    pub const CUSTODY_TRANSFERRED: &str = "custody.transferred";
    pub const CUSTODY_ASSUMED: &str = "custody.assumed";
    pub const CUSTODY_RETURNED: &str = "custody.returned";
    pub const CUSTODY_RETURN_CLAIMED: &str = "custody.return.claimed";
    pub const CUSTODY_RECLAIMED: &str = "custody.reclaimed";
    pub const CUSTODY_DIVERGENCE_OPENED: &str = "custody.divergence.opened";

    pub const PROJECT_CREATED: &str = "project.created";
    pub const PROJECT_ID_CHANGED: &str = "project.id.changed";
    pub const PROJECT_DERIVED: &str = "project.derived";
    pub const PROJECT_SEALED: &str = "project.sealed";

    pub const REPLICA_ACTIVATED: &str = "replica.activated";
    pub const REPLICA_DIVERGENCE_DETECTED: &str = "replica.divergence.detected";

    pub const RELEASE_CREATED: &str = "release.created";
    pub const RELEASE_TRACK_LINKED: &str = "release.track.linked";

    // Eventos del apartado 22.1 sin nombre fijado en la Tabla B.1. Se emplean
    // los mismos espacios de nombres para que el registro sea homogéneo.
    pub const PROJECT_STATUS_CHANGED: &str = "project.status.changed";
    pub const PROJECT_LEVEL_CHANGED: &str = "project.level.changed";
    pub const PROJECT_ARCHIVED: &str = "project.archived";
    pub const MATERIAL_IMPORTED: &str = "project.material.imported";
    pub const EXPORT_PRODUCED: &str = "project.export.produced";
    pub const INTEGRITY_VERIFIED: &str = "integrity.verified";
    pub const VOLUME_CONNECTED: &str = "volume.connected";
    pub const VOLUME_DISCONNECTED: &str = "volume.disconnected";
    pub const VOLUME_RECONCILED: &str = "volume.reconciled";
    pub const NONCONFORMITY_OPENED: &str = "nonconformity.opened";
    pub const NONCONFORMITY_CLOSED: &str = "nonconformity.closed";
    pub const ACCESS_GRANTED: &str = "access.granted";
    pub const ACCESS_REVOKED: &str = "access.revoked";
    pub const CLOCK_DRIFT_DETECTED: &str = "clock.drift.detected";
}

/// Entrada del registro (Anexo B.6).
#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub ts: String,
    pub actor: String,
    pub event: String,
    pub project: Option<String>,
    pub detail: Value,
    /// Resumen de la entrada anterior. Vacío en la primera.
    pub prev: String,
    /// Resumen de esta entrada.
    pub hash: String,
}

impl LogEntry {
    /// Cuerpo canónico sobre el que se calcula el resumen: la entrada completa
    /// salvo el propio campo `hash`, con las claves en orden fijo.
    fn canonical_body(&self) -> Vec<u8> {
        let mut m = JsonMap::new();
        m.insert("ts".into(), json!(self.ts));
        m.insert("actor".into(), json!(self.actor));
        m.insert("event".into(), json!(self.event));
        m.insert("project".into(), json!(self.project));
        m.insert("detail".into(), canonicalize(&self.detail));
        m.insert("prev".into(), json!(self.prev));
        serde_json::to_vec(&Value::Object(m)).unwrap_or_default()
    }

    fn compute_hash(&self) -> String {
        digest_bytes(&self.canonical_body())
    }

    /// Serializa la entrada como una única línea.
    pub fn render(&self) -> String {
        let mut m = JsonMap::new();
        m.insert("ts".into(), json!(self.ts));
        m.insert("actor".into(), json!(self.actor));
        m.insert("event".into(), json!(self.event));
        m.insert("project".into(), json!(self.project));
        m.insert("detail".into(), canonicalize(&self.detail));
        m.insert("prev".into(), json!(self.prev));
        m.insert("hash".into(), json!(self.hash));
        serde_json::to_string(&Value::Object(m)).unwrap_or_default()
    }

    pub fn parse(line: &str) -> Option<LogEntry> {
        let v: Value = serde_json::from_str(line).ok()?;
        let o = v.as_object()?;
        Some(LogEntry {
            ts: o.get("ts")?.as_str()?.to_string(),
            actor: o.get("actor")?.as_str().unwrap_or_default().to_string(),
            event: o.get("event")?.as_str()?.to_string(),
            project: o
                .get("project")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            detail: o.get("detail").cloned().unwrap_or(Value::Null),
            prev: o.get("prev").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            hash: o.get("hash").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        })
    }
}

/// Ordena las claves de un valor JSON de forma recursiva, para que el resumen no
/// dependa del orden de inserción.
fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Object(o) => {
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            let mut m = JsonMap::new();
            for k in keys {
                m.insert(k.clone(), canonicalize(&o[k]));
            }
            Value::Object(m)
        }
        Value::Array(a) => Value::Array(a.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

/// Registro de eventos de un repositorio.
pub struct EventLog {
    path: PathBuf,
}

impl EventLog {
    /// Abre el registro de un repositorio. No crea el archivo hasta la primera
    /// escritura.
    pub fn at(repo_root: &Path) -> EventLog {
        EventLog {
            path: repo_root.join(LOG_RELATIVE),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Anexa una entrada, encadenándola con la anterior.
    pub fn append(
        &self,
        actor: &str,
        event: &str,
        project: Option<&str>,
        detail: Value,
    ) -> Result<LogEntry> {
        let prev = self.last_hash()?;
        let mut entry = LogEntry {
            ts: clock::now_rfc3339(),
            actor: actor.to_string(),
            event: event.to_string(),
            project: project.map(str::to_string),
            detail,
            prev,
            hash: String::new(),
        };
        entry.hash = entry.compute_hash();

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        // Solo anexado: se abre en modo `append`, nunca en modo truncado.
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| Error::io(&self.path, e))?;
        let mut line = entry.render();
        line.push('\n');
        f.write_all(line.as_bytes())
            .map_err(|e| Error::io(&self.path, e))?;
        // Una entrada que no llegó al disco no acredita nada.
        f.sync_all().map_err(|e| Error::io(&self.path, e))?;
        Ok(entry)
    }

    fn last_hash(&self) -> Result<String> {
        if !self.path.exists() {
            return Ok(String::new());
        }
        let f = std::fs::File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        let mut last = String::new();
        for line in BufReader::new(f).lines() {
            let line = line.map_err(|e| Error::io(&self.path, e))?;
            if let Some(entry) = LogEntry::parse(&line) {
                last = entry.hash;
            }
        }
        Ok(last)
    }

    /// Lee todas las entradas.
    pub fn entries(&self) -> Result<Vec<LogEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let f = std::fs::File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        let mut out = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line.map_err(|e| Error::io(&self.path, e))?;
            if line.trim().is_empty() {
                continue;
            }
            match LogEntry::parse(&line) {
                Some(e) => out.push(e),
                None => {
                    return Err(Error::input(format!(
                        "El registro de eventos contiene una línea que no se ajusta a RFC 8259. El registro no se ha leído por completo. Revisar {}.",
                        self.path.display()
                    )))
                }
            }
        }
        Ok(out)
    }

    /// Entradas relativas a un proyecto.
    pub fn entries_for(&self, project_uid: &str) -> Result<Vec<LogEntry>> {
        Ok(self
            .entries()?
            .into_iter()
            .filter(|e| e.project.as_deref() == Some(project_uid))
            .collect())
    }

    /// Comprueba la cadena de resúmenes.
    pub fn verify_chain(&self) -> Result<ChainCheck> {
        let entries = self.entries()?;
        let mut check = ChainCheck {
            total: entries.len(),
            ..Default::default()
        };
        let mut expected_prev = String::new();
        for (i, entry) in entries.iter().enumerate() {
            if entry.prev != expected_prev {
                check.broken_links.push(i + 1);
            }
            if entry.compute_hash() != entry.hash {
                check.altered.push(i + 1);
            }
            expected_prev = entry.hash.clone();
        }
        Ok(check)
    }
}

/// Resultado de la comprobación del encadenamiento.
#[derive(Clone, Debug, Default)]
pub struct ChainCheck {
    pub total: usize,
    /// Números de línea cuyo campo `prev` no corresponde a la entrada anterior.
    /// Indican supresión o reordenación.
    pub broken_links: Vec<usize>,
    /// Números de línea cuyo contenido no corresponde a su propio resumen.
    pub altered: Vec<usize>,
}

impl ChainCheck {
    pub fn is_intact(&self) -> bool {
        self.broken_links.is_empty() && self.altered.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_entrada_reproduce_el_esquema_del_anexo_b_6() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        let e = log
            .append(
                "J. Duarte",
                event::PACKAGE_EMITTED,
                Some("01J9ZQ8F3K7N2VYB4T6XM0RSAE"),
                json!({"shipment_id": "20260806-AAAA", "profile": "E", "files": 128}),
            )
            .unwrap();

        let linea = e.render();
        let v: Value = serde_json::from_str(&linea).unwrap();
        for campo in ["ts", "actor", "event", "project", "detail", "prev", "hash"] {
            assert!(v.get(campo).is_some(), "falta {campo}");
        }
        assert_eq!(v["event"], "exchange.package.emitted");
        assert!(clock::parse_rfc3339(v["ts"].as_str().unwrap()).is_ok());
        assert_eq!(v["hash"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn el_registro_es_una_entrada_por_linea() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        for i in 0..5 {
            log.append("a", event::PROJECT_CREATED, Some("U"), json!({"n": i}))
                .unwrap();
        }
        let texto = std::fs::read_to_string(log.path()).unwrap();
        assert_eq!(texto.lines().count(), 5);
        assert!(texto.ends_with('\n'));
        for linea in texto.lines() {
            assert!(serde_json::from_str::<Value>(linea).is_ok());
        }
    }

    #[test]
    fn cada_entrada_encadena_con_la_anterior() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        let a = log.append("a", event::PROJECT_CREATED, None, json!({})).unwrap();
        let b = log.append("a", event::PROJECT_SEALED, None, json!({})).unwrap();
        assert_eq!(a.prev, "", "la primera entrada no tiene anterior");
        assert_eq!(b.prev, a.hash);
        assert!(log.verify_chain().unwrap().is_intact());
    }

    #[test]
    fn la_supresion_de_una_entrada_intermedia_es_detectable() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        for i in 0..4 {
            log.append("a", event::PROJECT_CREATED, None, json!({"n": i}))
                .unwrap();
        }
        assert!(log.verify_chain().unwrap().is_intact());

        // Se suprime la segunda entrada, como haría quien intentase ocultarla.
        let texto = std::fs::read_to_string(log.path()).unwrap();
        let lineas: Vec<&str> = texto.lines().collect();
        let mutilado = format!("{}\n{}\n{}\n", lineas[0], lineas[2], lineas[3]);
        std::fs::write(log.path(), mutilado).unwrap();

        let c = log.verify_chain().unwrap();
        assert!(!c.is_intact());
        assert_eq!(c.broken_links, vec![2], "la línea 2 deja de encadenar");
    }

    #[test]
    fn la_alteracion_del_contenido_es_detectable() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        log.append("a", event::PACKAGE_EMITTED, None, json!({"files": 128}))
            .unwrap();
        let texto = std::fs::read_to_string(log.path()).unwrap();
        std::fs::write(log.path(), texto.replace("128", "999")).unwrap();
        let c = log.verify_chain().unwrap();
        assert_eq!(c.altered, vec![1]);
    }

    #[test]
    fn el_resumen_no_depende_del_orden_de_las_claves() {
        let base = LogEntry {
            ts: "2026-08-06T10:00:00-05:00".into(),
            actor: "a".into(),
            event: "x".into(),
            project: None,
            detail: serde_json::from_str(r#"{"b":1,"a":2}"#).unwrap(),
            prev: String::new(),
            hash: String::new(),
        };
        let otro = LogEntry {
            detail: serde_json::from_str(r#"{"a":2,"b":1}"#).unwrap(),
            ..base.clone()
        };
        assert_eq!(base.compute_hash(), otro.compute_hash());
    }

    #[test]
    fn filtra_por_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        let log = EventLog::at(dir.path());
        log.append("a", event::PROJECT_CREATED, Some("U1"), json!({})).unwrap();
        log.append("a", event::PROJECT_CREATED, Some("U2"), json!({})).unwrap();
        log.append("a", event::PROJECT_SEALED, Some("U1"), json!({})).unwrap();
        assert_eq!(log.entries_for("U1").unwrap().len(), 2);
        assert_eq!(log.entries_for("U2").unwrap().len(), 1);
    }
}
