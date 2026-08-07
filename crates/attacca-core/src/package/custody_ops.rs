//! Retorno, reclamación y recuperación de la custodia (apartados 41.3 y 41.4).

use crate::clock;
use crate::custody::{self, ChronologyEntry, CustodyAction, CustodyState, Expiry};
use crate::doc::Node;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::fsx;
use crate::manifest::custody_lock;
use crate::manifest::exchange::ExchangeManifest;
use crate::manifest::project::ProjectManifest;
use crate::nonconformity::{Origin, Register, Severity};
use crate::repo::Repository;
use serde_json::json;
use std::path::Path;

/// Registra la recepción del acuse que completa la cesión (apartado 41.2,
/// paso 9 de la Tabla 33).
///
/// Un envío no se considera completado hasta que la parte emisora recibe el
/// acuse (apartado 31.2). Hasta ese momento el estado es `en_transito` y
/// ninguna de las dos partes debe modificar el proyecto. Con el acuse en la
/// mano, el estado del cedente pasa a `cedida`.
///
/// Cuando el acuse rechaza la custodia, o resulta en rechazo del envío, la
/// custodia revierte al cedente mediante un asiento `custody_reclaimed`
/// (apartado 41.2, penúltimo párrafo).
pub fn record_receipt(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    receipt: &crate::manifest::receipt::Receipt,
    org: &str,
) -> Result<CustodyState> {
    use crate::manifest::receipt::ReceiptResult;

    let estado = project.custody_state();
    if estado != CustodyState::InTransit {
        return Err(Error::Custody {
            state: estado.as_str().to_string(),
            detail: "No se ha registrado nada. Solo procede registrar el acuse de un proyecto en tránsito.".into(),
        });
    }

    let envio = receipt.shipment_id().map(str::to_string);
    let esperado = project
        .doc()
        .at("custody.transfer.shipment_id")
        .and_then(|n| n.present_str())
        .map(str::to_string);
    if envio.is_some() && esperado.is_some() && envio != esperado {
        return Err(Error::requirement(
            "32.2",
            format!(
                "El acuse corresponde al envío {} y la cesión se emitió con el envío {}. No se ha registrado nada. El identificador de envío es la clave que vincula ambos registros.",
                envio.unwrap_or_default(),
                esperado.unwrap_or_default()
            ),
        ));
    }

    // Un acuse que acepta el material pero no la custodia es una aceptación con
    // reservas, y la custodia revierte al cedente (apartado 41.2).
    let acepta_custodia = receipt.custody_accepted().unwrap_or(false);
    let rechazado = receipt.result() == Some(ReceiptResult::Rejected);
    let cede = acepta_custodia && !rechazado;

    let ahora = clock::now_rfc3339();
    let seq = project.next_custody_seq();
    let accion = if cede {
        CustodyAction::CustodyTransferred
    } else {
        CustodyAction::CustodyReclaimed
    };
    project
        .doc_mut()
        .ensure_map("custody")
        .ensure_seq("history")
        .push(
            ChronologyEntry {
                seq,
                action: accion,
                ts: receipt
                    .doc()
                    .at("receipt.issued")
                    .and_then(|n| n.present_str())
                    .unwrap_or(&ahora)
                    .to_string(),
                actor: actor.to_string(),
                org: org.to_string(),
                shipment_id: envio.clone(),
            }
            .to_node(),
        );

    let resultante = if cede {
        CustodyState::Ceded
    } else {
        CustodyState::Reclaimed
    };
    {
        let cust = project.doc_mut().ensure_map("custody");
        cust.set("state", Node::str(resultante.as_str()));
        cust.set("since", Node::str(&ahora));
        if !cede {
            cust.remove("transfer");
        }
    }
    project.save()?;

    if cede {
        // La copia local sigue bloqueada: la custodia es ahora de la otra parte.
        custody_lock::write_marker(
            project.root(),
            &crate::manifest::custody_lock::CustodyLock {
                state: CustodyState::Ceded,
                holder: format!(
                    "{} / {}",
                    receipt.doc().at("recipient.org").and_then(|n| n.present_str()).unwrap_or("desconocida"),
                    receipt.doc().at("recipient.officer").and_then(|n| n.present_str()).unwrap_or("")
                ),
                ceded_by: format!("{org} / {actor}"),
                shipment_id: envio.clone().unwrap_or_default(),
                ceded_at: ahora.clone(),
                expected_return: receipt
                    .doc()
                    .at("custody.expected_return")
                    .and_then(|n| n.present_str())
                    .unwrap_or_default()
                    .to_string(),
                grace_days: project
                    .doc()
                    .at("custody.transfer.grace_days")
                    .and_then(|n| n.as_int())
                    .unwrap_or(0),
            },
        )?;
    } else {
        // La custodia revierte: se restituye la escritura y se suprime el
        // marcador.
        fsx::readonly::set_tree_readonly(project.root(), false, &[])?;
        custody_lock::remove_marker(project.root())?;
        Register::at(repo.root()).open(
            repo,
            actor,
            Origin::ShipmentReception,
            Severity::Major,
            &[project.uid().unwrap_or_default().to_string()],
            &format!(
                "El acuse del envio {} no acepto la custodia. La custodia revirtio al cedente conforme al apartado 41.2.",
                envio.clone().unwrap_or_default()
            ),
        )?;
    }

    let log = repo.event_log();
    log.append(
        actor,
        event::RECEIPT_RECEIVED,
        project.uid(),
        json!({"shipment_id": envio, "result": receipt.result().map(|r| r.as_str())}),
    )?;
    log.append(
        actor,
        if cede { event::CUSTODY_TRANSFERRED } else { event::CUSTODY_RECLAIMED },
        project.uid(),
        json!({"shipment_id": envio}),
    )?;

    Ok(resultante)
}

