//! No conformidades y acciones correctivas (apartado 21).
//!
//! El registro es único para todo el repositorio. El análisis de causa raíz se
//! dirige al proceso y no a las personas: un registro que atribuye
//! responsabilidades individuales deja de alimentarse en pocas semanas.

use crate::clock;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::repo::Repository;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Ruta del registro dentro del repositorio.
pub const REGISTER_RELATIVE: &str = "00_SYSTEM/log/nonconformities.jsonl";

/// Severidad (Tabla 22).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Sin impacto sobre el entregable ni sobre terceros.
    Minor,
    /// Afecta a un entregable, a un plazo o a la reconstrucción futura.
    Major,
    /// Pérdida de material, filtración, incumplimiento legal o contractual.
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Minor => "menor",
            Severity::Major => "mayor",
            Severity::Critical => "critica",
        }
    }

    /// Respuesta exigible (Tabla 22).
    pub fn requires_root_cause(&self) -> bool {
        *self >= Severity::Major
    }

    pub fn requires_notification(&self) -> bool {
        *self == Severity::Critical
    }
}

/// Origen del hallazgo (Tabla 21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    QualityControl,
    Audit,
    OperationalIncident,
    ShipmentReception,
    ClientClaim,
    Spontaneous,
}

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Origin::QualityControl => "control_calidad",
            Origin::Audit => "auditoria",
            Origin::OperationalIncident => "incidente_operativo",
            Origin::ShipmentReception => "recepcion_envio",
            Origin::ClientClaim => "reclamacion_cliente",
            Origin::Spontaneous => "hallazgo_espontaneo",
        }
    }
}

/// Entrada del registro, con los campos de la Tabla 21.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonConformity {
    /// Único e irrepetible, con la fecha de apertura.
    pub id: String,
    pub opened: String,
    pub origin: Origin,
    pub severity: Severity,
    /// Proyectos, envíos, entregas o volúmenes afectados.
    pub scope: Vec<String>,
    /// Hecho observado, expresado sin interpretación.
    pub description: String,
    /// Medida inmediata adoptada para limitar el daño.
    pub containment: Option<String>,
    /// Análisis de las condiciones del proceso que hicieron posible el hecho.
    pub root_cause: Option<String>,
    /// Cambio que impide la repetición, con responsable y plazo.
    pub corrective_action: Option<String>,
    pub action_owner: Option<String>,
    pub action_due: Option<String>,
    /// Comprobación posterior de que la acción produjo el efecto previsto.
    pub effectiveness_check: Option<String>,
    pub closed: Option<String>,
    pub closed_by: Option<String>,
}

impl NonConformity {
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }

    /// Campos que la Tabla 22 exige para esta severidad y que faltan.
    pub fn missing_response(&self) -> Vec<&'static str> {
        let mut faltan = Vec::new();
        if self.severity.requires_root_cause() {
            if self.containment.is_none() {
                faltan.push("contención");
            }
            if self.root_cause.is_none() {
                faltan.push("causa raíz");
            }
            if self.corrective_action.is_none() {
                faltan.push("acción correctiva");
            }
            if self.effectiveness_check.is_none() {
                faltan.push("verificación de eficacia");
            }
        }
        faltan
    }
}

/// Registro único de no conformidades.
pub struct Register {
    path: PathBuf,
}

