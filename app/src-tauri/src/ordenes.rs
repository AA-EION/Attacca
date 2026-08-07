//! Órdenes que la interfaz invoca.
//!
//! Cada orden es una operación del núcleo. La interfaz no toma decisiones
//! normativas: consulta, presenta y transmite.

use crate::estado::{
    self, Conformidad, Entrada, Estado, Identidad, ProyectoResumen, VistaProyecto,
};
use attacca_core::clock::{self, SyncState};
use attacca_core::manifest::exchange::{Classification, Profile};
use attacca_core::manifest::project::{Level, ProjectManifest, Status};
use attacca_core::manifest::release::ReleaseClass;
use attacca_core::package::container::Progress;
use attacca_core::package::{custody_ops, emit, ingest};
use attacca_core::repo::Repository;
use attacca_core::{journal, project, release};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::State;

type R<T> = Result<T, String>;

fn err(e: attacca_core::Error) -> String {
    e.to_string()
}

// --- Repositorio ---

#[derive(Serialize)]
pub struct AperturaRepositorio {
    pub raiz: String,
    /// Servicio de sincronización detectado en la ruta, si lo hay.
    pub aviso_sincronizacion: Option<String>,
    /// Operaciones que quedaron sin concluir en el cierre anterior.
    pub operaciones_a_medias: usize,
    pub temporales_abandonados: usize,
}

#[tauri::command]
pub fn raiz_sugerida() -> Option<String> {
    Repository::suggested_local_root().map(|p| p.display().to_string())
}

#[tauri::command]
pub fn abrir_repositorio(estado: State<Estado>, raiz: String) -> R<AperturaRepositorio> {
    let ruta = PathBuf::from(&raiz);
    estado.abrir(ruta.clone())?;
    apertura(&ruta)
}

#[tauri::command]
pub fn crear_repositorio(estado: State<Estado>, raiz: String) -> R<AperturaRepositorio> {
    let ruta = PathBuf::from(&raiz);
    estado.crear(ruta.clone())?;
    apertura(&ruta)
}

fn apertura(ruta: &Path) -> R<AperturaRepositorio> {
    // Recuperación tras cierre inesperado: se detecta al arrancar.
    let recuperacion = journal::detect(ruta, ruta).map_err(err)?;
    Ok(AperturaRepositorio {
        raiz: ruta.display().to_string(),
        aviso_sincronizacion: Repository::warn_if_synced_location(ruta),
        operaciones_a_medias: recuperacion.incomplete.len(),
        temporales_abandonados: recuperacion.abandoned_temps.len(),
    })
}

#[tauri::command]
pub fn limpiar_temporales(estado: State<Estado>) -> R<usize> {
    let repo = estado.repositorio()?;
    let r = journal::detect(repo.root(), repo.root()).map_err(err)?;
    Ok(journal::clear_abandoned_temps(&r))
}

// --- Identidad y reloj ---

#[tauri::command]
pub fn identidad(estado: State<Estado>) -> Identidad {
    estado.identidad()
}

#[tauri::command]
pub fn set_identidad(estado: State<Estado>, identidad: Identidad) {
    estado.set_identidad(identidad);
}

#[derive(Serialize)]
pub struct EstadoReloj {
    /// `synced`, `drifted` o `unavailable`.
    pub estado: String,
    pub desviacion_ms: i64,
    /// `ntp` o `http_date`.
    pub precision: String,
    pub admite_emision: bool,
    pub marca: String,
}

#[tauri::command]
pub fn comprobar_reloj(estado: State<Estado>) -> EstadoReloj {
    let s = clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(4));
    estado.set_reloj(s);
    proyectar_reloj(s)
}

#[tauri::command]
pub fn reloj_conocido(estado: State<Estado>) -> EstadoReloj {
    proyectar_reloj(estado.reloj().unwrap_or(SyncState::Unavailable))
}

