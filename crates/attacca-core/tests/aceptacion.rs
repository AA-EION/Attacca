//! Suite de aceptación.
//!
//! Cubre, una por una, las pruebas que el apartado 10.3 del encargo exige, y las
//! del apartado 45 de la norma para la clase `M`. Cada prueba lleva el nombre
//! del caso que acredita.

// El andamiaje es el mismo que el de las pruebas unitarias; se incluye en
// lugar de duplicarlo.
#[path = "../src/pruebas.rs"]
mod pruebas;

use attacca_core::clock::SyncState;
use attacca_core::custody::{CustodyAction, CustodyState};
use attacca_core::doc::Node;
use attacca_core::index::Index;
use attacca_core::manifest::exchange::{Classification, ExchangeManifest, Profile};
use attacca_core::manifest::project::{Level, ProjectManifest, Status};
use attacca_core::manifest::receipt::{Outcome, ReceiptResult};
use attacca_core::package::container::{self, Progress};
use attacca_core::package::{bagit, custody_ops, emit, ingest};
use attacca_core::project::{self, NewProject};
use attacca_core::repo::Repository;
use attacca_core::{clock, custody, integrity, manifest, naming, validate};
use std::path::{Path, PathBuf};

// --- Andamiaje común ---

struct Estudio {
    _dir: tempfile::TempDir,
    repo: Repository,
    org: String,
    actor: String,
}

fn estudio(org: &str, actor: &str) -> Estudio {
    let dir = pruebas::raiz_temporal().unwrap();
    let repo = Repository::create(dir.path().join(".stave")).unwrap();
    Estudio {
        _dir: dir,
        repo,
        org: org.into(),
        actor: actor.into(),
    }
}

fn crear(e: &Estudio, titulo: &str) -> ProjectManifest {
    project::create(
        &e.repo,
        &e.actor,
        &NewProject {
            title: titulo.into(),
            artist: "Artista".into(),
            kind: "ORIG".into(),
            level: Level::B,
            release_uid: None,
            release_dir: None,
            sample_rate: 48000,
            bit_depth: 24,
            holder_org: e.org.clone(),
            holder_person: e.actor.clone(),
            active_volume: None,
        },
    )
    .unwrap()
}

/// Sitúa el proyecto en un estado apto para la emisión: material, parámetros de
/// audio cerrados e informe de control de calidad.
fn preparar_para_entrega(p: &mut ProjectManifest) {
    let raiz = p.root().to_path_buf();
    for sub in [
        "07_MASTER",
        "05_STEMS",
        "00_ADMIN/Credits",
        "00_ADMIN/Notes",
    ] {
        std::fs::create_dir_all(raiz.join(sub)).unwrap();
    }
    std::fs::write(raiz.join("07_MASTER/master.wav"), b"audio del master").unwrap();
    std::fs::write(raiz.join("05_STEMS/01_KICK.wav"), b"stem de bombo").unwrap();
    std::fs::write(raiz.join("00_ADMIN/Credits/creditos.csv"), b"rol,nombre\n").unwrap();
    std::fs::write(
        raiz.join("00_ADMIN/Notes/QC_informe.txt"),
        b"control aprobado\n",
    )
    .unwrap();

    let audio = p.doc_mut().ensure_map("audio");
    audio.set("tempo", Node::Int(96));
    audio.set("key", Node::str("Db major"));
    audio.set("origin", Node::str("00:00:00:00"));
    p.doc_mut()
        .set("vocabulary", Node::map(vec![("frozen", Node::Bool(true))]));
    let master = p.doc_mut().ensure_map("master");
    master.set("lufs_i", Node::Float(-9.8));
    master.set("lra", Node::Float(5.2));
    master.set("true_peak_db", Node::Float(-1.0));
    p.doc_mut()
        .ensure_map("tools")
        .set("primary", Node::str("Reaper 7.2"));
    p.doc_mut().ensure_seq("people").push(Node::map(vec![
        ("name", Node::str("A. Ruiz")),
        ("role", Node::str("mezcla")),
    ]));
    p.save().unwrap();
}

fn envio(perfil: Profile, cede: bool, org: &str, destinatario: &str) -> emit::Shipment {
    emit::Shipment {
        profile: perfil,
        classification: Classification::Internal,
        purpose: "Finalidad declarada del envio".into(),
        issuer_org: org.into(),
        issuer_contact: "intercambio@emisor.example".into(),
        issuer_key_id: None,
        recipient_org: destinatario.into(),
        recipient_contact: "recepcion@destino.example".into(),
        usage_permitted: vec!["uso declarado".into()],
        usage_territory: "mundial".into(),
        usage_term: "indefinido".into(),
        sublicensing: false,
        forwarding: false,
        retention_until: "2031-12-31".into(),
        destroy_on_expiry: true,
        personal_data: false,
        personal_data_categories: vec![],
        ack_deadline_hours: 72,
        ack_address: "intercambio@emisor.example".into(),
        transfers_custody: cede,
        returns_custody: false,
        supersedes_transfer: None,
        expected_return: cede.then(|| clock::add_days(&clock::today(), 60).unwrap()),
        grace_days: if cede { 15 } else { 0 },
        onward_allowed: false,
        supersedes: None,
        revision: "r0".into(),
        serialize: true,
        qc_approved: true,
    }
}

fn sincronizado() -> SyncState {
    SyncState::Synced { drift_ms: 30 }
}

fn emitir(e: &Estudio, p: &mut ProjectManifest, s: &emit::Shipment) -> emit::Emission {
    let (mut prog, canc) = container::silent_progress();
    let mut pr = Progress {
        on_progress: &mut prog,
        cancelled: &canc,
    };
    emit::emit(
        &e.repo,
        &e.actor,
        p,
        s,
        &emit::Payload::for_profile(s.profile),
        sincronizado(),
        &mut pr,
    )
    .unwrap()
}