impl Register {
    pub fn at(repo_root: &Path) -> Register {
        Register {
            path: repo_root.join(REGISTER_RELATIVE),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Abre una no conformidad.
    pub fn open(
        &self,
        repo: &Repository,
        actor: &str,
        origin: Origin,
        severity: Severity,
        scope: &[String],
        description: &str,
    ) -> Result<NonConformity> {
        let hoy = clock::today();
        let nc = NonConformity {
            id: format!("NC-{}-{}", hoy.replace('-', ""), &crate::ids::new_uid()[20..]),
            opened: clock::now_rfc3339(),
            origin,
            severity,
            scope: scope.to_vec(),
            description: description.to_string(),
            containment: None,
            root_cause: None,
            corrective_action: None,
            action_owner: None,
            action_due: None,
            effectiveness_check: None,
            closed: None,
            closed_by: None,
        };
        self.append(&nc)?;
        repo.event_log().append(
            actor,
            event::NONCONFORMITY_OPENED,
            scope.first().map(String::as_str),
            json!({"id": nc.id, "severity": severity.as_str(), "origin": origin.as_str()}),
        )?;
        Ok(nc)
    }

    /// Sustituye una entrada por su versión actualizada.
    ///
    /// El registro es de solo anexado: la versión anterior permanece. La lectura
    /// devuelve la última versión de cada identificador.
    pub fn update(&self, nc: &NonConformity) -> Result<()> {
        self.append(nc)
    }

    /// Cierra una no conformidad, comprobando la respuesta exigible.
    pub fn close(
        &self,
        repo: &Repository,
        actor: &str,
        mut nc: NonConformity,
    ) -> Result<NonConformity> {
        let faltan = nc.missing_response();
        if !faltan.is_empty() {
            return Err(Error::requirement(
                "21.2",
                format!(
                    "La no conformidad {} es de severidad {} y le faltan {} elementos de la respuesta exigible. No se ha cerrado. Completar: {}.",
                    nc.id,
                    nc.severity.as_str(),
                    faltan.len(),
                    faltan.join(", ")
                ),
            ));
        }
        nc.closed = Some(clock::now_rfc3339());
        nc.closed_by = Some(actor.to_string());
        self.append(&nc)?;
        repo.event_log().append(
            actor,
            event::NONCONFORMITY_CLOSED,
            nc.scope.first().map(String::as_str),
            json!({"id": nc.id, "severity": nc.severity.as_str()}),
        )?;
        Ok(nc)
    }

    fn append(&self, nc: &NonConformity) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut linea = serde_json::to_string(nc)
            .map_err(|e| Error::io(&self.path, std::io::Error::other(e.to_string())))?;
        linea.push('\n');
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| Error::io(&self.path, e))?;
        f.write_all(linea.as_bytes())
            .map_err(|e| Error::io(&self.path, e))?;
        f.sync_all().map_err(|e| Error::io(&self.path, e))?;
        Ok(())
    }

