//! Repositorio: raíz, dominios y descubrimiento (apartados 6 y 42).
//!
//! El repositorio es un árbol de carpetas corrientes. Attacca lo recorre; no lo
//! posee. Cualquier otra herramienta puede leerlo y escribirlo, y el material
//! sigue siendo utilizable sin Attacca instalado.

use crate::error::{Error, Result};
use crate::manifest;
use std::path::{Path, PathBuf};

/// Dominios de primer nivel (apartado 6.1). Los prefijos numéricos son
/// normativos: garantizan un orden de presentación idéntico en cualquier
/// sistema de archivos.
pub const DOMAINS: &[&str] = &[
    "00_SYSTEM",
    "10_LIBRARY",
    "20_PROJECTS",
    "30_ARCHIVE",
    "40_INBOX",
];

/// Estados dentro de `20_PROJECTS` (apartado 6.2).
pub const PROJECT_STATES: &[&str] = &["0_IDEAS", "1_ACTIVE", "2_ONHOLD", "3_DELIVERED"];

/// Nombre recomendado de la raíz local (apartado 6.5, cuarto guion).
pub const LOCAL_ROOT_NAME: &str = ".stave";

/// Carpetas obligatorias del proyecto (Tabla 7).
pub const REQUIRED_PROJECT_DIRS: &[&str] = &["00_ADMIN", "02_SESSIONS"];

/// Subcarpetas de `00_ADMIN` (apartado 7.2). Solo se crean las que se usan; la
/// lista sirve para colocar cada documento en su sitio sin preguntar.
pub const ADMIN_SUBDIRS: &[&str] = &[
    "Contracts", "Rights", "Credits", "Notes", "Score", "Art",
];

/// Carpetas del proyecto que no son obligatorias (Tabla 7). No se crean por
/// anticipado: el apartado 7.1 desaconseja el andamiaje sin contenido.
pub const OPTIONAL_PROJECT_DIRS: &[&str] = &[
    "01_REF",
    "03_RECORDINGS",
    "04_EDIT",
    "05_STEMS",
    "06_MIX",
    "07_MASTER",
    "08_DELIVERY",
    "09_TRANSFER",
];

