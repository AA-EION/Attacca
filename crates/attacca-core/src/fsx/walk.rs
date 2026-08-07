//! Recorrido del árbol con las exclusiones del Anexo C.
//!
//! Los archivos regenerables no entran en los respaldos, en los manifiestos de
//! integridad ni en los paquetes de intercambio. La exclusión es incondicional:
//! el Anexo C es normativo y el requisito 9 de la aplicación dice «siempre».

use std::path::{Path, PathBuf};

/// Extensiones y nombres de archivos regenerables (Anexo C).
const REGENERABLE_EXT: &[&str] = &[
    // Análisis y formas de onda precalculadas.
    "asd", "pkf", "peak", "ovw", "sfk", "sfap0", "rpp-bak", "reapeaks", "gpk", "wpk",
    // Cachés de reproducción y previsualización.
    "cache", "tmp", "temp",
];

/// Nombres exactos de archivos generados por el gestor de archivos del sistema
/// operativo (Anexo C, último guion).
const OS_METADATA_FILES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini", ".directory"];

/// Directorios cuyo contenido es regenerable en su totalidad.
const REGENERABLE_DIRS: &[&str] = &[
    "Audio Files Cache",
    "Freeze Files",
    "Peak Files",
    "Waveforms",
    "Analysis Files",
    "__MACOSX",
    ".Spotlight-V100",
    ".Trashes",
    "$RECYCLE.BIN",
    "System Volume Information",
];

/// Sufijos de copia de seguridad automática de sesión (Anexo C, cuarto guion).
const BACKUP_SUFFIXES: &[&str] = &[".bak", ".bk", "-bak", ".autosave", ".backup"];

/// Directorios de copia de seguridad automática de las estaciones de trabajo.
const BACKUP_DIRS: &[&str] = &[
    "Session File Backups",
    "Auto-Saves",
    "Backup",
    "Backups",
    "Project Backups",
];

/// Archivos propios de Attacca que no forman parte del material.
const APP_INTERNAL: &[&str] = &[".attacca-index", ".attacca-journal"];

/// Indica si un nombre corresponde a un elemento regenerable del Anexo C.
pub fn is_regenerable(name: &str, is_dir: bool) -> bool {
    if OS_METADATA_FILES.contains(&name) || APP_INTERNAL.contains(&name) {
        return true;
    }
    if name.ends_with(crate::fsx::atomic::TEMP_SUFFIX) {
        return true;
    }
    if is_dir {
        return REGENERABLE_DIRS.contains(&name) || BACKUP_DIRS.contains(&name);
    }
    let lower = name.to_ascii_lowercase();
    if BACKUP_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
        return true;
    }
    if let Some(ext) = lower.rsplit_once('.').map(|(_, e)| e) {
        if REGENERABLE_EXT.contains(&ext) {
            return true;
        }
    }
    false
}

/// Archivo encontrado durante el recorrido.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Ruta absoluta.
    pub path: PathBuf,
    /// Ruta relativa a la raíz del recorrido, con barra inclinada como
    /// separador. Es la forma exigida en los manifiestos de integridad y en los
    /// contenedores.
    pub relative: String,
    pub size: u64,
}

/// Recorre un árbol devolviendo los archivos conservables, en orden estable.
///
/// El orden lexicográfico de la ruta relativa hace que el manifiesto de
/// integridad y el contenedor sean deterministas: dos empaquetados del mismo
/// contenido producen la misma lista.
pub fn conserved_files(root: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    walk(root, root, &mut out, 0);
    out.sort_by(|a, b| a.relative.cmp(&b.relative));
    out
}

/// Todos los archivos del árbol, sin aplicar las exclusiones del Anexo C. Se
/// emplea para comprobar descriptores abiertos, donde la exclusión no procede:
/// una caché abierta por el programa de audio sigue indicando actividad.
pub fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_all(root, &mut out, 0);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Entry>, depth: usize) {
    if depth > 32 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(ft) = entry.file_type() else { continue };
        // Los enlaces simbólicos no se siguen: un enlace puede apuntar fuera del
        // proyecto y romper la atomicidad del apartado 4, principio 1.
        if ft.is_symlink() {
            continue;
        }
        if is_regenerable(&name, ft.is_dir()) {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            walk(root, &path, out, depth + 1);
        } else if ft.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if let Some(relative) = relative_slash(root, &path) {
                out.push(Entry { path, relative, size });
            }
        }
    }
}

fn walk_all(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 32 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ft.is_dir() {
            walk_all(&path, out, depth + 1);
        } else if ft.is_file() {
            out.push(path);
        }
    }
}

/// Ruta relativa con barra inclinada, conforme al apartado 32.4.2.
pub fn relative_slash(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut s = String::new();
    for component in rel.components() {
        if let std::path::Component::Normal(part) = component {
            if !s.is_empty() {
                s.push('/');
            }
            s.push_str(&part.to_string_lossy());
        }
    }
    Some(s)
}

/// Tamaño total de un conjunto de entradas.
pub fn total_bytes(entries: &[Entry]) -> u64 {
    entries.iter().map(|e| e.size).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn excluye_los_regenerables_del_anexo_c() {
        assert!(is_regenerable(".DS_Store", false));
        assert!(is_regenerable("Thumbs.db", false));
        assert!(is_regenerable("mezcla.asd", false));
        assert!(is_regenerable("sesion.rpp-bak", false));
        assert!(is_regenerable("proyecto.bak", false));
        assert!(is_regenerable("Freeze Files", true));
        assert!(is_regenerable("Session File Backups", true));
        assert!(!is_regenerable("mezcla.wav", false));
        assert!(!is_regenerable("PROJECT.yaml", false));
        assert!(!is_regenerable("02_SESSIONS", true));
    }

    #[test]
    fn el_recorrido_omite_los_regenerables_y_es_estable() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("02_SESSIONS/Reaper/Freeze Files")).unwrap();
        fs::write(dir.path().join("PROJECT.yaml"), b"uid: A").unwrap();
        fs::write(dir.path().join("02_SESSIONS/Reaper/s.rpp"), b"sesion").unwrap();
        fs::write(dir.path().join("02_SESSIONS/Reaper/s.rpp-bak"), b"copia").unwrap();
        fs::write(dir.path().join("02_SESSIONS/Reaper/Freeze Files/f.wav"), b"x").unwrap();
        fs::write(dir.path().join(".DS_Store"), b"basura").unwrap();

        let files = conserved_files(dir.path());
        let rutas: Vec<&str> = files.iter().map(|e| e.relative.as_str()).collect();
        assert_eq!(rutas, vec!["02_SESSIONS/Reaper/s.rpp", "PROJECT.yaml"]);
        assert_eq!(conserved_files(dir.path()).len(), files.len());
    }

    #[test]
    fn la_ruta_relativa_emplea_barra_inclinada() {
        let root = Path::new("/repo");
        let p = Path::new("/repo/a/b/c.wav");
        assert_eq!(relative_slash(root, p).unwrap(), "a/b/c.wav");
    }

    #[test]
    fn suma_los_tamanos() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), vec![0u8; 100]).unwrap();
        fs::write(dir.path().join("b"), vec![0u8; 55]).unwrap();
        assert_eq!(total_bytes(&conserved_files(dir.path())), 155);
    }
}
