//! Procedimiento de emisión (apartado 36).
//!
//! Los catorce pasos se ejecutan en orden. Un paso no se inicia si el anterior
//! no concluyó con resultado conforme.

use super::bagit::{self, PackageDir};
use super::container::{self, Progress};
use crate::clock;
use crate::custody::{ChronologyEntry, CustodyAction, CustodyState};
use crate::doc::Node;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::fsx;
use crate::ids;
use crate::manifest::custody_lock::{self, CustodyLock};
use crate::manifest::exchange::{Classification, ExchangeBuilder, ExchangeManifest, Profile};
use crate::manifest::project::ProjectManifest;
use crate::repo::Repository;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Datos del envío que la persona declara.
#[derive(Clone, Debug)]
pub struct Shipment {
    pub profile: Profile,
    pub classification: Classification,
    pub purpose: String,
    pub issuer_org: String,
    pub issuer_contact: String,
    pub issuer_key_id: Option<String>,
    pub recipient_org: String,
    pub recipient_contact: String,
    pub usage_permitted: Vec<String>,
    pub usage_territory: String,
    pub usage_term: String,
    pub sublicensing: bool,
    pub forwarding: bool,
    pub retention_until: String,
    pub destroy_on_expiry: bool,
    pub personal_data: bool,
    pub personal_data_categories: Vec<String>,
    pub ack_deadline_hours: i64,
    pub ack_address: String,
    /// El envío cede la custodia editorial (apartado 41.1).
    pub transfers_custody: bool,
    /// El envío devuelve la custodia (apartado 41.3).
    pub returns_custody: bool,
    /// Identificador del envío que cedió la custodia, en un retorno.
    pub supersedes_transfer: Option<String>,
    pub expected_return: Option<String>,
    pub grace_days: i64,
    pub onward_allowed: bool,
    /// Envío al que sustituye (apartado 39.2).
    pub supersedes: Option<String>,
    pub revision: String,
    /// El paquete se serializa como contenedor `.stave` (apartado 32.4.5).
    pub serialize: bool,
    /// El proyecto cuenta con informe de control de calidad aprobado
    /// (apartado 36, paso 1). Attacca no puede firmar el informe por la
    /// persona: comprueba que exista y que se declare aprobado.
    pub qc_approved: bool,
}

/// Resultado de una emisión.
#[derive(Clone, Debug)]
pub struct Emission {
    pub shipment_id: String,
    /// Contenedor emitido, o directorio del paquete en entrega sin serializar.
    pub artifact: PathBuf,
    /// Copia congelada en `08_DELIVERY` o en `09_TRANSFER` (apartado 32.3).
    pub frozen_copy: PathBuf,
    pub file_count: usize,
    pub total_bytes: u64,
    pub issued: String,
    pub custody_state: CustodyState,
}

/// Contenido que se incluye en el paquete, ya seleccionado y minimizado.
#[derive(Clone, Debug, Default)]
pub struct Payload {
    /// Carpetas del proyecto que se incluyen en `data/content/`.
    pub content_dirs: Vec<String>,
    /// Archivos de derechos que se incluyen en `data/rights/`, como rutas
    /// relativas al proyecto.
    pub rights_files: Vec<String>,
}

impl Payload {
    /// Contenido mínimo por perfil (Tabla 28).
    pub fn for_profile(profile: Profile) -> Payload {
        match profile {
            Profile::Delivery => Payload {
                content_dirs: vec!["07_MASTER".into(), "00_ADMIN/Credits".into(), "00_ADMIN/Art".into()],
                rights_files: vec![],
            },
            Profile::Production => Payload {
                content_dirs: vec![
                    "00_ADMIN".into(),
                    "02_SESSIONS".into(),
                    "05_STEMS".into(),
                    "06_MIX".into(),
                    "07_MASTER".into(),
                ],
                rights_files: vec![],
            },
            // El perfil `A` incluye el proyecto archivado completo y la
            // documentación de derechos íntegra (apartado 31.3).
            Profile::Archive => Payload {
                content_dirs: vec![],
                rights_files: vec![],
            },
        }
    }
}