/// Reclama por escrito el retorno de una cesión vencida (apartado 41.4).
///
/// La reclamación precede a la recuperación forzosa: vencida la fecha esperada
/// de retorno, el cedente debe reclamarlo y registrar la reclamación.
pub fn claim_return(
    repo: &Repository,
    actor: &str,
    project: &ProjectManifest,
) -> Result<Expiry> {
    let estado = project.custody_state();
    if estado != CustodyState::Ceded {
        return Err(Error::Custody {
            state: estado.as_str().to_string(),
            detail: "No se ha reclamado nada. Solo procede reclamar el retorno de un proyecto cedido.".into(),
        });
    }
    let (situacion, retorno) = expiry_of(project)?;
    if situacion == Expiry::Current {
        return Err(Error::requirement(
            "41.4",
            format!("La fecha esperada de retorno es {retorno} y no ha vencido. No se ha reclamado nada. La reclamación procede una vez vencida esa fecha."),
        ));
    }

    repo.event_log().append(
        actor,
        event::CUSTODY_RETURN_CLAIMED,
        project.uid(),
        json!({
            "expected_return": retorno,
            "shipment_id": project.doc().at("custody.transfer.shipment_id").and_then(|n| n.as_str()),
        }),
    )?;
    Ok(situacion)
}

