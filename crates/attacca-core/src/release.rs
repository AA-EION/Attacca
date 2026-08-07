//! Operaciones sobre releases (apartado 8).

use crate::clock;
use crate::doc::Node;
use crate::error::{Error, Result};
use crate::eventlog::event;
use crate::ids;
use crate::manifest::project::{Level, ProjectManifest};
use crate::manifest::release::{scaffold, ReleaseClass, ReleaseManifest};
use crate::manifest::RELEASE_FILE;
use crate::naming;
use crate::repo::Repository;
use serde_json::json;
use std::path::PathBuf;

/// Datos que se piden al crear un release.
#[derive(Clone, Debug)]
pub struct NewRelease {
    pub title: String,
    pub artist: String,
    pub class: ReleaseClass,
    pub level: Level,
    pub holder_org: String,
    pub holder_person: String,
}

/// Crea un release conforme al apartado 8.1.
///
/// La estructura es `ARTISTA/AAAA-MM-DD_Titulo_CLASE/_RELEASE/`. Solo se crean
/// las carpetas que se usan.
pub fn create(repo: &Repository, actor: &str, spec: &NewRelease) -> Result<ReleaseManifest> {
    if spec.artist.trim().is_empty() {
        return Err(Error::input(
            "El campo de artista está vacío. El release no se ha creado. Escribir el nombre del artista.",
        ));
    }
    let date = clock::today();
    let id = naming::release_id(&date, &spec.title, spec.class.as_str())?;
    let artista = naming::slugify(&spec.artist);
    naming::check_name(&artista)?;

    let raiz = repo
        .domain("20_PROJECTS")
        .join("1_ACTIVE")
        .join(&artista)
        .join(&id);
    if raiz.exists() {
        return Err(Error::input(format!(
            "Ya existe una carpeta {id}. El release no se ha creado. Elegir otro título."
        )));
    }

    let admin = raiz.join("_RELEASE/00_ADMIN");
    std::fs::create_dir_all(&admin).map_err(|e| Error::io(&admin, e))?;

    let uid = ids::new_uid();
    let mut m = ReleaseManifest::new(raiz.join("_RELEASE").join(RELEASE_FILE));
    *m.doc_mut() = scaffold(
        &uid,
        &id,
        &spec.title,
        &spec.artist,
        spec.class,
        spec.level,
        &spec.holder_org,
        &spec.holder_person,
    );
    m.save()?;

    repo.event_log().append(
        actor,
        event::RELEASE_CREATED,
        Some(&uid),
        json!({"id": id, "class": spec.class.as_str(), "artist": spec.artist}),
    )?;
    Ok(m)
}

/// Vincula un proyecto a un release (apartado 8.3).
///
/// La vinculación es recíproca y se expresa por identificador interno: el
/// tracklist referencia el proyecto y el manifiesto del proyecto declara el
/// release al que pertenece.
pub fn link_project(
    repo: &Repository,
    actor: &str,
    release: &mut ReleaseManifest,
    project: &mut ProjectManifest,
    position: Option<i64>,
) -> Result<i64> {
    let project_uid = project
        .uid()
        .ok_or_else(|| Error::input("El proyecto no declara identificador interno.".to_string()))?
        .to_string();
    let release_uid = release
        .uid()
        .ok_or_else(|| Error::input("El release no declara identificador interno.".to_string()))?
        .to_string();

    // Un proyecto no debe pertenecer a más de un release a la vez
    // (apartado 8.3, segundo guion). Una recopilación es la excepción: puede
    // incorporar un proyecto ya publicado sin que este deje de pertenecer a su
    // release de origen.
    if let Some(actual) = project.release_uid() {
        if actual != release_uid && !release.class().map(|c| c.references_without_duplicating()).unwrap_or(false) {
            return Err(Error::requirement(
                "8.3",
                format!("El proyecto ya pertenece al release {actual}. No se ha vinculado nada. Un proyecto no debe pertenecer a más de un release a la vez, salvo en una recopilación."),
            ));
        }
    }

    let pos = release.link_project(
        &project_uid,
        project.id().unwrap_or_default(),
        project.title().unwrap_or_default(),
        position,
    )?;

    // El release de origen se conserva cuando la vinculación es a una
    // recopilación (apartado 8.3, tercer guion).
    let es_recopilacion = release
        .class()
        .map(|c| c.references_without_duplicating())
        .unwrap_or(false);
    if !es_recopilacion || project.release_uid().is_none() {
        project.doc_mut().set("release", Node::str(&release_uid));
        project.save()?;
    }

    // El nivel del release es el más alto de los declarados por sus integrantes
    // (apartado 8.4).
    if project.level() > release.level() {
        release
            .doc_mut()
            .ensure_map("stave")
            .set("level", Node::str(project.level().as_str()));
    }
    release.save()?;

    repo.event_log().append(
        actor,
        event::RELEASE_TRACK_LINKED,
        Some(&release_uid),
        json!({"project_uid": project_uid, "position": pos}),
    )?;
    Ok(pos)
}