/// Un repositorio abierto.
#[derive(Clone, Debug)]
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// Abre un repositorio existente.
    pub fn open(root: impl AsRef<Path>) -> Result<Repository> {
        let root = root.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(Error::input(format!(
                "La ruta {} no existe o no es una carpeta. El repositorio no se ha abierto. Comprobar que el volumen está conectado.",
                root.display()
            )));
        }
        Ok(Repository { root })
    }

    /// Crea la jerarquía raíz de un repositorio (apartado 6.1).
    ///
    /// Los cinco dominios se crean siempre: son de primer nivel y su ausencia
    /// impide colocar el material. Dentro de `20_PROJECTS` se crean las cuatro
    /// carpetas de estado. Las carpetas de proyecto, en cambio, se crean bajo
    /// demanda (apartado 7.1).
    pub fn create(root: impl AsRef<Path>) -> Result<Repository> {
        let root = root.as_ref().to_path_buf();
        // La ruta de la raíz no debe contener espacios ni caracteres ajenos al
        // conjunto admitido (apartado 6.5, primer guion).
        if let Some(name) = root.file_name() {
            let n = name.to_string_lossy();
            // El punto inicial de `.stave` es deliberado y admisible.
            crate::naming::check_name(n.trim_start_matches('.'))?;
        }
        for domain in DOMAINS {
            std::fs::create_dir_all(root.join(domain))
                .map_err(|e| Error::io(root.join(domain), e))?;
        }
        for state in PROJECT_STATES {
            let p = root.join("20_PROJECTS").join(state);
            std::fs::create_dir_all(&p).map_err(|e| Error::io(&p, e))?;
        }
        let log_dir = root.join("00_SYSTEM/log");
        std::fs::create_dir_all(&log_dir).map_err(|e| Error::io(&log_dir, e))?;
        Ok(Repository { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn domain(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Cuarentena: único punto de entrada del material externo (apartado 6.1).
    pub fn inbox(&self) -> PathBuf {
        self.root.join("40_INBOX")
    }

    pub fn archive(&self) -> PathBuf {
        self.root.join("30_ARCHIVE")
    }

    pub fn event_log(&self) -> crate::eventlog::EventLog {
        crate::eventlog::EventLog::at(&self.root)
    }

    /// Ruta sugerida de la raíz local: `.stave` en la raíz del directorio
    /// personal (apartado 6.5).
    pub fn suggested_local_root() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)?;
        Some(home.join(LOCAL_ROOT_NAME))
    }

    /// Comprueba que la raíz no esté dentro de una carpeta sincronizada de forma
    /// automática con un servicio de almacenamiento de uso personal
    /// (apartado 6.5, quinto guion).
    ///
    /// La comprobación es por nombre de carpeta. No puede ser exhaustiva; su
    /// finalidad es advertir del caso habitual antes de que se produzca.
    pub fn warn_if_synced_location(root: &Path) -> Option<String> {
        const SERVICIOS: &[&str] = &[
            "Dropbox", "Google Drive", "GoogleDrive", "OneDrive", "iCloud Drive",
            "Mobile Documents", "Box Sync", "MEGA", "pCloud", "Nextcloud", "Syncthing",
        ];
        for componente in root.components() {
            let nombre = componente.as_os_str().to_string_lossy().to_string();
            if let Some(s) = SERVICIOS.iter().find(|s| nombre.eq_ignore_ascii_case(s)) {
                return Some((*s).to_string());
            }
        }
        None
    }

    /// Recorre el árbol y devuelve la ruta de cada proyecto.
    ///
    /// Un proyecto es toda carpeta que contenga un `PROJECT.yaml`. Esta es la
    /// operación que reconstruye el índice desde cero: el apartado 42 exige que
    /// el catálogo sea íntegramente reconstruible a partir del árbol.
    pub fn discover_projects(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for domain in ["20_PROJECTS", "30_ARCHIVE"] {
            find_manifests(&self.root.join(domain), manifest::PROJECT_FILE, &mut out, 0);
        }
        out.sort();
        out
    }

    /// Recorre el árbol y devuelve la ruta de cada release.
    pub fn discover_releases(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for domain in ["20_PROJECTS", "30_ARCHIVE"] {
            find_manifests(&self.root.join(domain), manifest::RELEASE_FILE, &mut out, 0);
        }
        out.sort();
        out
    }

    /// Paquetes de intercambio pendientes de verificación en la cuarentena
    /// (apartado 6.1, último párrafo).
    pub fn quarantined_packages(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(self.inbox()) else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map(|e| e == "stave").unwrap_or(false)
                    || p.join(manifest::EXCHANGE_FILE).exists()
            })
            .collect();
        out.sort();
        out
    }

    /// Elementos de la cuarentena que no son paquetes en verificación.
    ///
    /// El apartado 6.1 exige vaciar `40_INBOX` al menos una vez por semana:
    /// solo pueden permanecer en él los paquetes cuya verificación esté en
    /// curso.
    pub fn inbox_backlog(&self) -> Vec<PathBuf> {
        let paquetes = self.quarantined_packages();
        let Ok(entries) = std::fs::read_dir(self.inbox()) else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| !paquetes.contains(p))
            .filter(|p| {
                !p.file_name()
                    .map(|n| crate::fsx::walk::is_regenerable(&n.to_string_lossy(), p.is_dir()))
                    .unwrap_or(false)
            })
            .collect();
        out.sort();
        out
    }
}

