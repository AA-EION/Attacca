//! Solo lectura por medios técnicos (apartados 6.6.2, 14.3.3 y 27.2).
//!
//! La norma exige repetidamente que el bloqueo sea «por medios técnicos, y no
//! únicamente por convención». Cuando el sistema de archivos no puede
//! aplicarlo —exFAT en un disco externo, por ejemplo—, la limitación debe
//! declararse en el descriptor de volumen y suplirse mediante el archivo
//! marcador. Este módulo aplica el bloqueo y comprueba si el volumen lo sostiene.

use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

/// Aplica o retira el atributo de solo lectura de un archivo.
pub fn set_file_readonly(path: &Path, readonly: bool) -> Result<()> {
    let meta = fs::metadata(path).map_err(|e| Error::io(path, e))?;
    let mut perms = meta.permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.mode();
        // Se retiran o restituyen los bits de escritura conservando el resto,
        // incluidos los de ejecución de los directorios.
        let nuevo = if readonly { mode & !0o222 } else { mode | 0o200 };
        perms.set_mode(nuevo);
    }
    #[cfg(not(unix))]
    {
        perms.set_readonly(readonly);
    }

    fs::set_permissions(path, perms).map_err(|e| Error::io(path, e))
}

/// Aplica el régimen de solo lectura a un árbol completo.
///
/// Los directorios conservan el permiso de recorrido: la norma exige impedir la
/// modificación, no el acceso. El apartado 14.3.3 permite de forma expresa
/// consultar el proyecto, reproducir su material y copiarlo mientras la custodia
/// no sea propia.
pub fn set_tree_readonly(root: &Path, readonly: bool, exclude: &[&str]) -> Result<usize> {
    let mut count = 0usize;
    let mut stack = vec![root.to_path_buf()];
    let mut dirs = Vec::new();

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if exclude.contains(&name.as_str()) {
                continue;
            }
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => {
                    stack.push(path.clone());
                    dirs.push(path);
                }
                Ok(ft) if ft.is_file() => {
                    set_file_readonly(&path, readonly)?;
                    count += 1;
                }
                // Los enlaces simbólicos no se modifican: el atributo pertenece
                // al destino, que puede estar fuera del proyecto.
                _ => {}
            }
        }
    }

    // Los directorios se tratan al final y de dentro hacia fuera: retirar antes
    // el permiso de escritura de un directorio impediría recorrer su contenido.
    for dir in dirs.into_iter().rev() {
        set_dir_readonly(&dir, readonly)?;
    }
    set_dir_readonly(root, readonly)?;
    Ok(count)
}

fn set_dir_readonly(path: &Path, readonly: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(path).map_err(|e| Error::io(path, e))?;
        let mode = meta.permissions().mode();
        // Se conserva el bit de ejecución, que en un directorio es el permiso de
        // recorrido.
        let nuevo = if readonly {
            (mode & !0o222) | 0o500
        } else {
            mode | 0o700
        };
        let mut perms = meta.permissions();
        perms.set_mode(nuevo);
        fs::set_permissions(path, perms).map_err(|e| Error::io(path, e))
    }
    #[cfg(not(unix))]
    {
        // En Windows el atributo de solo lectura de un directorio no impide
        // crear archivos dentro. El bloqueo efectivo recae sobre los archivos y
        // sobre las listas de control de acceso; el marcador del apartado 6.6.5
        // o el del 14.3.3 lo suple ante la persona usuaria.
        let _ = (path, readonly);
        Ok(())
    }
}

/// Capacidad del sistema de archivos para sostener el atributo de solo lectura
/// (campo `filesystem.enforces_readonly` del Anexo B.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsCapabilities {
    /// El atributo de solo lectura se aplica y se relee.
    pub enforces_readonly: bool,
    /// El sistema de archivos distingue mayúsculas de minúsculas.
    pub preserves_case: bool,
}

/// Comprueba las capacidades escribiendo un archivo de prueba en el directorio
/// indicado. El archivo se elimina siempre, incluso si la comprobación falla.
pub fn probe(dir: &Path) -> FsCapabilities {
    let mut caps = FsCapabilities {
        enforces_readonly: false,
        preserves_case: false,
    };
    let probe_path = dir.join(".attacca-probe-Rw");
    if fs::write(&probe_path, b"probe").is_err() {
        return caps;
    }

    // Solo lectura: se aplica el atributo y se comprueba que la escritura
    // posterior es rechazada.
    if set_file_readonly(&probe_path, true).is_ok() {
        let denied = fs::OpenOptions::new()
            .write(true)
            .open(&probe_path)
            .is_err();
        // La comprobación no es concluyente cuando el proceso es administrador:
        // en ese caso se toma el atributo releído como indicio.
        let attr = fs::metadata(&probe_path)
            .map(|m| m.permissions().readonly())
            .unwrap_or(false);
        caps.enforces_readonly = denied || attr;
        let _ = set_file_readonly(&probe_path, false);
    }

    // Distinción de mayúsculas: si el nombre en minúsculas resuelve al mismo
    // archivo, el sistema no distingue.
    let lower = dir.join(".attacca-probe-rw");
    caps.preserves_case = !lower.exists();

    let _ = fs::remove_file(&probe_path);
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aplica_y_retira_el_bloqueo_de_un_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.txt");
        fs::write(&f, b"x").unwrap();
        set_file_readonly(&f, true).unwrap();
        assert!(fs::metadata(&f).unwrap().permissions().readonly());
        set_file_readonly(&f, false).unwrap();
        assert!(!fs::metadata(&f).unwrap().permissions().readonly());
    }

    #[test]
    fn bloquea_el_arbol_conservando_la_lectura() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("02_SESSIONS/proto");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("sesion.txt"), b"datos").unwrap();
        fs::write(dir.path().join("PROJECT.yaml"), b"uid: A").unwrap();

        let n = set_tree_readonly(dir.path(), true, &["PROJECT.yaml"]).unwrap();
        assert_eq!(n, 1, "el manifiesto queda excluido del bloqueo");

        // El material sigue siendo legible: el apartado 14.3.3 lo exige.
        assert_eq!(fs::read(sub.join("sesion.txt")).unwrap(), b"datos");
        assert!(fs::metadata(sub.join("sesion.txt")).unwrap().permissions().readonly());
        // El manifiesto excluido sigue admitiendo escritura.
        assert!(!fs::metadata(dir.path().join("PROJECT.yaml")).unwrap().permissions().readonly());

        set_tree_readonly(dir.path(), false, &[]).unwrap();
        assert!(fs::write(sub.join("sesion.txt"), b"otros").is_ok());
    }

    #[test]
    fn la_sonda_no_deja_residuos() {
        let dir = tempfile::tempdir().unwrap();
        let caps = probe(dir.path());
        let restos: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".attacca-probe"))
            .collect();
        assert!(restos.is_empty(), "la sonda dejó {} archivos", restos.len());
        // En un sistema de archivos corriente de Linux ambas capacidades se
        // sostienen; la aserción se limita a que la sonda produzca un resultado.
        let _ = caps.enforces_readonly;
    }
}
