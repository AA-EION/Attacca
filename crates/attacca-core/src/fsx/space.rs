//! Espacio disponible (requisito 9 de robustez).
//!
//! Debe comprobarse antes de empaquetar o sincronizar, no durante. Un paquete
//! interrumpido por agotamiento de espacio es el caso que el apartado 32.4.4
//! describe.

use crate::error::{Error, Result};
use std::path::Path;

/// Margen que se reserva sobre el tamaño estimado. Un empaquetado escribe el
/// contenedor y su temporal de verificación, y el sistema de archivos necesita
/// espacio para sus propias estructuras.
const MARGIN_RATIO: f64 = 1.15;

/// Espacio libre en el volumen que contiene la ruta, en bytes.
pub fn available_bytes(path: &Path) -> Option<u64> {
    platform::available_bytes(path)
}

/// Comprueba que el volumen admita una escritura del tamaño indicado.
///
/// Cuando el espacio libre no puede determinarse, la comprobación no bloquea: el
/// requisito es comprobar antes de escribir, no impedir trabajar en un sistema
/// de archivos que no informa de su capacidad.
pub fn ensure_available(path: &Path, required: u64) -> Result<()> {
    let Some(available) = available_bytes(path) else {
        return Ok(());
    };
    let with_margin = (required as f64 * MARGIN_RATIO) as u64;
    if available < with_margin {
        return Err(Error::Space {
            required: with_margin,
            available,
        });
    }
    Ok(())
}

#[cfg(unix)]
mod platform {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn available_bytes(path: &Path) -> Option<u64> {
        // Se consulta el directorio existente más próximo: el destino de una
        // escritura puede no existir todavía.
        let mut probe = path;
        while !probe.exists() {
            probe = probe.parent()?;
        }
        let c = CString::new(probe.as_os_str().as_bytes()).ok()?;
        // SEGURIDAD: `statvfs` solo lee; `stat` se inicializa a cero y la
        // llamada la rellena por completo cuando devuelve 0.
        let mut stat: libc_statvfs = unsafe { std::mem::zeroed() };
        let rc = unsafe { statvfs(c.as_ptr(), &mut stat) };
        if rc != 0 {
            return None;
        }
        // `f_bavail` son los bloques disponibles para un proceso sin privilegios,
        // que es la cifra que corresponde a esta comprobación.
        Some(stat.f_bavail.saturating_mul(stat.f_frsize as u64))
    }

    // Declaración mínima de `statvfs`. Evita arrastrar la dependencia `libc`
    // para dos campos. La disposición sigue la interfaz binaria de Linux y de
    // macOS en 64 bits.
    #[cfg(target_os = "linux")]
    #[repr(C)]
    #[derive(Default)]
    #[allow(non_camel_case_types)]
    struct libc_statvfs {
        f_bsize: u64,
        f_frsize: u64,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_favail: u64,
        f_fsid: u64,
        f_flag: u64,
        f_namemax: u64,
        __reserved: [i32; 6],
    }

    #[cfg(target_os = "macos")]
    #[repr(C)]
    #[derive(Default)]
    #[allow(non_camel_case_types)]
    struct libc_statvfs {
        f_bsize: u64,
        f_frsize: u64,
        f_blocks: u32,
        f_bfree: u32,
        f_bavail: u32,
        f_files: u32,
        f_ffree: u32,
        f_favail: u32,
        f_fsid: u64,
        f_flag: u64,
        f_namemax: u64,
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[repr(C)]
    #[derive(Default)]
    #[allow(non_camel_case_types)]
    struct libc_statvfs {
        f_bsize: u64,
        f_frsize: u64,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_favail: u64,
        f_fsid: u64,
        f_flag: u64,
        f_namemax: u64,
    }

    extern "C" {
        fn statvfs(path: *const std::os::raw::c_char, buf: *mut libc_statvfs) -> i32;
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    pub fn available_bytes(path: &Path) -> Option<u64> {
        let mut probe = path;
        while !probe.exists() {
            probe = probe.parent()?;
        }
        let wide: Vec<u16> = probe
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_for_caller: u64 = 0;
        // SEGURIDAD: la cadena termina en cero y vive durante la llamada; los
        // punteros de salida no exigidos se pasan como nulos, que la interfaz
        // admite.
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_for_caller,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            None
        } else {
            Some(free_for_caller)
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::path::Path;
    pub fn available_bytes(_path: &Path) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn informa_del_espacio_del_volumen() {
        let dir = tempfile::tempdir().unwrap();
        let libre = available_bytes(dir.path());
        if let Some(bytes) = libre {
            assert!(bytes > 0, "el volumen informa de 0 bytes libres");
        }
    }

    #[test]
    fn admite_una_escritura_pequena_y_rechaza_una_desmedida() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ensure_available(dir.path(), 1024).is_ok());
        if available_bytes(dir.path()).is_some() {
            let r = ensure_available(dir.path(), u64::MAX / 2);
            assert!(matches!(r, Err(Error::Space { .. })));
        }
    }

    #[test]
    fn consulta_una_ruta_que_todavia_no_existe() {
        let dir = tempfile::tempdir().unwrap();
        let futuro = dir.path().join("a/b/c/paquete.stave");
        assert!(ensure_available(&futuro, 1024).is_ok());
    }
}