fn proyectar_reloj(s: SyncState) -> EstadoReloj {
    EstadoReloj {
        estado: match s {
            SyncState::Synced { .. } => "synced",
            SyncState::Drifted { .. } => "drifted",
            SyncState::Unavailable => "unavailable",
        }
        .into(),
        desviacion_ms: match s {
            SyncState::Synced { drift_ms } | SyncState::Drifted { drift_ms } => drift_ms,
            SyncState::Unavailable => 0,
        },
        precision: match clock::last_precision() {
            clock::Precision::Ntp => "ntp",
            clock::Precision::HttpDate => "http_date",
        }
        .into(),
        admite_emision: s.allows_emission(),
        marca: clock::now_rfc3339(),
    }
}

// --- Listado ---

#[derive(Serialize)]
pub struct Catalogo {
    pub proyectos: Vec<ProyectoResumen>,
    pub releases: Vec<ReleaseResumen>,
    /// Manifiestos que no se pudieron interpretar. No se ocultan.
    pub ilegibles: Vec<(String, String)>,
    pub paquetes_en_cuarentena: usize,
    pub atraso_cuarentena: usize,
}

#[derive(Serialize)]
pub struct ReleaseResumen {
    pub uid: String,
    pub id: String,
    pub titulo: String,
    pub artista: String,
    pub clase: String,
    pub estado: String,
    pub temas: usize,
}

#[tauri::command]
pub fn catalogo(estado: State<Estado>) -> R<Catalogo> {
    let repo = estado.repositorio()?;
    let idx = estado::indice(&repo)?;
    Ok(Catalogo {
        proyectos: idx.projects.iter().map(estado::resumen_de).collect(),
        releases: idx
            .releases
            .iter()
            .map(|r| ReleaseResumen {
                uid: r.uid.clone(),
                id: r.id.clone(),
                titulo: r.title.clone(),
                artista: r.artist.clone(),
                clase: r.class.clone(),
                estado: r.status.clone(),
                temas: r.track_count,
            })
            .collect(),
        ilegibles: idx
            .unreadable
            .iter()
            .map(|(p, m)| (p.display().to_string(), m.clone()))
            .collect(),
        paquetes_en_cuarentena: repo.quarantined_packages().len(),
        atraso_cuarentena: repo.inbox_backlog().len(),
    })
}

#[tauri::command]
pub fn reconstruir_indice(estado: State<Estado>) -> R<usize> {
    let repo = estado.repositorio()?;
    let idx = attacca_core::index::Index::rebuild(&repo).map_err(err)?;
    let _ = idx.write_cache(&repo.root().join(attacca_core::index::CACHE_FILE));
    Ok(idx.projects.len())
}

// --- Proyecto ---

#[derive(Deserialize)]
pub struct DatosProyecto {
    pub titulo: String,
    pub artista: String,
    pub tipo: String,
    pub nivel: String,
    pub frecuencia: i64,
    pub bits: i64,
    pub release_uid: Option<String>,
}

#[tauri::command]
pub fn crear_proyecto(estado: State<Estado>, datos: DatosProyecto) -> R<VistaProyecto> {
    let repo = estado.repositorio()?;
    let identidad = estado.identidad();
    let nivel = Level::parse(&datos.nivel).unwrap_or(Level::B);

    // Un proyecto vinculado a un release reside dentro de su carpeta.
    let release_dir = datos
        .release_uid
        .as_deref()
        .and_then(|uid| release::find_by_uid(&repo, uid))
        .and_then(|p| {
            attacca_core::manifest::release::ReleaseManifest::load(&p)
                .ok()
                .map(|r| r.root().to_path_buf())
        });

    let actor = identidad.persona.clone();
    let m = project::create(
        &repo,
        &actor,
        &project::NewProject {
            title: datos.titulo,
            artist: datos.artista,
            kind: datos.tipo,
            level: nivel,
            release_uid: datos.release_uid,
            release_dir,
            sample_rate: datos.frecuencia,
            bit_depth: datos.bits,
            holder_org: identidad.organizacion,
            holder_person: identidad.persona,
            active_volume: None,
        },
    )
    .map_err(err)?;
    Ok(estado::vista_de(&m))
}

#[tauri::command]
pub fn ver_proyecto(estado: State<Estado>, uid: String) -> R<VistaProyecto> {
    let m = cargar(&estado, &uid)?;
    Ok(estado::vista_de(&m))
}