/// Ejecuta el procedimiento de emisión del apartado 36.
#[allow(clippy::too_many_arguments)]
pub fn emit(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    shipment: &Shipment,
    payload: &Payload,
    sync: crate::clock::SyncState,
    progress: &mut Progress,
) -> Result<Emission> {
    // El reloj debe estar sincronizado antes de emitir un paquete
    // (apartado 22.3.1, tercer guion).
    clock::require_sync_for_emission(sync)?;

    // Paso 1: aptitud del material.
    if !shipment.qc_approved {
        return Err(Error::requirement(
            "36",
            "El proyecto no cuenta con informe de control de calidad aprobado. El envío no se ha emitido. Ningún paquete debe entregarse ni incluirse en un envío sin informe de control aprobado.",
        ));
    }

    // Paso 2: derechos. Un proyecto con autorizaciones pendientes no se emite
    // en perfil E ni A; en perfil P declara el estado de cada una.
    let pendientes = pending_clearances(project);
    if !pendientes.is_empty() && shipment.profile != Profile::Production {
        return Err(Error::requirement(
            "36",
            format!(
                "El proyecto tiene {} autorizaciones de terceros sin resolver. El envío no se ha emitido. Resolverlas, o emitir en perfil P declarando su estado. Material afectado: {}.",
                pendientes.len(),
                pendientes.join(", ")
            ),
        ));
    }

    // La cesión exige que el proyecto no esté ya cedido (apartado 41.1).
    if shipment.transfers_custody {
        let estado = project.custody_state();
        if !estado.allows_cession() {
            return Err(Error::Custody {
                state: estado.as_str().to_string(),
                detail: "La custodia no se ha cedido. Un proyecto en tránsito o cedido no debe cederse de nuevo. Esperar el retorno, o recuperar la custodia si el plazo venció.".into(),
            });
        }
        if !shipment.profile.may_transfer_custody() {
            return Err(Error::requirement(
                "41.1",
                "El perfil declarado es E y el envío cede la custodia. El envío no se ha emitido. Un envío de perfil E es una entrega para uso final y no cede la custodia.",
            ));
        }
        if shipment.expected_return.is_none() {
            return Err(Error::requirement(
                "41.4",
                "El envío cede la custodia y no declara fecha esperada de retorno. El envío no se ha emitido. Todo envío que ceda la custodia debe declarar fecha esperada de retorno y plazo de gracia.",
            ));
        }
        // Ningún archivo abierto para escritura por otro proceso
        // (apartado 14.3.3, primer párrafo).
        let deteccion = fsx::openfiles::open_for_write(project.root());
        if !deteccion.files().is_empty() {
            return Err(Error::OpenFiles(deteccion.files().to_vec()));
        }
    }

    // Paso 3: el perfil `P` debe declarar los parámetros necesarios para
    // reanudar el trabajo (apartado 31.3). Un proyecto cuya composición no ha
    // concluido todavía no los tiene: emitirlo produciría un manifiesto que la
    // parte receptora debe rechazar conforme al apartado 33. La emisión se
    // detiene antes, indicando qué falta.
    if shipment.profile.requires_continuation() {
        let faltan = missing_continuation(project);
        if !faltan.is_empty() {
            return Err(Error::requirement(
                "31.3",
                format!(
                    "El proyecto no declara {} de los parámetros de continuación. El envío no se ha emitido. Un envío de perfil P debe declararlos para que la parte receptora pueda reanudar el trabajo. Campos pendientes: {}.",
                    faltan.len(),
                    faltan.join(", ")
                ),
            ));
        }
    }

    let project_uid = project
        .uid()
        .ok_or_else(|| Error::input("El proyecto no declara identificador interno.".to_string()))?
        .to_string();
    let project_id = project.id().unwrap_or_default().to_string();

    // Paso 8 (anticipado): el identificador de envío se asigna antes de
    // constituir el paquete porque figura en su nombre (apartado 32.2).
    let date = clock::today();
    let shipment_id = ids::new_shipment_id(&date);
    let package_name = bagit::package_name(
        &date,
        &shipment.issuer_org,
        &shipment.recipient_org,
        &shipment_id,
    )?;

    // Pasos 4 y 5: seleccionar el contenido, aplicar la minimización y
    // constituir el paquete.
    // El área de preparación vive dentro de `00_SYSTEM`, que es un dominio
    // declarado (apartado 6.1). Crear una carpeta de primer nivel propia
    // introduciría un sexto dominio ajeno a la norma.
    let staging = repo.domain("00_SYSTEM").join("staging");
    std::fs::create_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;
    let pkg = PackageDir::create(&staging, &package_name)?;

    let copiados = copy_payload(project.root(), &pkg, &shipment.profile, payload)?;
    if copiados == 0 {
        let _ = std::fs::remove_dir_all(pkg.root());
        return Err(Error::input(format!(
            "El alcance declarado no contiene ningún archivo. El envío no se ha emitido. Revisar el contenido seleccionado en {}.",
            project.root().display()
        )));
    }

    // El extracto del registro de eventos forma parte de `data/` y por tanto del
    // alcance declarado. Se escribe antes de contar: un recuento que no incluya
    // todos los archivos de la carga hace fallar la verificación 10 del
    // apartado 37.2 en el destino.
    write_log_extract(repo, &pkg, &project_uid)?;

    let issued = clock::now_rfc3339();
    let entradas = fsx::walk::conserved_files(&pkg.root().join("data"));
    let file_count = entradas.len();
    let total_bytes = fsx::walk::total_bytes(&entradas);

    // Paso 11 (parte): asientos de la cronología generados por el emisor.
    let mut history: Vec<ChronologyEntry> = Vec::new();
    if shipment.transfers_custody || shipment.returns_custody {
        let base = project.next_custody_seq();
        history.push(ChronologyEntry {
            seq: base,
            action: CustodyAction::Exported,
            ts: issued.clone(),
            actor: actor.to_string(),
            org: shipment.issuer_org.clone(),
            shipment_id: Some(shipment_id.clone()),
        });
        history.push(ChronologyEntry {
            seq: base + 1,
            action: CustodyAction::Sent,
            ts: issued.clone(),
            actor: actor.to_string(),
            org: shipment.issuer_org.clone(),
            shipment_id: Some(shipment_id.clone()),
        });
    }

    // Paso 6: redactar el manifiesto de intercambio.
    let firma = if shipment.classification.requires_signature() {
        Some(bagit::TAGMANIFEST_SIG.to_string())
    } else {
        None
    };
    let mut exchange = ExchangeManifest::new(pkg.exchange_manifest());
    *exchange.doc_mut() = ExchangeBuilder {
        profile: shipment.profile,
        level: project.level().as_str().to_string(),
        shipment_id: shipment_id.clone(),
        issued: issued.clone(),
        supersedes: shipment.supersedes.clone(),
        revision: shipment.revision.clone(),
        issuer_org: shipment.issuer_org.clone(),
        issuer_contact: shipment.issuer_contact.clone(),
        issuer_key_id: shipment.issuer_key_id.clone(),
        recipient_org: shipment.recipient_org.clone(),
        recipient_contact: shipment.recipient_contact.clone(),
        purpose: shipment.purpose.clone(),
        projects: vec![project_uid.clone()],
        releases: project.release_uid().map(|r| vec![r.to_string()]).unwrap_or_default(),
        file_count: file_count as i64,
        total_bytes: total_bytes as i64,
        signature: firma.clone(),
        classification: shipment.classification,
        usage_permitted: shipment.usage_permitted.clone(),
        usage_territory: shipment.usage_territory.clone(),
        usage_term: shipment.usage_term.clone(),
        sublicensing: shipment.sublicensing,
        forwarding: shipment.forwarding,
        retention_until: shipment.retention_until.clone(),
        destroy_on_expiry: shipment.destroy_on_expiry,
        personal_data: shipment.personal_data,
        personal_data_categories: shipment.personal_data_categories.clone(),
        serialized: shipment.serialize,
        ack_deadline_hours: shipment.ack_deadline_hours,
        ack_address: shipment.ack_address.clone(),
    }
    .build();

    // El perfil `P` declara los parámetros para reanudar el trabajo
    // (apartado 31.3).
    if shipment.profile.requires_continuation() {
        exchange.doc_mut().set(
            "continuation",
            Node::map(vec![
                ("sample_rate", Node::opt_int(project.sample_rate())),
                ("bit_depth", Node::opt_int(project.bit_depth())),
                (
                    "tuning_hz",
                    project.doc().at("audio.tuning_hz").cloned().unwrap_or(Node::Int(440)),
                ),
                ("tempo", project.doc().at("audio.tempo").cloned().unwrap_or(Node::Null)),
                (
                    "origin",
                    project.doc().at("audio.origin").cloned().unwrap_or(Node::Null),
                ),
                (
                    "vocabulary_frozen",
                    Node::Bool(
                        project
                            .doc()
                            .at("vocabulary.frozen")
                            .and_then(|n| n.as_bool())
                            .unwrap_or(false),
                    ),
                ),
            ]),
        );
        // El perfil P declara el estado de cada autorización pendiente.
        if !pendientes.is_empty() {
            let rights = exchange.doc_mut().ensure_map("rights");
            rights.set("clearances", Node::str("partial"));
            rights.set(
                "pending",
                Node::Seq(pendientes.iter().map(Node::str).collect()),
            );
        }
    }

    if shipment.transfers_custody || shipment.returns_custody {
        exchange.doc_mut().set(
            "custody",
            Node::map(vec![
                ("transfers", Node::Bool(shipment.transfers_custody)),
                ("returns", Node::Bool(shipment.returns_custody)),
                (
                    "supersedes_transfer",
                    Node::opt_str(shipment.supersedes_transfer.as_deref()),
                ),
                ("expected_return", Node::opt_str(shipment.expected_return.as_deref())),
                ("grace_days", Node::Int(shipment.grace_days)),
                ("onward_allowed", Node::Bool(shipment.onward_allowed)),
                (
                    "history",
                    Node::Seq(history.iter().map(ChronologyEntry::to_node).collect()),
                ),
            ]),
        );
    }
    exchange.save()?;

    // `CUSTODY.txt` se incluye únicamente cuando el envío cede o devuelve la
    // custodia, y su contenido coincide con el bloque del manifiesto
    // (apartado 32.1, sexto guion).
    if shipment.transfers_custody {
        let lock = CustodyLock {
            state: CustodyState::Ceded,
            holder: format!("{} / {}", shipment.recipient_org, shipment.recipient_contact),
            ceded_by: format!("{} / {actor}", shipment.issuer_org),
            shipment_id: shipment_id.clone(),
            ceded_at: issued.clone(),
            expected_return: shipment.expected_return.clone().unwrap_or_default(),
            grace_days: shipment.grace_days,
        };
        lock.write(&pkg.custody_txt())?;
    } else if shipment.returns_custody {
        let lock = CustodyLock {
            state: CustodyState::Own,
            holder: format!("{} / {}", shipment.recipient_org, shipment.recipient_contact),
            ceded_by: format!("{} / {actor}", shipment.issuer_org),
            shipment_id: shipment_id.clone(),
            ceded_at: issued.clone(),
            expected_return: String::new(),
            grace_days: 0,
        };
        lock.write(&pkg.custody_txt())?;
    }

    pkg.write_bagit()?;
    pkg.write_bag_info(
        &shipment.issuer_org,
        &date,
        total_bytes,
        file_count,
        &shipment_id,
    )?;
    pkg.write_leeme(
        &shipment_id,
        &shipment.issuer_org,
        &shipment.ack_address,
        shipment.ack_deadline_hours,
        firma.is_some(),
    )?;

    // Paso 7: generar y verificar el manifiesto de integridad.
    pkg.write_payload_manifest()?;
    pkg.write_tag_manifest()?;
    let verificacion = pkg.verify()?;
    if !verificacion.package_integrity_ok() || !verificacion.content_integrity_ok() {
        let _ = std::fs::remove_dir_all(pkg.root());
        return Err(Error::Integrity {
            checked: verificacion.payload_checked + verificacion.tag_checked,
            failed: verificacion.failed_paths(),
        });
    }

    let log = repo.event_log();
    log.append(
        actor,
        event::PACKAGE_BUILT,
        Some(&project_uid),
        json!({
            "shipment_id": shipment_id,
            "profile": shipment.profile.as_str(),
            "classification": shipment.classification.as_str(),
            "files": file_count,
            "total_bytes": total_bytes,
            "minimisation_verified": true,
        }),
    )?;

    // Paso 9: congelar la copia del paquete en la carpeta del perfil y aplicarle
    // acceso de solo lectura por medios técnicos (apartado 32.3).
    let frozen_dir = project.root().join(shipment.profile.frozen_copy_dir());
    let frozen_copy = frozen_dir.join(&package_name);
    fsx::space::ensure_available(&frozen_dir, total_bytes)?;
    fsx::copy_tree(pkg.root(), &frozen_copy, &[])?;
    fsx::readonly::set_tree_readonly(&frozen_copy, true, &[])?;

    // Paso 10: serializar el paquete, o declarar la entrega sin serializar.
    let artifact = if shipment.serialize {
        let destino = frozen_dir.join(format!("{package_name}.{}", container::EXTENSION));
        let c = container::build(pkg.root(), &destino, progress)?;
        crate::fsx::readonly::set_file_readonly(&c, true)?;
        c
    } else {
        frozen_copy.clone()
    };

    // El área de preparación no forma parte del repositorio.
    let _ = std::fs::remove_dir_all(pkg.root());

    // Paso 11: ceder la custodia, cuando el envío la ceda.
    let mut custody_state = project.custody_state();
    if shipment.transfers_custody {
        custody_state = CustodyState::InTransit;
        let cust = project.doc_mut().ensure_map("custody");
        cust.set("state", Node::str(custody_state.as_str()));
        cust.set("since", Node::str(&issued));
        cust.set(
            "transfer",
            Node::map(vec![
                ("shipment_id", Node::str(&shipment_id)),
                (
                    "to",
                    Node::map(vec![
                        ("org", Node::str(&shipment.recipient_org)),
                        ("person", Node::str(&shipment.recipient_contact)),
                    ]),
                ),
                (
                    "expected_return",
                    Node::opt_str(shipment.expected_return.as_deref()),
                ),
                ("grace_days", Node::Int(shipment.grace_days)),
                ("onward_allowed", Node::Bool(shipment.onward_allowed)),
            ]),
        );
    }

    // Paso 13: registrar la entrega en el manifiesto del proyecto y añadir los
    // asientos `exported` y `sent` a la cronología.
    let ruta_relativa = fsx::walk::relative_slash(project.root(), &artifact)
        .unwrap_or_else(|| artifact.display().to_string());
    project.record_delivery(
        &date,
        &shipment.recipient_org,
        &ruta_relativa,
        &shipment_id,
        &shipment.revision,
    );
    for entrada in &history {
        project
            .doc_mut()
            .ensure_map("custody")
            .ensure_seq("history")
            .push(entrada.to_node());
    }
    project.save()?;

    // Paso 11 (continuación): bloquear la copia local en solo lectura y escribir
    // el marcador. Se hace después de guardar el manifiesto para que este quede
    // escrito con el estado ya en tránsito.
    if shipment.transfers_custody {
        fsx::readonly::set_tree_readonly(
            project.root(),
            true,
            &[crate::manifest::PROJECT_FILE, crate::manifest::CUSTODY_LOCK_FILE],
        )?;
        custody_lock::write_marker(
            project.root(),
            &CustodyLock {
                state: CustodyState::InTransit,
                holder: format!("{} / {}", shipment.recipient_org, shipment.recipient_contact),
                ceded_by: format!("{} / {actor}", shipment.issuer_org),
                shipment_id: shipment_id.clone(),
                ceded_at: issued.clone(),
                expected_return: shipment.expected_return.clone().unwrap_or_default(),
                grace_days: shipment.grace_days,
            },
        )?;
    }

    log.append(
        actor,
        event::PACKAGE_EMITTED,
        Some(&project_uid),
        json!({
            "shipment_id": shipment_id,
            "profile": shipment.profile.as_str(),
            "classification": shipment.classification.as_str(),
            "recipient": shipment.recipient_org,
            "files": file_count,
            "transfers_custody": shipment.transfers_custody,
            "returns_custody": shipment.returns_custody,
            "artifact": artifact.display().to_string(),
            "project_id": project_id,
        }),
    )?;

    Ok(Emission {
        shipment_id,
        artifact,
        frozen_copy,
        file_count,
        total_bytes,
        issued,
        custody_state,
    })
}

