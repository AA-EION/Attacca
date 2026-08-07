//! Validación de conformidad (apartados 13.2, 19.1 y 21).
//!
//! El apartado 13.2 lo exige de forma expresa: la validación automática debe
//! distinguir entre un campo pendiente y un campo incumplido, y no debe tratar
//! como no conformidad la ausencia de un valor cuya etapa no ha concluido.
//!
//! De ahí las tres categorías de [`Severity`]. Un campo pendiente no es un
//! error, y la interfaz no debe presentarlo en rojo.

use crate::custody::CustodyState;
use crate::doc::Map;
use crate::manifest::project::{Level, ProjectManifest, Status};
use crate::naming;
use crate::stage::Stage;
use std::collections::HashSet;
use std::path::Path;

/// Naturaleza de un hallazgo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Campo exigible cuya etapa no ha concluido. No es un incumplimiento.
    Pending,
    /// Desviación registrada en el campo de excepciones del manifiesto.
    Exception,
    /// Incumplimiento real de un requisito.
    Breach,
}

impl Severity {
    pub fn key(&self) -> &'static str {
        match self {
            Severity::Pending => "pendiente",
            Severity::Exception => "excepcion",
            Severity::Breach => "incumplimiento",
        }
    }
}

/// Un hallazgo de la validación.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    /// Cláusula de la norma afectada.
    pub clause: String,
    /// Campo o elemento afectado.
    pub subject: String,
    /// Hecho observado, expresado sin interpretación (apartado 21.1).
    pub detail: String,
}

/// Informe de conformidad de un proyecto.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub level: Option<Level>,
    pub stage: Option<Stage>,
}

impl Report {
    pub fn of(&self, severity: Severity) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
    }

    pub fn pending(&self) -> Vec<&Finding> {
        self.of(Severity::Pending)
    }

    pub fn breaches(&self) -> Vec<&Finding> {
        self.of(Severity::Breach)
    }

    pub fn exceptions(&self) -> Vec<&Finding> {
        self.of(Severity::Exception)
    }

    /// Un proyecto es conforme cuando cumple todos los requisitos de su nivel y
    /// toda desviación está registrada en el campo de excepciones
    /// (apartado 19.1). Los campos pendientes no afectan a la conformidad.
    pub fn is_conformant(&self) -> bool {
        self.breaches().is_empty()
    }
}

/// Grupos de campos y la etapa en que pasan a ser exigibles (Tabla 11B).
const FIELD_GROUPS: &[(&str, &[&str], Option<Stage>, Level)] = &[
    // Identidad: exigible al crear el proyecto.
    (
        "Identidad",
        &["uid", "id", "artist", "type"],
        None,
        Level::A,
    ),
    // Audio: la frecuencia y la profundidad son exigibles en la creación; el
    // resto, al concluir la composición.
    (
        "Audio",
        &["audio.sample_rate", "audio.bit_depth"],
        None,
        Level::A,
    ),
    (
        "Audio",
        &[
            "audio.tempo",
            "audio.key",
            "audio.origin",
            "audio.tuning_hz",
        ],
        Some(Stage::Composition),
        Level::A,
    ),
    // Herramientas: al guardar la primera sesión.
    (
        "Herramientas",
        &["tools.primary"],
        Some(Stage::Recording),
        Level::B,
    ),
    // Personas: al concluir la grabación.
    ("Personas", &["people"], Some(Stage::Recording), Level::B),
    // Máster: al concluir el mastering.
    (
        "Máster",
        &["master.lufs_i", "master.lra", "master.true_peak_db"],
        Some(Stage::Mastering),
        Level::B,
    ),
    // Derechos: al concluir el control de calidad.
    (
        "Derechos",
        &["rights.isrc", "rights.split_sheet"],
        Some(Stage::QualityControl),
        Level::C,
    ),
    // Preservación: al archivar.
    (
        "Preservación",
        &["preservation.manifest"],
        Some(Stage::Archival),
        Level::C,
    ),
];