fn find_manifests(dir: &Path, filename: &str, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    let mut hallado = false;
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_file() && entry.file_name() == filename {
            hallado = true;
        } else if ft.is_dir() && !ft.is_symlink() {
            subdirs.push(entry.path());
        }
    }
    if hallado {
        out.push(dir.join(filename));
    }
    // Un release contiene proyectos: el recorrido no se detiene al encontrar un
    // manifiesto.
    for sub in subdirs {
        find_manifests(&sub, filename, out, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn crea_los_cinco_dominios_y_los_cuatro_estados() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join("stave")).unwrap();
        for d in DOMAINS {
            assert!(repo.domain(d).is_dir(), "falta {d}");
        }
        for s in PROJECT_STATES {
            assert!(repo.domain("20_PROJECTS").join(s).is_dir(), "falta {s}");
        }
    }

    #[test]
    fn no_crea_carpetas_de_proyecto_por_anticipado() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join("stave")).unwrap();
        // El apartado 7.1 desaconseja el andamiaje sin contenido.
        for d in OPTIONAL_PROJECT_DIRS {
            assert!(!repo.root().join(d).exists(), "{d} no debía crearse");
        }
    }

    #[test]
    fn descubre_los_proyectos_recorriendo_el_arbol() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join("stave")).unwrap();
        let a = repo.domain("20_PROJECTS").join("1_ACTIVE/2026-08-06_A_ORIG");
        let b = repo.domain("30_ARCHIVE").join("2025-01-01_B_ORIG");
        // Un proyecto dentro de un release.
        let c = repo
            .domain("20_PROJECTS")
            .join("1_ACTIVE/Artista/2026-08-06_Disco_ALBUM/01_Tema");
        for p in [&a, &b, &c] {
            fs::create_dir_all(p).unwrap();
            fs::write(p.join(manifest::PROJECT_FILE), b"uid: X\n").unwrap();
        }
        let hallados = repo.discover_projects();
        assert_eq!(hallados.len(), 3, "{hallados:?}");
        assert!(hallados.contains(&c.join(manifest::PROJECT_FILE)));
    }

    #[test]
    fn descubre_releases_y_sus_proyectos_por_separado() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join("stave")).unwrap();
        let rel = repo
            .domain("20_PROJECTS")
            .join("1_ACTIVE/Artista/2026-08-06_Disco_ALBUM");
        fs::create_dir_all(rel.join("_RELEASE")).unwrap();
        fs::write(rel.join("_RELEASE").join(manifest::RELEASE_FILE), b"uid: R\n").unwrap();
        fs::create_dir_all(rel.join("01_Tema")).unwrap();
        fs::write(rel.join("01_Tema").join(manifest::PROJECT_FILE), b"uid: P\n").unwrap();

        assert_eq!(repo.discover_releases().len(), 1);
        assert_eq!(repo.discover_projects().len(), 1);
    }

    #[test]
    fn distingue_paquetes_en_verificacion_del_atraso_de_la_cuarentena() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::create(dir.path().join("stave")).unwrap();
        fs::write(repo.inbox().join("STAVE-XCHG_2026-08-06_A_B_0007.stave"), b"pk").unwrap();
        fs::write(repo.inbox().join("suelto.wav"), b"audio").unwrap();
        fs::write(repo.inbox().join(".DS_Store"), b"basura").unwrap();

        assert_eq!(repo.quarantined_packages().len(), 1);
        let atraso = repo.inbox_backlog();
        assert_eq!(atraso.len(), 1);
        assert!(atraso[0].ends_with("suelto.wav"));
    }

    #[test]
    fn advierte_de_una_raiz_en_carpeta_sincronizada() {
        assert_eq!(
            Repository::warn_if_synced_location(Path::new("/home/u/Dropbox/stave")),
            Some("Dropbox".to_string())
        );
        assert_eq!(
            Repository::warn_if_synced_location(Path::new("/home/u/OneDrive/musica/.stave")),
            Some("OneDrive".to_string())
        );
        assert!(Repository::warn_if_synced_location(Path::new("/home/u/.stave")).is_none());
    }

    #[test]
    fn la_raiz_local_sugerida_sigue_el_apartado_6_5() {
        if let Some(p) = Repository::suggested_local_root() {
            assert!(p.ends_with(".stave"));
        }
    }

    #[test]
    fn rechaza_una_raiz_con_caracteres_no_admitidos() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Repository::create(dir.path().join("mi repositorio")).is_err());
        assert!(Repository::create(dir.path().join(".stave")).is_ok());
    }
}