/// Parámetros de continuación que el proyecto todavía no declara
/// (apartado 31.3).
pub fn missing_continuation(project: &ProjectManifest) -> Vec<String> {
    let mut faltan = Vec::new();
    for (ruta, nombre) in [
        ("audio.sample_rate", "frecuencia de muestreo"),
        ("audio.bit_depth", "profundidad de bits"),
        ("audio.tuning_hz", "afinación de referencia"),
        ("audio.tempo", "tempo"),
        ("audio.origin", "punto temporal de origen"),
    ] {
        let presente = project
            .doc()
            .at(ruta)
            .map(|n| !n.is_null())
            .unwrap_or(false);
        if !presente {
            faltan.push(nombre.to_string());
        }
    }
    faltan
}

/// Material de terceros con autorización sin resolver (apartado 12.2).
pub fn pending_clearances(project: &ProjectManifest) -> Vec<String> {
    project
        .doc()
        .get("sources")
        .and_then(|n| n.as_seq())
        .map(|s| {
            s.iter()
                .filter_map(|e| {
                    let m = e.as_map()?;
                    if m.get("clearance")?.as_str()? == "pending" {
                        Some(m.get("path")?.as_str()?.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn copy_payload(
    project_root: &Path,
    pkg: &PackageDir,
    profile: &Profile,
    payload: &Payload,
) -> Result<usize> {
    let mut copiados = 0usize;

    // El perfil `A` incluye el proyecto archivado completo (Tabla 28).
    if payload.content_dirs.is_empty() && *profile == Profile::Archive {
        copiados += fsx::copy_tree(project_root, &pkg.content(), &["08_DELIVERY", "09_TRANSFER"])?;
    } else {
        for dir in &payload.content_dirs {
            let origen = project_root.join(dir);
            if !origen.is_dir() {
                continue;
            }
            copiados += fsx::copy_tree(&origen, &pkg.content().join(dir), &[])?;
        }
        // El manifiesto del proyecto acompaña siempre al material: sin él, el
        // contenido de `data/content/` no es interpretable.
        let manifiesto = project_root.join(crate::manifest::PROJECT_FILE);
        if manifiesto.is_file() {
            let destino = pkg.content().join(crate::manifest::PROJECT_FILE);
            std::fs::copy(&manifiesto, &destino).map_err(|e| Error::io(&destino, e))?;
            copiados += 1;
        }
    }

    // Documentación de derechos. Los perfiles `E` y `P` aplican la minimización
    // del apartado 34.1; el perfil `A` la incluye íntegra.
    if profile.requires_full_rights() {
        let rights_src = project_root.join("00_ADMIN/Rights");
        if rights_src.is_dir() {
            copiados += fsx::copy_tree(&rights_src, &pkg.rights(), &[])?;
        }
    } else {
        for rel in &payload.rights_files {
            let origen = project_root.join(rel);
            if !origen.is_file() {
                continue;
            }
            let nombre = origen.file_name().unwrap_or_default();
            let destino = pkg.rights().join(nombre);
            std::fs::copy(&origen, &destino).map_err(|e| Error::io(&destino, e))?;
            copiados += 1;
        }
    }
    Ok(copiados)
}

fn write_log_extract(repo: &Repository, pkg: &PackageDir, project_uid: &str) -> Result<()> {
    let entradas = repo.event_log().entries_for(project_uid)?;
    let mut texto = String::new();
    for e in entradas {
        texto.push_str(&e.render());
        texto.push('\n');
    }
    fsx::atomic::write_str(&pkg.log_file(), &texto)
}

/// Notificación de revocación (apartado 39.3).
#[derive(Clone, Debug)]
pub struct Revocation {
    pub shipment_id: String,
    pub scope: String,
    pub grounds: String,
    pub deadline: String,
}

/// Emite una notificación de revocación.
pub fn revoke(
    repo: &Repository,
    actor: &str,
    project_uid: &str,
    revocation: &Revocation,
    sync: crate::clock::SyncState,
) -> Result<()> {
    clock::require_sync_for_emission(sync)?;
    repo.event_log().append(
        actor,
        event::REVOCATION_ISSUED,
        Some(project_uid),
        json!({
            "shipment_id": revocation.shipment_id,
            "scope": revocation.scope,
            "grounds": revocation.grounds,
            "deadline": revocation.deadline,
        }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::SyncState;
    use crate::manifest::project::Level;
    use crate::project::{self, NewProject};

    fn entorno() -> (tempfile::TempDir, Repository, ProjectManifest) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        let mut p = project::create(
            &repo,
            "J. Duarte",
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
                active_volume: Some("vol-local".into()),
            },
        )
        .unwrap();
        std::fs::create_dir_all(p.root().join("07_MASTER")).unwrap();
        std::fs::write(p.root().join("07_MASTER/master.wav"), b"audio del master").unwrap();
        std::fs::create_dir_all(p.root().join("00_ADMIN/Credits")).unwrap();
        std::fs::write(p.root().join("00_ADMIN/Credits/creditos.csv"), b"rol,nombre\n").unwrap();
        // La composición y la grabación han concluido: los parámetros de audio
        // quedan fijados y el proyecto puede emitirse en perfil P.
        let audio = p.doc_mut().ensure_map("audio");
        audio.set("tempo", crate::doc::Node::Int(96));
        audio.set("key", crate::doc::Node::str("Db major"));
        audio.set("origin", crate::doc::Node::str("00:00:00:00"));
        p.doc_mut().set("vocabulary", crate::doc::Node::map(vec![("frozen", crate::doc::Node::Bool(true))]));
        p.save().unwrap();
        (dir, repo, p)
    }

    fn envio() -> Shipment {
        Shipment {
            profile: Profile::Delivery,
            classification: Classification::Internal,
            purpose: "Publicacion digital".into(),
            issuer_org: "Estudio A".into(),
            issuer_contact: "intercambio@a.example".into(),
            issuer_key_id: None,
            recipient_org: "Sello B".into(),
            recipient_contact: "recepcion@b.example".into(),
            usage_permitted: vec!["distribucion digital".into()],
            usage_territory: "mundial".into(),
            usage_term: "indefinido".into(),
            sublicensing: false,
            forwarding: false,
            retention_until: "2031-08-06".into(),
            destroy_on_expiry: true,
            personal_data: false,
            personal_data_categories: vec![],
            ack_deadline_hours: 72,
            ack_address: "intercambio@a.example".into(),
            transfers_custody: false,
            returns_custody: false,
            supersedes_transfer: None,
            expected_return: None,
            grace_days: 0,
            onward_allowed: false,
            supersedes: None,
            revision: "r0".into(),
            serialize: true,
            qc_approved: true,
        }
    }

    fn sincronizado() -> SyncState {
        SyncState::Synced { drift_ms: 40 }
    }

    fn emitir(repo: &Repository, p: &mut ProjectManifest, s: &Shipment) -> Result<Emission> {
        let (mut prog, canc) = container::silent_progress();
        let mut pr = Progress { on_progress: &mut prog, cancelled: &canc };
        emit(repo, "J. Duarte", p, s, &Payload::for_profile(s.profile), sincronizado(), &mut pr)
    }

    #[test]
    fn la_emision_produce_contenedor_copia_congelada_y_registro() {
        let (_d, repo, mut p) = entorno();
        let e = emitir(&repo, &mut p, &envio()).unwrap();

        assert!(e.artifact.is_file(), "el contenedor existe");
        assert_eq!(e.artifact.extension().unwrap(), "stave");
        // El contenedor es verificable sin extraerlo.
        container::verify_structure(&e.artifact).unwrap();
        // La copia congelada va a 08_DELIVERY en perfil E y queda en solo lectura.
        assert!(e.frozen_copy.to_string_lossy().contains("08_DELIVERY"));
        let m = e.frozen_copy.join("EXCHANGE.yaml");
        assert!(std::fs::metadata(&m).unwrap().permissions().readonly());
        assert!(std::fs::metadata(&e.artifact).unwrap().permissions().readonly());

        // El manifiesto del proyecto registra la entrega.
        let entregas = p.doc().get("deliveries").unwrap().as_seq().unwrap();
        assert_eq!(entregas.len(), 1);
        assert_eq!(
            entregas[0].as_map().unwrap().get("shipment_id").unwrap().as_str(),
            Some(e.shipment_id.as_str())
        );

        // El registro lleva la constitución y la emisión por separado.
        let eventos: Vec<String> = repo
            .event_log()
            .entries_for(p.uid().unwrap())
            .unwrap()
            .iter()
            .map(|x| x.event.clone())
            .collect();
        assert!(eventos.contains(&event::PACKAGE_BUILT.to_string()));
        assert!(eventos.contains(&event::PACKAGE_EMITTED.to_string()));
        assert!(repo.event_log().verify_chain().unwrap().is_intact());
    }

    #[test]
    fn el_manifiesto_de_intercambio_del_paquete_valida() {
        let (_d, repo, mut p) = entorno();
        let e = emitir(&repo, &mut p, &envio()).unwrap();
        let m = ExchangeManifest::load(e.frozen_copy.join("EXCHANGE.yaml")).unwrap();
        m.validate_schema().unwrap();
        m.validate_consistency().unwrap();
        assert_eq!(m.shipment_id(), Some(e.shipment_id.as_str()));
        assert_eq!(m.file_count(), Some(e.file_count as i64));
        assert!(!m.affects_custody());
    }

    #[test]
    fn sin_informe_de_control_de_calidad_no_se_emite() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.qc_approved = false;
        let err = emitir(&repo, &mut p, &s).unwrap_err();
        assert_eq!(err.clause(), Some("36"));
        assert!(err.to_string().contains("no se ha emitido"));
    }

    #[test]
    fn sin_reloj_sincronizado_no_se_emite() {
        let (_d, repo, mut p) = entorno();
        let (mut prog, canc) = container::silent_progress();
        let mut pr = Progress { on_progress: &mut prog, cancelled: &canc };
        let s = envio();
        let err = emit(&repo, "a", &mut p, &s, &Payload::for_profile(s.profile), SyncState::Unavailable, &mut pr).unwrap_err();
        assert_eq!(err.clause(), Some("22.3.1"));
    }

    #[test]
    fn una_autorizacion_pendiente_bloquea_los_perfiles_e_y_a() {
        let (_d, repo, mut p) = entorno();
        p.record_source("01_REF/muestra.wav", "Biblioteca", None, "INTERNO", None, None, "pending");
        p.save().unwrap();

        let err = emitir(&repo, &mut p, &envio()).unwrap_err();
        assert!(err.to_string().contains("autorizaciones de terceros sin resolver"), "{err}");

        let mut s = envio();
        s.profile = Profile::Archive;
        assert!(emitir(&repo, &mut p, &s).is_err());

        // El perfil P sí se emite, declarando el estado de cada pendiente.
        let mut s = envio();
        s.profile = Profile::Production;
        let e = emitir(&repo, &mut p, &s).unwrap();
        let m = ExchangeManifest::load(e.frozen_copy.join("EXCHANGE.yaml")).unwrap();
        assert_eq!(m.doc().at("rights.clearances").unwrap().as_str(), Some("partial"));
        assert_eq!(m.doc().at("rights.pending").unwrap().as_seq().unwrap().len(), 1);
        m.validate_schema().unwrap();
    }

    #[test]
    fn la_cesion_bloquea_la_copia_local_y_escribe_el_marcador() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.profile = Profile::Production;
        s.transfers_custody = true;
        s.expected_return = Some("2026-12-31".into());
        s.grace_days = 15;

        let e = emitir(&repo, &mut p, &s).unwrap();

        assert_eq!(e.custody_state, CustodyState::InTransit);
        assert_eq!(p.custody_state(), CustodyState::InTransit);
        // El marcador existe y es legible en texto plano.
        let marcador = p.root().join(crate::manifest::CUSTODY_LOCK_FILE);
        assert!(marcador.is_file());
        let lock = crate::manifest::custody_lock::CustodyLock::load(&marcador).unwrap();
        assert_eq!(lock.state, CustodyState::InTransit);
        assert_eq!(lock.shipment_id, e.shipment_id);
        // La copia local queda en solo lectura por medios técnicos.
        let master = p.root().join("07_MASTER/master.wav");
        assert!(std::fs::metadata(&master).unwrap().permissions().readonly());
        // La cronología lleva los asientos exported y sent, consecutivos.
        let hist = p.custody_history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].action, CustodyAction::Exported);
        assert_eq!(hist[1].action, CustodyAction::Sent);
        assert_eq!(hist[0].seq, 1);
        assert_eq!(hist[1].seq, 2);
        assert!(crate::custody::check_chronology(&hist).is_clean());
        // El paquete lleva CUSTODY.txt.
        assert!(e.frozen_copy.join("CUSTODY.txt").is_file());
    }

    #[test]
    fn un_perfil_e_no_cede_la_custodia_y_una_cesion_exige_fecha_de_retorno() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.transfers_custody = true;
        s.expected_return = Some("2026-12-31".into());
        let err = emitir(&repo, &mut p, &s).unwrap_err();
        assert_eq!(err.clause(), Some("41.1"));

        let mut s = envio();
        s.profile = Profile::Production;
        s.transfers_custody = true;
        s.expected_return = None;
        let err = emitir(&repo, &mut p, &s).unwrap_err();
        assert_eq!(err.clause(), Some("41.4"));
    }

    #[test]
    fn un_proyecto_ya_cedido_no_se_cede_de_nuevo() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.profile = Profile::Production;
        s.transfers_custody = true;
        s.expected_return = Some("2026-12-31".into());
        s.grace_days = 15;
        emitir(&repo, &mut p, &s).unwrap();

        let err = emitir(&repo, &mut p, &s).unwrap_err();
        assert_eq!(err.clause(), Some("14.3"));
    }

    #[test]
    fn la_clasificacion_confidencial_declara_la_firma_y_el_leeme_la_explica() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.classification = Classification::Confidential;
        let e = emitir(&repo, &mut p, &s).unwrap();
        let m = ExchangeManifest::load(e.frozen_copy.join("EXCHANGE.yaml")).unwrap();
        assert_eq!(m.signature_name(), Some(bagit::TAGMANIFEST_SIG));
        let leeme = std::fs::read_to_string(e.frozen_copy.join("LEEME.txt")).unwrap();
        assert!(leeme.contains("gpg --verify"));
    }

    #[test]
    fn la_entrega_sin_serializar_conserva_los_archivos_del_apartado_32_1() {
        let (_d, repo, mut p) = entorno();
        let mut s = envio();
        s.serialize = false;
        let e = emitir(&repo, &mut p, &s).unwrap();
        assert!(e.artifact.is_dir());
        for archivo in ["bagit.txt", "EXCHANGE.yaml", "LEEME.txt", "manifest-sha256.txt", "tagmanifest-sha256.txt"] {
            assert!(e.artifact.join(archivo).is_file(), "falta {archivo}");
        }
        let m = ExchangeManifest::load(e.artifact.join("EXCHANGE.yaml")).unwrap();
        assert!(!m.is_serialized());
    }

    #[test]
    fn los_regenerables_del_anexo_c_no_entran_en_el_paquete() {
        let (_d, repo, mut p) = entorno();
        std::fs::write(p.root().join("07_MASTER/.DS_Store"), b"basura").unwrap();
        std::fs::write(p.root().join("07_MASTER/master.wav.asd"), b"analisis").unwrap();
        let e = emitir(&repo, &mut p, &envio()).unwrap();
        assert!(!e.frozen_copy.join("data/content/07_MASTER/.DS_Store").exists());
        assert!(!e.frozen_copy.join("data/content/07_MASTER/master.wav.asd").exists());
        assert!(e.frozen_copy.join("data/content/07_MASTER/master.wav").is_file());
    }

    #[test]
    fn el_perfil_p_no_se_emite_sin_los_parametros_de_continuacion() {
        let (_d, repo, mut p) = entorno();
        // El punto temporal de origen vuelve a estar pendiente: la composición
        // no ha concluido.
        p.doc_mut().ensure_map("audio").set("origin", crate::doc::Node::Null);
        p.save().unwrap();

        let mut s = envio();
        s.profile = Profile::Production;
        let err = emitir(&repo, &mut p, &s).unwrap_err();
        assert_eq!(err.clause(), Some("31.3"));
        assert!(err.to_string().contains("punto temporal de origen"), "{err}");
        assert!(err.to_string().contains("no se ha emitido"), "{err}");
        assert_eq!(missing_continuation(&p), vec!["punto temporal de origen"]);
    }

    #[test]
    fn el_alcance_declarado_coincide_con_el_contenido() {
        let (_d, repo, mut p) = entorno();
        let e = emitir(&repo, &mut p, &envio()).unwrap();
        let reales = fsx::walk::conserved_files(&e.frozen_copy.join("data"));
        let m = ExchangeManifest::load(e.frozen_copy.join("EXCHANGE.yaml")).unwrap();
        assert_eq!(m.file_count(), Some(reales.len() as i64));
        assert_eq!(m.total_bytes(), Some(fsx::walk::total_bytes(&reales) as i64));
        // El extracto del registro se cuenta como parte de la carga.
        assert!(reales.iter().any(|x| x.relative == "log/EXCHANGE.jsonl"));
    }

    #[test]
    fn cada_envio_recibe_un_identificador_distinto() {
        let (_d, repo, mut p) = entorno();
        let a = emitir(&repo, &mut p, &envio()).unwrap();
        let b = emitir(&repo, &mut p, &envio()).unwrap();
        assert_ne!(a.shipment_id, b.shipment_id);
        assert_ne!(a.artifact, b.artifact);
        // Y el paquete anterior no se ha modificado (apartado 32.3).
        assert!(a.artifact.is_file());
    }
}