#[tauri::command]
pub fn cambiar_titulo(estado: State<Estado>, uid: String, titulo: String) -> R<VistaProyecto> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    project::rename(&repo, &estado.actor(), &mut m, &titulo).map_err(err)?;
    Ok(estado::vista_de(&m))
}

#[tauri::command]
pub fn cambiar_estado(estado: State<Estado>, uid: String, nuevo: String) -> R<VistaProyecto> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    let s = Status::parse(&nuevo).ok_or_else(|| {
        format!("El estado «{nuevo}» no es admisible. El proyecto no se ha modificado.")
    })?;
    project::set_status(&repo, &estado.actor(), &mut m, s).map_err(err)?;
    Ok(estado::vista_de(&m))
}

#[derive(Deserialize)]
pub struct DatosAudio {
    pub tempo: Option<i64>,
    pub tonalidad: Option<String>,
    pub afinacion: Option<i64>,
    pub origen: Option<String>,
}

/// Registra los parámetros de audio que siguen siendo modificables.
///
/// El apartado 13.2 los admite mientras no concluya la grabación. Concluida esa
/// etapa quedan fijados.
#[tauri::command]
pub fn registrar_audio(estado: State<Estado>, uid: String, datos: DatosAudio) -> R<VistaProyecto> {
    let mut m = cargar(&estado, &uid)?;
    attacca_core::custody::require_writable(m.custody_state()).map_err(err)?;
    if m.audio_params_locked() {
        return Err(attacca_core::Error::requirement(
            "13.2",
            "La etapa de grabación ya concluyó. Los valores no se han modificado. El tempo, la tonalidad y la afinación de referencia quedan fijados al cerrar la grabación.",
        )
        .to_string());
    }
    {
        let audio = m.doc_mut().ensure_map("audio");
        if let Some(t) = datos.tempo {
            audio.set("tempo", attacca_core::doc::Node::Int(t));
        }
        if let Some(k) = datos.tonalidad {
            audio.set("key", attacca_core::doc::Node::str(k));
        }
        if let Some(a) = datos.afinacion {
            audio.set("tuning_hz", attacca_core::doc::Node::Int(a));
        }
        if let Some(o) = datos.origen {
            audio.set("origin", attacca_core::doc::Node::str(o));
        }
    }
    m.save().map_err(err)?;
    Ok(estado::vista_de(&m))
}

#[derive(Serialize)]
pub struct ResultadoDerivacion {
    pub uid_derivado: String,
    pub id_derivado: String,
    pub archivos: usize,
    pub origen_sellado: bool,
    pub ruta: String,
}

#[tauri::command]
pub fn derivar_proyecto(
    estado: State<Estado>,
    uid: String,
    motivo: String,
    paralelo: bool,
) -> R<ResultadoDerivacion> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    let d = project::derive(&repo, &estado.actor(), &mut m, &motivo, paralelo).map_err(err)?;
    Ok(ResultadoDerivacion {
        uid_derivado: d.derived_uid,
        id_derivado: d.derived_id,
        archivos: d.files_copied,
        origen_sellado: d.source_sealed,
        ruta: d.derived_root.display().to_string(),
    })
}

/// Crea la subcarpeta de sesión y devuelve la carpeta que debe abrirse.
#[tauri::command]
pub fn crear_sesion(
    estado: State<Estado>,
    uid: String,
    daw: String,
    etapa: String,
) -> R<CarpetaSesion> {
    let m = cargar(&estado, &uid)?;
    let carpeta = project::create_session_folder(&m, &daw, &etapa).map_err(err)?;
    let siguiente = siguiente_version_sesion(&carpeta);
    Ok(CarpetaSesion {
        ruta: carpeta.display().to_string(),
        nombre_sugerido: project::suggested_session_name(&m, &etapa, siguiente),
    })
}

#[derive(Serialize)]
pub struct CarpetaSesion {
    pub ruta: String,
    pub nombre_sugerido: String,
}

