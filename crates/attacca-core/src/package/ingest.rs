//! Procedimiento de recepción e ingesta (apartados 37, 38 y 39).
//!
//! Un paquete recibido permanece en cuarentena hasta que su verificación
//! concluye. Cuando cualquiera de las verificaciones 1 a 6 falla, el paquete se
//! rechaza íntegramente y no se extrae ningún archivo de él.

use super::bagit::{self, PackageDir};
use super::container::{self, Progress};
use crate::clock;
use crate::custody::{self, ChronologyEntry, CustodyAction, CustodyState};
use crate::doc::Node;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::fsx;
use crate::integrity;
use crate::manifest::custody_lock;
use crate::manifest::exchange::{ExchangeManifest, Profile};
use crate::manifest::project::ProjectManifest;
use crate::manifest::receipt::{
    self, CustodyAcceptance, Outcome, Receipt, ReceiptResult, VerificationSet,
};
use crate::repo::Repository;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Datos que la parte receptora aporta a la verificación.
#[derive(Clone, Debug)]
pub struct ReceptionContext {
    pub recipient_org: String,
    pub officer: String,
    pub key_id: Option<String>,
    /// Organizaciones con acuerdo de intercambio vigente (verificación 1).
    pub agreed_parties: Vec<String>,
    /// Identidades de firma declaradas y no revocadas (verificación 2).
    pub declared_identities: Vec<String>,
    /// Identidades revocadas (apartado 35.3, tercer guion).
    pub revoked_identities: Vec<String>,
    /// Versiones de la norma que la parte receptora admite (verificación 7).
    pub supported_versions: Vec<String>,
    /// Perfiles admitidos.
    pub supported_profiles: Vec<Profile>,
    /// Las condiciones de uso declaradas son compatibles con la finalidad para
    /// la que se solicitó el material (verificación 11). Es un juicio de la
    /// persona responsable, no algo que Attacca pueda decidir.
    pub usage_acceptable: bool,
    /// Identificadores de envío ya registrados, para detectar reenvíos.
    pub known_shipments: Vec<String>,
    /// Cronología ya registrada del proyecto, cuando se conoce.
    pub known_chronology: Vec<ChronologyEntry>,
}

/// Resultado de la verificación previa a la ingesta.
#[derive(Debug)]
pub struct Verification {
    pub shipment_id: String,
    pub checks: VerificationSet,
    pub result: ReceiptResult,
    pub discrepancies: Vec<(String, String)>,
    /// Directorio del paquete extraído en la cuarentena, cuando la extracción
    /// llegó a producirse.
    pub package_dir: Option<PathBuf>,
    /// Resumen del manifiesto de integridad recibido (apartado 38).
    pub manifest_digest: String,
    pub received_at: String,
    pub profile: Option<Profile>,
    pub transfers_custody: bool,
}

impl Verification {
    pub fn accepted(&self) -> bool {
        self.result.allows_ingest()
    }

    /// Las catorce verificaciones con su resultado, para presentarlas.
    pub fn checks_as_pairs(&self) -> Vec<(&'static str, &'static str)> {
        self.checks
            .as_pairs()
            .iter()
            .map(|(nombre, resultado)| (*nombre, resultado.as_str()))
            .collect()
    }
}