/// Valida un proyecto frente a los requisitos de su nivel declarado.
pub fn project(manifest: &ProjectManifest) -> Report {
    let doc = manifest.doc();
    let raiz = manifest.root();
    let nivel = manifest.level();
    let etapa = crate::stage::active_stage_of(doc, raiz);
    let excepciones: HashSet<String> = manifest.declared_exceptions().into_iter().collect();

    let mut r = Report {
        level: Some(nivel),
        stage: Some(etapa),
        ..Default::default()
    };

    // --- Campos del manifiesto (Tabla 13 y Tabla 11B) ---
    for (grupo, campos, exigible_tras, nivel_minimo) in FIELD_GROUPS {
        if nivel < *nivel_minimo {
            continue;
        }
        for campo in *campos {
            if field_present(doc, campo) {
                continue;
            }
            // Un campo cuya etapa aún no ha concluido está pendiente, no
            // incumplido (apartado 13.2).
            let concluida = match exigible_tras {
                None => true,
                Some(s) => etapa.index() > s.index(),
            };
            let severidad = if !concluida {
                Severity::Pending
            } else if excepciones.contains("13.1") || excepciones.contains(*campo) {
                Severity::Exception
            } else {
                Severity::Breach
            };
            r.findings.push(Finding {
                severity: severidad,
                clause: "13.1".into(),
                subject: (*campo).into(),
                detail: format!(
                    "El campo {campo} del grupo {grupo} no tiene valor. {}",
                    match severidad {
                        Severity::Pending => "La etapa que lo produce no ha concluido.",
                        Severity::Exception => "La desviación consta en el campo de excepciones.",
                        Severity::Breach => "La etapa que lo produce ya concluyó.",
                    }
                ),
            });
        }
        // El campo debe estar presente aunque sea en nulo (apartado 13.2,
        // «no deben omitirse»). Su ausencia total sí es un incumplimiento.
        for campo in *campos {
            if doc.at(campo).is_none() && !campo.contains('.') {
                continue;
            }
            if doc.at(campo).is_none() {
                let raiz_campo = campo.split('.').next().unwrap_or(campo);
                if doc.get(raiz_campo).is_some() {
                    r.findings.push(Finding {
                        severity: Severity::Breach,
                        clause: "13.2".into(),
                        subject: (*campo).into(),
                        detail: format!("El campo {campo} está omitido. Los campos aún no exigibles deben registrarse con valor nulo explícito, no omitirse."),
                    });
                }
            }
        }
    }

    // --- Estructura del proyecto (Tabla 7) ---
    for dir in crate::repo::REQUIRED_PROJECT_DIRS {
        if !raiz.join(dir).is_dir() {
            r.findings.push(Finding {
                severity: Severity::Breach,
                clause: "7.1".into(),
                subject: (*dir).into(),
                detail: format!("La carpeta obligatoria {dir} no existe."),
            });
        }
    }

    // --- Marcadores y estado de custodia (apartados 14.3.3 y 6.6.5) ---
    let custodia = manifest.custody_state();
    let marcador = raiz.join(crate::manifest::CUSTODY_LOCK_FILE);
    if custodia.requires_lock_marker() && !marcador.is_file() {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.3".into(),
            subject: crate::manifest::CUSTODY_LOCK_FILE.into(),
            detail: format!("El estado de custodia es {} y no existe el marcador CUSTODY.lock en la raíz del proyecto.", custodia.as_str()),
        });
    }
    if !custodia.requires_lock_marker() && marcador.is_file() {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.3".into(),
            subject: crate::manifest::CUSTODY_LOCK_FILE.into(),
            detail: format!("El estado de custodia es {} y el marcador CUSTODY.lock sigue presente. Debe suprimirse cuando el estado vuelva a ser propia o reclamada.", custodia.as_str()),
        });
    }

    // --- Cronología de custodia (apartado 14.3.4) ---
    let hist = manifest.custody_history();
    let c = crate::custody::check_chronology(&hist);
    for seq in &c.duplicate_seq {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.4".into(),
            subject: format!("custody.history[{seq}]"),
            detail: format!("El número de secuencia {seq} está duplicado. Un número duplicado constituye no conformidad mayor."),
        });
    }
    for seq in &c.missing_seq {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.4".into(),
            subject: format!("custody.history[{seq}]"),
            detail: format!("Falta el asiento de secuencia {seq}. Un asiento suprimido constituye no conformidad mayor."),
        });
    }
    for seq in &c.decreasing_timestamps {
        // Una marca decreciente no invalida la cronología por sí sola: el orden
        // lo determina el número de secuencia (apartado 14.3.4).
        r.findings.push(Finding {
            severity: Severity::Exception,
            clause: "22.3.2".into(),
            subject: format!("custody.history[{seq}]"),
            detail: format!("El asiento {seq} tiene marca temporal anterior a la del asiento precedente. El orden lo determina el número de secuencia; la desviación se trata conforme al apartado 22.3.2."),
        });
    }

    // --- Réplicas (apartado 6.6.1) ---
    let replicas = crate::replica::replicas_of(doc);
    if let Err(e) = crate::replica::check_single_active(&replicas, custodia) {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "6.6.1".into(),
            subject: "replication".into(),
            detail: e.to_string(),
        });
    }
    for replica in &replicas {
        if replica.state.requires_hold_marker()
            && replica.state != crate::replica::ReplicaState::Disconnected
        {
            // El marcador se comprueba solo en la réplica local: las demás no
            // son accesibles desde aquí.
            let es_local = manifest.active_volume() == Some(replica.volume.as_str());
            if !es_local && !raiz.join(crate::manifest::REPLICA_HOLD_FILE).is_file() {
                continue;
            }
        }
    }

    // --- Nomenclatura (Tabla 9) ---
    let presupuesto = crate::project::path_budget(manifest);
    if presupuesto.exceeded() {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "9.1".into(),
            subject: presupuesto.longest_path.clone(),
            detail: format!(
                "La ruta más larga mide {} de {} caracteres.",
                presupuesto.longest, presupuesto.limit
            ),
        });
    }
    check_names(raiz, &mut r);

    // --- Estado y ubicación (apartado 6.2) ---
    if let Some(carpeta) = manifest.status().folder() {
        let dentro_de_release = raiz
            .parent()
            .map(|p| p.join("_RELEASE").is_dir())
            .unwrap_or(false);
        let en_archivo = raiz.to_string_lossy().contains("30_ARCHIVE");
        let coincide = raiz.to_string_lossy().contains(carpeta);
        if !coincide && !dentro_de_release && !en_archivo {
            r.findings.push(Finding {
                severity: Severity::Breach,
                clause: "6.2".into(),
                subject: "status".into(),
                detail: format!(
                    "El manifiesto declara el estado {} y el proyecto no reside en {carpeta}. El estado debe registrarse simultáneamente en su ubicación y en su manifiesto.",
                    manifest.status().as_str()
                ),
            });
        }
    }

    // --- Archivo y custodia (apartado 14.3.5) ---
    if manifest.status() == Status::Archived && !custodia.allows_archival() {
        r.findings.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.5".into(),
            subject: "custody.state".into(),
            detail: format!("El proyecto está archivado y su estado de custodia es {}. Un proyecto cuya custodia no sea propia o reclamada no debe archivarse.", custodia.as_str()),
        });
    }

    // --- Excepciones declaradas (apartado 13.1) ---
    for entrada in doc
        .get("exceptions")
        .and_then(|n| n.as_seq())
        .unwrap_or(&[])
    {
        let Some(m) = entrada.as_map() else { continue };
        let clausula = m.get("clause").and_then(|n| n.as_str()).unwrap_or("");
        let motivo = m.get("reason").and_then(|n| n.present_str());
        let aprobada = m.get("approved_by").and_then(|n| n.present_str());
        if motivo.is_none() || aprobada.is_none() {
            r.findings.push(Finding {
                severity: Severity::Breach,
                clause: "13.1".into(),
                subject: format!("exceptions[{clausula}]"),
                detail: "La excepción no indica motivo o persona que la aprueba. Toda desviación debe registrarse con la cláusula afectada, el motivo, la persona que la aprueba y la fecha.".into(),
            });
        } else {
            r.findings.push(Finding {
                severity: Severity::Exception,
                clause: clausula.to_string(),
                subject: format!("exceptions[{clausula}]"),
                detail: format!(
                    "Desviación declarada: {}. Aprobada por {}.",
                    motivo.unwrap_or(""),
                    aprobada.unwrap_or("")
                ),
            });
        }
    }

    r.findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.clause.cmp(&b.clause))
            .then(a.subject.cmp(&b.subject))
    });
    r
}