fn contexto(e: &Estudio, emisor: &str) -> ingest::ReceptionContext {
    ingest::ReceptionContext {
        recipient_org: e.org.clone(),
        officer: e.actor.clone(),
        key_id: None,
        agreed_parties: vec![emisor.to_string()],
        declared_identities: vec![],
        revoked_identities: vec![],
        supported_versions: vec![attacca_core::STAVE_VERSION.to_string()],
        supported_profiles: vec![Profile::Delivery, Profile::Production, Profile::Archive],
        usage_acceptable: true,
        known_shipments: vec![],
        known_chronology: vec![],
    }
}

fn verificar(e: &Estudio, paquete: &Path, ctx: &ingest::ReceptionContext) -> ingest::Verification {
    let (mut prog, canc) = container::silent_progress();
    let mut pr = Progress {
        on_progress: &mut prog,
        cancelled: &canc,
    };
    ingest::verify(&e.repo, &e.actor, paquete, ctx, &mut pr).unwrap()
}

/// Deposita un contenedor en la cuarentena del receptor, como haría el
/// transporte real.
fn depositar(receptor: &Estudio, contenedor: &Path) -> PathBuf {
    let destino = receptor.repo.inbox().join(contenedor.file_name().unwrap());
    std::fs::copy(contenedor, &destino).unwrap();
    let _ = attacca_core::fsx::readonly::set_file_readonly(&destino, false);
    destino
}

// =========================================================================
// 1. Creación de proyecto y validación del manifiesto contra el esquema
// =========================================================================

#[test]
fn creacion_de_proyecto_y_validacion_del_manifiesto() {
    let e = estudio("Estudio A", "J. Duarte");
    let p = crear(&e, "Canción de Ejemplo");

    // Estructura del apartado 7: solo las carpetas obligatorias.
    assert!(p.root().join("00_ADMIN").is_dir());
    assert!(p.root().join("02_SESSIONS").is_dir());
    for opcional in attacca_core::repo::OPTIONAL_PROJECT_DIRS {
        assert!(
            !p.root().join(opcional).exists(),
            "{opcional} no debía crearse"
        );
    }

    // Identidad conforme al apartado 9.2.
    assert!(attacca_core::ids::is_uid(p.uid().unwrap()));
    assert_eq!(
        p.id().unwrap(),
        format!("{}_Cancion-de-Ejemplo_ORIG", clock::today())
    );
    assert!(naming::is_valid_name(p.id().unwrap()));

    // Todos los campos del Anexo B.1 están presentes; los pendientes, en nulo
    // explícito (apartado 13.2).
    let doc = p.doc();
    for campo in [
        "stave",
        "uid",
        "id",
        "id_history",
        "title",
        "artist",
        "type",
        "status",
        "release",
        "replication",
        "audio",
        "tools",
        "people",
        "sources",
        "deliveries",
        "master",
        "rights",
        "preservation",
        "custody",
        "exceptions",
    ] {
        assert!(doc.get(campo).is_some(), "el manifiesto omite {campo}");
    }
    for pendiente in [
        "audio.tempo",
        "audio.key",
        "audio.origin",
        "master.lufs_i",
        "rights.isrc",
    ] {
        assert!(
            doc.at(pendiente).unwrap().is_null(),
            "{pendiente} debe ser nulo explícito"
        );
    }

    // La validación no trata como no conformidad un campo cuya etapa no ha
    // concluido.
    let informe = validate::project(&p);
    assert!(informe.is_conformant(), "{:?}", informe.breaches());
    assert!(!informe.pending().is_empty());

    // El manifiesto es legible con un editor de texto y su reescritura es
    // idempotente.
    let texto = std::fs::read_to_string(p.path()).unwrap();
    assert!(texto.contains("uid: "));
    let mut releido = ProjectManifest::load(p.path()).unwrap();
    releido.save().unwrap();
    assert_eq!(std::fs::read_to_string(p.path()).unwrap(), texto);
}

// =========================================================================
// 2. Cambio de título con identificador interno estable y referencias intactas
// =========================================================================

#[test]
fn cambio_de_titulo_con_identificador_interno_estable() {
    let e = estudio("Estudio A", "J. Duarte");
    let mut r = attacca_core::release::create(
        &e.repo,
        &e.actor,
        &attacca_core::release::NewRelease {
            title: "Disco".into(),
            artist: "Artista".into(),
            class: attacca_core::manifest::release::ReleaseClass::Ep,
            level: Level::B,
            holder_org: e.org.clone(),
            holder_person: e.actor.clone(),
        },
    )
    .unwrap();
    let mut p = crear(&e, "Titulo Original");
    attacca_core::release::link_project(&e.repo, &e.actor, &mut r, &mut p, None).unwrap();

    let uid = p.uid().unwrap().to_string();
    let id_anterior = p.id().unwrap().to_string();
    let raiz_anterior = p.root().to_path_buf();

    project::rename(&e.repo, &e.actor, &mut p, "Titulo Nuevo").unwrap();

    // El identificador interno no cambia.
    assert_eq!(p.uid().unwrap(), uid);
    // La carpeta se renombró.
    assert!(!raiz_anterior.exists());
    assert!(p.root().join(manifest::PROJECT_FILE).is_file());
    assert!(p.id().unwrap().ends_with("_Titulo-Nuevo_ORIG"));
    // El valor anterior queda registrado con su marca temporal.
    let hist = p.doc().get("id_history").unwrap().as_seq().unwrap();
    assert_eq!(hist.len(), 1);
    let entrada = hist[0].as_map().unwrap();
    assert_eq!(
        entrada.get("previous").unwrap().as_str(),
        Some(id_anterior.as_str())
    );
    assert!(clock::parse_rfc3339(entrada.get("changed").unwrap().as_str().unwrap()).is_ok());

    // Ninguna referencia se rompe: el tracklist sigue resolviendo.
    let release = attacca_core::manifest::release::ReleaseManifest::load(r.path()).unwrap();
    assert_eq!(release.tracklist()[0].project_uid, uid);
    let hallado = project::find_by_uid(&e.repo, &uid).unwrap();
    assert_eq!(hallado, p.path());

    // Emitido un envío, el identificador legible queda fijado (apartado 9.2).
    preparar_para_entrega(&mut p);
    emitir(
        &e,
        &mut p,
        &envio(Profile::Delivery, false, &e.org, "Sello B"),
    );
    assert!(project::rename(&e.repo, &e.actor, &mut p, "Otro Mas").is_err());
}