/// Ejecuta las catorce verificaciones de la Tabla 31, en el orden indicado.
///
/// El paquete se deja en la cuarentena. La ingesta es una operación posterior y
/// separada: el apartado 37.1 exige que el material permanezca en `40_INBOX`
/// hasta que la verificación concluya con resultado conforme.
pub fn verify(
    repo: &Repository,
    actor: &str,
    package_path: &Path,
    ctx: &ReceptionContext,
    progress: &mut Progress,
) -> Result<Verification> {
    let received_at = clock::now_rfc3339();
    let mut checks = VerificationSet::default();
    let mut discrepancies: Vec<(String, String)> = Vec::new();
    let mut shipment_id = String::new();
    let mut manifest_digest = String::new();
    let mut profile = None;
    let mut transfers_custody = false;

    // Verificación 3: el contenedor. Se ejecuta antes de extraer nada, porque el
    // fallo obliga a no extraerlo (Tabla 31, consecuencia del fallo).
    let (package_dir, es_contenedor) = if package_path.is_file() {
        match container::verify_structure(package_path) {
            Ok(_) => {
                checks.container = Outcome::Pass;
                let destino = package_path
                    .parent()
                    .unwrap_or(&repo.inbox())
                    .join(".extraccion");
                match container::extract(package_path, &destino, progress) {
                    Ok(dir) => (Some(dir), true),
                    Err(e) => {
                        checks.container = Outcome::Fail;
                        discrepancies.push(("container".into(), e.to_string()));
                        (None, true)
                    }
                }
            }
            Err(e) => {
                checks.container = Outcome::Fail;
                discrepancies.push(("container".into(), e.to_string()));
                (None, true)
            }
        }
    } else if package_path.is_dir() {
        // Entrega sin serializar (apartado 32.4.5).
        checks.container = Outcome::NotApplicable;
        (Some(package_path.to_path_buf()), false)
    } else {
        checks.container = Outcome::Fail;
        discrepancies.push((
            "container".into(),
            format!("No existe {}.", package_path.display()),
        ));
        (None, false)
    };

    if let Some(dir) = &package_dir {
        let pkg = PackageDir::open(dir.clone());

        // Verificación 6: validez del manifiesto. Se lee antes que las de
        // procedencia y autenticidad porque de él salen los datos que estas
        // necesitan.
        let exchange = match ExchangeManifest::load(pkg.exchange_manifest()) {
            Ok(m) => {
                match m.validate_schema().and_then(|_| m.validate_consistency()) {
                    Ok(()) => checks.manifest_schema = Outcome::Pass,
                    Err(e) => {
                        checks.manifest_schema = Outcome::Fail;
                        discrepancies.push(("manifest_schema".into(), e.to_string()));
                    }
                }
                Some(m)
            }
            Err(e) => {
                checks.manifest_schema = Outcome::Fail;
                discrepancies.push(("manifest_schema".into(), e.to_string()));
                None
            }
        };

        if let Some(m) = &exchange {
            shipment_id = m.shipment_id().unwrap_or_default().to_string();
            profile = m.profile();
            transfers_custody = m.transfers_custody();

            // Verificación 1: procedencia.
            let emisor = m.issuer_org().unwrap_or_default();
            checks.provenance = Outcome::from_bool(ctx.agreed_parties.iter().any(|p| p == emisor));
            if checks.provenance.failed() {
                discrepancies.push((
                    "provenance".into(),
                    format!("El emisor «{emisor}» no tiene acuerdo de intercambio vigente."),
                ));
            }

            // Verificación 2: autenticidad. Cuando el manifiesto declara firma,
            // la identidad debe estar declarada y no revocada. Attacca no
            // implementa OpenPGP: la comprobación criptográfica corresponde a la
            // herramienta de firma, y su resultado se aporta en el contexto.
            match m.signature_name() {
                None => checks.authenticity = Outcome::NotApplicable,
                Some(_) => {
                    let key = m
                        .doc()
                        .at("parties.issuer.key_id")
                        .and_then(|n| n.present_str())
                        .unwrap_or_default();
                    let revocada = ctx.revoked_identities.iter().any(|k| k == key);
                    let declarada = ctx.declared_identities.iter().any(|k| k == key);
                    checks.authenticity = Outcome::from_bool(declarada && !revocada);
                    if checks.authenticity.failed() {
                        let motivo = if revocada {
                            "la identidad de firma está revocada"
                        } else {
                            "la identidad de firma no está declarada en el acuerdo de intercambio"
                        };
                        discrepancies.push((
                            "authenticity".into(),
                            format!("El envío se rechaza: {motivo}."),
                        ));
                    }
                }
            }

            // Verificación 7: perfil y versión.
            let version = m.stave_version().unwrap_or_default();
            let version_ok = ctx.supported_versions.iter().any(|v| v == version);
            let perfil_ok = profile
                .map(|p| ctx.supported_profiles.contains(&p))
                .unwrap_or(false);
            checks.profile_supported = Outcome::from_bool(version_ok && perfil_ok);
            if checks.profile_supported.failed() {
                discrepancies.push((
                    "profile_supported".into(),
                    format!("Version de la norma «{version}» o perfil declarado no admitidos."),
                ));
            }

            // Verificación 8: custodia.
            if m.affects_custody() {
                let mut ok = true;
                let mut motivo = String::new();
                if m.transfers_custody() {
                    if m.expected_return().is_none() {
                        ok = false;
                        motivo = "el envío cede la custodia y no declara fecha esperada de retorno"
                            .into();
                    } else if m.grace_days() <= 0 {
                        ok = false;
                        motivo = "el envío cede la custodia y no declara plazo de gracia".into();
                    } else if !profile.map(|p| p.may_transfer_custody()).unwrap_or(false) {
                        ok = false;
                        motivo = "el perfil declarado no admite la cesión de la custodia".into();
                    }
                }
                checks.custody = Outcome::from_bool(ok);
                if !ok {
                    discrepancies
                        .push(("custody".into(), format!("La cesión se rechaza: {motivo}.")));
                }
            } else {
                checks.custody = Outcome::NotApplicable;
            }

            // Verificación 9: cronología.
            let recibidos = m.custody_history();
            if recibidos.is_empty() {
                checks.chronology = Outcome::NotApplicable;
            } else {
                let fusion = custody::merge_chronology(&ctx.known_chronology, &recibidos);
                let c = custody::check_chronology(&fusion);
                // Un asiento duplicado o suprimido bloquea; una marca decreciente
                // no lo hace por sí sola (apartado 14.3.4).
                checks.chronology = Outcome::from_bool(!c.is_blocking());
                if c.is_blocking() {
                    discrepancies.push((
                        "chronology".into(),
                        format!(
                            "Los asientos recibidos presentan {} números duplicados y {} ausentes.",
                            c.duplicate_seq.len(),
                            c.missing_seq.len()
                        ),
                    ));
                } else if !c.decreasing_timestamps.is_empty() {
                    discrepancies.push((
                        "chronology".into(),
                        format!(
                            "{} asientos presentan marca temporal decreciente. El orden lo determina el número de secuencia; la desviación se trata conforme al apartado 22.3.2.",
                            c.decreasing_timestamps.len()
                        ),
                    ));
                }
            }

            // Verificación 11: condiciones de uso y retención.
            checks.usage_accepted = Outcome::from_bool(ctx.usage_acceptable);
            if checks.usage_accepted.failed() {
                discrepancies.push((
                    "usage_accepted".into(),
                    "Las condiciones de uso declaradas no son compatibles con la finalidad para la que se solicitó el material.".into(),
                ));
            }
        }

        // Verificaciones 4 y 5: integridad del empaquetado y del contenido.
        let pv = pkg.verify()?;
        checks.package_integrity = Outcome::from_bool(pv.package_integrity_ok());
        if !pv.package_integrity_ok() {
            discrepancies.push((
                "package_integrity".into(),
                format!(
                    "{} archivos de etiqueta no coinciden o faltan: {}.",
                    pv.tag_failures.len() + pv.missing_tag_files.len(),
                    pv.tag_failures
                        .iter()
                        .chain(pv.missing_tag_files.iter())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        checks.content_integrity = Outcome::from_bool(pv.content_integrity_ok());
        if !pv.content_integrity_ok() {
            discrepancies.push((
                "content_integrity".into(),
                format!(
                    "La verificación de integridad falló en {} de {} archivos: {}.",
                    pv.payload_missing.len()
                        + pv.payload_mismatched.len()
                        + pv.payload_undeclared.len(),
                    pv.payload_checked,
                    pv.failed_paths().join(", ")
                ),
            ));
        }

        // Resumen del manifiesto de integridad recibido, que acredita qué se
        // recibió exactamente.
        let manifest_path = dir.join(bagit::MANIFEST_TXT);
        if manifest_path.is_file() {
            manifest_digest = integrity::digest_file(&manifest_path)?;
        }

        // Verificación 10: correspondencia del alcance.
        if let Some(m) = &exchange {
            let reales = fsx::walk::conserved_files(&dir.join("data"));
            let n = reales.len() as i64;
            let bytes = fsx::walk::total_bytes(&reales) as i64;
            let coincide = m.file_count() == Some(n) && m.total_bytes() == Some(bytes);
            checks.scope_match = Outcome::from_bool(coincide);
            if !coincide {
                discrepancies.push((
                    "scope_match".into(),
                    format!(
                        "El alcance declara {} archivos y {} bytes; el contenido tiene {n} archivos y {bytes} bytes.",
                        m.file_count().unwrap_or(-1),
                        m.total_bytes().unwrap_or(-1)
                    ),
                ));
            }

            // Verificación 12: datos personales.
            let declara = m
                .doc()
                .at("personal_data.present")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            let hay_derechos = fsx::walk::conserved_files(&dir.join("data/rights")).is_empty();
            // Se comprueba lo comprobable: material de derechos presente sin
            // declaración de datos personales. La correspondencia de categorías
            // exige juicio humano y se recoge como discrepancia, no como fallo
            // automático.
            checks.personal_data_match = Outcome::from_bool(declara || hay_derechos);
            if checks.personal_data_match.failed() {
                discrepancies.push((
                    "personal_data_match".into(),
                    "El paquete incluye documentacion de derechos y el manifiesto declara que no contiene datos personales.".into(),
                ));
            }

            // Verificación 13: conformidad estructural de `data/content/`.
            checks.structure = Outcome::from_bool(check_structure(
                &dir.join("data/content"),
                &mut discrepancies,
            ));

            // Verificación 14: las comprobaciones que el emisor declara esperar.
            let esperadas = m.expected_checks();
            let ejecutadas = ["container", "integrity", "schema", "structure", "custody"];
            let sin_ejecutar: Vec<String> = esperadas
                .iter()
                .filter(|c| !ejecutadas.contains(&c.as_str()))
                .cloned()
                .collect();
            checks.declared_checks = if sin_ejecutar.is_empty() {
                Outcome::Pass
            } else {
                discrepancies.push((
                    "declared_checks".into(),
                    format!(
                        "El emisor declara comprobaciones que esta implementación no ejecuta de forma automática: {}. Deben ejecutarse a mano antes de la ingesta.",
                        sin_ejecutar.join(", ")
                    ),
                ));
                Outcome::Fail
            };
        }

        // Un paquete rechazado no debe dejar material extraído en la cuarentena
        // (apartado 39.1, tercer guion).
        if checks.blocking_failed() && es_contenedor {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    // Un reenvío del mismo identificador es una no conformidad: el
    // identificador no debe reutilizarse (apartado 32.2).
    if !shipment_id.is_empty() && ctx.known_shipments.contains(&shipment_id) {
        checks.scope_match = Outcome::Fail;
        discrepancies.push((
            "scope_match".into(),
            format!("El identificador de envío {shipment_id} ya consta registrado. Un identificador de envío no debe reutilizarse."),
        ));
    }

    let result = checks.result();

    repo.event_log().append(
        actor,
        event::PACKAGE_RECEIVED,
        None,
        json!({"shipment_id": shipment_id, "package": package_path.display().to_string()}),
    )?;
    repo.event_log().append(
        actor,
        event::PACKAGE_VERIFIED,
        None,
        json!({
            "shipment_id": shipment_id,
            "result": result.as_str(),
            "first_failure": checks.first_failure(),
            "discrepancies": discrepancies.len(),
        }),
    )?;

    let package_dir = if checks.blocking_failed() {
        None
    } else {
        package_dir
    };

    Ok(Verification {
        shipment_id,
        checks,
        result,
        discrepancies,
        package_dir,
        manifest_digest,
        received_at,
        profile,
        transfers_custody,
    })
}

/// Comprueba que `data/content/` respete la estructura de la Parte 1.
fn check_structure(content: &Path, discrepancies: &mut Vec<(String, String)>) -> bool {
    let Ok(entries) = std::fs::read_dir(content) else {
        discrepancies.push((
            "structure".into(),
            "El paquete no contiene data/content/.".into(),
        ));
        return false;
    };
    let admitidas: Vec<&str> = crate::repo::REQUIRED_PROJECT_DIRS
        .iter()
        .chain(crate::repo::OPTIONAL_PROJECT_DIRS.iter())
        .copied()
        .collect();
    let mut ajenas = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if !admitidas.contains(&name.as_str()) {
                ajenas.push(name);
            }
        } else if !matches!(
            name.as_str(),
            crate::manifest::PROJECT_FILE
                | crate::manifest::CUSTODY_LOCK_FILE
                | crate::manifest::REPLICA_HOLD_FILE
                | crate::manifest::INTEGRITY_FILE
        ) {
            ajenas.push(name);
        }
    }
    if ajenas.is_empty() {
        return true;
    }
    ajenas.sort();
    discrepancies.push((
        "structure".into(),
        format!(
            "data/content/ contiene {} elementos ajenos a la estructura de la Parte 1: {}.",
            ajenas.len(),
            ajenas.join(", ")
        ),
    ));
    false
}
// El Anexo B fija los campos de este artefacto; agruparlos en una
// estructura intermedia solo desplazaría la lista.
#[allow(clippy::too_many_arguments)]
/// Emite el acuse de recibo (apartado 38).
///
/// Se emite con independencia de que el resultado sea la aceptación o el
/// rechazo.
pub fn issue_receipt(
    repo: &Repository,
    actor: &str,
    verification: &Verification,
    ctx: &ReceptionContext,
    destination_class: &str,
    retention_until: Option<&str>,
    accept_custody: bool,
    expected_return: Option<&str>,
    sync: clock::SyncState,
    output: &Path,
) -> Result<Receipt> {
    clock::require_sync_for_emission(sync)?;

    let issued = clock::now_rfc3339();
    let custody = if verification.transfers_custody {
        // Un acuse que acepte el material pero no la custodia es una aceptación
        // con reservas, y la custodia revierte al cedente (apartado 41.2).
        let aceptada = accept_custody
            && verification.result != ReceiptResult::Rejected
            && !verification.checks.custody.failed();
        Some(CustodyAcceptance {
            accepted: aceptada,
            assumed_at: aceptada.then(|| issued.clone()),
            expected_return: expected_return.map(str::to_string),
        })
    } else {
        None
    };

    let mut checks = verification.checks.clone();
    if verification.transfers_custody && !accept_custody && !checks.custody.failed() {
        // El rechazo expreso de la custodia produce aceptación con reservas.
        checks.custody = Outcome::Fail;
    }

    let mut r = Receipt::new(output);
    *r.doc_mut() = receipt::build(
        &verification.shipment_id,
        &verification.received_at,
        &issued,
        &verification.manifest_digest,
        &ctx.recipient_org,
        &ctx.officer,
        ctx.key_id.as_deref(),
        &checks,
        &verification.discrepancies,
        custody,
        retention_until,
        destination_class,
    );
    r.save()?;

    repo.event_log().append(
        actor,
        event::RECEIPT_ISSUED,
        None,
        json!({
            "shipment_id": verification.shipment_id,
            "result": checks.result().as_str(),
            "custody_accepted": accept_custody && verification.transfers_custody,
        }),
    )?;
    if checks.result() == ReceiptResult::Rejected {
        repo.event_log().append(
            actor,
            event::PACKAGE_REJECTED,
            None,
            json!({
                "shipment_id": verification.shipment_id,
                "failed_check": checks.first_failure(),
            }),
        )?;
    }
    Ok(r)
}

/// Resultado de la ingesta.
#[derive(Clone, Debug)]
pub struct Ingestion {
    pub destination: PathBuf,
    /// Clase de ubicación de destino, conforme a la Tabla 32.
    pub destination_class: String,
    pub files: usize,
    /// El proyecto de destino asumió la custodia.
    pub custody_assumed: bool,
}

/// Incorpora el material al repositorio de destino (apartado 37.3).
///
/// Un paquete no se ingiere de forma parcial. La copia depositada en `40_INBOX`
/// se elimina a continuación.
#[allow(clippy::too_many_arguments)]
pub fn ingest(
    repo: &Repository,
    actor: &str,
    verification: &Verification,
    ctx: &ReceptionContext,
    target_project: Option<&mut ProjectManifest>,
    original_package: &Path,
) -> Result<Ingestion> {
    if !verification.accepted() {
        return Err(Error::requirement(
            "39.1",
            format!(
                "El envío {} fue rechazado. No se ha ingerido nada. Un envío rechazado no debe ingerirse ni siquiera de forma parcial, y su material no debe emplearse para ninguna finalidad.",
                verification.shipment_id
            ),
        ));
    }
    let Some(package_dir) = &verification.package_dir else {
        return Err(Error::input(
            "El paquete no está disponible para la ingesta. No se ha ingerido nada. Repetir la verificación.".to_string(),
        ));
    };

    let pkg = PackageDir::open(package_dir.clone());
    let exchange = ExchangeManifest::load(pkg.exchange_manifest())?;
    let perfil = exchange.profile().unwrap_or(Profile::Delivery);
    let contenido = pkg.content();

    // Tabla 32: destino del material según el perfil.
    let (destino, clase) = match (perfil, target_project.is_some()) {
        (Profile::Archive, _) => {
            let d = repo.archive().join(
                package_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| verification.shipment_id.clone()),
            );
            (d, "30_ARCHIVE".to_string())
        }
        (_, true) => {
            // El material recibido va a `01_REF` del proyecto de destino y
            // permanece en solo lectura (apartado 7.3 y Tabla 32).
            let p = target_project.as_ref().unwrap();
            (
                p.root().join("01_REF").join(&verification.shipment_id),
                "01_REF".to_string(),
            )
        }
        (Profile::Production, false) => {
            let d = repo
                .domain("20_PROJECTS")
                .join("1_ACTIVE")
                .join(&verification.shipment_id);
            (d, "project".to_string())
        }
        (Profile::Delivery, false) => {
            let d = repo
                .inbox()
                .join(format!("ingerido_{}", verification.shipment_id));
            (d, "01_REF".to_string())
        }
    };

    let entradas = fsx::walk::conserved_files(&contenido);
    fsx::space::ensure_available(&destino, fsx::walk::total_bytes(&entradas))?;
    let files = fsx::copy_tree(&contenido, &destino, &[])?;

    // El manifiesto de intercambio recibido y el de integridad se conservan
    // junto al material durante todo el periodo de retención (apartado 37.3).
    let admin = destino.join("00_ADMIN/Rights");
    std::fs::create_dir_all(&admin).map_err(|e| Error::io(&admin, e))?;
    for archivo in [crate::manifest::EXCHANGE_FILE, bagit::MANIFEST_TXT] {
        let origen = package_dir.join(archivo);
        if origen.is_file() {
            let d = admin.join(archivo);
            std::fs::copy(&origen, &d).map_err(|e| Error::io(&d, e))?;
        }
    }
    // La documentación de derechos acompaña al material.
    let rights = pkg.rights();
    if rights.is_dir() && !fsx::walk::conserved_files(&rights).is_empty() {
        fsx::copy_tree(&rights, &admin, &[])?;
    }

    // El material original permanece en solo lectura (Tabla 32).
    if clase == "01_REF" || clase == "30_ARCHIVE" {
        fsx::readonly::set_tree_readonly(&destino, true, &[])?;
    }

    let mut custody_assumed = false;

    if let Some(project) = target_project {
        // El manifiesto del proyecto de destino registra el origen del material
        // (apartado 37.3, primer guion).
        project.record_source(
            &fsx::walk::relative_slash(project.root(), &destino).unwrap_or_default(),
            exchange.issuer_org().unwrap_or("desconocido"),
            Some(&verification.shipment_id),
            exchange
                .classification()
                .map(|c| c.as_str())
                .unwrap_or("INTERNO"),
            exchange
                .doc()
                .at("usage.permitted")
                .and_then(|n| n.as_seq())
                .map(|s| {
                    s.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .as_deref(),
            exchange
                .doc()
                .at("retention.until")
                .and_then(|n| n.present_str()),
            "cleared",
        );

        // Cesión de la custodia: la parte receptora registra `custody_assumed`,
        // establece el estado en propia y lo declara en el acuse
        // (apartado 37.3, último guion).
        if verification.transfers_custody && !verification.checks.custody.failed() {
            let recibidos = exchange.custody_history();
            let propios = project.custody_history();
            let mut fusion = custody::merge_chronology(&propios, &recibidos);
            let siguiente = fusion.iter().map(|e| e.seq).max().unwrap_or(0) + 1;
            let ahora = clock::now_rfc3339();
            fusion.push(ChronologyEntry {
                seq: siguiente,
                action: CustodyAction::Received,
                ts: verification.received_at.clone(),
                actor: actor.to_string(),
                org: ctx.recipient_org.clone(),
                shipment_id: Some(verification.shipment_id.clone()),
            });
            fusion.push(ChronologyEntry {
                seq: siguiente + 1,
                action: CustodyAction::Imported,
                ts: ahora.clone(),
                actor: actor.to_string(),
                org: ctx.recipient_org.clone(),
                shipment_id: Some(verification.shipment_id.clone()),
            });
            fusion.push(ChronologyEntry {
                seq: siguiente + 2,
                action: CustodyAction::CustodyAssumed,
                ts: ahora.clone(),
                actor: actor.to_string(),
                org: ctx.recipient_org.clone(),
                shipment_id: Some(verification.shipment_id.clone()),
            });

            let cust = project.doc_mut().ensure_map("custody");
            cust.set("state", Node::str(CustodyState::Own.as_str()));
            cust.set("since", Node::str(&ahora));
            cust.set(
                "history",
                Node::Seq(fusion.iter().map(ChronologyEntry::to_node).collect()),
            );
            cust.remove("transfer");
            custody_lock::remove_marker(project.root())?;
            custody_assumed = true;

            repo.event_log().append(
                actor,
                event::CUSTODY_ASSUMED,
                project.uid(),
                json!({"shipment_id": verification.shipment_id}),
            )?;
        }
        project.save()?;
    }

    let log = repo.event_log();
    log.append(
        actor,
        event::PACKAGE_INGESTED,
        None,
        json!({
            "shipment_id": verification.shipment_id,
            "destination": destino.display().to_string(),
            "destination_class": clase,
            "files": files,
            "profile": perfil.as_str(),
        }),
    )?;

    // La copia depositada en la cuarentena se elimina tras la ingesta
    // (apartado 37.3, primer párrafo).
    let _ = std::fs::remove_dir_all(package_dir);
    if original_package.is_file() {
        let _ = fsx::readonly::set_file_readonly(original_package, false);
        let _ = std::fs::remove_file(original_package);
    } else if original_package.is_dir() && original_package != package_dir {
        let _ = std::fs::remove_dir_all(original_package);
    }

    Ok(Ingestion {
        destination: destino,
        destination_class: clase,
        files,
        custody_assumed,
    })
}

/// Suprime de la cuarentena un paquete rechazado (apartado 39.1, tercer guion).
pub fn discard_rejected(
    repo: &Repository,
    actor: &str,
    package_path: &Path,
    shipment_id: &str,
) -> Result<()> {
    if package_path.is_file() {
        let _ = fsx::readonly::set_file_readonly(package_path, false);
        std::fs::remove_file(package_path).map_err(|e| Error::io(package_path, e))?;
    } else if package_path.is_dir() {
        let _ = fsx::readonly::set_tree_readonly(package_path, false, &[]);
        std::fs::remove_dir_all(package_path).map_err(|e| Error::io(package_path, e))?;
    }
    let extraccion = repo.inbox().join(".extraccion");
    if extraccion.is_dir() {
        let _ = std::fs::remove_dir_all(&extraccion);
    }
    repo.event_log().append(
        actor,
        event::MATERIAL_DESTROYED,
        None,
        json!({"shipment_id": shipment_id, "reason": "envio rechazado"}),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::emit::{self, Payload, Shipment};
    use super::*;
    use crate::clock::SyncState;
    use crate::manifest::exchange::Classification;
    use crate::manifest::project::Level;
    use crate::project::{self, NewProject};

    struct Escenario {
        _dir_a: tempfile::TempDir,
        _dir_b: tempfile::TempDir,
        emisor: Repository,
        receptor: Repository,
        proyecto: ProjectManifest,
    }

    fn escenario() -> Escenario {
        let dir_a = crate::pruebas::raiz_temporal().unwrap();
        let dir_b = crate::pruebas::raiz_temporal().unwrap();
        let emisor = Repository::create(dir_a.path().join(".stave")).unwrap();
        let receptor = Repository::create(dir_b.path().join(".stave")).unwrap();
        let mut p = project::create(
            &emisor,
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
                active_volume: Some("vol-a".into()),
            },
        )
        .unwrap();
        std::fs::create_dir_all(p.root().join("07_MASTER")).unwrap();
        std::fs::write(p.root().join("07_MASTER/master.wav"), b"audio del master").unwrap();
        std::fs::create_dir_all(p.root().join("05_STEMS")).unwrap();
        std::fs::write(p.root().join("05_STEMS/01_KICK.wav"), b"stem").unwrap();
        let audio = p.doc_mut().ensure_map("audio");
        audio.set("tempo", Node::Int(96));
        audio.set("origin", Node::str("00:00:00:00"));
        p.doc_mut()
            .set("vocabulary", Node::map(vec![("frozen", Node::Bool(true))]));
        p.save().unwrap();
        Escenario {
            _dir_a: dir_a,
            _dir_b: dir_b,
            emisor,
            receptor,
            proyecto: p,
        }
    }

    fn envio(profile: Profile, cede: bool) -> Shipment {
        Shipment {
            profile,
            classification: Classification::Internal,
            purpose: "Continuacion de la mezcla".into(),
            issuer_org: "Estudio A".into(),
            issuer_contact: "intercambio@a.example".into(),
            issuer_key_id: None,
            recipient_org: "Estudio B".into(),
            recipient_contact: "recepcion@b.example".into(),
            usage_permitted: vec!["mezcla".into()],
            usage_territory: "mundial".into(),
            usage_term: "6 meses".into(),
            sublicensing: false,
            forwarding: false,
            retention_until: "2027-08-06".into(),
            destroy_on_expiry: true,
            personal_data: false,
            personal_data_categories: vec![],
            ack_deadline_hours: 72,
            ack_address: "intercambio@a.example".into(),
            transfers_custody: cede,
            returns_custody: false,
            supersedes_transfer: None,
            expected_return: cede.then(|| "2026-12-31".to_string()),
            grace_days: if cede { 15 } else { 0 },
            onward_allowed: false,
            supersedes: None,
            revision: "r0".into(),
            serialize: true,
            qc_approved: true,
        }
    }

    fn contexto() -> ReceptionContext {
        ReceptionContext {
            recipient_org: "Estudio B".into(),
            officer: "M. Rivas".into(),
            key_id: None,
            agreed_parties: vec!["Estudio A".into()],
            declared_identities: vec![],
            revoked_identities: vec![],
            supported_versions: vec!["2.0".into()],
            supported_profiles: vec![Profile::Delivery, Profile::Production, Profile::Archive],
            usage_acceptable: true,
            known_shipments: vec![],
            known_chronology: vec![],
        }
    }

    fn sync() -> SyncState {
        SyncState::Synced { drift_ms: 20 }
    }

    /// Emite un paquete y lo deposita en la cuarentena del receptor.
    fn emitir_y_depositar(e: &mut Escenario, s: &Shipment) -> (String, PathBuf) {
        let (mut prog, canc) = container::silent_progress();
        let mut pr = Progress {
            on_progress: &mut prog,
            cancelled: &canc,
        };
        let em = emit::emit(
            &e.emisor,
            "J. Duarte",
            &mut e.proyecto,
            s,
            &Payload::for_profile(s.profile),
            sync(),
            &mut pr,
        )
        .unwrap();
        let destino = e.receptor.inbox().join(em.artifact.file_name().unwrap());
        std::fs::copy(&em.artifact, &destino).unwrap();
        let _ = crate::fsx::readonly::set_file_readonly(&destino, false);
        (em.shipment_id, destino)
    }

    fn verificar(e: &Escenario, paquete: &Path, ctx: &ReceptionContext) -> Verification {
        let (mut prog, canc) = container::silent_progress();
        let mut pr = Progress {
            on_progress: &mut prog,
            cancelled: &canc,
        };
        verify(&e.receptor, "M. Rivas", paquete, ctx, &mut pr).unwrap()
    }

    #[test]
    fn el_ciclo_completo_de_intercambio_concluye_con_ingesta() {
        let mut e = escenario();
        let (envio_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));

        // Emisión, recepción, verificación.
        let v = verificar(&e, &paquete, &contexto());
        assert_eq!(v.result, ReceiptResult::Accepted, "{:?}", v.discrepancies);
        assert_eq!(v.shipment_id, envio_id);
        assert!(!v.manifest_digest.is_empty());

        // Acuse de recibo.
        let acuse_path = e.receptor.root().join("acuse.yaml");
        let acuse = issue_receipt(
            &e.receptor,
            "M. Rivas",
            &v,
            &contexto(),
            "01_REF",
            Some("2027-08-06"),
            false,
            None,
            sync(),
            &acuse_path,
        )
        .unwrap();
        assert_eq!(acuse.result(), Some(ReceiptResult::Accepted));
        assert_eq!(acuse.shipment_id(), Some(envio_id.as_str()));
        assert_eq!(acuse.manifest_digest(), Some(v.manifest_digest.as_str()));

        // Ingesta.
        let ing = ingest(&e.receptor, "M. Rivas", &v, &contexto(), None, &paquete).unwrap();
        assert!(ing.files > 0);
        assert!(ing.destination.exists());
        // La cuarentena queda vacía de este paquete (apartado 37.3).
        assert!(!paquete.exists());

        assert!(e.receptor.event_log().verify_chain().unwrap().is_intact());
    }

    #[test]
    fn el_ciclo_de_custodia_traslada_el_titular() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Production, true));

        // El cedente queda en tránsito y bloqueado.
        assert_eq!(e.proyecto.custody_state(), CustodyState::InTransit);
        assert!(e
            .proyecto
            .root()
            .join(crate::manifest::CUSTODY_LOCK_FILE)
            .is_file());

        let v = verificar(&e, &paquete, &contexto());
        assert!(v.transfers_custody);
        assert_eq!(v.checks.custody, Outcome::Pass);
        assert_eq!(v.result, ReceiptResult::Accepted, "{:?}", v.discrepancies);

        // El cesionario ingiere y asume la custodia en un proyecto propio.
        let mut destino = project::create(
            &e.receptor,
            "M. Rivas",
            &NewProject {
                title: "Tema Recibido".into(),
                artist: "Artista".into(),
                kind: "MIX".into(),
                level: Level::B,
                release_uid: None,
                release_dir: None,
                sample_rate: 48000,
                bit_depth: 24,
                holder_org: "Estudio B".into(),
                holder_person: "M. Rivas".into(),
                active_volume: Some("vol-b".into()),
            },
        )
        .unwrap();

        let ing = ingest(
            &e.receptor,
            "M. Rivas",
            &v,
            &contexto(),
            Some(&mut destino),
            &paquete,
        )
        .unwrap();
        assert!(ing.custody_assumed);
        assert_eq!(destino.custody_state(), CustodyState::Own);
        assert!(!destino
            .root()
            .join(crate::manifest::CUSTODY_LOCK_FILE)
            .exists());

        // La cronología fusionada es consecutiva y contiene los asientos de
        // ambas partes.
        let hist = destino.custody_history();
        let acciones: Vec<CustodyAction> = hist.iter().map(|x| x.action).collect();
        assert!(acciones.contains(&CustodyAction::Exported));
        assert!(acciones.contains(&CustodyAction::Sent));
        assert!(acciones.contains(&CustodyAction::Received));
        assert!(acciones.contains(&CustodyAction::Imported));
        assert!(acciones.contains(&CustodyAction::CustodyAssumed));
        assert!(custody::check_chronology(&hist).is_clean(), "{hist:?}");

        // El material recibido queda en solo lectura (Tabla 32).
        assert_eq!(ing.destination_class, "01_REF");
    }

    #[test]
    fn rechaza_por_integridad_y_no_ingiere_nada() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));

        // El material se altera en tránsito.
        alterar_contenedor(&paquete, "07_MASTER/master.wav", b"audio manipulado");

        let v = verificar(&e, &paquete, &contexto());
        assert_eq!(v.result, ReceiptResult::Rejected);
        assert_eq!(v.checks.content_integrity, Outcome::Fail);
        assert!(v.checks.blocking_failed());
        // No se ha extraído material utilizable.
        assert!(v.package_dir.is_none());

        let err = ingest(&e.receptor, "M. Rivas", &v, &contexto(), None, &paquete).unwrap_err();
        assert_eq!(err.clause(), Some("39.1"));

        // El acuse de rechazo indica la verificación que falló.
        let acuse = issue_receipt(
            &e.receptor,
            "M. Rivas",
            &v,
            &contexto(),
            "01_REF",
            None,
            false,
            None,
            sync(),
            &e.receptor.root().join("acuse.yaml"),
        )
        .unwrap();
        assert_eq!(acuse.result(), Some(ReceiptResult::Rejected));
        let disc = acuse.doc().get("discrepancies").unwrap().as_seq().unwrap();
        assert!(!disc.is_empty());
    }

    #[test]
    fn rechaza_por_firma_no_declarada_o_revocada() {
        let mut e = escenario();
        let mut s = envio(Profile::Delivery, false);
        s.classification = Classification::Confidential;
        s.issuer_key_id = Some("CLAVE-A".into());
        let (_id, paquete) = emitir_y_depositar(&mut e, &s);

        // Identidad no declarada en el acuerdo de intercambio.
        let mut ctx = contexto();
        ctx.declared_identities = vec![];
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.authenticity, Outcome::Fail);
        assert_eq!(v.result, ReceiptResult::Rejected);

        // Identidad declarada pero revocada.
        let mut ctx = contexto();
        ctx.declared_identities = vec!["CLAVE-A".into()];
        ctx.revoked_identities = vec!["CLAVE-A".into()];
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.authenticity, Outcome::Fail);

        // Identidad declarada y vigente.
        let mut ctx = contexto();
        ctx.declared_identities = vec!["CLAVE-A".into()];
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.authenticity, Outcome::Pass);
        assert_eq!(v.result, ReceiptResult::Accepted, "{:?}", v.discrepancies);
    }

    #[test]
    fn rechaza_por_ruta_no_admitida_en_el_contenedor() {
        let e = escenario();
        // Contenedor con una entrada que escaparía del directorio de destino.
        let malicioso = e.receptor.inbox().join("malicioso.stave");
        {
            use std::io::Write;
            use zip::write::SimpleFileOptions;
            use zip::{CompressionMethod, ZipWriter};
            let mut zip = ZipWriter::new(std::fs::File::create(&malicioso).unwrap());
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(container::MIMETYPE.as_bytes()).unwrap();
            zip.start_file("paquete/../../escapado.txt", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"fuera").unwrap();
            zip.finish().unwrap();
        }

        let v = verificar(&e, &malicioso, &contexto());
        assert_eq!(v.checks.container, Outcome::Fail);
        assert_eq!(v.result, ReceiptResult::Rejected);
        assert!(v.package_dir.is_none());
        // Nada se escribió fuera del destino.
        assert!(!e.receptor.root().join("escapado.txt").exists());
        assert!(!e.receptor.inbox().join("escapado.txt").exists());
    }

    #[test]
    fn rechaza_por_procedencia_sin_acuerdo_vigente() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));
        let mut ctx = contexto();
        ctx.agreed_parties = vec!["Otro Estudio".into()];
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.provenance, Outcome::Fail);
        assert_eq!(v.result, ReceiptResult::Rejected);
    }

    #[test]
    fn una_verificacion_no_bloqueante_produce_aceptacion_con_reservas() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));
        let mut ctx = contexto();
        // Las condiciones de uso no son compatibles con la finalidad prevista.
        ctx.usage_acceptable = false;
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.usage_accepted, Outcome::Fail);
        assert!(!v.checks.blocking_failed());
        assert_eq!(v.result, ReceiptResult::AcceptedWithReservations);
        // El material puede ingerirse: la aceptación con reservas lo admite.
        assert!(v.accepted());
    }

    #[test]
    fn un_identificador_de_envio_no_se_reutiliza() {
        let mut e = escenario();
        let (id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));
        let mut ctx = contexto();
        ctx.known_shipments = vec![id.clone()];
        let v = verificar(&e, &paquete, &ctx);
        assert_eq!(v.checks.scope_match, Outcome::Fail);
        assert!(v.discrepancies.iter().any(|(c, _)| c == "scope_match"));
    }

    #[test]
    fn un_acuse_que_no_acepta_la_custodia_produce_reservas() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Production, true));
        let v = verificar(&e, &paquete, &contexto());
        assert_eq!(v.result, ReceiptResult::Accepted);

        // Aceptar el material sin aceptar la custodia (apartado 41.2).
        let acuse = issue_receipt(
            &e.receptor,
            "M. Rivas",
            &v,
            &contexto(),
            "01_REF",
            None,
            false,
            None,
            sync(),
            &e.receptor.root().join("acuse.yaml"),
        )
        .unwrap();
        assert_eq!(
            acuse.result(),
            Some(ReceiptResult::AcceptedWithReservations)
        );
        assert_eq!(acuse.custody_accepted(), Some(false));
    }

    #[test]
    fn el_acuse_exige_reloj_sincronizado() {
        let mut e = escenario();
        let (_id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));
        let v = verificar(&e, &paquete, &contexto());
        let err = issue_receipt(
            &e.receptor,
            "M. Rivas",
            &v,
            &contexto(),
            "01_REF",
            None,
            false,
            None,
            SyncState::Unavailable,
            &e.receptor.root().join("acuse.yaml"),
        )
        .unwrap_err();
        assert_eq!(err.clause(), Some("22.3.1"));
    }

    #[test]
    fn el_paquete_rechazado_se_suprime_de_la_cuarentena() {
        let mut e = escenario();
        let (id, paquete) = emitir_y_depositar(&mut e, &envio(Profile::Delivery, false));
        discard_rejected(&e.receptor, "M. Rivas", &paquete, &id).unwrap();
        assert!(!paquete.exists());
        assert!(e.receptor.quarantined_packages().is_empty());
    }

    /// Reescribe una entrada del contenedor conservando el resto, para simular
    /// una alteración en tránsito.
    fn alterar_contenedor(contenedor: &Path, sufijo_ruta: &str, contenido: &[u8]) {
        use std::io::{Read, Write};
        use zip::write::SimpleFileOptions;
        use zip::{CompressionMethod, ZipArchive, ZipWriter};

        let mut origen = ZipArchive::new(std::fs::File::open(contenedor).unwrap()).unwrap();
        let temporal = contenedor.with_extension("tmp");
        {
            let mut destino = ZipWriter::new(std::fs::File::create(&temporal).unwrap());
            for i in 0..origen.len() {
                let mut entrada = origen.by_index(i).unwrap();
                let nombre = entrada.name().to_string();
                let mut datos = Vec::new();
                entrada.read_to_end(&mut datos).unwrap();
                let opciones = if nombre == "mimetype" {
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
                } else {
                    SimpleFileOptions::default()
                };
                destino.start_file(&nombre, opciones).unwrap();
                if nombre.ends_with(sufijo_ruta) {
                    destino.write_all(contenido).unwrap();
                } else {
                    destino.write_all(&datos).unwrap();
                }
            }
            destino.finish().unwrap();
        }
        std::fs::rename(&temporal, contenedor).unwrap();
    }
}
