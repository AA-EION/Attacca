//! Acceso al sistema de archivos.
//!
//! El apartado 42 prohíbe interponer entre el material y el sistema de archivos
//! una capa que solo Attacca sea capaz de interpretar. Todo lo que hay aquí son
//! operaciones corrientes sobre archivos corrientes: no hay formato propio, no
//! hay base de datos autoritativa y no hay indirección.

pub mod atomic;
pub mod openfiles;
pub mod readonly;
pub mod space;
pub mod walk;

use crate::error::{Error, Result};
use std::path::Path;

/// Abre una ruta en el explorador de archivos del sistema operativo.
///
/// El apartado 44.2 exige ofrecer esta acción en todo momento: ocultar la ruta
/// es guiar, impedirla es incumplir.
pub fn reveal_in_file_manager(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(Error::input(format!(
            "La ruta {} no existe. No se ha abierto nada.",
            path.display()
        )));
    }
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map_err(|e| Error::io(path, e))?;
    Ok(())
}

/// Copia un árbol completo aplicando las exclusiones del Anexo C.
///
/// Se emplea en la derivación de proyecto (apartado 14.4.2) y en la constitución
/// de paquetes. Devuelve el número de archivos copiados.
pub fn copy_tree(src: &Path, dst: &Path, skip_dirs: &[&str]) -> Result<usize> {
    let mut copied = 0usize;
    copy_dir(src, dst, skip_dirs, &mut copied, 0)?;
    Ok(copied)
}

fn copy_dir(
    src: &Path,
    dst: &Path,
    skip_dirs: &[&str],
    copied: &mut usize,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Ok(());
    }
    std::fs::create_dir_all(dst).map_err(|e| Error::io(dst, e))?;
    let entries = std::fs::read_dir(src).map_err(|e| Error::io(src, e))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() || walk::is_regenerable(&name, ft.is_dir()) {
            continue;
        }
        if ft.is_dir() {
            if skip_dirs.contains(&name.as_str()) {
                continue;
            }
            copy_dir(&entry.path(), &dst.join(&name), skip_dirs, copied, depth + 1)?;
        } else if ft.is_file() {
            let target = dst.join(&name);
            std::fs::copy(entry.path(), &target).map_err(|e| Error::io(&target, e))?;
            // La copia hereda los permisos del origen. Un proyecto de origen ya
            // sellado produciría un derivado inmodificable, de modo que se
            // restituye la escritura sobre la copia.
            let _ = readonly::set_file_readonly(&target, false);
            *copied += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn la_copia_omite_regenerables_y_carpetas_excluidas() {
        let origen = tempfile::tempdir().unwrap();
        fs::create_dir_all(origen.path().join("08_DELIVERY/2026-08-06_Sello")).unwrap();
        fs::create_dir_all(origen.path().join("00_ADMIN/Notes")).unwrap();
        fs::write(origen.path().join("PROJECT.yaml"), b"uid: A").unwrap();
        fs::write(origen.path().join("00_ADMIN/Notes/n.md"), b"notas").unwrap();
        fs::write(origen.path().join("00_ADMIN/.DS_Store"), b"basura").unwrap();
        fs::write(origen.path().join("08_DELIVERY/2026-08-06_Sello/p.zip"), b"pkg").unwrap();

        let destino = tempfile::tempdir().unwrap();
        let dst = destino.path().join("derivado");
        // El apartado 14.4.2 excluye de la derivación los paquetes ya emitidos.
        let n = copy_tree(origen.path(), &dst, &["08_DELIVERY", "09_TRANSFER"]).unwrap();

        assert_eq!(n, 2);
        assert!(dst.join("PROJECT.yaml").exists());
        assert!(dst.join("00_ADMIN/Notes/n.md").exists());
        assert!(!dst.join("00_ADMIN/.DS_Store").exists());
        assert!(!dst.join("08_DELIVERY").exists());
    }

    #[test]
    fn la_copia_restituye_la_escritura_sobre_el_destino() {
        let origen = tempfile::tempdir().unwrap();
        let f = origen.path().join("a.txt");
        fs::write(&f, b"x").unwrap();
        readonly::set_file_readonly(&f, true).unwrap();

        let destino = tempfile::tempdir().unwrap();
        let dst = destino.path().join("copia");
        copy_tree(origen.path(), &dst, &[]).unwrap();
        assert!(!fs::metadata(dst.join("a.txt")).unwrap().permissions().readonly());
    }
}
