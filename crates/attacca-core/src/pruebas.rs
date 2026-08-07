//! Andamiaje común a las pruebas. No forma parte de la superficie normativa:
//! el módulo solo se compila al compilar pruebas.
//!
//! Las pruebas de integración lo incluyen con `#[path]`, de modo que hay una
//! sola copia de estas funciones.

#![allow(dead_code)]

use std::path::PathBuf;

/// Directorio temporal con la raíz más corta que ofrezca el sistema.
///
/// El presupuesto de ruta del apartado 9.1 son 200 caracteres, contados desde
/// la raíz del repositorio. El directorio temporal de algunas máquinas consume
/// ya una tercera parte —el de macOS es de la forma
/// `/var/folders/3s/j8k2m4n90qz5x7cvbn3lp2rw0000gn/T`—, con lo que una prueba
/// que no mide el presupuesto de ruta fallaría por él y ocultaría lo que sí
/// mide. Las pruebas que comprueban el presupuesto lo hacen a propósito y
/// construyen su propia raíz.
pub fn raiz_temporal() -> std::io::Result<tempfile::TempDir> {
    for base in bases() {
        if !base.is_dir() {
            continue;
        }
        if let Ok(dir) = tempfile::Builder::new().prefix("at").tempdir_in(&base) {
            return Ok(dir);
        }
    }
    tempfile::Builder::new().prefix("at").tempdir()
}

/// Bases candidatas, de la más corta a la más larga.
fn bases() -> Vec<PathBuf> {
    let mut candidatas = Vec::new();
    if cfg!(windows) {
        // No se crea: si la máquina no la tiene, se pasa a la siguiente.
        candidatas.push(PathBuf::from("C:\\Temp"));
    } else {
        candidatas.push(PathBuf::from("/tmp"));
    }
    candidatas.push(std::env::temp_dir());
    candidatas
}
