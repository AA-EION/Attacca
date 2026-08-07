//! Línea de órdenes de Attacca.
//!
//! Ofrece el ciclo completo sin interfaz gráfica. Su existencia acredita dos
//! requisitos: que el material sigue siendo utilizable sin la aplicación de
//! escritorio, y que el repositorio puede operarse con herramientas de consola
//! (Anexo G y Anexo H).
//!
//! El texto que emite sigue las mismas reglas de redacción que la interfaz
//! gráfica: sin signos de exclamación, sin celebraciones, sin antropomorfismo,
//! y con los tres elementos en los mensajes de error.

use attacca_core::clock::SyncState;
use attacca_core::manifest::exchange::{Classification, Profile};
use attacca_core::manifest::project::{Level, ProjectManifest, Status};
use attacca_core::manifest::release::ReleaseClass;
use attacca_core::package::container::Progress;
use attacca_core::package::{custody_ops, emit, ingest};
use attacca_core::repo::Repository;
use attacca_core::validate::Severity;
use attacca_core::{
    clock, conformance, eventlog, index, integrity, journal, project, release, replica,
};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "attacca",
    version,
    about = "Gestion de un repositorio conforme a STAVE 2.0",
    long_about = None
)]
struct Cli {
    /// Raiz del repositorio. Por defecto, .stave en el directorio personal.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    /// Identidad que se registra como actor de los eventos.
    #[arg(long, global = true, default_value = "consola")]
    actor: String,

    /// Organizacion titular de la custodia.
    #[arg(long, global = true, default_value = "Organizacion")]
    org: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Crear la jerarquia raiz de un repositorio
    Init,
    /// Declaracion de conformidad de esta implementacion
    Conformidad,
    /// Listar los proyectos del repositorio
    Listar {
        /// Reconstruir el indice recorriendo el arbol, sin usar la cache
        #[arg(long)]
        reconstruir: bool,
    },
    /// Crear un proyecto
    Crear {
        titulo: String,
        #[arg(long)]
        artista: String,
        #[arg(long, default_value = "ORIG")]
        tipo: String,
        #[arg(long, default_value = "B")]
        nivel: String,
        #[arg(long, default_value_t = 48000)]
        frecuencia: i64,
        #[arg(long, default_value_t = 24)]
        bits: i64,
        #[arg(long)]
        release: Option<String>,
    },
    /// Cambiar el titulo de un proyecto
    Renombrar { uid: String, titulo: String },
    /// Derivar un proyecto conforme al apartado 14.4
    Derivar {
        uid: String,
        #[arg(long)]
        motivo: String,
        /// Continuar el trabajo sobre ambos proyectos en paralelo
        #[arg(long)]
        paralelo: bool,
    },
    /// Cambiar el estado de un proyecto
    Estado {
        uid: String,
        /// idea, active, onhold, delivered, sealed, archived
        estado: String,
    },
    /// Crear un release
    CrearRelease {
        titulo: String,
        #[arg(long)]
        artista: String,
        #[arg(long, default_value = "SINGLE")]
        clase: String,
    },
    /// Vincular un proyecto a un release
    Vincular {
        release_uid: String,
        proyecto_uid: String,
        #[arg(long)]
        posicion: Option<i64>,
    },
    /// Validar la conformidad de un proyecto
    Validar { uid: Option<String> },
    /// Generar el manifiesto de integridad de un proyecto
    Integridad {
        uid: String,
        /// Verificar en lugar de generar
        #[arg(long)]
        verificar: bool,
    },
    /// Emitir un paquete de intercambio
    Emitir {
        uid: String,
        #[arg(long, default_value = "E")]
        perfil: String,
        #[arg(long, default_value = "INTERNO")]
        clasificacion: String,
        #[arg(long)]
        destinatario: String,
        #[arg(long, default_value = "recepcion@ejemplo")]
        contacto: String,
        #[arg(long)]
        finalidad: String,
        #[arg(long, default_value = "2031-12-31")]
        retencion: String,
        #[arg(long, default_value = "intercambio@ejemplo")]
        acuse: String,
        /// El envio cede la custodia editorial
        #[arg(long)]
        ceder_custodia: bool,
        #[arg(long)]
        retorno_esperado: Option<String>,
        #[arg(long, default_value_t = 15)]
        gracia: i64,
        /// Entregar sin serializar, como directorio
        #[arg(long)]
        sin_serializar: bool,
        /// Declarar que el informe de control de calidad esta aprobado
        #[arg(long)]
        control_aprobado: bool,
    },
    /// Verificar un paquete recibido, sin ingerirlo
    Verificar {
        paquete: PathBuf,
        #[arg(long)]
        emisor: String,
        /// Salida del acuse de recibo
        #[arg(long)]
        acuse: Option<PathBuf>,
    },
    /// Ingerir un paquete ya verificado
    Ingerir {
        paquete: PathBuf,
        #[arg(long)]
        emisor: String,
        /// Proyecto de destino
        #[arg(long)]
        destino: Option<String>,
        /// Aceptar la custodia que el envio cede
        #[arg(long)]
        aceptar_custodia: bool,
    },
    /// Reclamar el retorno de una cesion vencida
    Reclamar { uid: String },
    /// Recuperar la custodia por vencimiento
    Recuperar { uid: String },
    /// Estado del registro de eventos
    Registro {
        /// Comprobar el encadenamiento por resumen
        #[arg(long)]
        verificar: bool,
        #[arg(long, default_value_t = 20)]
        ultimas: usize,
    },
    /// Detectar operaciones a medias tras un cierre inesperado
    Recuperacion {
        /// Suprimir los temporales abandonados
        #[arg(long)]
        limpiar: bool,
    },
    /// Comprobar la sincronizacion del reloj
    Reloj,
    /// Reconciliar una replica reconectada
    Reconciliar {
        activa: PathBuf,
        reconectada: PathBuf,
    },
}