fn siguiente_version_sesion(carpeta: &Path) -> u32 {
    let Ok(entradas) = std::fs::read_dir(carpeta) else {
        return 1;
    };
    let mut max = 0u32;
    for e in entradas.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        let base = n.rsplit_once('.').map(|(b, _)| b.to_string()).unwrap_or(n);
        if let Some(v) = attacca_core::naming::version_suffix_of(&base) {
            max = max.max(v);
        }
    }
    max + 1
}

/// Ruta libre para una exportación, sin sobrescribir lo ya exportado.
#[tauri::command]
pub fn ruta_exportacion(
    estado: State<Estado>,
    uid: String,
    carpeta: String,
    nombre: String,
    extension: String,
) -> R<String> {
    let m = cargar(&estado, &uid)?;
    project::next_export_path(&m, &carpeta, &nombre, &extension)
        .map(|p| p.display().to_string())
        .map_err(err)
}

#[derive(Deserialize)]
pub struct DatosIncorporacion {
    pub origen_en_cuarentena: String,
    pub procedencia: String,
    pub autorizacion: String,
    pub clasificacion: String,
}

#[tauri::command]
pub fn incorporar_material(
    estado: State<Estado>,
    uid: String,
    datos: DatosIncorporacion,
) -> R<VistaProyecto> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    project::ingest_from_inbox(
        &repo,
        &estado.actor(),
        &mut m,
        Path::new(&datos.origen_en_cuarentena),
        &datos.procedencia,
        &datos.autorizacion,
        &datos.clasificacion,
        None,
        None,
    )
    .map_err(err)?;
    Ok(estado::vista_de(&m))
}

// --- Navegador propio ---

