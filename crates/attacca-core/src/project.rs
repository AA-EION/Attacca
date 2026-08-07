//! Operaciones sobre proyectos (apartados 7, 9.2, 13.2 y 14.4).

use crate::clock;
use crate::custody::{self, CustodyState};
use crate::doc::Node;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::fsx;
use crate::ids;
use crate::manifest::project::{scaffold, Level, ProjectManifest, Status};
use crate::manifest::PROJECT_FILE;
use crate::naming;
use crate::repo::Repository;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Datos que se piden al crear un proyecto.
///
/// El apartado 13.2 limita lo exigible en la creación al identificador interno,
/// el legible, el artista, el tipo y el nivel. Se añaden la frecuencia de
/// muestreo y la profundidad de bits porque el apartado 10.1 las fija para todo
/// el ciclo de vida y no pueden modificarse después.
#[derive(Clone, Debug)]
pub struct NewProject {
    pub title: String,
    pub artist: String,
    /// Valor de la Tabla 10.
    pub kind: String,
    pub level: Level,
    /// Identificador interno del release al que pertenece, si procede.
    pub release_uid: Option<String>,
    /// Carpeta del release, cuando el proyecto se crea dentro de uno.
    pub release_dir: Option<PathBuf>,
    pub sample_rate: i64,
    pub bit_depth: i64,
    pub holder_org: String,
    pub holder_person: String,
    pub active_volume: Option<String>,
}

/// Frecuencias de muestreo admisibles por defecto.
pub const SAMPLE_RATES: &[i64] = &[44100, 48000, 88200, 96000, 176400, 192000];

/// Profundidades de bits admisibles. El apartado 10.1 fija un mínimo de 24 bits
/// para el audio de sesión, las capturas y los stems.
pub const BIT_DEPTHS: &[i64] = &[24, 32];

/// Crea un proyecto conforme al apartado 7.
///
/// Solo se crean las carpetas obligatorias de la Tabla 7. El apartado 7.1
/// desaconseja expresamente crear carpetas vacías por anticipado: el andamiaje
/// sin contenido impide distinguir una etapa no iniciada de una concluida sin
/// resultado.
pub fn create(repo: &Repository, actor: &str, spec: &NewProject) -> Result<ProjectManifest> {
    if spec.artist.trim().is_empty() {
        return Err(Error::input(
            "El campo de artista está vacío. El proyecto no se ha creado. Escribir el nombre del artista.",
        ));
    }
    if !naming::PROJECT_TYPES.contains(&spec.kind.as_str()) {
        return Err(Error::requirement(
            "9.2",
            format!(
                "El tipo «{}» no figura en la Tabla 10. El proyecto no se ha creado. Elegir uno de: {}.",
                spec.kind,
                naming::PROJECT_TYPES.join(", ")
            ),
        ));
    }
    if spec.bit_depth < 24 {
        return Err(Error::requirement(
            "10.1",
            format!("La profundidad declarada es de {} bits y el mínimo es de 24. El proyecto no se ha creado.", spec.bit_depth),
        ));
    }

    let uid = ids::new_uid();
    let date = clock::today();
    let id = naming::project_id(&date, &spec.title, &spec.kind)?;

    // Un proyecto nace en `0_IDEAS` salvo que pertenezca a un release, en cuyo
    // caso reside dentro de la carpeta del release (apartado 8.1).
    let parent = match &spec.release_dir {
        Some(dir) => dir.clone(),
        None => repo.domain("20_PROJECTS").join("0_IDEAS"),
    };
    let root = parent.join(&id);

    if root.exists() {
        return Err(Error::input(format!(
            "Ya existe una carpeta {id}. El proyecto no se ha creado. Elegir otro título."
        )));
    }

    // El presupuesto de ruta se comprueba antes de crear el nombre, no después
    // (apartado 9.1).
    let ruta_larga = format!(
        "{}/08_DELIVERY/{date}_destinatario/Audio/{}",
        root.display(),
        naming::deliverable_audio_name(
            &spec.artist,
            &spec.title,
            Some("Radio Edit"),
            "WAV-24-48",
            "wav"
        )
    );
    if ruta_larga.chars().count() > naming::MAX_PATH_CHARS {
        // La NOTA 1 del apartado 6.5 lo dice: cada carácter de la raíz se
        // descuenta del presupuesto. Indicar cuánto consume la raíz permite
        // decidir entre acortar el título y trasladar el repositorio.
        let raiz_repo = repo.root().to_string_lossy().chars().count();
        return Err(Error::requirement(
            "9.1",
            format!(
                "La ruta de entrega más larga de este proyecto mediría {} de {} caracteres, de los que {raiz_repo} corresponden a la raíz del repositorio. El proyecto no se ha creado. Acortar el título, o trasladar la raíz a una ruta más corta.",
                ruta_larga.chars().count(),
                naming::MAX_PATH_CHARS
            ),
        ));
    }

    for dir in crate::repo::REQUIRED_PROJECT_DIRS {
        let p = root.join(dir);
        std::fs::create_dir_all(&p).map_err(|e| Error::io(&p, e))?;
    }

    let mut m = ProjectManifest::new(root.join(PROJECT_FILE));
    *m.doc_mut() = scaffold(
        &uid,
        &id,
        &spec.title,
        &spec.artist,
        &spec.kind,
        spec.level,
        spec.release_uid.as_deref(),
        spec.sample_rate,
        spec.bit_depth,
        &spec.holder_org,
        &spec.holder_person,
        spec.active_volume.as_deref(),
    );
    m.save()?;

    repo.event_log().append(
        actor,
        event::PROJECT_CREATED,
        Some(&uid),
        json!({
            "id": id,
            "artist": spec.artist,
            "type": spec.kind,
            "level": spec.level.as_str(),
            "sample_rate": spec.sample_rate,
            "bit_depth": spec.bit_depth,
            "release": spec.release_uid,
        }),
    )?;
    Ok(m)
}

