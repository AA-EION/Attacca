//! Escritura atómica (apartados 32.4.4 y 9 de los requisitos de robustez).
//!
//! Nunca debe quedar un manifiesto a medio escribir. La secuencia es siempre la
//! misma: archivo temporal en el mismo volumen, sincronización a disco,
//! verificación del contenido escrito y renombrado sobre el destino.

use crate::error::{Error, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Sufijo de los archivos temporales. Se reconoce al arrancar para detectar
/// operaciones interrumpidas (recuperación tras cierre inesperado).
pub const TEMP_SUFFIX: &str = ".attacca-tmp";

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Escribe un archivo de forma atómica respecto de su destino.
///
/// El temporal se crea en el mismo directorio para garantizar que el renombrado
/// ocurre dentro del mismo volumen: un renombrado entre volúmenes se degrada a
/// copia y deja de ser atómico.
pub fn write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        Error::input(format!(
            "La ruta {} no tiene directorio contenedor. No se ha escrito nada.",
            path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;

    let temp = temp_path(path);
    {
        let mut f = File::create(&temp).map_err(|e| Error::io(&temp, e))?;
        f.write_all(contents).map_err(|e| Error::io(&temp, e))?;
        f.flush().map_err(|e| Error::io(&temp, e))?;
        // La sincronización debe preceder al renombrado: sin ella, un corte de
        // alimentación puede dejar el nombre definitivo apuntando a un contenido
        // parcial.
        f.sync_all().map_err(|e| Error::io(&temp, e))?;
    }

    // Verificación antes de situar el archivo en su destino.
    let written = fs::read(&temp).map_err(|e| Error::io(&temp, e))?;
    if written != contents {
        let _ = fs::remove_file(&temp);
        return Err(Error::Integrity {
            checked: 1,
            failed: vec![path.display().to_string()],
        });
    }

    // Un destino en solo lectura impide el renombrado en Windows. El bloqueo se
    // levanta solo el tiempo estrictamente necesario y se restituye después.
    let restore_readonly = clear_readonly_if_needed(path);

    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        Error::io(path, e)
    })?;

    if restore_readonly {
        let _ = super::readonly::set_file_readonly(path, true);
    }

    sync_dir(parent);
    Ok(())
}

/// Escribe texto de forma atómica.
pub fn write_str(path: &Path, contents: &str) -> Result<()> {
    write(path, contents.as_bytes())
}

fn temp_path(path: &Path) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "archivo".to_string());
    path.with_file_name(format!(".{name}.{pid}.{n}{TEMP_SUFFIX}"))
}

fn clear_readonly_if_needed(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if meta.permissions().readonly() {
        super::readonly::set_file_readonly(path, false).is_ok()
    } else {
        false
    }
}

/// Sincroniza la entrada de directorio para que el renombrado sea duradero.
/// En los sistemas que no admiten abrir un directorio, la llamada se omite.
fn sync_dir(dir: &Path) {
    if let Ok(f) = File::open(dir) {
        let _ = f.sync_all();
    }
}

/// Localiza temporales abandonados por una operación interrumpida.
pub fn find_abandoned(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &mut out, 0);
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 24 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            collect(&path, out, depth + 1);
        } else if path
            .file_name()
            .map(|n| n.to_string_lossy().ends_with(TEMP_SUFFIX))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escribe_y_sustituye_sin_dejar_temporales() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("PROJECT.yaml");
        write_str(&destino, "uid: A\n").unwrap();
        assert_eq!(fs::read_to_string(&destino).unwrap(), "uid: A\n");
        write_str(&destino, "uid: B\n").unwrap();
        assert_eq!(fs::read_to_string(&destino).unwrap(), "uid: B\n");
        assert!(find_abandoned(dir.path()).is_empty());
    }

    #[test]
    fn crea_el_directorio_contenedor() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("a/b/c/PROJECT.yaml");
        write_str(&destino, "x: 1\n").unwrap();
        assert!(destino.exists());
    }

    #[test]
    fn detecta_temporales_abandonados() {
        let dir = tempfile::tempdir().unwrap();
        let huerfano = dir.path().join(format!(".PROJECT.yaml.1.0{TEMP_SUFFIX}"));
        fs::write(&huerfano, b"parcial").unwrap();
        let encontrados = find_abandoned(dir.path());
        assert_eq!(encontrados.len(), 1);
        assert_eq!(encontrados[0], huerfano);
    }

    #[test]
    fn sustituye_un_destino_en_solo_lectura() {
        let dir = tempfile::tempdir().unwrap();
        let destino = dir.path().join("CUSTODY.lock");
        write_str(&destino, "a\n").unwrap();
        super::super::readonly::set_file_readonly(&destino, true).unwrap();
        write_str(&destino, "b\n").unwrap();
        assert_eq!(fs::read_to_string(&destino).unwrap(), "b\n");
    }
}