/// Lista el contenido de una carpeta del proyecto.
///
/// El recorrido no sale del proyecto. La acción de abrir en el explorador del
/// sistema es la vía explícita hacia el resto del sistema de archivos.
#[tauri::command]
pub fn listar_carpeta(estado: State<Estado>, uid: String, relativa: String) -> R<Vec<Entrada>> {
    let m = cargar(&estado, &uid)?;
    let raiz = m.root();
    let destino = if relativa.is_empty() {
        raiz.to_path_buf()
    } else {
        raiz.join(&relativa)
    };
    if !project::is_inside_project(raiz, &destino) {
        return Err(
            "La ruta solicitada está fuera del proyecto. No se ha listado nada. Emplear la acción de abrir en el explorador del sistema.".into(),
        );
    }
    let Ok(entradas) = std::fs::read_dir(&destino) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<Entrada> = entradas
        .flatten()
        .filter_map(|e| {
            let nombre = e.file_name().to_string_lossy().to_string();
            let ft = e.file_type().ok()?;
            if attacca_core::fsx::walk::is_regenerable(&nombre, ft.is_dir()) {
                return None;
            }
            let tamano = e.metadata().map(|md| md.len()).unwrap_or(0);
            Some(Entrada {
                nombre: nombre.clone(),
                es_carpeta: ft.is_dir(),
                tamano,
                ruta: if relativa.is_empty() {
                    nombre
                } else {
                    format!("{relativa}/{nombre}")
                },
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.es_carpeta
            .cmp(&a.es_carpeta)
            .then(a.nombre.cmp(&b.nombre))
    });
    Ok(out)
}

/// Abre una ruta en el explorador de archivos del sistema operativo.
///
/// El apartado 44.2 exige que esta acción esté disponible en todo momento.
#[tauri::command]
pub fn abrir_en_explorador(estado: State<Estado>, uid: String, relativa: String) -> R<()> {
    let m = cargar(&estado, &uid)?;
    let destino = if relativa.is_empty() {
        m.root().to_path_buf()
    } else {
        m.root().join(&relativa)
    };
    attacca_core::fsx::reveal_in_file_manager(&destino).map_err(err)
}

// --- Conformidad e integridad ---

#[tauri::command]
pub fn validar(estado: State<Estado>, uid: String) -> R<Conformidad> {
    let m = cargar(&estado, &uid)?;
    Ok(estado::vista_de(&m).conformidad)
}

#[derive(Serialize)]
pub struct ResultadoIntegridad {
    pub comprobados: usize,
    pub fallidos: usize,
    pub rutas: Vec<String>,
}

#[tauri::command]
pub fn verificar_integridad(estado: State<Estado>, uid: String) -> R<ResultadoIntegridad> {
    let m = cargar(&estado, &uid)?;
    let v = attacca_core::integrity::verify_tree(m.root()).map_err(err)?;
    Ok(ResultadoIntegridad {
        comprobados: v.checked,
        fallidos: v.failed_count(),
        rutas: v.failed_paths(),
    })
}

#[tauri::command]
pub fn generar_integridad(estado: State<Estado>, uid: String) -> R<usize> {
    let m = cargar(&estado, &uid)?;
    attacca_core::integrity::generate(m.root())
        .map(|man| man.len())
        .map_err(err)
}

// --- Release ---

#[derive(Deserialize)]
pub struct DatosRelease {
    pub titulo: String,
    pub artista: String,
    pub clase: String,
}

#[tauri::command]
pub fn crear_release(estado: State<Estado>, datos: DatosRelease) -> R<String> {
    let repo = estado.repositorio()?;
    let identidad = estado.identidad();
    let clase = ReleaseClass::parse(&datos.clase).ok_or_else(|| {
        format!(
            "La clase «{}» no figura en la Tabla 8. El release no se ha creado.",
            datos.clase
        )
    })?;
    let actor = identidad.persona.clone();
    let m = release::create(
        &repo,
        &actor,
        &release::NewRelease {
            title: datos.titulo,
            artist: datos.artista,
            class: clase,
            level: Level::B,
            holder_org: identidad.organizacion,
            holder_person: identidad.persona,
        },
    )
    .map_err(err)?;
    Ok(m.uid().unwrap_or_default().to_string())
}

#[tauri::command]
pub fn vincular_a_release(
    estado: State<Estado>,
    release_uid: String,
    proyecto_uid: String,
    posicion: Option<i64>,
) -> R<i64> {
    let repo = estado.repositorio()?;
    let ruta = release::find_by_uid(&repo, &release_uid).ok_or_else(|| {
        format!(
            "No existe ningún release con identificador {release_uid}. No se ha vinculado nada."
        )
    })?;
    let mut r = attacca_core::manifest::release::ReleaseManifest::load(&ruta).map_err(err)?;
    let mut p = cargar(&estado, &proyecto_uid)?;
    release::link_project(&repo, &estado.actor(), &mut r, &mut p, posicion).map_err(err)
}

/// Comprueba si un release puede cerrarse y entregarse (apartado 8.5).
#[tauri::command]
pub fn release_listo(estado: State<Estado>, release_uid: String) -> R<Vec<String>> {
    let repo = estado.repositorio()?;
    let ruta = release::find_by_uid(&repo, &release_uid)
        .ok_or_else(|| format!("No existe ningún release con identificador {release_uid}."))?;
    let r = attacca_core::manifest::release::ReleaseManifest::load(&ruta).map_err(err)?;
    let integrantes = release::members(&repo, &r);
    Ok(attacca_core::validate::release_ready(&r, &integrantes)
        .into_iter()
        .map(|f| f.detail)
        .collect())
}

// --- Intercambio ---

#[derive(Deserialize)]
pub struct DatosEnvio {
    pub perfil: String,
    pub clasificacion: String,
    pub destinatario: String,
    pub contacto: String,
    pub finalidad: String,
    pub retencion: String,
    pub acuse: String,
    pub ceder_custodia: bool,
    pub retorno_esperado: Option<String>,
    pub gracia: i64,
    pub serializar: bool,
    pub control_aprobado: bool,
}

#[derive(Serialize)]
pub struct ResultadoEmision {
    pub envio: String,
    pub artefacto: String,
    pub copia_congelada: String,
    pub archivos: usize,
    pub bytes: u64,
    pub custodia: String,
}

#[tauri::command]
pub fn emitir_envio(estado: State<Estado>, uid: String, datos: DatosEnvio) -> R<ResultadoEmision> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    let identidad = estado.identidad();
    let perfil = Profile::parse(&datos.perfil).ok_or_else(|| {
        "El perfil declarado no es admisible. El envío no se ha emitido.".to_string()
    })?;
    let clasificacion = Classification::parse(&datos.clasificacion).ok_or_else(|| {
        "El nivel de clasificación no es admisible. El envío no se ha emitido.".to_string()
    })?;

    let envio = emit::Shipment {
        profile: perfil,
        classification: clasificacion,
        purpose: datos.finalidad.clone(),
        issuer_org: identidad.organizacion.clone(),
        issuer_contact: datos.acuse.clone(),
        issuer_key_id: None,
        recipient_org: datos.destinatario,
        recipient_contact: datos.contacto,
        usage_permitted: vec![datos.finalidad],
        usage_territory: "mundial".into(),
        usage_term: "el declarado en el acuerdo de intercambio".into(),
        sublicensing: false,
        forwarding: false,
        retention_until: datos.retencion,
        destroy_on_expiry: true,
        personal_data: false,
        personal_data_categories: vec![],
        ack_deadline_hours: 72,
        ack_address: datos.acuse,
        transfers_custody: datos.ceder_custodia,
        returns_custody: false,
        supersedes_transfer: None,
        expected_return: datos.retorno_esperado,
        grace_days: datos.gracia,
        onward_allowed: false,
        supersedes: None,
        revision: "r0".into(),
        serialize: datos.serializar,
        qc_approved: datos.control_aprobado,
    };

    let sync = estado.reloj().unwrap_or_else(|| {
        clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(4))
    });

    estado.reiniciar_cancelacion();
    let bandera = estado.bandera_cancelacion();
    let cancelado = move || bandera.load(Ordering::Relaxed);
    let mut avance = |_: usize, _: usize| {};
    let mut progreso = Progress {
        on_progress: &mut avance,
        cancelled: &cancelado,
    };

    let e = emit::emit(
        &repo,
        &estado.actor(),
        &mut m,
        &envio,
        &emit::Payload::for_profile(perfil),
        sync,
        &mut progreso,
    )
    .map_err(err)?;

    Ok(ResultadoEmision {
        envio: e.shipment_id,
        artefacto: e.artifact.display().to_string(),
        copia_congelada: e.frozen_copy.display().to_string(),
        archivos: e.file_count,
        bytes: e.total_bytes,
        custodia: e.custody_state.as_str().to_string(),
    })
}