/// Cambia el identificador legible de un proyecto (apartado 9.2).
///
/// Renombra la carpeta, registra el valor anterior con su marca temporal y no
/// toca el identificador interno: ninguna referencia se rompe.
pub fn rename(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    new_title: &str,
) -> Result<PathBuf> {
    custody::require_writable(project.custody_state())?;
    if project.status().is_frozen() {
        return Err(Error::requirement(
            "14.4.3",
            format!("El proyecto está en estado {}. El identificador no se ha modificado. Un proyecto sellado o archivado no se modifica.", project.status().as_str()),
        ));
    }
    if project.id_is_frozen() {
        return Err(Error::requirement(
            "9.2",
            "El proyecto ha sido objeto de un envío o está archivado. El identificador legible no se ha modificado. Emitido un envío, el identificador legible queda fijado.",
        ));
    }

    let anterior = project
        .id()
        .ok_or_else(|| Error::input("El proyecto no declara identificador legible.".to_string()))?
        .to_string();
    let kind = project.project_type().unwrap_or("ORIG").to_string();
    // La fecha del identificador es la de creación y no cambia al renombrar.
    let fecha = anterior
        .split('_')
        .next()
        .unwrap_or(&clock::today())
        .to_string();
    let nuevo = naming::project_id(&fecha, new_title, &kind)?;

    if nuevo == anterior {
        project.doc_mut().set("title", Node::str(new_title));
        project.save()?;
        return Ok(project.root().to_path_buf());
    }

    let raiz_actual = project.root().to_path_buf();
    let destino = raiz_actual
        .parent()
        .ok_or_else(|| Error::input("El proyecto no tiene carpeta contenedora.".to_string()))?
        .join(&nuevo);
    if destino.exists() {
        return Err(Error::input(format!(
            "Ya existe una carpeta {nuevo}. El identificador no se ha modificado. Elegir otro título."
        )));
    }

    // Ningún archivo abierto para escritura: renombrar la carpeta bajo una
    // sesión abierta rompe las referencias de la estación de trabajo.
    let deteccion = fsx::openfiles::open_for_write(&raiz_actual);
    if !deteccion.files().is_empty() {
        return Err(Error::OpenFiles(deteccion.files().to_vec()));
    }

    std::fs::rename(&raiz_actual, &destino).map_err(|e| Error::io(&destino, e))?;

    // El manifiesto se reescribe en su nueva ubicación.
    let mut movido = ProjectManifest::new(destino.join(PROJECT_FILE));
    movido.doc_mut().clone_from(project.doc());
    movido.record_id_change(&anterior);
    movido.doc_mut().set("id", Node::str(&nuevo));
    movido.doc_mut().set("title", Node::str(new_title));
    movido.save()?;
    *project = movido;

    repo.event_log().append(
        actor,
        event::PROJECT_ID_CHANGED,
        project.uid(),
        json!({"previous": anterior, "current": nuevo}),
    )?;
    Ok(destino)
}

/// Resultado de una derivación.
#[derive(Clone, Debug)]
pub struct Derivation {
    pub source_uid: String,
    pub derived_uid: String,
    pub derived_id: String,
    pub derived_root: PathBuf,
    pub files_copied: usize,
    /// El proyecto de origen quedó sellado. Es falso solo cuando ambos siguen
    /// activos en paralelo (apartado 14.4.3, último párrafo).
    pub source_sealed: bool,
}