fn field_present(doc: &Map, path: &str) -> bool {
    match doc.at(path) {
        None => false,
        Some(crate::doc::Node::Null) => false,
        Some(crate::doc::Node::Str(s)) => !s.is_empty(),
        Some(crate::doc::Node::Seq(s)) => !s.is_empty(),
        Some(crate::doc::Node::Map(m)) => !m.is_empty(),
        Some(_) => true,
    }
}

fn check_names(root: &Path, report: &mut Report) {
    let mut por_directorio: std::collections::HashMap<std::path::PathBuf, Vec<String>> =
        std::collections::HashMap::new();

    for entry in crate::fsx::walk::conserved_files(root) {
        let nombre = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Err(e) = naming::check_name(&nombre) {
            report.findings.push(Finding {
                severity: Severity::Breach,
                clause: "9.1".into(),
                subject: entry.relative.clone(),
                detail: e.to_string(),
            });
        }
        if let Some(dir) = entry.path.parent() {
            por_directorio
                .entry(dir.to_path_buf())
                .or_default()
                .push(nombre);
        }
    }

    // Regla 10: dos nombres de un mismo directorio no coinciden al compararse
    // sin distinción entre mayúsculas y minúsculas.
    for (dir, nombres) in por_directorio {
        for (a, b) in naming::case_collisions(&nombres) {
            report.findings.push(Finding {
                severity: Severity::Breach,
                clause: "9.1".into(),
                subject: crate::fsx::walk::relative_slash(root, &dir).unwrap_or_default(),
                detail: format!("Los nombres «{a}» y «{b}» coinciden al compararse sin distinguir mayúsculas de minúsculas."),
            });
        }
    }
}