/// Carpeta en la que crear un proyecto integrante de un release.
///
/// La numeración de las carpetas de tema refleja el orden del tracklist
/// (apartado 8.3, último guion).
pub fn track_dir(release: &ReleaseManifest, position: i64) -> PathBuf {
    release.root().join(format!("{position:02}_Tema"))
}

/// Localiza un release por su identificador interno.
pub fn find_by_uid(repo: &Repository, uid: &str) -> Option<PathBuf> {
    repo.discover_releases().into_iter().find(|p| {
        ReleaseManifest::load(p)
            .ok()
            .and_then(|m| m.uid().map(|u| u == uid))
            .unwrap_or(false)
    })
}

/// Proyectos integrantes de un release, cargados desde el árbol.
pub fn members(repo: &Repository, release: &ReleaseManifest) -> Vec<(String, ProjectManifest)> {
    let mut out = Vec::new();
    for track in release.tracklist() {
        if let Some(ruta) = crate::project::find_by_uid(repo, &track.project_uid) {
            if let Ok(m) = ProjectManifest::load(&ruta) {
                out.push((track.project_uid.clone(), m));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::NewProject;

    fn entorno() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        (dir, repo)
    }

    fn nuevo_release(class: ReleaseClass) -> NewRelease {
        NewRelease {
            title: "Disco de Ejemplo".into(),
            artist: "Artista".into(),
            class,
            level: Level::C,
            holder_org: "Estudio A".into(),
            holder_person: "J. Duarte".into(),
        }
    }

    fn nuevo_proyecto(titulo: &str) -> NewProject {
        NewProject {
            title: titulo.into(),
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
        }
    }

    #[test]
    fn la_creacion_sigue_la_estructura_del_apartado_8_1() {
        let (_d, repo) = entorno();
        let r = create(&repo, "a", &nuevo_release(ReleaseClass::Album)).unwrap();
        assert!(r.path().ends_with("_RELEASE/RELEASE.yaml"));
        assert!(r.root().join("_RELEASE/00_ADMIN").is_dir());
        assert_eq!(r.class(), Some(ReleaseClass::Album));
        assert!(r.id().unwrap().ends_with("_Disco-de-Ejemplo_ALBUM"));
        assert!(crate::ids::is_uid(r.uid().unwrap()));
    }

    #[test]
    fn la_vinculacion_es_reciproca_y_por_identificador_interno() {
        let (_d, repo) = entorno();
        let mut r = create(&repo, "a", &nuevo_release(ReleaseClass::Ep)).unwrap();
        let mut p = crate::project::create(&repo, "a", &nuevo_proyecto("Tema Uno")).unwrap();

        let pos = link_project(&repo, "a", &mut r, &mut p, None).unwrap();
        assert_eq!(pos, 1);
        assert_eq!(p.release_uid(), r.uid());
        assert_eq!(r.tracklist()[0].project_uid, p.uid().unwrap());
    }

    #[test]
    fn renombrar_un_tema_no_rompe_el_tracklist() {
        let (_d, repo) = entorno();
        let mut r = create(&repo, "a", &nuevo_release(ReleaseClass::Ep)).unwrap();
        let mut p = crate::project::create(&repo, "a", &nuevo_proyecto("Titulo Viejo")).unwrap();
        link_project(&repo, "a", &mut r, &mut p, None).unwrap();
        let uid = p.uid().unwrap().to_string();

        crate::project::rename(&repo, "a", &mut p, "Titulo Nuevo").unwrap();

        // El tracklist sigue resolviendo al proyecto por identificador interno.
        let recargado = ReleaseManifest::load(r.path()).unwrap();
        assert_eq!(recargado.tracklist()[0].project_uid, uid);
        let hallado = crate::project::find_by_uid(&repo, &uid).unwrap();
        let m = ProjectManifest::load(&hallado).unwrap();
        assert!(m.id().unwrap().ends_with("_Titulo-Nuevo_ORIG"));
        assert_eq!(m.release_uid(), recargado.uid());
    }

    #[test]
    fn un_proyecto_no_pertenece_a_dos_releases_salvo_recopilacion() {
        let (_d, repo) = entorno();
        let mut a = create(&repo, "a", &nuevo_release(ReleaseClass::Ep)).unwrap();
        let mut b = create(&repo, "a", &NewRelease { title: "Otro".into(), ..nuevo_release(ReleaseClass::Album) }).unwrap();
        let mut p = crate::project::create(&repo, "a", &nuevo_proyecto("Tema")).unwrap();

        link_project(&repo, "a", &mut a, &mut p, None).unwrap();
        let e = link_project(&repo, "a", &mut b, &mut p, None).unwrap_err();
        assert_eq!(e.clause(), Some("8.3"));

        // Una recopilación sí puede referenciarlo, sin que deje su release de
        // origen (apartado 8.3, tercer guion).
        let mut comp = create(&repo, "a", &NewRelease { title: "Recopilacion".into(), ..nuevo_release(ReleaseClass::Comp) }).unwrap();
        link_project(&repo, "a", &mut comp, &mut p, None).unwrap();
        assert_eq!(p.release_uid(), a.uid(), "conserva su release de origen");
        assert_eq!(comp.tracklist().len(), 1);
    }

    #[test]
    fn el_nivel_del_release_es_el_mas_alto_de_sus_integrantes() {
        let (_d, repo) = entorno();
        let mut r = create(&repo, "a", &NewRelease { level: Level::A, ..nuevo_release(ReleaseClass::Single) }).unwrap();
        assert_eq!(r.level(), Level::A);
        let mut p = crate::project::create(&repo, "a", &NewProject { level: Level::C, ..nuevo_proyecto("Tema") }).unwrap();
        link_project(&repo, "a", &mut r, &mut p, None).unwrap();
        assert_eq!(r.level(), Level::C);
    }

    #[test]
    fn los_huecos_de_numeracion_se_registran_sin_renumerar() {
        let (_d, repo) = entorno();
        let mut r = create(&repo, "a", &nuevo_release(ReleaseClass::Album)).unwrap();
        for (i, titulo) in [(1, "Uno"), (2, "Dos"), (4, "Cuatro")] {
            let mut p = crate::project::create(&repo, "a", &nuevo_proyecto(titulo)).unwrap();
            link_project(&repo, "a", &mut r, &mut p, Some(i)).unwrap();
        }
        r.record_gap(3, "Tema descartado en preproduccion");
        r.save().unwrap();

        let recargado = ReleaseManifest::load(r.path()).unwrap();
        let posiciones: Vec<i64> = recargado.tracklist().iter().map(|t| t.position).collect();
        assert_eq!(posiciones, vec![1, 2, 4]);
        assert_eq!(recargado.gaps(), vec![3]);
    }

    #[test]
    fn localiza_los_integrantes_desde_el_arbol() {
        let (_d, repo) = entorno();
        let mut r = create(&repo, "a", &nuevo_release(ReleaseClass::Ep)).unwrap();
        for titulo in ["Uno", "Dos"] {
            let mut p = crate::project::create(&repo, "a", &nuevo_proyecto(titulo)).unwrap();
            link_project(&repo, "a", &mut r, &mut p, None).unwrap();
        }
        assert_eq!(members(&repo, &r).len(), 2);
        assert!(find_by_uid(&repo, r.uid().unwrap()).is_some());
    }
}