// =========================================================================
// 3. Derivación de proyecto y sellado del origen
// =========================================================================

#[test]
fn derivacion_de_proyecto_y_sellado_del_origen() {
    let e = estudio("Estudio A", "J. Duarte");
    let mut origen = crear(&e, "Tema");
    preparar_para_entrega(&mut origen);
    std::fs::write(origen.root().join("00_ADMIN/notas.txt"), b"notas de sesion").unwrap();
    emitir(
        &e,
        &mut origen,
        &envio(Profile::Delivery, false, &e.org, "Sello B"),
    );

    let uid_origen = origen.uid().unwrap().to_string();
    let d = project::derive(&e.repo, &e.actor, &mut origen, "Cambio de tonalidad", false).unwrap();

    // Identificador interno propio y sufijo _vNN de dos dígitos.
    assert_ne!(d.derived_uid, uid_origen);
    assert!(attacca_core::ids::is_uid(&d.derived_uid));
    assert_eq!(naming::version_suffix_of(&d.derived_id), Some(2));

    let derivado = ProjectManifest::load(d.derived_root.join(manifest::PROJECT_FILE)).unwrap();
    // Ascendencia declarada.
    assert_eq!(
        derivado.doc().at("lineage.parent_uid").unwrap().as_str(),
        Some(uid_origen.as_str())
    );
    assert_eq!(
        derivado.doc().at("lineage.reason").unwrap().as_str(),
        Some("Cambio de tonalidad")
    );
    assert!(clock::parse_rfc3339(
        derivado
            .doc()
            .at("lineage.derived_at")
            .unwrap()
            .as_str()
            .unwrap()
    )
    .is_ok());
    // Copia íntegra: notas, referencias, sesiones y parámetros.
    assert!(d.derived_root.join("00_ADMIN/notas.txt").is_file());
    assert!(d.derived_root.join("05_STEMS/01_KICK.wav").is_file());
    assert_eq!(derivado.sample_rate(), Some(48000));
    // Salvo los paquetes ya emitidos (apartado 14.4.2).
    assert!(!d.derived_root.join("08_DELIVERY").exists());
    // No hereda los identificadores de grabación (apartado 14.4.4).
    assert!(derivado.doc().at("rights.isrc").unwrap().is_null());
    // Custodia y ciclo de vida independientes.
    assert_eq!(derivado.custody_state(), CustodyState::Own);
    assert!(derivado.custody_history().is_empty());

    // El origen queda sellado y en solo lectura por medios técnicos.
    assert!(d.source_sealed);
    assert_eq!(origen.status(), Status::Sealed);
    let notas = origen.root().join("00_ADMIN/notas.txt");
    assert!(std::fs::metadata(&notas).unwrap().permissions().readonly());
    // La escritura efectiva se comprueba solo cuando el proceso no es
    // administrador: en Unix, root ignora los bits de permiso.
    if !es_administrador() {
        assert!(std::fs::write(&notas, b"intento").is_err());
    }
    // Y declara los proyectos derivados de él.
    assert_eq!(
        origen.doc().get("derived").unwrap().as_seq().unwrap().len(),
        1
    );
}

// =========================================================================
// 4. Ciclo completo de intercambio: emisión, recepción, acuse, ingesta
// =========================================================================

#[test]
fn ciclo_completo_de_intercambio() {
    let emisor = estudio("Estudio A", "J. Duarte");
    let receptor = estudio("Sello B", "M. Rivas");
    let mut p = crear(&emisor, "Tema");
    preparar_para_entrega(&mut p);

    // Emisión.
    let em = emitir(
        &emisor,
        &mut p,
        &envio(Profile::Delivery, false, &emisor.org, &receptor.org),
    );
    assert!(em.artifact.is_file());
    // La copia congelada queda en solo lectura (apartado 32.3).
    assert!(std::fs::metadata(em.frozen_copy.join("EXCHANGE.yaml"))
        .unwrap()
        .permissions()
        .readonly());
    // El manifiesto del proyecto registra la entrega.
    let entregas = p.doc().get("deliveries").unwrap().as_seq().unwrap();
    assert_eq!(
        entregas[0]
            .as_map()
            .unwrap()
            .get("shipment_id")
            .unwrap()
            .as_str(),
        Some(em.shipment_id.as_str())
    );

    // Recepción y verificación.
    let paquete = depositar(&receptor, &em.artifact);
    let ctx = contexto(&receptor, &emisor.org);
    let v = verificar(&receptor, &paquete, &ctx);
    assert_eq!(v.result, ReceiptResult::Accepted, "{:?}", v.discrepancies);
    // Las catorce verificaciones del apartado 37.2 constan.
    assert_eq!(v.checks_as_pairs().len(), 14);

    // Acuse de recibo.
    let ruta_acuse = receptor.repo.root().join("acuse.yaml");
    let acuse = ingest::issue_receipt(
        &receptor.repo,
        &receptor.actor,
        &v,
        &ctx,
        "01_REF",
        Some("2031-12-31"),
        false,
        None,
        sincronizado(),
        &ruta_acuse,
    )
    .unwrap();
    assert_eq!(acuse.result(), Some(ReceiptResult::Accepted));
    assert_eq!(acuse.shipment_id(), Some(em.shipment_id.as_str()));
    // El resumen del manifiesto de integridad acredita qué se recibió.
    assert_eq!(acuse.manifest_digest(), Some(v.manifest_digest.as_str()));

    // Ingesta.
    let ing = ingest::ingest(&receptor.repo, &receptor.actor, &v, &ctx, None, &paquete).unwrap();
    assert!(ing.files > 0);
    assert!(ing.destination.join("07_MASTER/master.wav").is_file());
    // La copia de la cuarentena se elimina (apartado 37.3).
    assert!(!paquete.exists());

    // Ambos registros llevan el identificador de envío como clave común.
    for (est, ev) in [
        (&emisor, "exchange.package.emitted"),
        (&receptor, "exchange.package.ingested"),
    ] {
        let entradas = est.repo.event_log().entries().unwrap();
        assert!(
            entradas.iter().any(|x| x.event == ev
                && x.detail.get("shipment_id").and_then(|s| s.as_str())
                    == Some(em.shipment_id.as_str())),
            "falta {ev}"
        );
        assert!(est.repo.event_log().verify_chain().unwrap().is_intact());
    }
}