fn main() -> ExitCode {
    clock::init_local_offset();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let mut err = std::io::stderr();
            let _ = writeln!(err, "{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> attacca_core::Result<()> {
    match &cli.command {
        Command::Conformidad => {
            print!("{}", conformance::render());
            return Ok(());
        }
        Command::Reloj => {
            let estado =
                clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(3));
            match estado {
                SyncState::Synced { drift_ms } => {
                    println!("Reloj sincronizado. Desviacion: {drift_ms} ms.");
                    println!("Marca actual: {}", clock::now_rfc3339());
                }
                SyncState::Drifted { drift_ms } => {
                    println!(
                        "El reloj presenta una desviacion de {} s respecto de la fuente de tiempo de red.",
                        drift_ms / 1000
                    );
                    println!("La emision de paquetes y de acuses queda bloqueada.");
                    println!("Sincronizar el reloj del sistema y repetir la comprobacion.");
                }
                SyncState::Unavailable => {
                    println!("No se pudo consultar ninguna fuente de tiempo de red.");
                    println!(
                        "Se puede trabajar; la emision de paquetes y de acuses queda bloqueada."
                    );
                    println!("Conectar el equipo a la red y repetir la comprobacion.");
                }
            }
            return Ok(());
        }
        Command::Init => {
            let raiz = repo_path(cli)?;
            let repo = Repository::create(&raiz)?;
            println!("Repositorio creado en {}", repo.root().display());
            if let Some(servicio) = Repository::warn_if_synced_location(repo.root()) {
                println!();
                println!("La raiz esta dentro de una carpeta sincronizada con {servicio}.");
                println!("El apartado 6.5 lo desaconseja: la sincronizacion automatica corrompe");
                println!("sesiones abiertas y es causa habitual de filtracion involuntaria.");
            }
            return Ok(());
        }
        _ => {}
    }

    let repo = Repository::open(repo_path(cli)?)?;

    match &cli.command {
        Command::Init | Command::Conformidad | Command::Reloj => unreachable!(),

        Command::Listar { reconstruir } => {
            let (idx, resultado) = if *reconstruir {
                (index::Index::rebuild(&repo)?, None)
            } else {
                let (i, r) = index::Index::load_or_rebuild(&repo, repo.root())?;
                (i, Some(r))
            };
            if idx.projects.is_empty() {
                println!("El repositorio no contiene ningun proyecto.");
            }
            for p in &idx.projects {
                println!(
                    "{}  {:<38}  etapa {:<22}  custodia {:<12}  nivel {}",
                    &p.uid[..8],
                    p.id,
                    p.stage,
                    p.custody,
                    p.level
                );
            }
            if !idx.releases.is_empty() {
                println!();
                for r in &idx.releases {
                    println!(
                        "{}  {:<38}  {:<7}  {} temas",
                        &r.uid[..8],
                        r.id,
                        r.class,
                        r.track_count
                    );
                }
            }
            if !idx.unreadable.is_empty() {
                println!();
                println!(
                    "{} manifiestos no se pudieron interpretar:",
                    idx.unreadable.len()
                );
                for (ruta, motivo) in &idx.unreadable {
                    println!("  {}: {motivo}", ruta.display());
                }
            }
            if let Some(r) = resultado {
                println!();
                println!(
                    "{} proyectos. Indice {} en {} ms.",
                    idx.projects.len(),
                    if r.rebuilt {
                        "reconstruido desde el arbol"
                    } else {
                        "leido de la cache"
                    },
                    r.elapsed_ms
                );
            }
        }

        Command::Crear {
            titulo,
            artista,
            tipo,
            nivel,
            frecuencia,
            bits,
            release: release_uid,
        } => {
            let m = project::create(
                &repo,
                &cli.actor,
                &project::NewProject {
                    title: titulo.clone(),
                    artist: artista.clone(),
                    kind: tipo.clone(),
                    level: parse_level(nivel)?,
                    release_uid: release_uid.clone(),
                    release_dir: None,
                    sample_rate: *frecuencia,
                    bit_depth: *bits,
                    holder_org: cli.org.clone(),
                    holder_person: cli.actor.clone(),
                    active_volume: None,
                },
            )?;
            println!("{}", m.id().unwrap_or_default());
            println!("{}", m.root().display());
            println!();
            println!(
                "Frecuencia de muestreo {} Hz y profundidad de {} bits.",
                m.sample_rate().unwrap_or(0),
                m.bit_depth().unwrap_or(0)
            );
            println!(
                "Ambas quedan fijadas para todo el ciclo de vida del proyecto (apartado 10.1)."
            );
        }

        Command::Renombrar { uid, titulo } => {
            let mut m = cargar_proyecto(&repo, uid)?;
            let anterior = m.id().unwrap_or_default().to_string();
            let destino = project::rename(&repo, &cli.actor, &mut m, titulo)?;
            println!("{anterior}");
            println!("{}", m.id().unwrap_or_default());
            println!("{}", destino.display());
            println!();
            println!("El identificador interno no ha cambiado. Ninguna referencia se ha roto.");
        }

        Command::Derivar {
            uid,
            motivo,
            paralelo,
        } => {
            let mut m = cargar_proyecto(&repo, uid)?;
            let d = project::derive(&repo, &cli.actor, &mut m, motivo, *paralelo)?;
            println!("{}", d.derived_id);
            println!("{}", d.derived_root.display());
            println!();
            println!("{} archivos copiados.", d.files_copied);
            if d.source_sealed {
                println!("El proyecto de origen paso a estado sellado y quedo en solo lectura.");
            } else {
                println!("Ambos proyectos siguen activos y su trabajo en paralelo consta en los dos manifiestos.");
            }
        }

        Command::Estado { uid, estado } => {
            let mut m = cargar_proyecto(&repo, uid)?;
            let nuevo = Status::parse(estado).ok_or_else(|| {
                attacca_core::Error::input(format!(
                    "El estado «{estado}» no es admisible. El proyecto no se ha modificado. Los estados son idea, active, onhold, delivered, sealed y archived."
                ))
            })?;
            let destino = project::set_status(&repo, &cli.actor, &mut m, nuevo)?;
            println!("{}", destino.display());
        }

        Command::CrearRelease {
            titulo,
            artista,
            clase,
        } => {
            let clase = ReleaseClass::parse(clase).ok_or_else(|| {
                attacca_core::Error::requirement(
                    "8.2",
                    format!("La clase «{clase}» no figura en la Tabla 8. El release no se ha creado. Las clases son SINGLE, EP, ALBUM, COMP, LIVE y SYNC."),
                )
            })?;
            let m = release::create(
                &repo,
                &cli.actor,
                &release::NewRelease {
                    title: titulo.clone(),
                    artist: artista.clone(),
                    class: clase,
                    level: Level::B,
                    holder_org: cli.org.clone(),
                    holder_person: cli.actor.clone(),
                },
            )?;
            println!("{}", m.uid().unwrap_or_default());
            println!("{}", m.root().display());
        }

        Command::Vincular {
            release_uid,
            proyecto_uid,
            posicion,
        } => {
            let ruta = release::find_by_uid(&repo, release_uid).ok_or_else(|| {
                attacca_core::Error::input(format!("No existe ningun release con identificador {release_uid}. No se ha vinculado nada."))
            })?;
            let mut r = attacca_core::manifest::release::ReleaseManifest::load(&ruta)?;
            let mut p = cargar_proyecto(&repo, proyecto_uid)?;
            let pos = release::link_project(&repo, &cli.actor, &mut r, &mut p, *posicion)?;
            println!("Posicion {pos} del tracklist.");
        }

        Command::Validar { uid } => {
            let uids: Vec<String> = match uid {
                Some(u) => vec![u.clone()],
                None => index::Index::rebuild(&repo)?
                    .projects
                    .iter()
                    .map(|p| p.uid.clone())
                    .collect(),
            };
            let mut conformes = 0usize;
            for u in &uids {
                let m = cargar_proyecto(&repo, u)?;
                let informe = attacca_core::validate::project(&m);
                let incumplimientos = informe.breaches().len();
                let pendientes = informe.pending().len();
                let excepciones = informe.exceptions().len();
                if informe.is_conformant() {
                    conformes += 1;
                }
                println!(
                    "{:<38}  {} incumplimientos, {} pendientes, {} excepciones",
                    m.id().unwrap_or_default(),
                    incumplimientos,
                    pendientes,
                    excepciones
                );
                for f in informe
                    .findings
                    .iter()
                    .filter(|f| f.severity == Severity::Breach)
                {
                    println!("    apartado {:<8} {}", f.clause, f.detail);
                }
                if uids.len() == 1 {
                    for f in informe
                        .findings
                        .iter()
                        .filter(|f| f.severity == Severity::Pending)
                    {
                        println!("    pendiente  {:<8} {}", f.clause, f.detail);
                    }
                }
            }
            println!();
            println!("{conformes} de {} proyectos conformes.", uids.len());
        }

        Command::Integridad { uid, verificar } => {
            let m = cargar_proyecto(&repo, uid)?;
            if *verificar {
                let v = integrity::verify_tree(m.root())?;
                if v.is_ok() {
                    println!("{} archivos verificados. Todos coinciden.", v.checked);
                } else {
                    println!(
                        "La verificacion de integridad fallo en {} de {} archivos.",
                        v.failed_count(),
                        v.checked
                    );
                    for p in v.failed_paths() {
                        println!("    {p}");
                    }
                }
            } else {
                let man = integrity::generate(m.root())?;
                println!("{} archivos.", man.len());
                println!("{}", m.root().join("MANIFEST.sha256").display());
            }
        }

        Command::Emitir {
            uid,
            perfil,
            clasificacion,
            destinatario,
            contacto,
            finalidad,
            retencion,
            acuse,
            ceder_custodia,
            retorno_esperado,
            gracia,
            sin_serializar,
            control_aprobado,
        } => {
            let mut m = cargar_proyecto(&repo, uid)?;
            let perfil = Profile::parse(perfil).ok_or_else(|| {
                attacca_core::Error::requirement("31.3", "El perfil declarado no es admisible. El envio no se ha emitido. Los perfiles son E, P y A.")
            })?;
            let clasificacion = Classification::parse(clasificacion).ok_or_else(|| {
                attacca_core::Error::requirement("34.2", "El nivel de clasificacion no es admisible. El envio no se ha emitido. Los niveles son PUBLICO, INTERNO, CONFIDENCIAL y RESTRINGIDO.")
            })?;

            let envio = emit::Shipment {
                profile: perfil,
                classification: clasificacion,
                purpose: finalidad.clone(),
                issuer_org: cli.org.clone(),
                issuer_contact: acuse.clone(),
                issuer_key_id: None,
                recipient_org: destinatario.clone(),
                recipient_contact: contacto.clone(),
                usage_permitted: vec![finalidad.clone()],
                usage_territory: "mundial".into(),
                usage_term: "el declarado en el acuerdo de intercambio".into(),
                sublicensing: false,
                forwarding: false,
                retention_until: retencion.clone(),
                destroy_on_expiry: true,
                personal_data: false,
                personal_data_categories: vec![],
                ack_deadline_hours: 72,
                ack_address: acuse.clone(),
                transfers_custody: *ceder_custodia,
                returns_custody: false,
                supersedes_transfer: None,
                expected_return: retorno_esperado.clone(),
                grace_days: *gracia,
                onward_allowed: false,
                supersedes: None,
                revision: "r0".into(),
                serialize: !*sin_serializar,
                qc_approved: *control_aprobado,
            };

            let estado =
                clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(3));
            let mut avance = barra_de_progreso();
            let cancelado = || false;
            let mut progreso = Progress {
                on_progress: &mut avance,
                cancelled: &cancelado,
            };

            let e = emit::emit(
                &repo,
                &cli.actor,
                &mut m,
                &envio,
                &emit::Payload::for_profile(perfil),
                estado,
                &mut progreso,
            )?;
            println!();
            println!("{}", e.shipment_id);
            println!("{}", e.artifact.display());
            println!();
            println!("{} archivos, {} bytes.", e.file_count, e.total_bytes);
            println!("Copia congelada en {}", e.frozen_copy.display());
            if *ceder_custodia {
                println!();
                println!("Estado de custodia: {}.", e.custody_state.as_str());
                println!(
                    "La copia local queda en solo lectura hasta que se obtenga el acuse de recibo."
                );
            }
        }

        Command::Verificar {
            paquete,
            emisor,
            acuse,
        } => {
            let ctx = contexto_recepcion(cli, emisor);
            let mut avance = barra_de_progreso();
            let cancelado = || false;
            let mut progreso = Progress {
                on_progress: &mut avance,
                cancelled: &cancelado,
            };
            let v = ingest::verify(&repo, &cli.actor, paquete, &ctx, &mut progreso)?;
            println!();
            imprimir_verificacion(&v);

            if let Some(salida) = acuse {
                let estado =
                    clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(3));
                ingest::issue_receipt(
                    &repo, &cli.actor, &v, &ctx, "01_REF", None, false, None, estado, salida,
                )?;
                println!();
                println!("Acuse de recibo en {}", salida.display());
            }
        }

        Command::Ingerir {
            paquete,
            emisor,
            destino,
            aceptar_custodia,
        } => {
            let ctx = contexto_recepcion(cli, emisor);
            let mut avance = barra_de_progreso();
            let cancelado = || false;
            let mut progreso = Progress {
                on_progress: &mut avance,
                cancelled: &cancelado,
            };
            let v = ingest::verify(&repo, &cli.actor, paquete, &ctx, &mut progreso)?;
            println!();
            imprimir_verificacion(&v);
            if !v.accepted() {
                println!();
                println!("El envio no se ha ingerido.");
                return Ok(());
            }

            let mut objetivo = match destino {
                Some(u) => Some(cargar_proyecto(&repo, u)?),
                None => None,
            };
            let estado =
                clock::check_sync(clock::DEFAULT_SOURCES, std::time::Duration::from_secs(3));
            ingest::issue_receipt(
                &repo,
                &cli.actor,
                &v,
                &ctx,
                "01_REF",
                None,
                *aceptar_custodia,
                None,
                estado,
                &repo
                    .root()
                    .join(format!("00_SYSTEM/acuse_{}.yaml", v.shipment_id)),
            )?;
            let ing = ingest::ingest(&repo, &cli.actor, &v, &ctx, objetivo.as_mut(), paquete)?;
            println!();
            println!(
                "{} archivos incorporados en {}",
                ing.files,
                ing.destination.display()
            );
            println!("Clase de destino: {}.", ing.destination_class);
            if ing.custody_assumed {
                println!("Custodia asumida. El estado del proyecto de destino es propia.");
            }
        }

        Command::Reclamar { uid } => {
            let m = cargar_proyecto(&repo, uid)?;
            let situacion = custody_ops::claim_return(&repo, &cli.actor, &m)?;
            println!("Reclamacion registrada.");
            match situacion {
                attacca_core::custody::Expiry::ReclaimAvailable => {
                    println!("El plazo de gracia tambien vencio. Procede la recuperacion forzosa.");
                }
                _ => println!("El plazo de gracia sigue en curso."),
            }
        }

        Command::Recuperar { uid } => {
            let mut m = cargar_proyecto(&repo, uid)?;
            println!("La recuperacion forzosa es irreversible en la practica:");
            println!("todo retorno posterior sera una version divergente que habra que");
            println!("reconciliar a mano.");
            println!();
            custody_ops::reclaim(&repo, &cli.actor, &mut m, &cli.org)?;
            println!("Estado de custodia: {}.", m.custody_state().as_str());
            println!("Acceso de escritura restituido. Se abrio una no conformidad mayor.");
        }

        Command::Registro { verificar, ultimas } => {
            let log = repo.event_log();
            let entradas = log.entries()?;
            if *verificar {
                let c = log.verify_chain()?;
                if c.is_intact() {
                    println!(
                        "{} entradas. El encadenamiento por resumen esta intacto.",
                        c.total
                    );
                } else {
                    println!("{} entradas.", c.total);
                    if !c.broken_links.is_empty() {
                        println!(
                            "{} entradas no encadenan con la anterior: lineas {}.",
                            c.broken_links.len(),
                            numeros(&c.broken_links)
                        );
                        println!("Indica supresion o reordenacion de entradas intermedias.");
                    }
                    if !c.altered.is_empty() {
                        println!(
                            "{} entradas no corresponden a su propio resumen: lineas {}.",
                            c.altered.len(),
                            numeros(&c.altered)
                        );
                    }
                }
                return Ok(());
            }
            for e in entradas
                .iter()
                .rev()
                .take(*ultimas)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                println!(
                    "{}  {:<34}  {:<10}  {}",
                    e.ts,
                    e.event,
                    e.actor,
                    e.project.as_deref().unwrap_or("")
                );
            }
        }

        Command::Recuperacion { limpiar } => {
            let r = journal::detect(repo.root(), repo.root())?;
            if r.is_clean() {
                println!("No hay operaciones a medias.");
                return Ok(());
            }
            for op in &r.incomplete {
                println!("{}  {}  iniciada {}", op.id, op.kind, op.started);
                for a in &op.artifacts {
                    println!("    {}", a.display());
                }
            }
            if !r.abandoned_temps.is_empty() {
                println!(
                    "{} archivos temporales de escritura atomica quedaron abandonados.",
                    r.abandoned_temps.len()
                );
                for p in &r.abandoned_temps {
                    println!("    {}", p.display());
                }
                if *limpiar {
                    let n = journal::clear_abandoned_temps(&r);
                    println!(
                        "{n} suprimidos. Los archivos de destino conservan su contenido anterior."
                    );
                }
            }
        }

        Command::Reconciliar {
            activa,
            reconectada,
        } => {
            match replica::reconcile(activa, reconectada)? {
                replica::Reconciliation::Synchronizable => {
                    println!("La replica reconectada no contiene cambios posteriores.");
                    println!("Puede pasar a estado en_espera y sincronizarse desde la activa.");
                }
                replica::Reconciliation::Divergent { changed } => {
                    println!(
                        "La replica reconectada contiene {} archivos con cambios no presentes en la activa.",
                        changed.len()
                    );
                    println!("La replica queda marcada divergente y no se sobrescribe de forma automatica.");
                    println!("La reconciliacion es manual y su resultado debe registrarse.");
                    println!();
                    for c in &changed {
                        println!("    {c}");
                    }
                    repo.event_log().append(
                        &cli.actor,
                        eventlog::event::REPLICA_DIVERGENCE_DETECTED,
                        None,
                        serde_json::json!({
                            "reconnected": reconectada.display().to_string(),
                            "changed": changed.len(),
                        }),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn repo_path(cli: &Cli) -> attacca_core::Result<PathBuf> {
    if let Some(p) = &cli.repo {
        return Ok(p.clone());
    }
    Repository::suggested_local_root().ok_or_else(|| {
        attacca_core::Error::input(
            "No se pudo determinar el directorio personal. No se ha abierto ningun repositorio. Indicar la raiz con --repo.",
        )
    })
}

fn cargar_proyecto(repo: &Repository, uid: &str) -> attacca_core::Result<ProjectManifest> {
    // Se admite un prefijo del identificador interno, como en las referencias
    // abreviadas que muestra `listar`.
    let candidatos: Vec<PathBuf> = repo
        .discover_projects()
        .into_iter()
        .filter(|p| {
            ProjectManifest::load(p)
                .ok()
                .and_then(|m| m.uid().map(|u| u.starts_with(uid)))
                .unwrap_or(false)
        })
        .collect();

    match candidatos.len() {
        1 => ProjectManifest::load(&candidatos[0]),
        0 => Err(attacca_core::Error::input(format!(
            "No existe ningun proyecto cuyo identificador interno empiece por {uid}. No se ha hecho nada. Consultar la lista con «attacca listar»."
        ))),
        n => Err(attacca_core::Error::input(format!(
            "{n} proyectos tienen un identificador interno que empieza por {uid}. No se ha hecho nada. Indicar mas caracteres."
        ))),
    }
}

fn parse_level(s: &str) -> attacca_core::Result<Level> {
    Level::parse(s).ok_or_else(|| {
        attacca_core::Error::requirement(
            "5.1",
            format!("El nivel «{s}» no figura en la Tabla 4. El proyecto no se ha creado. Los niveles son A, B y C."),
        )
    })
}

fn contexto_recepcion(cli: &Cli, emisor: &str) -> ingest::ReceptionContext {
    ingest::ReceptionContext {
        recipient_org: cli.org.clone(),
        officer: cli.actor.clone(),
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

fn imprimir_verificacion(v: &ingest::Verification) {
    println!("Envio {}", v.shipment_id);
    println!(
        "Resultado: {}",
        match v.result {
            attacca_core::manifest::receipt::ReceiptResult::Accepted => "aceptado",
            attacca_core::manifest::receipt::ReceiptResult::AcceptedWithReservations =>
                "aceptado con reservas",
            attacca_core::manifest::receipt::ReceiptResult::Rejected => "rechazado",
        }
    );
    if let Some(fallo) = v.checks.first_failure() {
        println!("Primera verificacion fallida: {fallo}.");
    }
    for (comprobacion, detalle) in &v.discrepancies {
        println!("    {comprobacion}: {detalle}");
    }
}

fn barra_de_progreso() -> impl FnMut(usize, usize) {
    // Progreso visible en operaciones largas, sin animacion que retrase nada.
    let mut ultimo = 0usize;
    move |hecho, total| {
        if total == 0 {
            return;
        }
        let porcentaje = hecho * 100 / total;
        if porcentaje >= ultimo + 10 || hecho == total {
            ultimo = porcentaje;
            print!("\r{hecho} de {total} archivos");
            let _ = std::io::stdout().flush();
        }
    }
}

fn numeros(v: &[usize]) -> String {
    v.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