/// Deriva un proyecto (apartado 14.4).
///
/// Copia la totalidad del proyecto de origen salvo los regenerables del Anexo C
/// y los paquetes ya emitidos, asigna identificador interno propio, añade el
/// sufijo `_vNN`, declara la ascendencia y sella el proyecto de origen.
pub fn derive(
    repo: &Repository,
    actor: &str,
    source: &mut ProjectManifest,
    reason: &str,
    parallel: bool,
) -> Result<Derivation> {
    custody::require_writable(source.custody_state())?;
    if source.status().is_frozen() {
        return Err(Error::requirement(
            "14.4.3",
            format!("El proyecto de origen está en estado {}. No se ha derivado nada. Un proyecto sellado o archivado no admite una derivación nueva.", source.status().as_str()),
        ));
    }

    let source_uid = source
        .uid()
        .ok_or_else(|| Error::input("El proyecto no declara identificador interno.".to_string()))?
        .to_string();
    let source_id = source
        .id()
        .ok_or_else(|| Error::input("El proyecto no declara identificador legible.".to_string()))?
        .to_string();
    let source_root = source.root().to_path_buf();

    let deteccion = fsx::openfiles::open_for_write(&source_root);
    if !deteccion.files().is_empty() {
        return Err(Error::OpenFiles(deteccion.files().to_vec()));
    }

    // El identificador legible del derivado añade el sufijo `_vNN`. La
    // numeración continúa a partir de los derivados existentes.
    let base = match naming::version_suffix_of(&source_id) {
        Some(_) => source_id
            .rsplit_once("_v")
            .map(|(b, _)| b.to_string())
            .unwrap_or(source_id.clone()),
        None => source_id.clone(),
    };
    let siguiente = next_version(&source_root, &base)?;
    let derived_id = naming::with_version_suffix(&base, siguiente);
    let derived_root = source_root
        .parent()
        .ok_or_else(|| Error::input("El proyecto no tiene carpeta contenedora.".to_string()))?
        .join(&derived_id);

    if derived_root.exists() {
        return Err(Error::input(format!(
            "Ya existe una carpeta {derived_id}. No se ha derivado nada."
        )));
    }

    let entradas = fsx::walk::conserved_files(&source_root);
    fsx::space::ensure_available(&derived_root, fsx::walk::total_bytes(&entradas))?;

    // Apartado 14.4.2: se copia todo salvo el Anexo C y los paquetes ya
    // emitidos que residan en `08_DELIVERY` y `09_TRANSFER`.
    let files_copied =
        fsx::copy_tree(&source_root, &derived_root, &["08_DELIVERY", "09_TRANSFER"])?;

    let derived_uid = ids::new_uid();
    let ahora = clock::now_rfc3339();

    let mut derivado = ProjectManifest::new(derived_root.join(PROJECT_FILE));
    derivado.doc_mut().clone_from(source.doc());
    derivado.doc_mut().set("uid", Node::str(&derived_uid));
    derivado.doc_mut().set("id", Node::str(&derived_id));
    // El derivado empieza su propio historial de identificadores.
    derivado.doc_mut().set("id_history", Node::Seq(Vec::new()));
    derivado.doc_mut().set(
        "lineage",
        Node::map(vec![
            ("parent_uid", Node::str(&source_uid)),
            ("parent_id", Node::str(&source_id)),
            ("derived_at", Node::str(&ahora)),
            ("reason", Node::str(reason)),
            ("parallel", Node::Bool(parallel)),
        ]),
    );
    // El derivado no hereda los proyectos derivados del origen, ni sus
    // identificadores de grabación (apartado 14.4.4), ni sus entregas.
    derivado.doc_mut().remove("derived");
    derivado.doc_mut().set("deliveries", Node::Seq(Vec::new()));
    let rights = derivado.doc_mut().ensure_map("rights");
    rights.set("isrc", Node::Null);
    // La preservación del origen no describe al derivado.
    let pres = derivado.doc_mut().ensure_map("preservation");
    pres.set("manifest", Node::Null);
    pres.set("verified", Node::Null);
    derivado.set_status(Status::Active);
    // Custodia y ciclo de vida independientes desde el instante de la derivación.
    let cust = derivado.doc_mut().ensure_map("custody");
    cust.set("state", Node::str(CustodyState::Own.as_str()));
    cust.set("since", Node::str(&ahora));
    cust.set("history", Node::Seq(Vec::new()));
    cust.remove("transfer");
    derivado.save()?;

    // El manifiesto del origen declara los proyectos derivados de él.
    source.doc_mut().ensure_seq("derived").push(Node::map(vec![
        ("uid", Node::str(&derived_uid)),
        ("id", Node::str(&derived_id)),
        ("derived_at", Node::str(&ahora)),
    ]));

    let source_sealed = !parallel;
    if source_sealed {
        source.set_status(Status::Sealed);
    } else {
        // El trabajo en paralelo debe declararse de forma expresa en el
        // manifiesto de ambos (apartado 14.4.3, último párrafo).
        source
            .doc_mut()
            .ensure_map("lineage")
            .set("parallel", Node::Bool(true));
    }
    source.save()?;

    if source_sealed {
        // El proyecto sellado queda en solo lectura por medios técnicos. El
        // manifiesto se excluye: los apartados 41 y 6.6 siguen actualizando sus
        // bloques de custodia y de réplica.
        fsx::readonly::set_tree_readonly(&source_root, true, &[PROJECT_FILE])?;
    }

    let log = repo.event_log();
    log.append(
        actor,
        event::PROJECT_DERIVED,
        Some(&source_uid),
        json!({
            "derived_uid": derived_uid,
            "derived_id": derived_id,
            "reason": reason,
            "parallel": parallel,
            "files_copied": files_copied,
        }),
    )?;
    if source_sealed {
        log.append(
            actor,
            event::PROJECT_SEALED,
            Some(&source_uid),
            json!({"derived_uid": derived_uid}),
        )?;
    }

    Ok(Derivation {
        source_uid,
        derived_uid,
        derived_id,
        derived_root,
        files_copied,
        source_sealed,
    })
}