// =========================================================================
// 5a. Rechazo por integridad
// =========================================================================

#[test]
fn rechazo_por_integridad() {
    let emisor = estudio("Estudio A", "J. Duarte");
    let receptor = estudio("Sello B", "M. Rivas");
    let mut p = crear(&emisor, "Tema");
    preparar_para_entrega(&mut p);
    let em = emitir(
        &emisor,
        &mut p,
        &envio(Profile::Delivery, false, &emisor.org, &receptor.org),
    );

    let paquete = depositar(&receptor, &em.artifact);
    alterar_entrada(
        &paquete,
        "07_MASTER/master.wav",
        b"audio manipulado en transito",
    );

    let ctx = contexto(&receptor, &emisor.org);
    let v = verificar(&receptor, &paquete, &ctx);

    assert_eq!(v.result, ReceiptResult::Rejected);
    assert_eq!(v.checks.content_integrity, Outcome::Fail);
    // No se extrae material utilizable de un paquete rechazado.
    assert!(v.package_dir.is_none());
    // Y no se ingiere ni siquiera de forma parcial.
    let err =
        ingest::ingest(&receptor.repo, &receptor.actor, &v, &ctx, None, &paquete).unwrap_err();
    assert_eq!(err.clause(), Some("39.1"));
}

// =========================================================================
// 5b. Rechazo por firma
// =========================================================================

#[test]
fn rechazo_por_firma() {
    let emisor = estudio("Estudio A", "J. Duarte");
    let receptor = estudio("Sello B", "M. Rivas");
    let mut p = crear(&emisor, "Tema");
    preparar_para_entrega(&mut p);

    let mut s = envio(Profile::Delivery, false, &emisor.org, &receptor.org);
    // Una clasificación confidencial exige firma (apartado 35.1).
    s.classification = Classification::Confidential;
    s.issuer_key_id = Some("CLAVE-EMISOR".into());
    let em = emitir(&emisor, &mut p, &s);
    let paquete = depositar(&receptor, &em.artifact);

    // Identidad no declarada en el acuerdo de intercambio.
    let v = verificar(&receptor, &paquete, &contexto(&receptor, &emisor.org));
    assert_eq!(v.checks.authenticity, Outcome::Fail);
    assert_eq!(v.result, ReceiptResult::Rejected);

    // Identidad declarada pero revocada (apartado 35.3, tercer guion).
    let mut ctx = contexto(&receptor, &emisor.org);
    ctx.declared_identities = vec!["CLAVE-EMISOR".into()];
    ctx.revoked_identities = vec!["CLAVE-EMISOR".into()];
    assert_eq!(
        verificar(&receptor, &paquete, &ctx).checks.authenticity,
        Outcome::Fail
    );

    // Identidad declarada y vigente.
    let mut ctx = contexto(&receptor, &emisor.org);
    ctx.declared_identities = vec!["CLAVE-EMISOR".into()];
    let v = verificar(&receptor, &paquete, &ctx);
    assert_eq!(v.checks.authenticity, Outcome::Pass);
    assert_eq!(
        v.result,
        ReceiptResult::Accepted,
        "{:?}",
        v.discrepancias_debug()
    );
}

// =========================================================================
// 5c. Rechazo por ruta no admitida en el contenedor
// =========================================================================

#[test]
fn rechazo_por_ruta_no_admitida_en_el_contenedor() {
    let receptor = estudio("Sello B", "M. Rivas");
    let ctx = contexto(&receptor, "Estudio A");

    for (nombre, entrada) in [
        ("ascenso.stave", "paquete/../../escapado.txt"),
        ("absoluta.stave", "/etc/passwd"),
        ("unidad.stave", "C:/Windows/system.ini"),
    ] {
        let contenedor = receptor.repo.inbox().join(nombre);
        contenedor_con_entrada(&contenedor, entrada);

        // La comprobación precede a la extracción.
        let e = container::verify_structure(&contenedor).unwrap_err();
        assert_eq!(e.clause(), Some("32.4.2"), "{nombre}");

        let v = verificar(&receptor, &contenedor, &ctx);
        assert_eq!(v.checks.container, Outcome::Fail, "{nombre}");
        assert_eq!(v.result, ReceiptResult::Rejected, "{nombre}");
        assert!(v.package_dir.is_none(), "{nombre}");
        // Nada se escribió fuera del directorio de destino.
        assert!(!receptor.repo.root().join("escapado.txt").exists());
        assert!(!receptor.repo.inbox().join("escapado.txt").exists());
    }

    // Un enlace simbólico tampoco se admite.
    let contenedor = receptor.repo.inbox().join("enlace.stave");
    contenedor_con_enlace(&contenedor);
    assert!(container::verify_structure(&contenedor).is_err());
}

// =========================================================================
// 6. Ciclo de custodia: cesión, retorno, vencimiento y recuperación forzosa
// =========================================================================

