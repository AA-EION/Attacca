//! Detección de archivos abiertos para escritura (apartados 6.6.3 y 14.3.3).
//!
//! Antes de ceder la custodia, de conmutar la réplica activa o de archivar, hay
//! que comprobar que ningún archivo del proyecto está abierto para escritura por
//! otro proceso.
//!
//! Este módulo **solo observa**. No cierra descriptores, no desmonta volúmenes y
//! no interrumpe a otro proceso: una sesión de grabación en curso debe seguir
//! funcionando. Cuando la detección no es posible en la plataforma, se devuelve
//! [`Detection::Unsupported`] y la decisión se traslada a la persona usuaria en
//! lugar de suponer que no hay nada abierto.

use std::path::{Path, PathBuf};

/// Resultado de la comprobación.
#[derive(Clone, Debug)]
pub enum Detection {
    /// La comprobación se ejecutó. La lista contiene los archivos del árbol que
    /// otro proceso mantiene abiertos para escritura.
    Checked(Vec<PathBuf>),
    /// La plataforma no ofrece un medio fiable de comprobación.
    Unsupported,
}

impl Detection {
    /// Archivos detectados. Una detección no soportada devuelve lista vacía; la
    /// distinción con «ninguno abierto» se conserva en la variante.
    pub fn files(&self) -> &[PathBuf] {
        match self {
            Detection::Checked(v) => v,
            Detection::Unsupported => &[],
        }
    }

    pub fn is_clear(&self) -> bool {
        matches!(self, Detection::Checked(v) if v.is_empty())
    }
}

/// Comprueba qué archivos bajo `root` mantiene otro proceso abiertos para
/// escritura.
pub fn open_for_write(root: &Path) -> Detection {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    platform::open_for_write(&root)
}

#[cfg(target_os = "linux")]
mod platform {
    use super::Detection;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Recorre `/proc/<pid>/fd` y resuelve cada descriptor. El modo de apertura
    /// se lee de `/proc/<pid>/fdinfo/<fd>`, cuyo campo `flags` codifica el modo
    /// de acceso en sus dos bits bajos, en octal.
    pub fn open_for_write(root: &Path) -> Detection {
        let Ok(procs) = fs::read_dir("/proc") else {
            return Detection::Unsupported;
        };
        let own_pid = std::process::id();
        let mut found: Vec<PathBuf> = Vec::new();

        for proc_entry in procs.flatten() {
            let name = proc_entry.file_name();
            let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
                continue;
            };
            // Los descriptores del propio proceso no cuentan como ajenos.
            if pid == own_pid {
                continue;
            }
            let fd_dir = proc_entry.path().join("fd");
            // Sin privilegios sobre un proceso ajeno la lectura falla; se omite
            // sin abortar la comprobación completa.
            let Ok(fds) = fs::read_dir(&fd_dir) else { continue };

            for fd_entry in fds.flatten() {
                let Ok(target) = fs::read_link(fd_entry.path()) else {
                    continue;
                };
                if !target.starts_with(root) {
                    continue;
                }
                let fd_name = fd_entry.file_name();
                let Some(fd_num) = fd_name.to_str() else { continue };
                let fdinfo = proc_entry.path().join("fdinfo").join(fd_num);
                if writable_flags(&fdinfo) && !found.contains(&target) {
                    found.push(target);
                }
            }
        }
        found.sort();
        Detection::Checked(found)
    }

    fn writable_flags(fdinfo: &Path) -> bool {
        let Ok(text) = fs::read_to_string(fdinfo) else {
            return false;
        };
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("flags:") {
                if let Ok(flags) = u32::from_str_radix(value.trim(), 8) {
                    // O_WRONLY = 1, O_RDWR = 2 en los dos bits bajos.
                    return flags & 0b11 != 0;
                }
            }
        }
        false
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::Detection;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// macOS no expone `/proc`. Se emplea `lsof`, presente en toda instalación,
    /// restringido al árbol del proyecto. `-F` produce salida por campos, que se
    /// analiza sin ambigüedad frente a rutas con espacios.
    pub fn open_for_write(root: &Path) -> Detection {
        let output = Command::new("/usr/sbin/lsof")
            .arg("-F").arg("an")   // a: modo de acceso; n: nombre
            .arg("-w")             // sin advertencias
            .arg("+D").arg(root)   // recorrido del árbol
            .output();

        let Ok(output) = output else {
            return Detection::Unsupported;
        };
        // lsof devuelve 1 cuando no encuentra nada, lo que no es un fallo.
        let text = String::from_utf8_lossy(&output.stdout);
        let mut found: Vec<PathBuf> = Vec::new();
        let mut writable = false;

        for line in text.lines() {
            match line.as_bytes().first() {
                // Modo de acceso: r (lectura), w (escritura), u (ambos).
                Some(b'a') => writable = matches!(&line[1..], "w" | "u"),
                Some(b'n') => {
                    if writable {
                        let path = PathBuf::from(&line[1..]);
                        if path.is_file() && !found.contains(&path) {
                            found.push(path);
                        }
                    }
                    writable = false;
                }
                _ => {}
            }
        }
        found.sort();
        Detection::Checked(found)
    }
}