#[tauri::command]
pub fn cancelar_operacion(estado: State<Estado>) {
    estado.cancelar();
}

#[derive(Serialize)]
pub struct ResultadoVerificacion {
    pub envio: String,
    pub resultado: String,
    pub verificaciones: Vec<(String, String)>,
    pub discrepancias: Vec<(String, String)>,
    pub cede_custodia: bool,
    pub admite_ingesta: bool,
}

#[derive(Deserialize)]
pub struct DatosRecepcion {
    pub paquete: String,
    pub emisor: String,
    pub condiciones_aceptables: bool,
}

#[tauri::command]
pub fn verificar_paquete(estado: State<Estado>, datos: DatosRecepcion) -> R<ResultadoVerificacion> {
    let repo = estado.repositorio()?;
    let ctx = contexto(&estado, &datos.emisor, datos.condiciones_aceptables);
    estado.reiniciar_cancelacion();
    let bandera = estado.bandera_cancelacion();
    let cancelado = move || bandera.load(Ordering::Relaxed);
    let mut avance = |_: usize, _: usize| {};
    let mut progreso = Progress {
        on_progress: &mut avance,
        cancelled: &cancelado,
    };

    let v = ingest::verify(
        &repo,
        &estado.actor(),
        Path::new(&datos.paquete),
        &ctx,
        &mut progreso,
    )
    .map_err(err)?;

    let verificaciones = v
        .checks_as_pairs()
        .into_iter()
        .map(|(nombre, resultado)| (nombre.to_string(), resultado.to_string()))
        .collect();

    Ok(ResultadoVerificacion {
        envio: v.shipment_id.clone(),
        resultado: match v.result {
            attacca_core::manifest::receipt::ReceiptResult::Accepted => "accepted",
            attacca_core::manifest::receipt::ReceiptResult::AcceptedWithReservations => {
                "accepted_with_reservations"
            }
            attacca_core::manifest::receipt::ReceiptResult::Rejected => "rejected",
        }
        .into(),
        verificaciones,
        discrepancias: v.discrepancies.clone(),
        cede_custodia: v.transfers_custody,
        admite_ingesta: v.accepted(),
    })
}