#[test]
fn ciclo_de_custodia_cesion_y_retorno() {
    let cedente = estudio("Estudio A", "J. Duarte");
    let cesionario = estudio("Estudio B", "M. Rivas");
    let mut p = crear(&cedente, "Tema");
    preparar_para_entrega(&mut p);

    // Cesión.
    let em = emitir(
        &cedente,
        &mut p,
        &envio(Profile::Production, true, &cedente.org, &cesionario.org),
    );
    assert_eq!(p.custody_state(), CustodyState::InTransit);
    // La copia local queda en solo lectura por medios técnicos.
    let master = p.root().join("07_MASTER/master.wav");
    assert!(std::fs::metadata(&master).unwrap().permissions().readonly());
    // Y con marcador en texto plano.
    let lock =
        manifest::custody_lock::CustodyLock::load(&p.root().join(manifest::CUSTODY_LOCK_FILE))
            .unwrap();
    assert_eq!(lock.shipment_id, em.shipment_id);
    // Ninguna modificación es posible mientras el estado sea en tránsito.
    assert!(project::rename(&cedente.repo, &cedente.actor, &mut p, "Otro").is_err());
    assert!(project::derive(&cedente.repo, &cedente.actor, &mut p, "x", false).is_err());

    // El cesionario recibe, verifica y asume la custodia.
    let paquete = depositar(&cesionario, &em.artifact);
    let ctx = contexto(&cesionario, &cedente.org);
    let v = verificar(&cesionario, &paquete, &ctx);
    assert!(v.transfers_custody);
    assert_eq!(v.checks.custody, Outcome::Pass);

    let mut destino = crear(&cesionario, "Tema Recibido");
    // El cesionario emite el acuse aceptando la custodia (paso 8 de la Tabla 33).
    let ruta_acuse = cesionario.repo.root().join("acuse.yaml");
    ingest::issue_receipt(
        &cesionario.repo,
        &cesionario.actor,
        &v,
        &ctx,
        "01_REF",
        None,
        true,
        v.checks_as_pairs().first().map(|_| "2026-12-31"),
        sincronizado(),
        &ruta_acuse,
    )
    .unwrap();

    let ing = ingest::ingest(
        &cesionario.repo,
        &cesionario.actor,
        &v,
        &ctx,
        Some(&mut destino),
        &paquete,
    )
    .unwrap();
    assert!(ing.custody_assumed);
    assert_eq!(destino.custody_state(), CustodyState::Own);
    assert!(!destino.root().join(manifest::CUSTODY_LOCK_FILE).exists());

    // La cronología fusionada es consecutiva y contiene los actos de la Tabla 16.
    let hist = destino.custody_history();
    let acciones: Vec<CustodyAction> = hist.iter().map(|x| x.action).collect();
    for esperada in [
        CustodyAction::Exported,
        CustodyAction::Sent,
        CustodyAction::Received,
        CustodyAction::Imported,
        CustodyAction::CustodyAssumed,
    ] {
        assert!(
            acciones.contains(&esperada),
            "falta {esperada:?} en {acciones:?}"
        );
    }
    assert!(custody::check_chronology(&hist).is_clean());

    // Paso 9: el cedente recibe el acuse y la cesión se completa. Hasta este
    // momento el estado era en tránsito.
    let acuse = attacca_core::manifest::receipt::Receipt::load(&ruta_acuse).unwrap();
    let resultante =
        custody_ops::record_receipt(&cedente.repo, &cedente.actor, &mut p, &acuse, &cedente.org)
            .unwrap();
    assert_eq!(resultante, CustodyState::Ceded);
    assert_eq!(p.custody_state(), CustodyState::Ceded);
    // La copia local sigue bloqueada y con su marcador.
    assert!(p.root().join(manifest::CUSTODY_LOCK_FILE).is_file());
    assert!(p
        .custody_history()
        .iter()
        .any(|x| x.action == CustodyAction::CustodyTransferred));

    // Retorno: el cesionario devuelve la custodia.
    let pkg = bagit::PackageDir::create(cesionario.repo.root(), "STAVE-XCHG_retorno").unwrap();
    std::fs::write(
        pkg.content().join("mezcla_v02.wav"),
        b"trabajo del cesionario",
    )
    .unwrap();
    let mut ex = ExchangeManifest::new(pkg.exchange_manifest());
    ex.doc_mut().set(
        "shipment",
        Node::map(vec![("id", Node::str("RETORNO-0001"))]),
    );
    ex.doc_mut().set(
        "custody",
        Node::map(vec![
            ("transfers", Node::Bool(false)),
            ("returns", Node::Bool(true)),
            ("supersedes_transfer", Node::str(&em.shipment_id)),
            (
                "history",
                Node::Seq(
                    destino
                        .custody_history()
                        .iter()
                        .map(custody::ChronologyEntry::to_node)
                        .collect(),
                ),
            ),
        ]),
    );
    ex.save().unwrap();

    let r = custody_ops::accept_return(
        &cedente.repo,
        &cedente.actor,
        &mut p,
        pkg.root(),
        &cedente.org,
    )
    .unwrap();

    assert!(r.custody_restored);
    assert!(!r.divergent);
    assert_eq!(p.custody_state(), CustodyState::Own);
    assert!(!p.root().join(manifest::CUSTODY_LOCK_FILE).exists());
    // El acceso de escritura se restituye una vez aceptado el retorno.
    assert!(!std::fs::metadata(&master).unwrap().permissions().readonly());
    // La copia congelada anterior no se sobrescribió.
    assert!(r.incorporated_at.join("mezcla_v02.wav").is_file());
    assert!(master.is_file());
    // La cronología incorpora los asientos de la otra parte.
    let hist = p.custody_history();
    assert!(hist
        .iter()
        .any(|x| x.action == CustodyAction::CustodyAssumed));
    assert!(hist
        .iter()
        .any(|x| x.action == CustodyAction::CustodyReturned));
    assert!(custody::check_chronology(&hist).is_clean(), "{hist:?}");
}