/// Valida el estado de custodia frente a los archivos presentes.
pub fn custody_consistency(manifest: &ProjectManifest) -> Vec<Finding> {
    let raiz = manifest.root();
    let declarado = manifest.custody_state();
    let marcador = raiz.join(crate::manifest::CUSTODY_LOCK_FILE);
    let mut out = Vec::new();

    if !marcador.is_file() {
        return out;
    }
    let Ok(lock) = crate::manifest::custody_lock::CustodyLock::load(&marcador) else {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.3".into(),
            subject: crate::manifest::CUSTODY_LOCK_FILE.into(),
            detail: "El marcador de custodia no se pudo interpretar.".into(),
        });
        return out;
    };
    if lock.state != declarado {
        // El manifiesto prevalece sobre el archivo marcador en caso de
        // discrepancia (apartado 14.3.3, cuarto guion).
        out.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.3".into(),
            subject: "custody.state".into(),
            detail: format!(
                "El manifiesto declara el estado {} y el marcador declara {}. El manifiesto prevalece; el marcador debe actualizarse.",
                declarado.as_str(),
                lock.state.as_str()
            ),
        });
    }
    out
}

/// Comprueba si un proyecto puede entregarse (apartados 12.2, 15 y 36).
pub fn ready_for_delivery(manifest: &ProjectManifest) -> Vec<Finding> {
    let mut out = Vec::new();

    let pendientes = crate::package::emit::pending_clearances(manifest);
    if !pendientes.is_empty() {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "12.2".into(),
            subject: "sources".into(),
            detail: format!(
                "{} materiales de terceros tienen autorización sin resolver: {}.",
                pendientes.len(),
                pendientes.join(", ")
            ),
        });
    }

    if !manifest.custody_state().allows_write() {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "14.3".into(),
            subject: "custody.state".into(),
            detail: format!(
                "El estado de custodia es {}. Solo el titular de la custodia puede emitir un envío.",
                manifest.custody_state().as_str()
            ),
        });
    }

    let master = manifest.root().join("07_MASTER");
    if !master.is_dir() || crate::fsx::walk::conserved_files(&master).is_empty() {
        out.push(Finding {
            severity: Severity::Pending,
            clause: "10.3".into(),
            subject: "07_MASTER".into(),
            detail: "No hay ningún máster en 07_MASTER.".into(),
        });
    }
    out
}