/// Recupera la custodia por vencimiento (apartado 41.4).
///
/// Solo procede vencido además el plazo de gracia. La recuperación es
/// irreversible en la práctica: todo retorno posterior es una versión divergente
/// que habrá que reconciliar a mano.
pub fn reclaim(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    org: &str,
) -> Result<()> {
    let estado = project.custody_state();
    if estado != CustodyState::Ceded {
        return Err(Error::Custody {
            state: estado.as_str().to_string(),
            detail: "No se ha recuperado nada. Solo procede recuperar la custodia de un proyecto cedido.".into(),
        });
    }
    let (situacion, retorno) = expiry_of(project)?;
    if situacion != Expiry::ReclaimAvailable {
        let gracia = project
            .doc()
            .at("custody.transfer.grace_days")
            .and_then(|n| n.as_int())
            .unwrap_or(0);
        return Err(Error::requirement(
            "41.4",
            format!("La fecha esperada de retorno es {retorno} y el plazo de gracia de {gracia} días no ha vencido. No se ha recuperado nada. La recuperación forzosa procede vencido además el plazo de gracia."),
        ));
    }

    let envio = project
        .doc()
        .at("custody.transfer.shipment_id")
        .and_then(|n| n.present_str())
        .map(str::to_string);
    let ahora = clock::now_rfc3339();

    // Paso 2: registrar el asiento `custody_reclaimed`.
    let seq = project.next_custody_seq();
    project
        .doc_mut()
        .ensure_map("custody")
        .ensure_seq("history")
        .push(
            ChronologyEntry {
                seq,
                action: CustodyAction::CustodyReclaimed,
                ts: ahora.clone(),
                actor: actor.to_string(),
                org: org.to_string(),
                shipment_id: envio.clone(),
            }
            .to_node(),
        );

    // Paso 3: estado `reclamada` y restitución del acceso de escritura.
    let cust = project.doc_mut().ensure_map("custody");
    cust.set("state", Node::str(CustodyState::Reclaimed.as_str()));
    cust.set("since", Node::str(&ahora));
    cust.remove("transfer");
    project.save()?;

    fsx::readonly::set_tree_readonly(project.root(), false, &[])?;
    custody_lock::remove_marker(project.root())?;

    repo.event_log().append(
        actor,
        event::CUSTODY_RECLAIMED,
        project.uid(),
        json!({"shipment_id": envio, "expected_return": retorno}),
    )?;

    // Paso 4: abrir una no conformidad mayor.
    Register::at(repo.root()).open(
        repo,
        actor,
        Origin::OperationalIncident,
        Severity::Major,
        &[project.uid().unwrap_or_default().to_string()],
        &format!(
            "La cesion del envio {} vencio el {retorno} y su plazo de gracia sin que el retorno se produjera. La custodia se recupero conforme al apartado 41.4.",
            envio.as_deref().unwrap_or("desconocido")
        ),
    )?;
    Ok(())
}

/// Resultado de la aceptación de un envío de retorno.
#[derive(Clone, Debug)]
pub struct ReturnOutcome {
    /// El material se incorporó como versión nueva, sin sobrescribir la copia
    /// congelada anterior (apartado 41.3, paso 1).
    pub incorporated_at: std::path::PathBuf,
    /// La custodia volvió a ser propia.
    pub custody_restored: bool,
    /// El retorno llegó después de una recuperación forzosa: es una versión
    /// divergente que debe reconciliarse a mano (apartado 41.4, último párrafo).
    pub divergent: bool,
}