    /// Última versión de cada no conformidad.
    pub fn entries(&self) -> Result<Vec<NonConformity>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let f = std::fs::File::open(&self.path).map_err(|e| Error::io(&self.path, e))?;
        let mut por_id: std::collections::HashMap<String, NonConformity> =
            std::collections::HashMap::new();
        let mut orden: Vec<String> = Vec::new();
        for linea in BufReader::new(f).lines() {
            let linea = linea.map_err(|e| Error::io(&self.path, e))?;
            if linea.trim().is_empty() {
                continue;
            }
            if let Ok(nc) = serde_json::from_str::<NonConformity>(&linea) {
                if !por_id.contains_key(&nc.id) {
                    orden.push(nc.id.clone());
                }
                por_id.insert(nc.id.clone(), nc);
            }
        }
        Ok(orden.into_iter().filter_map(|id| por_id.remove(&id)).collect())
    }

    pub fn open_entries(&self) -> Result<Vec<NonConformity>> {
        Ok(self.entries()?.into_iter().filter(|n| n.is_open()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entorno() -> (tempfile::TempDir, Repository, Register) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        let reg = Register::at(repo.root());
        (dir, repo, reg)
    }

    #[test]
    fn la_entrada_lleva_los_campos_de_la_tabla_21() {
        let (_d, repo, reg) = entorno();
        let nc = reg
            .open(
                &repo,
                "J. Duarte",
                Origin::ShipmentReception,
                Severity::Major,
                &["01J9ZQ8F3K7N2VYB4T6XM0RSAE".into()],
                "La verificacion de integridad fallo en 3 de 128 archivos del envio 20260806-AAAA",
            )
            .unwrap();

        assert!(nc.id.starts_with("NC-"));
        assert!(nc.is_open());
        assert_eq!(nc.severity, Severity::Major);
        assert_eq!(nc.scope.len(), 1);
        assert!(clock::parse_rfc3339(&nc.opened).is_ok());
        assert_eq!(reg.open_entries().unwrap().len(), 1);
    }

    #[test]
    fn una_no_conformidad_mayor_no_se_cierra_sin_su_respuesta_exigible() {
        let (_d, repo, reg) = entorno();
        let nc = reg
            .open(&repo, "a", Origin::Audit, Severity::Major, &[], "Hecho observado")
            .unwrap();

        // Tabla 22: contención, causa raíz, acción correctiva y verificación de
        // eficacia.
        let e = reg.close(&repo, "a", nc.clone()).unwrap_err();
        assert_eq!(e.clause(), Some("21.2"));
        assert!(e.to_string().contains("causa raíz"), "{e}");

        let mut completa = nc;
        completa.containment = Some("Envio retenido en cuarentena".into());
        completa.root_cause = Some("El procedimiento no comprobaba el espacio antes de empaquetar".into());
        completa.corrective_action = Some("Comprobacion de espacio previa al empaquetado".into());
        completa.action_owner = Some("Responsable de intercambio".into());
        completa.effectiveness_check = Some("Diez envios sucesivos sin incidencia".into());
        let cerrada = reg.close(&repo, "a", completa).unwrap();
        assert!(!cerrada.is_open());
        assert!(reg.open_entries().unwrap().is_empty());
    }

    #[test]
    fn una_no_conformidad_menor_se_cierra_con_correccion_y_registro() {
        let (_d, repo, reg) = entorno();
        let nc = reg
            .open(&repo, "a", Origin::Spontaneous, Severity::Minor, &[], "Nombre con espacio")
            .unwrap();
        assert!(nc.missing_response().is_empty());
        assert!(reg.close(&repo, "a", nc).is_ok());
    }

    #[test]
    fn el_registro_es_de_solo_anexado_y_devuelve_la_ultima_version() {
        let (_d, repo, reg) = entorno();
        let mut nc = reg
            .open(&repo, "a", Origin::Audit, Severity::Minor, &[], "Primera redaccion")
            .unwrap();
        nc.containment = Some("Medida adoptada".into());
        reg.update(&nc).unwrap();

        let texto = std::fs::read_to_string(reg.path()).unwrap();
        assert_eq!(texto.lines().count(), 2, "las dos versiones constan");

        let entradas = reg.entries().unwrap();
        assert_eq!(entradas.len(), 1, "se devuelve una sola entrada por identificador");
        assert_eq!(entradas[0].containment.as_deref(), Some("Medida adoptada"));
    }

    #[test]
    fn la_apertura_y_el_cierre_dejan_su_entrada_en_el_registro_de_eventos() {
        let (_d, repo, reg) = entorno();
        let nc = reg
            .open(&repo, "a", Origin::QualityControl, Severity::Minor, &["UID".into()], "Hecho")
            .unwrap();
        reg.close(&repo, "a", nc).unwrap();
        let eventos: Vec<String> = repo
            .event_log()
            .entries()
            .unwrap()
            .iter()
            .map(|e| e.event.clone())
            .collect();
        assert!(eventos.contains(&event::NONCONFORMITY_OPENED.to_string()));
        assert!(eventos.contains(&event::NONCONFORMITY_CLOSED.to_string()));
        assert!(repo.event_log().verify_chain().unwrap().is_intact());
    }

    #[test]
    fn la_severidad_determina_la_respuesta_exigible() {
        assert!(!Severity::Minor.requires_root_cause());
        assert!(Severity::Major.requires_root_cause());
        assert!(Severity::Critical.requires_root_cause());
        assert!(Severity::Critical.requires_notification());
        assert!(!Severity::Major.requires_notification());
    }
}