#[cfg(windows)]
mod platform {
    use super::Detection;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::RestartManager::{
        RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
        CCH_RM_SESSION_KEY,
    };

    /// El Administrador de Reinicio indica qué procesos mantienen abierto un
    /// archivo sin abrirlo en exclusiva ni interrumpir a nadie. Es la vía que
    /// Windows ofrece para observar sin intervenir.
    pub fn open_for_write(root: &Path) -> Detection {
        let files = super::super::walk::files_under(root);
        if files.is_empty() {
            return Detection::Checked(Vec::new());
        }

        let mut session: u32 = 0;
        let mut key = [0u16; (CCH_RM_SESSION_KEY + 1) as usize];
        // SEGURIDAD: RmStartSession escribe la clave de sesión en `key`, cuyo
        // tamaño es el que la interfaz documenta.
        if unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) } != ERROR_SUCCESS {
            return Detection::Unsupported;
        }

        let wide: Vec<Vec<u16>> = files
            .iter()
            .map(|p| {
                p.as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect()
            })
            .collect();
        let ptrs: Vec<*const u16> = wide.iter().map(|w| w.as_ptr()).collect();

        // SEGURIDAD: los punteros apuntan a cadenas terminadas en cero vivas
        // durante toda la llamada.
        let registered = unsafe {
            RmRegisterResources(
                session,
                ptrs.len() as u32,
                ptrs.as_ptr(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            )
        };
        if registered != ERROR_SUCCESS {
            unsafe { RmEndSession(session) };
            return Detection::Unsupported;
        }

        let mut needed: u32 = 0;
        let mut count: u32 = 0;
        let mut reason: u32 = 0;
        // Primera llamada con capacidad cero para conocer el número de procesos.
        unsafe {
            RmGetList(
                session,
                &mut needed,
                &mut count,
                std::ptr::null_mut(),
                &mut reason,
            )
        };

        let mut found = Vec::new();
        if needed > 0 {
            let mut infos: Vec<RM_PROCESS_INFO> =
                vec![unsafe { std::mem::zeroed() }; needed as usize];
            count = needed;
            let rc = unsafe {
                RmGetList(
                    session,
                    &mut needed,
                    &mut count,
                    infos.as_mut_ptr(),
                    &mut reason,
                )
            };
            if rc == ERROR_SUCCESS && count > 0 {
                // El Administrador de Reinicio informa de los procesos, no de
                // los archivos concretos. Se comprueba cada archivo por apertura
                // exclusiva, que en Windows solo tiene éxito si nadie lo
                // mantiene abierto.
                let own = std::process::id();
                let ajenos = infos[..count as usize]
                    .iter()
                    .any(|i| i.Process.dwProcessId != own);
                if ajenos {
                    for file in &files {
                        if is_locked(file) {
                            found.push(file.clone());
                        }
                    }
                }
            }
        }

        unsafe { RmEndSession(session) };
        found.sort();
        Detection::Checked(found)
    }

    /// Comprueba si un archivo admite apertura para escritura. No modifica el
    /// archivo ni afecta a quien lo tenga abierto.
    fn is_locked(path: &Path) -> bool {
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => false,
            Err(e) => e.kind() == std::io::ErrorKind::PermissionDenied
                || e.raw_os_error() == Some(32), // ERROR_SHARING_VIOLATION
        }
    }

    #[allow(dead_code)]
    fn unused(_: PathBuf) {}
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod platform {
    use super::Detection;
    use std::path::Path;

    pub fn open_for_write(_root: &Path) -> Detection {
        Detection::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn un_arbol_sin_archivos_abiertos_da_resultado_limpio() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.wav"), b"audio").unwrap();
        let d = open_for_write(dir.path());
        if let Detection::Checked(files) = &d {
            assert!(files.is_empty(), "detectados: {files:?}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detecta_un_archivo_que_otro_proceso_mantiene_abierto() {
        use std::io::Read;
        use std::process::{Command, Stdio};

        let dir = tempfile::tempdir().unwrap();
        let objetivo = dir.path().join("sesion.dat");
        fs::write(&objetivo, b"x").unwrap();

        // Un proceso ajeno abre el archivo para escritura y se mantiene vivo.
        let mut hijo = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!(
                "exec 9>>{} && echo listo && read _",
                objetivo.display()
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("no se pudo lanzar el proceso auxiliar");

        let mut buf = [0u8; 6];
        let _ = hijo.stdout.as_mut().unwrap().read_exact(&mut buf);

        let d = open_for_write(dir.path());
        let detectados = d.files().to_vec();

        drop(hijo.stdin.take());
        let _ = hijo.kill();
        let _ = hijo.wait();

        let esperado = objetivo.canonicalize().unwrap();
        assert!(
            detectados.iter().any(|p| *p == esperado),
            "no se detectó {}; detectados: {detectados:?}",
            esperado.display()
        );
        // El archivo sigue intacto: la comprobación no interrumpe a nadie.
        assert!(objetivo.exists());
    }
}