/// Acepta un envío de retorno de custodia (apartado 41.3).
///
/// El acceso de escritura no se restituye antes de que el envío de retorno haya
/// sido aceptado.
pub fn accept_return(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    package_dir: &Path,
    org: &str,
) -> Result<ReturnOutcome> {
    let estado_previo = project.custody_state();
    if !matches!(estado_previo, CustodyState::Ceded | CustodyState::Reclaimed) {
        return Err(Error::Custody {
            state: estado_previo.as_str().to_string(),
            detail: "No se ha aceptado nada. Un retorno solo procede sobre un proyecto cedido o reclamado.".into(),
        });
    }

    let pkg = crate::package::bagit::PackageDir::open(package_dir.to_path_buf());
    let exchange = ExchangeManifest::load(pkg.exchange_manifest())?;
    if !exchange.returns_custody() {
        return Err(Error::requirement(
            "41.3",
            "El manifiesto de intercambio no declara que devuelva la custodia. No se ha aceptado nada. Un envío de retorno debe declararlo de forma expresa.",
        ));
    }

    // Un retorno posterior a una recuperación forzosa no se ingiere de forma
    // automática: es una versión divergente (apartado 41.4, último párrafo).
    let divergent = estado_previo == CustodyState::Reclaimed;

    let ahora = clock::now_rfc3339();
    // Paso 1: incorporar el material como versión nueva, sin sobrescribir la
    // copia congelada anterior.
    let sufijo = if divergent { "divergente" } else { "retorno" };
    let destino = project
        .root()
        .join("09_TRANSFER")
        .join(format!("{}_{sufijo}_{}", clock::today(), exchange.shipment_id().unwrap_or("sn")));
    let contenido = pkg.content();
    fsx::space::ensure_available(&destino, fsx::walk::total_bytes(&fsx::walk::conserved_files(&contenido)))?;
    fsx::copy_tree(&contenido, &destino, &[])?;

    if divergent {
        // No se restituye la custodia ni se sobrescribe nada: la reconciliación
        // es manual y su resultado se registra en la no conformidad abierta.
        repo.event_log().append(
            actor,
            event::CUSTODY_DIVERGENCE_OPENED,
            project.uid(),
            json!({
                "shipment_id": exchange.shipment_id(),
                "incorporated_at": destino.display().to_string(),
                "reason": "retorno recibido despues de una recuperacion forzosa",
            }),
        )?;
        Register::at(repo.root()).open(
            repo,
            actor,
            Origin::ShipmentReception,
            Severity::Major,
            &[project.uid().unwrap_or_default().to_string()],
            &format!(
                "El envio de retorno {} llego despues de la recuperacion forzosa de la custodia. El material se incorporo en {} como version divergente y no se ingirio de forma automatica.",
                exchange.shipment_id().unwrap_or("sn"),
                destino.display()
            ),
        )?;
        return Ok(ReturnOutcome {
            incorporated_at: destino,
            custody_restored: false,
            divergent: true,
        });
    }

    // Paso 2: fusionar los asientos de la cronología (apartado 14.3.4).
    let recibidos = exchange.custody_history();
    let propios = project.custody_history();
    let mut fusion = custody::merge_chronology(&propios, &recibidos);
    let siguiente = fusion.iter().map(|e| e.seq).max().unwrap_or(0) + 1;

    // Paso 3: registrar `custody_returned` y restablecer el estado a propia.
    fusion.push(ChronologyEntry {
        seq: siguiente,
        action: CustodyAction::CustodyReturned,
        ts: ahora.clone(),
        actor: actor.to_string(),
        org: org.to_string(),
        shipment_id: exchange.shipment_id().map(str::to_string),
    });

    let cust = project.doc_mut().ensure_map("custody");
    cust.set("state", Node::str(CustodyState::Own.as_str()));
    cust.set("since", Node::str(&ahora));
    cust.set(
        "history",
        Node::Seq(fusion.iter().map(ChronologyEntry::to_node).collect()),
    );
    cust.remove("transfer");
    project.save()?;

    // Pasos 4: suprimir el marcador y restituir el acceso de escritura. El
    // orden importa: el acceso no se restituye antes de aceptar el retorno.
    fsx::readonly::set_tree_readonly(project.root(), false, &[])?;
    custody_lock::remove_marker(project.root())?;

    repo.event_log().append(
        actor,
        event::CUSTODY_RETURNED,
        project.uid(),
        json!({
            "shipment_id": exchange.shipment_id(),
            "incorporated_at": destino.display().to_string(),
        }),
    )?;

    Ok(ReturnOutcome {
        incorporated_at: destino,
        custody_restored: true,
        divergent: false,
    })
}