/// Comprueba si un release puede cerrarse y entregarse (apartado 8.5).
pub fn release_ready(
    release: &crate::manifest::release::ReleaseManifest,
    integrantes: &[(String, ProjectManifest)],
) -> Vec<Finding> {
    let mut out = Vec::new();
    let tracklist = release.tracklist();

    // La entrega se bloquea si falta cualquier tema del tracklist.
    let presentes: HashSet<&str> = integrantes.iter().map(|(uid, _)| uid.as_str()).collect();
    for track in &tracklist {
        if !presentes.contains(track.project_uid.as_str()) {
            out.push(Finding {
                severity: Severity::Breach,
                clause: "8.5".into(),
                subject: format!("tracklist[{}]", track.position),
                detail: format!(
                    "El tema {} de la posición {} no está disponible. Un envío cuyo alcance sea un release debe rechazarse cuando falte cualquiera de los declarados en el tracklist.",
                    track.title, track.position
                ),
            });
        }
    }

    for (uid, proyecto) in integrantes {
        let etiqueta = proyecto.id().unwrap_or(uid);

        // Autorizaciones sin resolver.
        let pendientes = crate::package::emit::pending_clearances(proyecto);
        if !pendientes.is_empty() {
            out.push(Finding {
                severity: Severity::Breach,
                clause: "8.5".into(),
                subject: etiqueta.to_string(),
                detail: format!(
                    "El tema {etiqueta} tiene {} autorizaciones de terceros sin resolver.",
                    pendientes.len()
                ),
            });
        }

        // Informe de control de calidad aprobado.
        let notas = proyecto.root().join("00_ADMIN/Notes");
        let hay_informe = std::fs::read_dir(&notas)
            .map(|e| {
                e.flatten().any(|x| {
                    let n = x.file_name().to_string_lossy().to_ascii_uppercase();
                    n.contains("QC") || n.contains("CONTROL") || n.contains("CALIDAD")
                })
            })
            .unwrap_or(false);
        if !hay_informe {
            out.push(Finding {
                severity: Severity::Breach,
                clause: "8.5".into(),
                subject: etiqueta.to_string(),
                detail: format!("El tema {etiqueta} carece de informe de control de calidad aprobado en 00_ADMIN/Notes."),
            });
        }

        // Parámetros comunes exigidos por el canal.
        for fallo in crate::manifest::release::check_common_requirements(release, proyecto.doc()) {
            out.push(Finding {
                severity: Severity::Breach,
                clause: "8.5".into(),
                subject: etiqueta.to_string(),
                detail: fallo,
            });
        }
    }
    out
}

/// Comprueba si un proyecto puede archivarse (apartados 14.3.5 y 17.1).
pub fn ready_for_archival(manifest: &ProjectManifest) -> Vec<Finding> {
    let mut out = Vec::new();
    let custodia = manifest.custody_state();
    if !custodia.allows_archival() && manifest.status() != Status::Sealed {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "14.3.5".into(),
            subject: "custody.state".into(),
            detail: format!(
                "El estado de custodia es {}. Un proyecto cuya custodia no sea propia, reclamada o sellada no debe archivarse.",
                custodia.as_str()
            ),
        });
    }
    let deteccion = crate::fsx::openfiles::open_for_write(manifest.root());
    if !deteccion.files().is_empty() {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "17.1".into(),
            subject: "archivos abiertos".into(),
            detail: format!(
                "{} archivos del proyecto están abiertos para escritura por otro proceso.",
                deteccion.files().len()
            ),
        });
    }
    out
}

