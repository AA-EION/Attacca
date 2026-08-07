//! Índice del repositorio (apartado 42, Tabla 34).
//!
//! El índice es **derivado**. Constituye una caché y debe poder reconstruirse
//! íntegramente a partir del árbol de archivos. Si se borra, no se pierde nada.
//!
//! Ninguna información normativa reside aquí en exclusiva: cada campo procede de
//! un manifiesto que sigue estando en el disco y sigue siendo legible con un
//! editor de texto.

use crate::custody::CustodyState;
use crate::error::{Error, Result};
use crate::manifest::project::{Level, ProjectManifest, Status};
use crate::manifest::release::ReleaseManifest;
use crate::repo::Repository;
use crate::stage::Stage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Nombre del archivo de caché. Empieza por punto y figura entre los elementos
/// que el recorrido del Anexo C excluye: no es material.
pub const CACHE_FILE: &str = ".attacca-index";

/// Versión del formato de la caché. Un cambio invalida la caché existente, que
/// se reconstruye sin pérdida.
const CACHE_VERSION: u32 = 1;

/// Entrada del índice correspondiente a un proyecto.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub uid: String,
    pub id: String,
    pub title: String,
    pub artist: String,
    pub kind: String,
    pub level: String,
    pub status: String,
    pub custody: String,
    pub stage: String,
    pub release_uid: Option<String>,
    pub path: PathBuf,
    /// Momento de la última modificación del manifiesto, para invalidar la
    /// caché sin releerlo.
    pub manifest_mtime: u64,
    pub manifest_size: u64,
}

impl ProjectEntry {
    pub fn custody_state(&self) -> CustodyState {
        CustodyState::parse(&self.custody).unwrap_or(CustodyState::Own)
    }

    pub fn stage(&self) -> Option<Stage> {
        Stage::parse(&self.stage)
    }

    pub fn status(&self) -> Status {
        Status::parse(&self.status).unwrap_or(Status::Idea)
    }

    pub fn level(&self) -> Level {
        Level::parse(&self.level).unwrap_or(Level::B)
    }
}

/// Entrada del índice correspondiente a un release.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReleaseEntry {
    pub uid: String,
    pub id: String,
    pub title: String,
    pub artist: String,
    pub class: String,
    pub status: String,
    pub track_count: usize,
    pub path: PathBuf,
}