#[test]
fn ciclo_de_custodia_vencimiento_y_recuperacion_forzosa() {
    let cedente = estudio("Estudio A", "J. Duarte");
    let mut p = crear(&cedente, "Tema");
    preparar_para_entrega(&mut p);
    let mut s = envio(Profile::Production, true, &cedente.org, "Estudio B");
    // La fecha esperada de retorno ya venció, y también el plazo de gracia.
    s.expected_return = Some(clock::add_days(&clock::today(), -40).unwrap());
    s.grace_days = 15;
    let em = emitir(&cedente, &mut p, &s);

    // La cesión se completa al obtenerse el acuse: el estado pasa a cedida.
    let seq = p.next_custody_seq();
    p.doc_mut()
        .ensure_map("custody")
        .ensure_seq("history")
        .push(
            custody::ChronologyEntry {
                seq,
                action: CustodyAction::CustodyTransferred,
                ts: clock::now_rfc3339(),
                actor: cedente.actor.clone(),
                org: cedente.org.clone(),
                shipment_id: Some(em.shipment_id.clone()),
            }
            .to_node(),
        );
    p.doc_mut()
        .ensure_map("custody")
        .set("state", Node::str("cedida"));
    p.save().unwrap();

    // Reclamación por vencimiento de la fecha esperada de retorno.
    let situacion = custody_ops::claim_return(&cedente.repo, &cedente.actor, &p).unwrap();
    assert_eq!(situacion, custody::Expiry::ReclaimAvailable);

    // Recuperación forzosa, vencido además el plazo de gracia.
    custody_ops::reclaim(&cedente.repo, &cedente.actor, &mut p, &cedente.org).unwrap();
    assert_eq!(p.custody_state(), CustodyState::Reclaimed);
    assert!(p.custody_state().allows_write());
    assert!(!p.root().join(manifest::CUSTODY_LOCK_FILE).exists());
    assert!(!std::fs::metadata(p.root().join("07_MASTER/master.wav"))
        .unwrap()
        .permissions()
        .readonly());
    assert!(p
        .custody_history()
        .iter()
        .any(|x| x.action == CustodyAction::CustodyReclaimed));
    // Se abre una no conformidad mayor (apartado 41.4, paso 4).
    let ncs = attacca_core::nonconformity::Register::at(cedente.repo.root())
        .open_entries()
        .unwrap();
    assert!(ncs
        .iter()
        .any(|n| n.severity == attacca_core::nonconformity::Severity::Major));

    // Un retorno posterior no se ingiere de forma automática: es divergente.
    let pkg = bagit::PackageDir::create(cedente.repo.root(), "STAVE-XCHG_tardio").unwrap();
    std::fs::write(pkg.content().join("mezcla.wav"), b"trabajo tardio").unwrap();
    let mut ex = ExchangeManifest::new(pkg.exchange_manifest());
    ex.doc_mut().set(
        "shipment",
        Node::map(vec![("id", Node::str("TARDIO-0001"))]),
    );
    ex.doc_mut().set(
        "custody",
        Node::map(vec![
            ("transfers", Node::Bool(false)),
            ("returns", Node::Bool(true)),
        ]),
    );
    ex.save().unwrap();

    let r = custody_ops::accept_return(
        &cedente.repo,
        &cedente.actor,
        &mut p,
        pkg.root(),
        &cedente.org,
    )
    .unwrap();
    assert!(r.divergent);
    assert!(!r.custody_restored);
    assert_eq!(p.custody_state(), CustodyState::Reclaimed);
    assert!(r.incorporated_at.join("mezcla.wav").is_file());
}

#[test]
fn el_proyecto_derivado_hace_visible_la_divergencia_sobre_material_cedido() {
    let cedente = estudio("Estudio A", "J. Duarte");
    let mut p = crear(&cedente, "Tema");
    preparar_para_entrega(&mut p);
    let mut s = envio(Profile::Production, true, &cedente.org, "Estudio B");
    s.expected_return = Some(clock::add_days(&clock::today(), 60).unwrap());
    emitir(&cedente, &mut p, &s);

    // El apartado 14.3.3 ofrece una salida en lugar de bloquear sin más.
    let d = custody_ops::open_divergent_project(&cedente.repo, &cedente.actor, &p).unwrap();
    let derivado = ProjectManifest::load(d.derived_root.join(manifest::PROJECT_FILE)).unwrap();
    assert_eq!(derivado.custody_state(), CustodyState::Own);
    assert_eq!(
        derivado.doc().at("lineage.parent_uid").unwrap().as_str(),
        p.uid()
    );
    // Y el proyecto cedido sigue bloqueado.
    assert_eq!(p.custody_state(), CustodyState::InTransit);
    assert!(std::fs::metadata(p.root().join("07_MASTER/master.wav"))
        .unwrap()
        .permissions()
        .readonly());
}

// =========================================================================
// 7. Conmutación de réplica y detección de divergencia
// =========================================================================