/// Verifica, acusa recibo e ingiere en un solo acto.
///
/// Un paquete no se ingiere de forma parcial (apartado 37.2, último párrafo).
#[derive(Deserialize)]
pub struct DatosIngesta {
    pub paquete: String,
    pub emisor: String,
    pub condiciones_aceptables: bool,
    pub proyecto_destino: Option<String>,
    pub aceptar_custodia: bool,
}

#[derive(Serialize)]
pub struct ResultadoIngesta {
    pub envio: String,
    pub resultado: String,
    pub destino: String,
    pub clase_destino: String,
    pub archivos: usize,
    pub custodia_asumida: bool,
    pub ruta_acuse: String,
}

#[tauri::command]
pub fn ingerir_paquete(estado: State<Estado>, datos: DatosIngesta) -> R<ResultadoIngesta> {
    let repo = estado.repositorio()?;
    let ctx = contexto(&estado, &datos.emisor, datos.condiciones_aceptables);
    estado.reiniciar_cancelacion();
    let bandera = estado.bandera_cancelacion();
    let cancelado = move || bandera.load(Ordering::Relaxed);
    let mut avance = |_: usize, _: usize| {};
    let mut progreso = Progress {
        on_progress: &mut avance,
        cancelled: &cancelado,
    };

    let paquete = PathBuf::from(&datos.paquete);
    let v = ingest::verify(&repo, &estado.actor(), &paquete, &ctx, &mut progreso).map_err(err)?;

    let sync = estado.reloj().unwrap_or_else(|| {
        clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(4))
    });
    let ruta_acuse = repo
        .root()
        .join(format!("00_SYSTEM/acuse_{}.yaml", v.shipment_id));
    ingest::issue_receipt(
        &repo,
        &estado.actor(),
        &v,
        &ctx,
        "01_REF",
        None,
        datos.aceptar_custodia,
        None,
        sync,
        &ruta_acuse,
    )
    .map_err(err)?;

    let resultado = match v.result {
        attacca_core::manifest::receipt::ReceiptResult::Accepted => "accepted",
        attacca_core::manifest::receipt::ReceiptResult::AcceptedWithReservations => {
            "accepted_with_reservations"
        }
        attacca_core::manifest::receipt::ReceiptResult::Rejected => "rejected",
    };

    if !v.accepted() {
        // Un envío rechazado se retira de la cuarentena y su material no se
        // emplea para ninguna finalidad (apartado 39.1).
        ingest::discard_rejected(&repo, &estado.actor(), &paquete, &v.shipment_id).map_err(err)?;
        return Ok(ResultadoIngesta {
            envio: v.shipment_id,
            resultado: resultado.into(),
            destino: String::new(),
            clase_destino: String::new(),
            archivos: 0,
            custodia_asumida: false,
            ruta_acuse: ruta_acuse.display().to_string(),
        });
    }

    let mut destino = match &datos.proyecto_destino {
        Some(u) => Some(cargar(&estado, u)?),
        None => None,
    };
    let ing = ingest::ingest(&repo, &estado.actor(), &v, &ctx, destino.as_mut(), &paquete)
        .map_err(err)?;

    Ok(ResultadoIngesta {
        envio: v.shipment_id,
        resultado: resultado.into(),
        destino: ing.destination.display().to_string(),
        clase_destino: ing.destination_class,
        archivos: ing.files,
        custodia_asumida: ing.custody_assumed,
        ruta_acuse: ruta_acuse.display().to_string(),
    })
}

#[tauri::command]
pub fn paquetes_en_cuarentena(estado: State<Estado>) -> R<Vec<String>> {
    let repo = estado.repositorio()?;
    Ok(repo
        .quarantined_packages()
        .into_iter()
        .map(|p| p.display().to_string())
        .collect())
}

// --- Custodia ---