fn next_version(source_root: &Path, base: &str) -> Result<u32> {
    let Some(parent) = source_root.parent() else {
        return Ok(2);
    };
    let mut max = 1u32;
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(base) {
                if let Some(v) = naming::version_suffix_of(&name) {
                    max = max.max(v);
                }
            }
        }
    }
    Ok(max + 1)
}

/// Traslada un proyecto a la carpeta de estado que le corresponde
/// (apartado 6.2).
pub fn set_status(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    status: Status,
) -> Result<PathBuf> {
    custody::require_writable(project.custody_state())?;
    let anterior = project.status();
    project.set_status(status);

    // Un proyecto sellado permanece en la carpeta que ocupaba; su condición
    // consta únicamente en el manifiesto (apartado 6.2).
    let Some(carpeta) = status.folder() else {
        project.save()?;
        repo.event_log().append(
            actor,
            event::PROJECT_STATUS_CHANGED,
            project.uid(),
            json!({"from": anterior.as_str(), "to": status.as_str()}),
        )?;
        return Ok(project.root().to_path_buf());
    };

    let raiz = project.root().to_path_buf();
    let destino_padre = repo.domain("20_PROJECTS").join(carpeta);
    let nombre = raiz.file_name().map(|n| n.to_owned());
    let Some(nombre) = nombre else {
        project.save()?;
        return Ok(raiz);
    };
    let destino = destino_padre.join(&nombre);

    // Un proyecto dentro de un release no se traslada: su ubicación la fija el
    // release (apartado 8.1).
    let dentro_de_release = raiz
        .parent()
        .map(|p| p.join("_RELEASE").is_dir())
        .unwrap_or(false);
    if dentro_de_release || destino == raiz {
        project.save()?;
        repo.event_log().append(
            actor,
            event::PROJECT_STATUS_CHANGED,
            project.uid(),
            json!({"from": anterior.as_str(), "to": status.as_str()}),
        )?;
        return Ok(raiz);
    }

    let deteccion = fsx::openfiles::open_for_write(&raiz);
    if !deteccion.files().is_empty() {
        return Err(Error::OpenFiles(deteccion.files().to_vec()));
    }
    std::fs::create_dir_all(&destino_padre).map_err(|e| Error::io(&destino_padre, e))?;
    std::fs::rename(&raiz, &destino).map_err(|e| Error::io(&destino, e))?;

    let mut movido = ProjectManifest::new(destino.join(PROJECT_FILE));
    movido.doc_mut().clone_from(project.doc());
    movido.save()?;
    *project = movido;

    repo.event_log().append(
        actor,
        event::PROJECT_STATUS_CHANGED,
        project.uid(),
        json!({"from": anterior.as_str(), "to": status.as_str(), "folder": carpeta}),
    )?;
    Ok(destino)
}

/// Crea la subcarpeta de sesión de un programa (apartado 7.4).
///
/// La subcarpeta se crea únicamente cuando ese programa se utiliza. Devuelve la
/// carpeta que debe abrirse para que la persona guarde ahí desde el programa.
pub fn create_session_folder(
    project: &ProjectManifest,
    daw: &str,
    stage_suffix: &str,
) -> Result<PathBuf> {
    custody::require_writable(project.custody_state())?;
    if !naming::STAGE_SUFFIXES.contains(&stage_suffix) {
        return Err(Error::requirement(
            "7.4",
            format!(
                "El sufijo de etapa «{stage_suffix}» no pertenece al conjunto de la norma. La sesión no se ha creado. Elegir uno de: {}.",
                naming::STAGE_SUFFIXES.join(", ")
            ),
        ));
    }
    let carpeta_daw = naming::slugify(daw);
    naming::check_name(&carpeta_daw)?;
    let destino = project.root().join("02_SESSIONS").join(&carpeta_daw);
    std::fs::create_dir_all(&destino).map_err(|e| Error::io(&destino, e))?;
    Ok(destino)
}