/// Comprueba las réplicas conocidas y devuelve las que no cumplen el
/// invariante (apartado 6.6).
pub fn replica_consistency(manifest: &ProjectManifest) -> Vec<Finding> {
    let replicas = crate::replica::replicas_of(manifest.doc());
    let custodia = manifest.custody_state();
    let mut out = Vec::new();
    if let Err(e) = crate::replica::check_single_active(&replicas, custodia) {
        out.push(Finding {
            severity: Severity::Breach,
            clause: "6.6.1".into(),
            subject: "replication".into(),
            detail: e.to_string(),
        });
    }
    for r in &replicas {
        if r.state == crate::replica::ReplicaState::Divergent {
            out.push(Finding {
                severity: Severity::Breach,
                clause: "6.6.4".into(),
                subject: r.volume.clone(),
                detail: "La réplica está marcada como divergente. La reconciliación debe ser manual y su resultado debe registrarse.".into(),
            });
        }
    }
    out
}

/// Estado de la custodia respecto de sus plazos, para presentarlo sin exigir
/// ninguna acción (apartados 41.4 y 8 de la interfaz visual).
pub fn custody_expiry(manifest: &ProjectManifest) -> Option<(crate::custody::Expiry, String)> {
    if manifest.custody_state() != CustodyState::Ceded {
        return None;
    }
    let retorno = manifest
        .doc()
        .at("custody.transfer.expected_return")
        .and_then(|n| n.present_str())?;
    let gracia = manifest
        .doc()
        .at("custody.transfer.grace_days")
        .and_then(|n| n.as_int())
        .unwrap_or(0);
    crate::custody::expiry_state(retorno, gracia)
        .ok()
        .map(|e| (e, retorno.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Node;
    use crate::project::{self, NewProject};
    use crate::repo::Repository;

    fn entorno() -> (tempfile::TempDir, Repository, ProjectManifest) {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        let p = project::create(
            &repo,
            "a",
            &NewProject {
                title: "Tema".into(),
                artist: "Artista".into(),
                kind: "ORIG".into(),
                level: Level::B,
                release_uid: None,
                release_dir: None,
                sample_rate: 48000,
                bit_depth: 24,
                holder_org: "Estudio A".into(),
                holder_person: "J. Duarte".into(),
                active_volume: None,
            },
        )
        .unwrap();
        (dir, repo, p)
    }

    #[test]
    fn un_proyecto_recien_creado_es_conforme_con_campos_pendientes() {
        let (_d, _r, p) = entorno();
        let informe = project(&p);

        // Apartado 13.2: un campo pendiente no es una no conformidad.
        assert!(
            informe.is_conformant(),
            "incumplimientos: {:?}",
            informe.breaches()
        );
        assert!(!informe.pending().is_empty(), "hay campos pendientes");
        // El tempo está pendiente, no incumplido.
        let tempo = informe
            .findings
            .iter()
            .find(|f| f.subject == "audio.tempo")
            .expect("el tempo debe figurar");
        assert_eq!(tempo.severity, Severity::Pending);
    }

    #[test]
    fn distingue_pendiente_de_incumplimiento_al_avanzar_la_etapa() {
        let (_d, _r, mut p) = entorno();
        // El proyecto avanza: hay máster, luego el mastering ya concluyó.
        std::fs::create_dir_all(p.root().join("07_MASTER")).unwrap();
        std::fs::write(p.root().join("07_MASTER/m.wav"), b"master").unwrap();
        p.doc_mut()
            .ensure_map("master")
            .set("true_peak_db", Node::Float(-1.0));
        p.save().unwrap();

        let informe = project(&p);
        // La etapa activa pasó a control de calidad: las mediciones que faltan
        // ya son incumplimiento, no pendientes.
        assert_eq!(informe.stage, Some(Stage::QualityControl));
        let lufs = informe
            .findings
            .iter()
            .find(|f| f.subject == "master.lufs_i")
            .expect("lufs_i debe figurar");
        assert_eq!(lufs.severity, Severity::Breach);
        assert!(!informe.is_conformant());
    }

    #[test]
    fn una_excepcion_declarada_no_es_incumplimiento() {
        let (_d, _r, mut p) = entorno();
        p.record_exception("17.1", "El programa no exporta AES31-3", "J. Duarte");
        p.save().unwrap();

        let informe = project(&p);
        let exc = informe.exceptions();
        assert!(exc.iter().any(|f| f.clause == "17.1"), "{:?}", exc);
        assert!(informe.is_conformant());
    }

    #[test]
    fn una_excepcion_sin_motivo_ni_aprobacion_es_incumplimiento() {
        let (_d, _r, mut p) = entorno();
        p.doc_mut().ensure_seq("exceptions").push(Node::map(vec![
            ("clause", Node::str("17.1")),
            ("reason", Node::Null),
            ("approved_by", Node::Null),
        ]));
        p.save().unwrap();
        let informe = project(&p);
        assert!(!informe.is_conformant());
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.subject.contains("exceptions")));
    }

    #[test]
    fn detecta_el_marcador_de_custodia_ausente_o_sobrante() {
        let (_d, _r, mut p) = entorno();
        // Estado cedida sin marcador.
        p.doc_mut()
            .ensure_map("custody")
            .set("state", Node::str("cedida"));
        p.save().unwrap();
        let informe = project(&p);
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.clause == "14.3.3" && f.detail.contains("no existe el marcador")));

        // Estado propia con marcador presente.
        p.doc_mut()
            .ensure_map("custody")
            .set("state", Node::str("propia"));
        p.save().unwrap();
        std::fs::write(
            p.root().join(crate::manifest::CUSTODY_LOCK_FILE),
            b"STAVE CUSTODY LOCK\nestado: cedida\n",
        )
        .unwrap();
        let informe = project(&p);
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.detail.contains("sigue presente")));
    }

    #[test]
    fn el_manifiesto_prevalece_sobre_el_marcador() {
        let (_d, _r, mut p) = entorno();
        p.doc_mut()
            .ensure_map("custody")
            .set("state", Node::str("cedida"));
        p.save().unwrap();
        crate::manifest::custody_lock::write_marker(
            p.root(),
            &crate::manifest::custody_lock::CustodyLock {
                state: CustodyState::InTransit,
                holder: "B".into(),
                ceded_by: "A".into(),
                shipment_id: "0007".into(),
                ceded_at: "2026-08-06T10:00:00-05:00".into(),
                expected_return: "2026-08-20".into(),
                grace_days: 15,
            },
        )
        .unwrap();
        let hallazgos = custody_consistency(&p);
        assert_eq!(hallazgos.len(), 1);
        assert!(hallazgos[0].detail.contains("El manifiesto prevalece"));
    }

    #[test]
    fn detecta_una_cronologia_con_asiento_suprimido() {
        let (_d, _r, mut p) = entorno();
        let hist = p.doc_mut().ensure_map("custody").ensure_seq("history");
        for (seq, accion) in [(1, "exported"), (3, "received")] {
            hist.push(Node::map(vec![
                ("seq", Node::Int(seq)),
                ("action", Node::str(accion)),
                ("ts", Node::str("2026-08-06T10:00:00-05:00")),
            ]));
        }
        p.save().unwrap();
        let informe = project(&p);
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.clause == "14.3.4" && f.detail.contains("Falta el asiento")));
    }

    #[test]
    fn una_marca_decreciente_se_declara_como_excepcion_y_no_como_incumplimiento() {
        let (_d, _r, mut p) = entorno();
        let hist = p.doc_mut().ensure_map("custody").ensure_seq("history");
        hist.push(Node::map(vec![
            ("seq", Node::Int(1)),
            ("action", Node::str("exported")),
            ("ts", Node::str("2026-08-06T12:00:00-05:00")),
        ]));
        hist.push(Node::map(vec![
            ("seq", Node::Int(2)),
            ("action", Node::str("sent")),
            ("ts", Node::str("2026-08-06T10:00:00-05:00")),
        ]));
        p.save().unwrap();
        let informe = project(&p);
        assert!(informe.exceptions().iter().any(|f| f.clause == "22.3.2"));
        assert!(
            !informe.breaches().iter().any(|f| f.clause == "14.3.4"),
            "una marca decreciente no invalida la cronología por sí sola"
        );
    }

    #[test]
    fn detecta_una_carpeta_obligatoria_ausente() {
        let (_d, _r, p) = entorno();
        std::fs::remove_dir_all(p.root().join("00_ADMIN")).unwrap();
        let informe = project(&p);
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.clause == "7.1" && f.subject == "00_ADMIN"));
    }

    #[test]
    fn detecta_nombres_contrarios_a_la_tabla_9() {
        let (_d, _r, p) = entorno();
        std::fs::write(p.root().join("00_ADMIN/nombre con espacios.txt"), b"x").unwrap();
        let informe = project(&p);
        assert!(informe
            .breaches()
            .iter()
            .any(|f| f.clause == "9.1" && f.detail.contains("espacio")));
    }

    #[test]
    fn la_entrega_se_bloquea_con_autorizaciones_pendientes() {
        let (_d, _r, mut p) = entorno();
        p.record_source(
            "01_REF/muestra.wav",
            "Biblioteca",
            None,
            "INTERNO",
            None,
            None,
            "pending",
        );
        p.save().unwrap();
        let h = ready_for_delivery(&p);
        assert!(h
            .iter()
            .any(|f| f.clause == "12.2" && f.severity == Severity::Breach));
    }

    #[test]
    fn el_release_no_se_cierra_con_un_tema_ausente_o_sin_control_de_calidad() {
        let (_d, _r, p) = entorno();
        let mut rel = crate::manifest::release::ReleaseManifest::new(std::path::Path::new(
            "_RELEASE/RELEASE.yaml",
        ));
        *rel.doc_mut() = crate::manifest::release::scaffold(
            "R",
            "2026-08-06_D_ALBUM",
            "D",
            "A",
            crate::manifest::release::ReleaseClass::Album,
            Level::C,
            "Estudio A",
            "J. Duarte",
        );
        rel.link_project("UID-FALTANTE", "id", "Tema ausente", Some(1))
            .unwrap();
        let uid = p.uid().unwrap().to_string();
        rel.link_project(&uid, p.id().unwrap(), "Tema presente", Some(2))
            .unwrap();

        let h = release_ready(&rel, &[(uid, p)]);
        // Falta un tema del tracklist.
        assert!(h.iter().any(|f| f.detail.contains("no está disponible")));
        // Y el presente carece de informe de control de calidad.
        assert!(h.iter().any(|f| f.detail.contains("control de calidad")));
    }

    #[test]
    fn un_proyecto_cedido_no_puede_archivarse() {
        let (_d, _r, mut p) = entorno();
        p.doc_mut()
            .ensure_map("custody")
            .set("state", Node::str("cedida"));
        p.save().unwrap();
        let h = ready_for_archival(&p);
        assert!(h.iter().any(|f| f.clause == "14.3.5"));
    }

    #[test]
    fn informa_del_vencimiento_de_una_cesion() {
        let (_d, _r, mut p) = entorno();
        let cust = p.doc_mut().ensure_map("custody");
        cust.set("state", Node::str("cedida"));
        cust.set(
            "transfer",
            Node::map(vec![
                (
                    "expected_return",
                    Node::str(crate::clock::add_days(&crate::clock::today(), -40).unwrap()),
                ),
                ("grace_days", Node::Int(15)),
            ]),
        );
        p.save().unwrap();
        let (estado, _) = custody_expiry(&p).unwrap();
        assert_eq!(estado, crate::custody::Expiry::ReclaimAvailable);
    }
}