#[tauri::command]
pub fn reclamar_retorno(estado: State<Estado>, uid: String) -> R<String> {
    let repo = estado.repositorio()?;
    let m = cargar(&estado, &uid)?;
    let situacion = custody_ops::claim_return(&repo, &estado.actor(), &m).map_err(err)?;
    Ok(match situacion {
        attacca_core::custody::Expiry::ReclaimAvailable => "reclaim_available",
        attacca_core::custody::Expiry::ClaimDue => "claim_due",
        attacca_core::custody::Expiry::Current => "current",
    }
    .into())
}

#[tauri::command]
pub fn recuperar_custodia(estado: State<Estado>, uid: String) -> R<VistaProyecto> {
    let repo = estado.repositorio()?;
    let mut m = cargar(&estado, &uid)?;
    let org = estado.identidad().organizacion;
    custody_ops::reclaim(&repo, &estado.actor(), &mut m, &org).map_err(err)?;
    Ok(estado::vista_de(&m))
}

#[tauri::command]
pub fn derivar_sobre_cedido(estado: State<Estado>, uid: String) -> R<ResultadoDerivacion> {
    let repo = estado.repositorio()?;
    let m = cargar(&estado, &uid)?;
    let d = custody_ops::open_divergent_project(&repo, &estado.actor(), &m).map_err(err)?;
    Ok(ResultadoDerivacion {
        uid_derivado: d.derived_uid,
        id_derivado: d.derived_id,
        archivos: d.files_copied,
        origen_sellado: false,
        ruta: d.derived_root.display().to_string(),
    })
}

// --- Registro y conformidad de la implementación ---

#[derive(Serialize)]
pub struct EntradaRegistro {
    pub marca: String,
    pub actor: String,
    pub evento: String,
    pub proyecto: Option<String>,
    pub detalle: serde_json::Value,
}

#[tauri::command]
pub fn registro(estado: State<Estado>, ultimas: usize) -> R<Vec<EntradaRegistro>> {
    let repo = estado.repositorio()?;
    let mut entradas = repo.event_log().entries().map_err(err)?;
    if entradas.len() > ultimas {
        entradas = entradas.split_off(entradas.len() - ultimas);
    }
    Ok(entradas
        .into_iter()
        .map(|e| EntradaRegistro {
            marca: e.ts,
            actor: e.actor,
            evento: e.event,
            proyecto: e.project,
            detalle: e.detail,
        })
        .collect())
}

#[derive(Serialize)]
pub struct EstadoRegistro {
    pub total: usize,
    pub intacto: bool,
    pub sin_encadenar: Vec<usize>,
    pub alteradas: Vec<usize>,
}

#[tauri::command]
pub fn verificar_registro(estado: State<Estado>) -> R<EstadoRegistro> {
    let repo = estado.repositorio()?;
    let c = repo.event_log().verify_chain().map_err(err)?;
    Ok(EstadoRegistro {
        total: c.total,
        intacto: c.is_intact(),
        sin_encadenar: c.broken_links,
        alteradas: c.altered,
    })
}

#[tauri::command]
pub fn declaracion_conformidad() -> String {
    attacca_core::conformance::render()
}

// --- Auxiliares ---

fn cargar(estado: &State<Estado>, uid: &str) -> R<ProjectManifest> {
    let repo = estado.repositorio()?;
    let ruta = project::find_by_uid(&repo, uid).ok_or_else(|| {
        format!("No existe ningún proyecto con identificador interno {uid}. No se ha hecho nada.")
    })?;
    ProjectManifest::load(&ruta).map_err(err)
}

fn contexto(
    estado: &State<Estado>,
    emisor: &str,
    condiciones_aceptables: bool,
) -> ingest::ReceptionContext {
    let identidad = estado.identidad();
    ingest::ReceptionContext {
        recipient_org: identidad.organizacion,
        officer: identidad.persona,
        key_id: None,
        agreed_parties: vec![emisor.to_string()],
        declared_identities: vec![],
        revoked_identities: vec![],
        supported_versions: vec![attacca_core::STAVE_VERSION.to_string()],
        supported_profiles: vec![Profile::Delivery, Profile::Production, Profile::Archive],
        usage_acceptable: condiciones_aceptables,
        known_shipments: vec![],
        known_chronology: vec![],
    }
}