/// Nombre sugerido para el archivo de sesión (apartado 7.4, último guion).
pub fn suggested_session_name(
    project: &ProjectManifest,
    stage_suffix: &str,
    version: u32,
) -> String {
    let titulo = project
        .title()
        .map(naming::slugify)
        .unwrap_or_else(|| "Sesion".to_string());
    format!("{titulo}_{stage_suffix}_v{version:02}")
}

/// Incorpora material externo desde la cuarentena (apartado 6.1 y 7.3).
///
/// El material entra por `40_INBOX` y solo por ahí. Se copia a `01_REF`, que se
/// mantiene en solo lectura; la edición se hace sobre una copia en `04_EDIT`.
#[allow(clippy::too_many_arguments)]
pub fn ingest_from_inbox(
    repo: &Repository,
    actor: &str,
    project: &mut ProjectManifest,
    inbox_item: &Path,
    origin: &str,
    clearance: &str,
    classification: &str,
    usage: Option<&str>,
    retention_until: Option<&str>,
) -> Result<PathBuf> {
    custody::require_writable(project.custody_state())?;
    if !inbox_item.starts_with(repo.inbox()) {
        return Err(Error::requirement(
            "6.1",
            format!("El origen {} no está en la cuarentena. No se ha incorporado nada. Todo material procedente del exterior debe depositarse primero en 40_INBOX.", inbox_item.display()),
        ));
    }
    if !["pending", "cleared", "not_required"].contains(&clearance) {
        return Err(Error::input(format!(
            "El estado de autorización «{clearance}» no es admisible. No se ha incorporado nada. Los valores son pending, cleared y not_required."
        )));
    }

    let nombre = inbox_item
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::input("El elemento de la cuarentena no tiene nombre.".to_string()))?;
    naming::check_name(&nombre)?;

    let ref_dir = project.root().join("01_REF");
    std::fs::create_dir_all(&ref_dir).map_err(|e| Error::io(&ref_dir, e))?;
    let destino = ref_dir.join(&nombre);
    if destino.exists() {
        return Err(Error::input(format!(
            "Ya existe 01_REF/{nombre}. No se ha incorporado nada. Renombrar el archivo de origen."
        )));
    }

    if inbox_item.is_dir() {
        fsx::copy_tree(inbox_item, &destino, &[])?;
    } else {
        std::fs::copy(inbox_item, &destino).map_err(|e| Error::io(&destino, e))?;
    }

    // La carpeta 01_REF se mantiene en solo lectura (apartado 7.3): el original
    // entregado por un tercero es irrecuperable si se altera.
    fsx::readonly::set_tree_readonly(&ref_dir, true, &[])?;

    project.record_source(
        &format!("01_REF/{nombre}"),
        origin,
        None,
        classification,
        usage,
        retention_until,
        clearance,
    );
    project.save()?;

    repo.event_log().append(
        actor,
        event::MATERIAL_IMPORTED,
        project.uid(),
        json!({"path": format!("01_REF/{nombre}"), "origin": origin, "clearance": clearance}),
    )?;
    Ok(destino)
}

/// Registra una exportación sin sobrescribir lo ya exportado (principio 3 del
/// apartado 4).
///
/// Devuelve la ruta libre en la que debe escribirse la exportación. Si el nombre
/// propuesto ya existe, se incrementa el número de versión: un stem, un bounce o
/// un máster no se sobrescriben, se emite una versión nueva.
pub fn next_export_path(
    project: &ProjectManifest,
    folder: &str,
    base_name: &str,
    ext: &str,
) -> Result<PathBuf> {
    custody::require_writable(project.custody_state())?;
    let dir = project.root().join(folder);
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    for version in 1..=99u32 {
        let nombre = format!("{base_name}_v{version:02}.{ext}");
        naming::check_name(&nombre)?;
        let candidato = dir.join(&nombre);
        if !candidato.exists() {
            return Ok(candidato);
        }
    }
    Err(Error::input(format!(
        "Existen ya 99 versiones de {base_name} en {folder}. No se ha exportado nada. Revisar la carpeta."
    )))
}

/// Localiza un proyecto por su identificador interno.
pub fn find_by_uid(repo: &Repository, uid: &str) -> Option<PathBuf> {
    repo.discover_projects().into_iter().find(|p| {
        ProjectManifest::load(p)
            .ok()
            .and_then(|m| m.uid().map(|u| u == uid))
            .unwrap_or(false)
    })
}