/// Índice completo.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Index {
    version: u32,
    pub projects: Vec<ProjectEntry>,
    pub releases: Vec<ReleaseEntry>,
    /// Manifiestos que no se pudieron interpretar. No se ocultan: un manifiesto
    /// ilegible es un hecho que la persona debe conocer.
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Index {
    /// Reconstruye el índice recorriendo el árbol.
    ///
    /// Esta es la operación que acredita el apartado 42: la totalidad del índice
    /// procede del árbol de archivos y de los manifiestos.
    pub fn rebuild(repo: &Repository) -> Result<Index> {
        let mut index = Index {
            version: CACHE_VERSION,
            ..Default::default()
        };

        for manifest_path in repo.discover_projects() {
            match ProjectManifest::load(&manifest_path) {
                Ok(m) => {
                    let raiz = m.root().to_path_buf();
                    let (mtime, size) = file_stamp(&manifest_path);
                    let etapa = crate::stage::active_stage_of(m.doc(), &raiz);
                    index.projects.push(ProjectEntry {
                        uid: m.uid().unwrap_or_default().to_string(),
                        id: m.id().unwrap_or_default().to_string(),
                        title: m.title().unwrap_or_default().to_string(),
                        artist: m.artist().unwrap_or_default().to_string(),
                        kind: m.project_type().unwrap_or_default().to_string(),
                        level: m.level().as_str().to_string(),
                        status: m.status().as_str().to_string(),
                        custody: m.custody_state().as_str().to_string(),
                        stage: etapa.key().to_string(),
                        release_uid: m.release_uid().map(str::to_string),
                        path: manifest_path.clone(),
                        manifest_mtime: mtime,
                        manifest_size: size,
                    });
                }
                Err(e) => index.unreadable.push((manifest_path, e.to_string())),
            }
        }

        for manifest_path in repo.discover_releases() {
            match ReleaseManifest::load(&manifest_path) {
                Ok(m) => index.releases.push(ReleaseEntry {
                    uid: m.uid().unwrap_or_default().to_string(),
                    id: m.id().unwrap_or_default().to_string(),
                    title: m.title().unwrap_or_default().to_string(),
                    artist: m.artist().unwrap_or_default().to_string(),
                    class: m
                        .class()
                        .map(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    status: m.status().as_str().to_string(),
                    track_count: m.tracklist().len(),
                    path: manifest_path.clone(),
                }),
                Err(e) => index.unreadable.push((manifest_path, e.to_string())),
            }
        }

        index.projects.sort_by(|a, b| a.id.cmp(&b.id));
        index.releases.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(index)
    }

    /// Carga el índice desde la caché, o lo reconstruye si no es utilizable.
    ///
    /// La caché se descarta cuando su versión no corresponde, cuando no puede
    /// interpretarse o cuando algún manifiesto cambió. En todos esos casos la
    /// reconstrucción es completa y silenciosa: no hay nada que recuperar.
    pub fn load_or_rebuild(repo: &Repository, cache_dir: &Path) -> Result<(Index, LoadOutcome)> {
        let inicio = Instant::now();
        let cache_path = cache_dir.join(CACHE_FILE);

        if let Some(index) = read_cache(&cache_path) {
            if index.version == CACHE_VERSION && index.is_current() {
                return Ok((
                    index,
                    LoadOutcome {
                        rebuilt: false,
                        elapsed_ms: inicio.elapsed().as_millis() as u64,
                    },
                ));
            }
        }

        let index = Index::rebuild(repo)?;
        let _ = index.write_cache(&cache_path);
        Ok((
            index,
            LoadOutcome {
                rebuilt: true,
                elapsed_ms: inicio.elapsed().as_millis() as u64,
            },
        ))
    }

    /// La caché describe el estado actual del árbol.
    fn is_current(&self) -> bool {
        self.projects.iter().all(|p| {
            let (mtime, size) = file_stamp(&p.path);
            mtime == p.manifest_mtime && size == p.manifest_size && size != 0
        })
    }

    pub fn write_cache(&self, path: &Path) -> Result<()> {
        let datos = serde_json::to_vec(self)
            .map_err(|e| Error::io(path, std::io::Error::other(e.to_string())))?;
        crate::fsx::atomic::write(path, &datos)
    }

    pub fn find_project(&self, uid: &str) -> Option<&ProjectEntry> {
        self.projects.iter().find(|p| p.uid == uid)
    }

    pub fn find_release(&self, uid: &str) -> Option<&ReleaseEntry> {
        self.releases.iter().find(|r| r.uid == uid)
    }

    /// Proyectos que integran un release, en el orden del índice.
    pub fn projects_of_release(&self, release_uid: &str) -> Vec<&ProjectEntry> {
        self.projects
            .iter()
            .filter(|p| p.release_uid.as_deref() == Some(release_uid))
            .collect()
    }

    /// Proyectos cuya custodia no permite escribir.
    pub fn ceded_projects(&self) -> Vec<&ProjectEntry> {
        self.projects
            .iter()
            .filter(|p| !p.custody_state().allows_write())
            .collect()
    }
}

/// Resultado de la carga del índice.
#[derive(Clone, Copy, Debug)]
pub struct LoadOutcome {
    /// El índice se reconstruyó recorriendo el árbol.
    pub rebuilt: bool,
    pub elapsed_ms: u64,
}

fn read_cache(path: &Path) -> Option<Index> {
    let datos = std::fs::read(path).ok()?;
    serde_json::from_slice(&datos).ok()
}

fn file_stamp(path: &Path) -> (u64, u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (mtime, meta.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::project::Level;
    use crate::project::{self, NewProject};

    fn entorno(n: usize) -> (tempfile::TempDir, Repository) {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        let repo = Repository::create(dir.path().join(".stave")).unwrap();
        for i in 0..n {
            project::create(
                &repo,
                "a",
                &NewProject {
                    title: format!("Tema {i}"),
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
        }
        (dir, repo)
    }

    #[test]
    fn el_indice_se_reconstruye_integramente_desde_el_arbol() {
        let (dir, repo) = entorno(5);
        let cache = dir.path().to_path_buf();

        let (primero, r1) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(r1.rebuilt);
        assert_eq!(primero.projects.len(), 5);

        // Se borra la base de datos de la aplicación.
        std::fs::remove_file(cache.join(CACHE_FILE)).unwrap();

        let (segundo, r2) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(r2.rebuilt);
        assert_eq!(segundo.projects.len(), primero.projects.len());
        // No se ha perdido nada: cada entrada coincide.
        for (a, b) in primero.projects.iter().zip(segundo.projects.iter()) {
            assert_eq!(a.uid, b.uid);
            assert_eq!(a.id, b.id);
            assert_eq!(a.title, b.title);
            assert_eq!(a.artist, b.artist);
            assert_eq!(a.custody, b.custody);
            assert_eq!(a.stage, b.stage);
            assert_eq!(a.path, b.path);
        }
    }

    #[test]
    fn la_cache_se_reutiliza_mientras_los_manifiestos_no_cambien() {
        let (dir, repo) = entorno(3);
        let cache = dir.path().to_path_buf();
        let (_, r1) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(r1.rebuilt);
        let (_, r2) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(!r2.rebuilt, "la segunda carga usa la caché");
    }

    #[test]
    fn un_manifiesto_modificado_invalida_la_cache() {
        let (dir, repo) = entorno(2);
        let cache = dir.path().to_path_buf();
        let (index, _) = Index::load_or_rebuild(&repo, &cache).unwrap();

        // Se modifica un manifiesto desde fuera de Attacca.
        let ruta = index.projects[0].path.clone();
        let mut texto = std::fs::read_to_string(&ruta).unwrap();
        texto.push_str("x_otra_marca: azul\n");
        std::fs::write(&ruta, texto).unwrap();

        let (_, r) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(r.rebuilt, "un cambio en el árbol reconstruye el índice");
    }

    #[test]
    fn una_cache_ilegible_no_impide_abrir_el_repositorio() {
        let (dir, repo) = entorno(2);
        let cache = dir.path().to_path_buf();
        std::fs::write(cache.join(CACHE_FILE), b"esto no es JSON").unwrap();
        let (index, r) = Index::load_or_rebuild(&repo, &cache).unwrap();
        assert!(r.rebuilt);
        assert_eq!(index.projects.len(), 2);
    }

    #[test]
    fn un_manifiesto_ilegible_se_declara_sin_ocultarse() {
        let (_dir, repo) = entorno(1);
        let roto = repo.domain("20_PROJECTS").join("0_IDEAS/proyecto-roto");
        std::fs::create_dir_all(&roto).unwrap();
        std::fs::write(roto.join("PROJECT.yaml"), b"esto: [no cierra\n").unwrap();

        let index = Index::rebuild(&repo).unwrap();
        assert_eq!(index.projects.len(), 1);
        assert_eq!(index.unreadable.len(), 1);
    }

    #[test]
    fn el_arranque_en_frio_con_quinientos_proyectos_esta_por_debajo_del_presupuesto() {
        let (dir, repo) = entorno(500);
        let cache = dir.path().to_path_buf();
        let inicio = Instant::now();
        let (index, r) = Index::load_or_rebuild(&repo, &cache).unwrap();
        let transcurrido = inicio.elapsed();

        assert_eq!(index.projects.len(), 500);
        assert!(r.rebuilt);
        // El presupuesto de arranque en frío de la aplicación es de dos
        // segundos. La reconstrucción íntegra del índice es la parte que crece
        // con el número de proyectos; se le reserva la mitad.
        assert!(
            transcurrido.as_millis() < 1000,
            "la reconstrucción tardó {} ms con 500 proyectos",
            transcurrido.as_millis()
        );
    }

    #[test]
    fn localiza_por_identificador_interno_y_filtra_por_custodia() {
        let (dir, repo) = entorno(3);
        let (mut index, _) = Index::load_or_rebuild(&repo, dir.path()).unwrap();
        let uid = index.projects[0].uid.clone();
        assert!(index.find_project(&uid).is_some());
        assert!(index.find_project("NOEXISTE").is_none());

        index.projects[1].custody = "cedida".into();
        assert_eq!(index.ceded_projects().len(), 1);
    }
}