/// Abre un proyecto derivado sobre material cedido (apartado 14.3.3, último
/// párrafo).
///
/// Cuando resulte inevitable trabajar sobre un proyecto cedido, el trabajo se
/// realiza sobre un derivado que hace visible la divergencia en lugar de
/// ocultarla.
pub fn open_divergent_project(
    repo: &Repository,
    actor: &str,
    ceded: &ProjectManifest,
) -> Result<crate::project::Derivation> {
    let estado = ceded.custody_state();
    if estado.allows_write() {
        return Err(Error::Custody {
            state: estado.as_str().to_string(),
            detail: "No se ha derivado nada. El proyecto derivado del apartado 14.3.3 procede cuando la custodia no es propia; con custodia propia, la derivación corriente del apartado 14.4 es la que corresponde.".into(),
        });
    }

    // La derivación corriente exige custodia propia. Aquí se construye a mano,
    // sin tocar el proyecto cedido más allá de lo que el apartado 14.3.3
    // permite.
    let source_uid = ceded
        .uid()
        .ok_or_else(|| Error::input("El proyecto no declara identificador interno.".to_string()))?
        .to_string();
    let source_id = ceded.id().unwrap_or_default().to_string();
    let envio = ceded
        .doc()
        .at("custody.transfer.shipment_id")
        .and_then(|n| n.present_str())
        .map(str::to_string);

    let base = source_id
        .rsplit_once("_v")
        .map(|(b, _)| b.to_string())
        .unwrap_or_else(|| source_id.clone());
    let derived_id = format!("{base}_divergente");
    crate::naming::check_name(&derived_id)?;
    let derived_root = ceded
        .root()
        .parent()
        .ok_or_else(|| Error::input("El proyecto no tiene carpeta contenedora.".to_string()))?
        .join(&derived_id);
    if derived_root.exists() {
        return Err(Error::input(format!(
            "Ya existe una carpeta {derived_id}. No se ha derivado nada."
        )));
    }

    let files_copied = fsx::copy_tree(
        ceded.root(),
        &derived_root,
        &["08_DELIVERY", "09_TRANSFER"],
    )?;

    let derived_uid = crate::ids::new_uid();
    let ahora = clock::now_rfc3339();
    let mut derivado = ProjectManifest::new(derived_root.join(crate::manifest::PROJECT_FILE));
    derivado.doc_mut().clone_from(ceded.doc());
    derivado.doc_mut().set("uid", Node::str(&derived_uid));
    derivado.doc_mut().set("id", Node::str(&derived_id));
    derivado.doc_mut().set("id_history", Node::Seq(Vec::new()));
    derivado.doc_mut().set("deliveries", Node::Seq(Vec::new()));
    derivado.doc_mut().remove("derived");
    // El manifiesto declara el proyecto y el envío de los que deriva
    // (apartado 14.3.3).
    derivado.doc_mut().set(
        "lineage",
        Node::map(vec![
            ("parent_uid", Node::str(&source_uid)),
            ("parent_id", Node::str(&source_id)),
            ("derived_at", Node::str(&ahora)),
            (
                "reason",
                Node::str("Trabajo sobre material cedido, conforme al apartado 14.3.3"),
            ),
            ("parallel", Node::Bool(true)),
            ("shipment_id", Node::opt_str(envio.as_deref())),
        ]),
    );
    derivado.set_status(crate::manifest::project::Status::Active);
    let cust = derivado.doc_mut().ensure_map("custody");
    cust.set("state", Node::str(CustodyState::Own.as_str()));
    cust.set("since", Node::str(&ahora));
    cust.set("history", Node::Seq(Vec::new()));
    cust.remove("transfer");
    derivado.save()?;
    custody_lock::remove_marker(&derived_root)?;
    fsx::readonly::set_tree_readonly(&derived_root, false, &[])?;

    repo.event_log().append(
        actor,
        event::CUSTODY_DIVERGENCE_OPENED,
        Some(&source_uid),
        json!({
            "derived_uid": derived_uid,
            "derived_id": derived_id,
            "shipment_id": envio,
        }),
    )?;

    // La reconciliación posterior es manual y se registra como no conformidad
    // (apartado 14.3.3, último párrafo).
    Register::at(repo.root()).open(
        repo,
        actor,
        Origin::OperationalIncident,
        Severity::Major,
        &[source_uid.clone(), derived_uid.clone()],
        &format!(
            "Se abrio el proyecto derivado {derived_id} sobre el proyecto cedido {source_id}. La reconciliacion de ambos al retorno de la custodia es manual."
        ),
    )?;

    Ok(crate::project::Derivation {
        source_uid,
        derived_uid,
        derived_id,
        derived_root,
        files_copied,
        source_sealed: false,
    })
}