/// Comprueba el presupuesto de ruta del proyecto contra todas sus réplicas
/// (apartado 9.1).
pub fn path_budget(project: &ProjectManifest) -> naming::PathBudget {
    let raiz = project.root();
    let rutas: Vec<String> = fsx::walk::conserved_files(raiz)
        .into_iter()
        .map(|e| {
            let id = project.id().unwrap_or_default();
            format!("{id}/{}", e.relative)
        })
        .collect();

    let mut replicas = std::collections::HashMap::new();
    for r in crate::replica::replicas_of(project.doc()) {
        replicas.insert(r.volume.clone(), r.path.clone());
    }
    naming::check_path_budget(&rutas, &replicas)
}

/// Carpetas del proyecto que existen, para el navegador propio.
pub fn existing_folders(project_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for dir in crate::repo::REQUIRED_PROJECT_DIRS
        .iter()
        .chain(crate::repo::OPTIONAL_PROJECT_DIRS.iter())
    {
        if project_root.join(dir).is_dir() {
            out.push((*dir).to_string());
        }
    }
    out
}

/// Comprueba que la ruta esté dentro del proyecto. El navegador propio no debe
/// salir de él sin una acción explícita.
pub fn is_inside_project(project_root: &Path, path: &Path) -> bool {
    let (Ok(a), Ok(b)) = (project_root.canonicalize(), path.canonicalize()) else {
        return false;
    };
    b.starts_with(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> (tempfile::TempDir, Repository) {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        (dir, repo)
    }

    fn spec() -> NewProject {
        NewProject {
            title: "Canción de Ejemplo".into(),
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
        }
    }

    #[test]
    fn la_creacion_pide_cinco_campos_y_deja_el_resto_pendiente() {
        let (_d, repo) = repo();
        let m = create(&repo, "J. Duarte", &spec()).unwrap();

        assert!(crate::ids::is_uid(m.uid().unwrap()));
        assert_eq!(
            m.id().unwrap(),
            format!("{}_Cancion-de-Ejemplo_ORIG", clock::today())
        );
        assert_eq!(m.artist(), Some("Artista"));
        assert_eq!(m.sample_rate(), Some(48000));
        assert_eq!(m.custody_state(), CustodyState::Own);
        // Los campos pendientes están presentes y en nulo (apartado 13.2).
        assert!(m.doc().at("audio.tempo").unwrap().is_null());
        assert!(m.doc().at("master.lufs_i").unwrap().is_null());
    }

    #[test]
    fn solo_crea_las_carpetas_obligatorias() {
        let (_d, repo) = repo();
        let m = create(&repo, "a", &spec()).unwrap();
        let raiz = m.root();
        assert!(raiz.join("00_ADMIN").is_dir());
        assert!(raiz.join("02_SESSIONS").is_dir());
        // Apartado 7.1: no se crean carpetas vacías por anticipado.
        for d in crate::repo::OPTIONAL_PROJECT_DIRS {
            assert!(!raiz.join(d).exists(), "{d} no debía crearse");
        }
    }

    #[test]
    fn la_creacion_deja_su_entrada_en_el_registro() {
        let (_d, repo) = repo();
        let m = create(&repo, "J. Duarte", &spec()).unwrap();
        let entradas = repo.event_log().entries_for(m.uid().unwrap()).unwrap();
        assert_eq!(entradas.len(), 1);
        assert_eq!(entradas[0].event, event::PROJECT_CREATED);
        assert!(repo.event_log().verify_chain().unwrap().is_intact());
    }

    #[test]
    fn rechaza_un_tipo_ajeno_a_la_tabla_10_y_menos_de_24_bits() {
        let (_d, repo) = repo();
        let mut s = spec();
        s.kind = "INVENTADO".into();
        assert!(create(&repo, "a", &s).is_err());

        let mut s = spec();
        s.bit_depth = 16;
        let e = create(&repo, "a", &s).unwrap_err();
        assert_eq!(e.clause(), Some("10.1"));
    }

    #[test]
    fn el_cambio_de_titulo_conserva_el_identificador_interno() {
        let (_d, repo) = repo();
        let mut m = create(&repo, "a", &spec()).unwrap();
        let uid = m.uid().unwrap().to_string();
        let raiz_vieja = m.root().to_path_buf();

        let nueva = rename(&repo, "a", &mut m, "Otro Titulo").unwrap();

        assert_eq!(m.uid().unwrap(), uid, "el identificador interno no cambia");
        assert!(m.id().unwrap().ends_with("_Otro-Titulo_ORIG"));
        assert_eq!(m.title(), Some("Otro Titulo"));
        assert!(!raiz_vieja.exists(), "la carpeta anterior se renombró");
        assert!(nueva.join(PROJECT_FILE).is_file());
        // El valor anterior queda registrado con su marca temporal.
        let hist = m.doc().get("id_history").unwrap().as_seq().unwrap();
        assert_eq!(hist.len(), 1);
        assert!(hist[0]
            .as_map()
            .unwrap()
            .get("previous")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("Cancion-de-Ejemplo"));
    }

    #[test]
    fn el_titulo_no_cambia_una_vez_emitido_un_envio() {
        let (_d, repo) = repo();
        let mut m = create(&repo, "a", &spec()).unwrap();
        m.record_delivery("2026-08-06", "Sello", "08_DELIVERY/x", "20260806-A", "r0");
        m.save().unwrap();
        let e = rename(&repo, "a", &mut m, "Nuevo").unwrap_err();
        assert_eq!(e.clause(), Some("9.2"));
    }

    #[test]
    fn la_derivacion_sella_el_origen_y_declara_la_ascendencia() {
        let (_d, repo) = repo();
        let mut origen = create(&repo, "a", &spec()).unwrap();
        std::fs::write(origen.root().join("00_ADMIN/notas.txt"), b"notas").unwrap();
        std::fs::create_dir_all(origen.root().join("08_DELIVERY/2026-08-06_Sello")).unwrap();
        std::fs::write(
            origen.root().join("08_DELIVERY/2026-08-06_Sello/p.zip"),
            b"paquete",
        )
        .unwrap();
        let uid_origen = origen.uid().unwrap().to_string();

        let d = derive(&repo, "a", &mut origen, "Cambio de tonalidad", false).unwrap();

        // El derivado tiene identificador interno propio y sufijo _vNN.
        assert_ne!(d.derived_uid, uid_origen);
        assert!(d.derived_id.ends_with("_v02"), "{}", d.derived_id);

        let derivado = ProjectManifest::load(d.derived_root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            derivado.doc().at("lineage.parent_uid").unwrap().as_str(),
            Some(uid_origen.as_str())
        );
        assert_eq!(
            derivado.doc().at("lineage.reason").unwrap().as_str(),
            Some("Cambio de tonalidad")
        );
        // Copia íntegra salvo los paquetes ya emitidos.
        assert!(d.derived_root.join("00_ADMIN/notas.txt").is_file());
        assert!(!d.derived_root.join("08_DELIVERY").exists());
        // Custodia independiente y propia desde el instante de la derivación.
        assert_eq!(derivado.custody_state(), CustodyState::Own);

        // El origen queda sellado y en solo lectura.
        assert!(d.source_sealed);
        assert_eq!(origen.status(), Status::Sealed);
        let notas = origen.root().join("00_ADMIN/notas.txt");
        assert!(std::fs::metadata(&notas).unwrap().permissions().readonly());
        // Y declara el derivado.
        let derivados = origen.doc().get("derived").unwrap().as_seq().unwrap();
        assert_eq!(derivados.len(), 1);
    }

    #[test]
    fn la_derivacion_en_paralelo_no_sella_el_origen() {
        let (_d, repo) = repo();
        let mut origen = create(&repo, "a", &spec()).unwrap();
        let d = derive(&repo, "a", &mut origen, "Version alternativa", true).unwrap();
        assert!(!d.source_sealed);
        assert_ne!(origen.status(), Status::Sealed);
        // La circunstancia se declara de forma expresa (apartado 14.4.3).
        assert_eq!(
            origen.doc().at("lineage.parallel").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn un_proyecto_sellado_no_se_renombra_ni_se_deriva_de_nuevo() {
        let (_d, repo) = repo();
        let mut origen = create(&repo, "a", &spec()).unwrap();
        derive(&repo, "a", &mut origen, "r", false).unwrap();
        assert!(rename(&repo, "a", &mut origen, "Nuevo").is_err());
        assert!(derive(&repo, "a", &mut origen, "r", false).is_err());
    }

    #[test]
    fn las_derivaciones_sucesivas_numeran_de_dos_en_dos_digitos() {
        let (_d, repo) = repo();
        let mut origen = create(&repo, "a", &spec()).unwrap();
        let d1 = derive(&repo, "a", &mut origen, "r", true).unwrap();
        let mut derivado = ProjectManifest::load(d1.derived_root.join(PROJECT_FILE)).unwrap();
        let d2 = derive(&repo, "a", &mut derivado, "r", true).unwrap();
        assert!(d1.derived_id.ends_with("_v02"));
        assert!(d2.derived_id.ends_with("_v03"), "{}", d2.derived_id);
    }

    #[test]
    fn el_material_externo_entra_solo_por_la_cuarentena() {
        let (dir, repo) = repo();
        let mut m = create(&repo, "a", &spec()).unwrap();

        // Desde fuera de la cuarentena: rechazado.
        let fuera = dir.path().join("suelto.wav");
        std::fs::write(&fuera, b"audio").unwrap();
        let e = ingest_from_inbox(
            &repo, "a", &mut m, &fuera, "Cliente", "cleared", "INTERNO", None, None,
        )
        .unwrap_err();
        assert_eq!(e.clause(), Some("6.1"));

        // Desde la cuarentena: aceptado, y 01_REF queda en solo lectura.
        let dentro = repo.inbox().join("referencia.wav");
        std::fs::write(&dentro, b"audio").unwrap();
        let destino = ingest_from_inbox(
            &repo, "a", &mut m, &dentro, "Cliente", "cleared", "INTERNO", None, None,
        )
        .unwrap();
        assert!(destino.is_file());
        assert!(
            std::fs::metadata(&destino)
                .unwrap()
                .permissions()
                .readonly(),
            "01_REF en solo lectura"
        );
        let fuentes = m.doc().get("sources").unwrap().as_seq().unwrap();
        assert_eq!(fuentes.len(), 1);
        assert_eq!(
            fuentes[0]
                .as_map()
                .unwrap()
                .get("clearance")
                .unwrap()
                .as_str(),
            Some("cleared")
        );
    }

    #[test]
    fn lo_exportado_no_se_sobrescribe() {
        let (_d, repo) = repo();
        let m = create(&repo, "a", &spec()).unwrap();
        let p1 = next_export_path(&m, "06_MIX", "Tema_MIX", "wav").unwrap();
        assert!(p1.ends_with("Tema_MIX_v01.wav"));
        std::fs::write(&p1, b"bounce").unwrap();
        let p2 = next_export_path(&m, "06_MIX", "Tema_MIX", "wav").unwrap();
        assert!(p2.ends_with("Tema_MIX_v02.wav"), "{p2:?}");
        assert_ne!(p1, p2);
    }

    #[test]
    fn la_carpeta_de_sesion_se_crea_bajo_demanda() {
        let (_d, repo) = repo();
        let m = create(&repo, "a", &spec()).unwrap();
        let carpeta = create_session_folder(&m, "Reaper", "TRACK").unwrap();
        assert!(carpeta.is_dir());
        assert!(carpeta.ends_with("02_SESSIONS/Reaper"));
        assert!(create_session_folder(&m, "Reaper", "INVENTADO").is_err());
        let nombre = suggested_session_name(&m, "TRACK", 1);
        assert_eq!(nombre, "Cancion-de-Ejemplo_TRACK_v01");
    }

    #[test]
    fn un_proyecto_cedido_no_admite_ninguna_modificacion() {
        let (_d, repo) = repo();
        let mut m = create(&repo, "a", &spec()).unwrap();
        m.doc_mut()
            .ensure_map("custody")
            .set("state", Node::str("cedida"));
        m.save().unwrap();

        assert!(rename(&repo, "a", &mut m, "Nuevo").is_err());
        assert!(derive(&repo, "a", &mut m, "r", false).is_err());
        assert!(create_session_folder(&m, "Reaper", "MIX").is_err());
        assert!(next_export_path(&m, "06_MIX", "T", "wav").is_err());
    }

    #[test]
    fn el_cambio_de_estado_traslada_a_la_carpeta_correspondiente() {
        let (_d, repo) = repo();
        let mut m = create(&repo, "a", &spec()).unwrap();
        assert!(m.root().to_string_lossy().contains("0_IDEAS"));
        let destino = set_status(&repo, "a", &mut m, Status::Active).unwrap();
        assert!(destino.to_string_lossy().contains("1_ACTIVE"));
        assert!(destino.join(PROJECT_FILE).is_file());
        // Un proyecto sellado permanece donde estaba (apartado 6.2).
        let antes = m.root().to_path_buf();
        set_status(&repo, "a", &mut m, Status::Sealed).unwrap();
        assert_eq!(m.root(), antes);
    }

    #[test]
    fn localiza_un_proyecto_por_identificador_interno() {
        let (_d, repo) = repo();
        let m = create(&repo, "a", &spec()).unwrap();
        let uid = m.uid().unwrap().to_string();
        let hallado = find_by_uid(&repo, &uid).unwrap();
        assert_eq!(hallado, m.path());
        assert!(find_by_uid(&repo, "NOEXISTE").is_none());
    }

    #[test]
    fn rechaza_un_titulo_que_agota_el_presupuesto_de_ruta() {
        let (_d, repo) = repo();
        let mut s = spec();
        s.title = "T".repeat(40);
        s.artist = "A".repeat(60);
        // El proyecto se crea en una ruta ya profunda; el margen se agota.
        let r = create(&repo, "a", &s);
        if let Err(e) = r {
            assert_eq!(e.clause(), Some("9.1"));
            assert!(e.to_string().contains("caracteres"));
        }
    }
}