#[test]
fn conmutacion_de_replica_y_deteccion_de_divergencia() {
    let e = estudio("Estudio A", "J. Duarte");
    let mut p = crear(&e, "Tema");
    preparar_para_entrega(&mut p);

    // Se declara la réplica local como activa.
    attacca_core::replica::set_replicas(
        p.doc_mut(),
        &[attacca_core::replica::Replica {
            volume: "vol-local".into(),
            path: "20_PROJECTS/0_IDEAS".into(),
            state: attacca_core::replica::ReplicaState::Active,
            last_synced: Some(clock::now_rfc3339()),
        }],
    );
    p.doc_mut()
        .ensure_map("replication")
        .set("active_volume", Node::str("vol-local"));
    p.save().unwrap();

    // Conmutación hacia un volumen portátil.
    let portatil = pruebas::raiz_temporal().unwrap();
    let destino = portatil.path().join("2026-08-06_Tema_ORIG");
    let origen = p.root().to_path_buf();
    let informe = attacca_core::replica::switch_active(
        &mut p,
        &origen,
        &destino,
        "vol-portatil",
        "Estudio - Portable 01",
        true,
    )
    .unwrap();

    assert_eq!(informe.to_volume, "vol-portatil");
    assert!(informe.files_verified > 0);
    // La activa pasa a ser la nueva, en un solo acto.
    assert_eq!(p.active_volume(), Some("vol-portatil"));
    let replicas = attacca_core::replica::replicas_of(p.doc());
    assert_eq!(
        replicas
            .iter()
            .filter(|r| r.state == attacca_core::replica::ReplicaState::Active)
            .count(),
        1
    );
    // La anterior queda en solo lectura y con su marcador.
    assert!(origen.join(manifest::REPLICA_HOLD_FILE).is_file());
    let hold = manifest::replica_hold::ReplicaHold::parse(
        &std::fs::read_to_string(origen.join(manifest::REPLICA_HOLD_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(hold.state, attacca_core::replica::ReplicaState::Standby);
    assert_eq!(hold.active_uuid, "vol-portatil");
    assert!(std::fs::metadata(origen.join("07_MASTER/master.wav"))
        .unwrap()
        .permissions()
        .readonly());
    // La nueva activa no lleva marcador.
    assert!(!destino.join(manifest::REPLICA_HOLD_FILE).exists());
    // No se escribe en una réplica que no sea la activa.
    let copia = ProjectManifest::load(destino.join(manifest::PROJECT_FILE)).unwrap();
    assert!(attacca_core::replica::require_active_replica(&copia, "vol-local").is_err());
    assert!(attacca_core::replica::require_active_replica(&copia, "vol-portatil").is_ok());

    // Detección de divergencia al reconectar.
    assert_eq!(
        attacca_core::replica::reconcile(&destino, &origen).unwrap(),
        attacca_core::replica::Reconciliation::Synchronizable
    );
    let _ = attacca_core::fsx::readonly::set_tree_readonly(&origen, false, &[]);
    std::fs::write(
        origen.join("07_MASTER/master.wav"),
        b"cambio hecho en la replica desconectada",
    )
    .unwrap();
    match attacca_core::replica::reconcile(&destino, &origen).unwrap() {
        attacca_core::replica::Reconciliation::Divergent { changed } => {
            assert!(
                changed.contains(&"07_MASTER/master.wav".to_string()),
                "{changed:?}"
            );
        }
        otro => panic!("se esperaba divergencia, se obtuvo {otro:?}"),
    }
}

// =========================================================================
// 8. Contenedor .stave extraído con una utilidad del sistema, sin la aplicación
// =========================================================================

#[test]
fn contenedor_extraido_con_utilidad_del_sistema() {
    let e = estudio("Estudio A", "J. Duarte");
    let mut p = crear(&e, "Tema");
    preparar_para_entrega(&mut p);
    let em = emitir(
        &e,
        &mut p,
        &envio(Profile::Delivery, false, &e.org, "Sello B"),
    );

    // El contenedor es un ZIP conforme a ISO/IEC 21320-1 y se abre con una
    // biblioteca de propósito general, sin conocimiento de la norma.
    let archivo = std::fs::File::open(&em.artifact).unwrap();
    let mut zip = zip::ZipArchive::new(archivo).unwrap();

    // Primera entrada: mimetype, sin comprimir, contenido exacto, sin
    // terminador de línea.
    {
        use std::io::Read;
        let mut primera = zip.by_index(0).unwrap();
        assert_eq!(primera.name(), "mimetype");
        assert_eq!(primera.compression(), zip::CompressionMethod::Stored);
        let mut s = String::new();
        primera.read_to_string(&mut s).unwrap();
        assert_eq!(s, "application/vnd.stave.package");
        assert!(!s.ends_with('\n'));
    }

    // Solo almacenamiento o desinflado, sin cifrado, rutas relativas con barra
    // inclinada.
    let mut raices = std::collections::BTreeSet::new();
    for i in 0..zip.len() {
        let entrada = zip.by_index(i).unwrap();
        let nombre = entrada.name().to_string();
        assert!(matches!(
            entrada.compression(),
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
        ));
        assert!(!entrada.encrypted());
        assert!(!nombre.starts_with('/'));
        assert!(!nombre.contains('\\'));
        assert!(!nombre.split('/').any(|s| s == ".."));
        if nombre != "mimetype" {
            raices.insert(nombre.split('/').next().unwrap().to_string());
        }
    }
    // Salvo mimetype, una única entrada de primer nivel.
    assert_eq!(raices.len(), 1);

    // Extracción a un directorio limpio y verificación con los manifiestos, tal
    // como haría `sha256sum -c`.
    let fuera = pruebas::raiz_temporal().unwrap();
    let (mut prog, canc) = container::silent_progress();
    let mut pr = Progress {
        on_progress: &mut prog,
        cancelled: &canc,
    };
    let raiz = container::extract(&em.artifact, fuera.path(), &mut pr).unwrap();

    for archivo in [
        bagit::BAGIT_TXT,
        "EXCHANGE.yaml",
        bagit::LEEME_TXT,
        bagit::MANIFEST_TXT,
    ] {
        assert!(raiz.join(archivo).is_file(), "falta {archivo}");
    }
    // Cada línea del manifiesto de integridad se comprueba con el mismo
    // procedimiento que emplea una utilidad del sistema.
    let manifiesto = integrity::IntegrityManifest::load(&raiz.join(bagit::MANIFEST_TXT)).unwrap();
    assert!(!manifiesto.is_empty());
    for (ruta, resumen) in manifiesto.entries() {
        let real = integrity::digest_file(&raiz.join(ruta)).unwrap();
        assert_eq!(real, resumen, "{ruta}");
    }
    // El LEEME explica el procedimiento a quien no tiene Attacca.
    let leeme = std::fs::read_to_string(raiz.join(bagit::LEEME_TXT)).unwrap();
    assert!(leeme.contains(".zip"));
    assert!(leeme.contains("sha256sum -c"));
    // Y el material se abre con cualquier programa.
    assert_eq!(
        std::fs::read(raiz.join("data/content/07_MASTER/master.wav")).unwrap(),
        b"audio del master"
    );
}

// =========================================================================
// 9. Reconstrucción íntegra del índice tras borrar la base de datos
// =========================================================================

#[test]
fn reconstruccion_integra_del_indice_tras_borrar_la_base_de_datos() {
    let e = estudio("Estudio A", "J. Duarte");
    for i in 0..12 {
        let mut p = crear(&e, &format!("Tema {i}"));
        if i % 3 == 0 {
            preparar_para_entrega(&mut p);
        }
    }
    let cache = e.repo.root().join(attacca_core::index::CACHE_FILE);

    let (antes, r1) = Index::load_or_rebuild(&e.repo, e.repo.root()).unwrap();
    assert!(r1.rebuilt);
    assert_eq!(antes.projects.len(), 12);
    assert!(cache.is_file());

    // Se borra la base de datos de la aplicación.
    std::fs::remove_file(&cache).unwrap();

    // Reconstrucción íntegra recorriendo el árbol.
    let (despues, r2) = Index::load_or_rebuild(&e.repo, e.repo.root()).unwrap();
    assert!(r2.rebuilt);
    assert_eq!(despues.projects.len(), antes.projects.len());

    // No se ha perdido nada: cada campo coincide.
    for (a, b) in antes.projects.iter().zip(despues.projects.iter()) {
        assert_eq!(a.uid, b.uid);
        assert_eq!(a.id, b.id);
        assert_eq!(a.title, b.title);
        assert_eq!(a.artist, b.artist);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.level, b.level);
        assert_eq!(a.status, b.status);
        assert_eq!(a.custody, b.custody);
        assert_eq!(a.stage, b.stage);
        assert_eq!(a.release_uid, b.release_uid);
        assert_eq!(a.path, b.path);
    }

    // El registro de eventos, que es autoritativo, sigue intacto.
    assert!(e.repo.event_log().verify_chain().unwrap().is_intact());
}

// =========================================================================
// Obligaciones comunes a toda implementación (apartado 44.1)
// =========================================================================

#[test]
fn preserva_los_campos_de_manifiesto_que_no_comprende() {
    let e = estudio("Estudio A", "J. Duarte");
    let mut p = crear(&e, "Tema");

    // Otra implementación añade campos que Attacca no conoce, en varios niveles.
    let ruta = p.path().to_path_buf();
    let mut texto = std::fs::read_to_string(&ruta).unwrap();
    texto.push_str("x_otrofab_bloque:\n  campo: valor\n  lista:\n    - uno\n    - dos\n");
    texto.push_str("campo_de_version_futura: 42\n");
    std::fs::write(&ruta, &texto).unwrap();

    // Attacca reescribe el manifiesto varias veces.
    p = ProjectManifest::load(&ruta).unwrap();
    project::rename(&e.repo, &e.actor, &mut p, "Otro Titulo").unwrap();
    p.doc_mut().ensure_map("audio").set("tempo", Node::Int(120));
    p.save().unwrap();

    let final_ = std::fs::read_to_string(p.path()).unwrap();
    assert!(final_.contains("x_otrofab_bloque:"), "{final_}");
    assert!(final_.contains("campo: valor"));
    assert!(final_.contains("- uno"));
    assert!(final_.contains("campo_de_version_futura: 42"));
    // Y registra su nombre y versión al modificar.
    assert!(final_.contains(&format!("written_by: {}", attacca_core::written_by())));
}

#[test]
fn la_escritura_de_manifiestos_es_determinista() {
    let e = estudio("Estudio A", "J. Duarte");
    let p = crear(&e, "Tema");
    let primera = std::fs::read_to_string(p.path()).unwrap();

    for _ in 0..5 {
        let mut m = ProjectManifest::load(p.path()).unwrap();
        m.save().unwrap();
    }
    assert_eq!(std::fs::read_to_string(p.path()).unwrap(), primera);
}

#[test]
fn declara_su_clase_de_conformidad_y_sus_extensiones() {
    let d = attacca_core::conformance::declaration();
    assert_eq!(d.class, attacca_core::conformance::Class::Manager);
    assert_eq!(d.stave_version, "2.0");
    let texto = attacca_core::conformance::render();
    assert!(texto.contains("STAVE 2.0"));
    assert!(texto.contains("M — Gestora"));
    for ext in attacca_core::conformance::EXTENSIONS {
        assert!(ext.field.starts_with(attacca_core::EXT_PREFIX));
    }
}

// --- Auxiliares de prueba ---

/// Reescribe una entrada del contenedor conservando el resto.
fn alterar_entrada(contenedor: &Path, sufijo: &str, contenido: &[u8]) {
    use std::io::{Read, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};

    let mut origen = ZipArchive::new(std::fs::File::open(contenedor).unwrap()).unwrap();
    let temporal = contenedor.with_extension("tmpzip");
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
            destino
                .write_all(if nombre.ends_with(sufijo) {
                    contenido
                } else {
                    &datos
                })
                .unwrap();
        }
        destino.finish().unwrap();
    }
    std::fs::rename(&temporal, contenedor).unwrap();
}