fn expiry_of(project: &ProjectManifest) -> Result<(Expiry, String)> {
    let retorno = project
        .doc()
        .at("custody.transfer.expected_return")
        .and_then(|n| n.present_str())
        .ok_or_else(|| {
            Error::requirement(
                "41.4",
                "El proyecto no declara fecha esperada de retorno. No se ha podido determinar el vencimiento.",
            )
        })?
        .to_string();
    let gracia = project
        .doc()
        .at("custody.transfer.grace_days")
        .and_then(|n| n.as_int())
        .unwrap_or(0);
    Ok((custody::expiry_state(&retorno, gracia)?, retorno))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::project::Level;
    use crate::project::{self, NewProject};

    fn entorno() -> (tempfile::TempDir, Repository, ProjectManifest) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        let p = project::create(
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
                active_volume: None,
            },
        )
        .unwrap();
        (dir, repo, p)
    }

    /// Sitúa el proyecto en estado cedido con los plazos indicados.
    fn ceder(p: &mut ProjectManifest, dias_desde_retorno: i64, gracia: i64) {
        let retorno = clock::add_days(&clock::today(), -dias_desde_retorno).unwrap();
        let seq = p.next_custody_seq();
        p.doc_mut()
            .ensure_map("custody")
            .ensure_seq("history")
            .push(
                ChronologyEntry {
                    seq,
                    action: CustodyAction::CustodyTransferred,
                    ts: clock::now_rfc3339(),
                    actor: "J. Duarte".into(),
                    org: "Estudio A".into(),
                    shipment_id: Some("20260806-AAAA".into()),
                }
                .to_node(),
            );
        let cust = p.doc_mut().ensure_map("custody");
        cust.set("state", Node::str("cedida"));
        cust.set(
            "transfer",
            Node::map(vec![
                ("shipment_id", Node::str("20260806-AAAA")),
                ("expected_return", Node::str(&retorno)),
                ("grace_days", Node::Int(gracia)),
            ]),
        );
        p.save().unwrap();
        fsx::readonly::set_tree_readonly(
            p.root(),
            true,
            &[crate::manifest::PROJECT_FILE, crate::manifest::CUSTODY_LOCK_FILE],
        )
        .unwrap();
    }

    #[test]
    fn la_reclamacion_procede_vencida_la_fecha_de_retorno() {
        let (_d, repo, mut p) = entorno();
        ceder(&mut p, -10, 15); // fecha de retorno futura
        assert!(claim_return(&repo, "a", &p).is_err(), "aún no ha vencido");

        ceder(&mut p, 5, 15); // vencida, dentro del plazo de gracia
        let situacion = claim_return(&repo, "a", &p).unwrap();
        assert_eq!(situacion, Expiry::ClaimDue);
        let eventos = repo.event_log().entries_for(p.uid().unwrap()).unwrap();
        assert!(eventos.iter().any(|e| e.event == event::CUSTODY_RETURN_CLAIMED));
    }

    #[test]
    fn la_recuperacion_forzosa_exige_que_venza_tambien_el_plazo_de_gracia() {
        let (_d, repo, mut p) = entorno();
        ceder(&mut p, 5, 15); // vencida la fecha, no el plazo de gracia
        let e = reclaim(&repo, "a", &mut p, "Estudio A").unwrap_err();
        assert_eq!(e.clause(), Some("41.4"));
        assert_eq!(p.custody_state(), CustodyState::Ceded);

        ceder(&mut p, 40, 15); // vencido además el plazo de gracia
        reclaim(&repo, "a", &mut p, "Estudio A").unwrap();
        assert_eq!(p.custody_state(), CustodyState::Reclaimed);
    }

    #[test]
    fn la_recuperacion_restituye_la_escritura_y_abre_una_no_conformidad_mayor() {
        let (_d, repo, mut p) = entorno();
        std::fs::write(p.root().join("00_ADMIN/notas.txt"), b"notas").unwrap();
        ceder(&mut p, 40, 15);
        crate::manifest::custody_lock::write_marker(
            p.root(),
            &crate::manifest::custody_lock::CustodyLock {
                state: CustodyState::Ceded,
                holder: "Estudio B".into(),
                ceded_by: "Estudio A".into(),
                shipment_id: "20260806-AAAA".into(),
                ceded_at: clock::now_rfc3339(),
                expected_return: clock::add_days(&clock::today(), -40).unwrap(),
                grace_days: 15,
            },
        )
        .unwrap();

        reclaim(&repo, "a", &mut p, "Estudio A").unwrap();

        // Estado reclamada, escritura restituida, marcador suprimido.
        assert_eq!(p.custody_state(), CustodyState::Reclaimed);
        assert!(p.custody_state().allows_write());
        assert!(!p.root().join(crate::manifest::CUSTODY_LOCK_FILE).exists());
        let notas = p.root().join("00_ADMIN/notas.txt");
        assert!(!std::fs::metadata(&notas).unwrap().permissions().readonly());
        // El bloque de cesión ya no consta.
        assert!(p.doc().at("custody.transfer").is_none());

        // Asiento en la cronología y no conformidad mayor abierta.
        let hist = p.custody_history();
        assert!(hist.iter().any(|e| e.action == CustodyAction::CustodyReclaimed));
        let ncs = Register::at(repo.root()).open_entries().unwrap();
        assert_eq!(ncs.len(), 1);
        assert_eq!(ncs[0].severity, Severity::Major);
    }

    #[test]
    fn un_retorno_posterior_a_la_recuperacion_es_una_version_divergente() {
        let (dir, repo, mut p) = entorno();
        ceder(&mut p, 40, 15);
        reclaim(&repo, "a", &mut p, "Estudio A").unwrap();

        // Llega el paquete de retorno, tarde.
        let paquete = dir.path().join("STAVE-XCHG_retorno");
        let pkg = crate::package::bagit::PackageDir::create(dir.path(), "STAVE-XCHG_retorno").unwrap();
        std::fs::write(pkg.content().join("mezcla.wav"), b"trabajo del cesionario").unwrap();
        let mut ex = ExchangeManifest::new(pkg.exchange_manifest());
        ex.doc_mut().set(
            "custody",
            Node::map(vec![
                ("transfers", Node::Bool(false)),
                ("returns", Node::Bool(true)),
            ]),
        );
        ex.doc_mut().set(
            "shipment",
            Node::map(vec![("id", Node::str("20260901-BBBB"))]),
        );
        ex.save().unwrap();

        let r = accept_return(&repo, "a", &mut p, &paquete, "Estudio A").unwrap();

        assert!(r.divergent);
        assert!(!r.custody_restored);
        // La custodia sigue reclamada: no se ingiere de forma automática.
        assert_eq!(p.custody_state(), CustodyState::Reclaimed);
        assert!(r.incorporated_at.join("mezcla.wav").is_file());
        // Y se abre una no conformidad para la reconciliación manual.
        let ncs = Register::at(repo.root()).open_entries().unwrap();
        assert_eq!(ncs.len(), 2, "la de la recuperación y la del retorno tardío");
    }

    #[test]
    fn un_retorno_en_plazo_restituye_la_custodia_y_fusiona_la_cronologia() {
        let (dir, repo, mut p) = entorno();
        ceder(&mut p, -10, 15);
        let asientos_previos = p.custody_history().len();

        let paquete = dir.path().join("STAVE-XCHG_retorno");
        let pkg = crate::package::bagit::PackageDir::create(dir.path(), "STAVE-XCHG_retorno").unwrap();
        std::fs::write(pkg.content().join("mezcla.wav"), b"trabajo del cesionario").unwrap();
        let mut ex = ExchangeManifest::new(pkg.exchange_manifest());
        ex.doc_mut().set("shipment", Node::map(vec![("id", Node::str("20260901-BBBB"))]));
        ex.doc_mut().set(
            "custody",
            Node::map(vec![
                ("transfers", Node::Bool(false)),
                ("returns", Node::Bool(true)),
                (
                    "history",
                    Node::Seq(vec![ChronologyEntry {
                        seq: (asientos_previos + 1) as i64,
                        action: CustodyAction::CustodyAssumed,
                        ts: clock::now_rfc3339(),
                        actor: "M. Rivas".into(),
                        org: "Estudio B".into(),
                        shipment_id: Some("20260806-AAAA".into()),
                    }
                    .to_node()]),
                ),
            ]),
        );
        ex.save().unwrap();

        let r = accept_return(&repo, "a", &mut p, &paquete, "Estudio A").unwrap();

        assert!(r.custody_restored);
        assert!(!r.divergent);
        assert_eq!(p.custody_state(), CustodyState::Own);
        assert!(!p.root().join(crate::manifest::CUSTODY_LOCK_FILE).exists());
        // La copia congelada anterior no se sobrescribió: el retorno entró como
        // versión nueva.
        assert!(r.incorporated_at.join("mezcla.wav").is_file());
        // La cronología incorpora los asientos de la otra parte y el retorno.
        let hist = p.custody_history();
        assert!(hist.iter().any(|e| e.action == CustodyAction::CustodyAssumed));
        assert!(hist.iter().any(|e| e.action == CustodyAction::CustodyReturned));
        assert!(custody::check_chronology(&hist).is_clean(), "{hist:?}");
    }

    #[test]
    fn un_envio_que_no_declara_el_retorno_no_se_acepta_como_tal() {
        let (dir, repo, mut p) = entorno();
        ceder(&mut p, -10, 15);
        let paquete = dir.path().join("STAVE-XCHG_otro");
        let pkg = crate::package::bagit::PackageDir::create(dir.path(), "STAVE-XCHG_otro").unwrap();
        let mut ex = ExchangeManifest::new(pkg.exchange_manifest());
        ex.doc_mut().set("shipment", Node::map(vec![("id", Node::str("X"))]));
        ex.save().unwrap();
        let e = accept_return(&repo, "a", &mut p, &paquete, "Estudio A").unwrap_err();
        assert_eq!(e.clause(), Some("41.3"));
    }

    #[test]
    fn el_proyecto_derivado_hace_visible_la_divergencia_sobre_material_cedido() {
        let (_d, repo, mut p) = entorno();
        std::fs::write(p.root().join("00_ADMIN/notas.txt"), b"notas").unwrap();
        ceder(&mut p, -10, 15);

        let d = open_divergent_project(&repo, "a", &p).unwrap();

        assert!(d.derived_id.ends_with("_divergente"));
        assert_ne!(d.derived_uid, d.source_uid);
        let derivado = ProjectManifest::load(d.derived_root.join(crate::manifest::PROJECT_FILE)).unwrap();
        // El derivado declara el proyecto y el envío de los que deriva.
        assert_eq!(derivado.doc().at("lineage.parent_uid").unwrap().as_str(), Some(d.source_uid.as_str()));
        assert_eq!(derivado.doc().at("lineage.shipment_id").unwrap().as_str(), Some("20260806-AAAA"));
        // El derivado es modificable; el cedido sigue bloqueado.
        assert_eq!(derivado.custody_state(), CustodyState::Own);
        assert!(!std::fs::metadata(d.derived_root.join("00_ADMIN/notas.txt")).unwrap().permissions().readonly());
        assert!(std::fs::metadata(p.root().join("00_ADMIN/notas.txt")).unwrap().permissions().readonly());
        assert_eq!(p.custody_state(), CustodyState::Ceded);

        // Y la divergencia queda registrada como no conformidad.
        assert_eq!(Register::at(repo.root()).open_entries().unwrap().len(), 1);
    }

    #[test]
    fn el_derivado_por_divergencia_no_procede_con_custodia_propia() {
        let (_d, repo, p) = entorno();
        let e = open_divergent_project(&repo, "a", &p).unwrap_err();
        assert_eq!(e.clause(), Some("14.3"));
    }
}