fn contenedor_con_entrada(destino: &Path, entrada: &str) {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut zip = ZipWriter::new(std::fs::File::create(destino).unwrap());
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(container::MIMETYPE.as_bytes()).unwrap();
    zip.start_file(entrada, SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"contenido fuera de lugar").unwrap();
    zip.finish().unwrap();
}

fn contenedor_con_enlace(destino: &Path) {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut zip = ZipWriter::new(std::fs::File::create(destino).unwrap());
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(container::MIMETYPE.as_bytes()).unwrap();
    zip.add_symlink(
        "paquete/enlace",
        "/etc/passwd",
        SimpleFileOptions::default(),
    )
    .unwrap();
    zip.finish().unwrap();
}

/// Presenta las discrepancias en los mensajes de fallo de las aserciones.
trait Diagnostico {
    fn discrepancias_debug(&self) -> String;
}

impl Diagnostico for ingest::Verification {
    fn discrepancias_debug(&self) -> String {
        self.discrepancies
            .iter()
            .map(|(c, d)| format!("{c}: {d}"))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// Indica si el proceso puede eludir los bits de permiso del sistema de
/// archivos. En Unix, root los ignora.
fn es_administrador() -> bool {
    #[cfg(unix)]
    {
        // SEGURIDAD: `geteuid` solo consulta el identificador efectivo.
        extern "C" {
            fn geteuid() -> u32;
        }
        unsafe { geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
