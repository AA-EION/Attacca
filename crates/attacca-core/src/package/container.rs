//! Contenedor `.stave` (apartado 32.4).
//!
//! Un archivo comprimido conforme a ISO/IEC 21320-1. La extensión identifica la
//! función del archivo; no introduce ningún formato nuevo. Renombrado a `.zip`
//! se abre con la utilidad de descompresión de cualquier sistema operativo, y lo
//! que aparece al extraerlo es la estructura del apartado 32.1.

use crate::error::{Error, Result};
use crate::fsx::{atomic, space, walk};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Tipo de medio del contenedor (apartado 32.4.2).
pub const MIMETYPE: &str = "application/vnd.stave.package";

/// Nombre de la primera entrada del contenedor.
pub const MIMETYPE_ENTRY: &str = "mimetype";

/// Extensión del contenedor.
pub const EXTENSION: &str = "stave";

/// Informe del progreso de una operación larga.
pub struct Progress<'a> {
    /// Se invoca con el número de archivos procesados y el total.
    pub on_progress: &'a mut dyn FnMut(usize, usize),
    /// Se consulta entre archivos. Al devolver `true`, la operación se cancela
    /// sin dejar nada en el destino.
    pub cancelled: &'a dyn Fn() -> bool,
}

impl Progress<'_> {
    fn report(&mut self, done: usize, total: usize) {
        (self.on_progress)(done, total);
    }

    fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }
}

/// Operación cancelada por la persona usuaria.
pub fn cancelled_error() -> Error {
    Error::input(
        "La operación se canceló. No se ha dejado ningún archivo en el destino. Repetir la operación cuando proceda.",
    )
}

/// Comprueba un nombre de entrada frente al apartado 32.4.2.
///
/// Se aplica tanto al construir como al extraer: un contenedor con una entrada
/// no admisible no se construye ni se extrae.
pub fn check_entry_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Container {
            check: "contenedor",
            detail: "El contenedor declara una entrada sin nombre.".into(),
        });
    }
    // Ruta absoluta, en forma Unix o con unidad de Windows.
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(Error::Container {
            check: "contenedor",
            detail: format!("La entrada «{name}» es una ruta absoluta. Las entradas deben expresarse siempre como rutas relativas."),
        });
    }
    let bytes = name.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(Error::Container {
            check: "contenedor",
            detail: format!("La entrada «{name}» lleva una unidad de disco y es por tanto absoluta."),
        });
    }
    // La barra invertida no es separador admitido (apartado 32.4.2).
    if name.contains('\\') {
        return Err(Error::Container {
            check: "contenedor",
            detail: format!("La entrada «{name}» emplea la barra invertida como separador. El separador admitido es la barra inclinada."),
        });
    }
    // Referencia al directorio superior, en cualquier posición.
    for segment in name.split('/') {
        if segment == ".." {
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La entrada «{name}» contiene una referencia al directorio superior, que permitiría escribir fuera del directorio de destino."),
            });
        }
    }
    Ok(())
}

/// Construye un contenedor `.stave` a partir de un directorio de paquete.
///
/// La escritura es atómica respecto de su destino (apartado 32.4.4): el
/// contenedor se construye en una ubicación temporal, se verifica y solo
/// entonces se sitúa en su destino mediante un renombrado dentro del mismo
/// volumen. Un contenedor cuya escritura se interrumpa no queda en el destino.
pub fn build(package_dir: &Path, destination: &Path, progress: &mut Progress) -> Result<PathBuf> {
    let base_name = package_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| Error::input("El directorio del paquete no tiene nombre.".to_string()))?;
    crate::naming::check_name(&base_name)?;

    let entries = walk::conserved_files(package_dir);
    if entries.is_empty() {
        return Err(Error::input(format!(
            "El directorio {} no contiene ningún archivo. El contenedor no se ha construido.",
            package_dir.display()
        )));
    }
    // Se comprueba el espacio antes de empezar, no durante.
    space::ensure_available(destination.parent().unwrap_or(destination), walk::total_bytes(&entries))?;

    // Toda entrada se comprueba antes de escribirla: no se construye un
    // contenedor con una ruta no admitida.
    for entry in &entries {
        check_entry_name(&format!("{base_name}/{}", entry.relative))?;
    }

    let temp = destination.with_extension(format!("{EXTENSION}{}", atomic::TEMP_SUFFIX));
    if let Some(parent) = temp.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let resultado = write_container(&temp, &base_name, &entries, progress);

    // Un fallo o una cancelación no deben dejar nada en el destino ni el
    // temporal a medias.
    if let Err(e) = resultado {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    // Verificación antes de situarlo en su destino.
    if let Err(e) = verify_structure(&temp) {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    std::fs::rename(&temp, destination).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        Error::io(destination, e)
    })?;
    Ok(destination.to_path_buf())
}

fn write_container(
    temp: &Path,
    base_name: &str,
    entries: &[walk::Entry],
    progress: &mut Progress,
) -> Result<()> {
    let file = File::create(temp).map_err(|e| Error::io(temp, e))?;
    let mut zip = ZipWriter::new(file);

    // La primera entrada debe ser `mimetype`, sin comprimir, con el contenido
    // exacto y sin terminador de línea (apartado 32.4.2).
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file(MIMETYPE_ENTRY, stored)
        .map_err(|e| zip_error(temp, e))?;
    zip.write_all(MIMETYPE.as_bytes())
        .map_err(|e| Error::io(temp, e))?;

    // Solo almacenamiento o desinflado (apartado 32.4.2). El desinflado es el
    // método por defecto; ZIP64 se activa cuando el tamaño lo exige.
    let total = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        if progress.is_cancelled() {
            return Err(cancelled_error());
        }
        let name = format!("{base_name}/{}", entry.relative);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .large_file(entry.size >= u32::MAX as u64);
        zip.start_file(&name, options)
            .map_err(|e| zip_error(temp, e))?;

        let mut src = File::open(&entry.path).map_err(|e| Error::io(&entry.path, e))?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = src.read(&mut buf).map_err(|e| Error::io(&entry.path, e))?;
            if n == 0 {
                break;
            }
            zip.write_all(&buf[..n]).map_err(|e| Error::io(temp, e))?;
        }
        progress.report(i + 1, total);
    }

    let mut file = zip.finish().map_err(|e| zip_error(temp, e))?;
    file.flush().map_err(|e| Error::io(temp, e))?;
    file.sync_all().map_err(|e| Error::io(temp, e))?;
    Ok(())
}

fn zip_error(path: &Path, e: zip::result::ZipError) -> Error {
    Error::io(path, std::io::Error::other(e.to_string()))
}

/// Comprueba la estructura de un contenedor sin extraerlo (verificación 3 del
/// apartado 37.2).
///
/// Se ejecuta antes de cualquier extracción: un contenedor con rutas no
/// admitidas no debe extraerse.
pub fn verify_structure(container: &Path) -> Result<ContainerInfo> {
    let file = File::open(container).map_err(|e| Error::io(container, e))?;
    let mut zip = ZipArchive::new(file).map_err(|e| Error::Container {
        check: "contenedor",
        detail: format!("El archivo no es un contenedor conforme a ISO/IEC 21320-1: {e}"),
    })?;

    if zip.is_empty() {
        return Err(Error::Container {
            check: "contenedor",
            detail: "El contenedor no tiene ninguna entrada.".into(),
        });
    }

    // Primera entrada: `mimetype`, sin comprimir, contenido exacto.
    {
        let mut first = zip.by_index(0).map_err(|e| Error::Container {
            check: "contenedor",
            detail: format!("La primera entrada no se pudo leer: {e}"),
        })?;
        if first.name() != MIMETYPE_ENTRY {
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La primera entrada se denomina «{}» y debe denominarse «{MIMETYPE_ENTRY}».", first.name()),
            });
        }
        if first.compression() != CompressionMethod::Stored {
            return Err(Error::Container {
                check: "contenedor",
                detail: "La entrada mimetype está comprimida y debe almacenarse sin compresión.".into(),
            });
        }
        let mut contenido = String::new();
        first.read_to_string(&mut contenido).map_err(|e| Error::io(container, e))?;
        if contenido != MIMETYPE {
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La entrada mimetype contiene «{contenido}» y debe contener exactamente «{MIMETYPE}», sin terminador de línea."),
            });
        }
    }

    let mut base_dirs: Vec<String> = Vec::new();
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;

    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| Error::Container {
            check: "contenedor",
            detail: format!("La entrada {i} no se pudo leer: {e}"),
        })?;
        let name = entry.name().to_string();
        if name == MIMETYPE_ENTRY {
            continue;
        }
        check_entry_name(&name)?;

        // Enlaces simbólicos y archivos especiales (apartado 32.4.2).
        if let Some(mode) = entry.unix_mode() {
            const S_IFMT: u32 = 0o170000;
            const S_IFREG: u32 = 0o100000;
            const S_IFDIR: u32 = 0o040000;
            let tipo = mode & S_IFMT;
            if tipo != 0 && tipo != S_IFREG && tipo != S_IFDIR {
                let clase = if tipo == 0o120000 {
                    "un enlace simbólico"
                } else {
                    "un archivo especial del sistema"
                };
                return Err(Error::Container {
                    check: "contenedor",
                    detail: format!("La entrada «{name}» es {clase}, que el contenedor no admite."),
                });
            }
        }

        // Métodos de compresión admitidos (apartado 32.4.2).
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La entrada «{name}» emplea un método de compresión distinto del almacenamiento y del desinflado."),
            });
        }
        // Cifrado interno (apartado 32.4.2).
        if !entry.encrypted() {
            // La ausencia de cifrado es lo correcto; no se hace nada.
        } else {
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La entrada «{name}» está cifrada. El contenedor no debe llevar cifrado interno; la protección se aplica sobre el contenedor completo o sobre el canal."),
            });
        }

        let raiz = name.split('/').next().unwrap_or("").to_string();
        if !raiz.is_empty() && !base_dirs.contains(&raiz) {
            base_dirs.push(raiz);
        }
        if !entry.name().ends_with('/') {
            file_count += 1;
            total_bytes += entry.size();
        }
    }

    // Salvo `mimetype`, una única entrada de primer nivel (apartado 32.4.2).
    if base_dirs.len() != 1 {
        return Err(Error::Container {
            check: "contenedor",
            detail: format!("El contenedor tiene {} entradas de primer nivel además de mimetype y debe tener exactamente una, el directorio base del paquete.", base_dirs.len()),
        });
    }

    Ok(ContainerInfo {
        base_dir: base_dirs.remove(0),
        file_count,
        total_bytes,
    })
}

/// Datos observados de un contenedor.
#[derive(Clone, Debug)]
pub struct ContainerInfo {
    /// Nombre del directorio base del paquete.
    pub base_dir: String,
    pub file_count: usize,
    pub total_bytes: u64,
}

/// Extrae un contenedor, tras comprobar su estructura.
///
/// La comprobación precede a la extracción de cualquier archivo. Cada entrada se
/// vuelve a comprobar y se resuelve contra el destino: ninguna escritura ocurre
/// fuera del directorio indicado.
pub fn extract(container: &Path, destination: &Path, progress: &mut Progress) -> Result<PathBuf> {
    let info = verify_structure(container)?;

    let file = File::open(container).map_err(|e| Error::io(container, e))?;
    let mut zip = ZipArchive::new(file).map_err(|e| zip_error(container, e))?;

    let raiz_destino = destination.join(&info.base_dir);
    let canon_destino = canonical_prefix(destination)?;
    space::ensure_available(destination, info.total_bytes)?;
    std::fs::create_dir_all(destination).map_err(|e| Error::io(destination, e))?;

    let total = zip.len();
    for i in 0..total {
        if progress.is_cancelled() {
            // Lo extraído hasta el momento se retira: una cancelación no debe
            // dejar un paquete a medio extraer en la cuarentena.
            let _ = std::fs::remove_dir_all(&raiz_destino);
            return Err(cancelled_error());
        }
        let mut entry = zip.by_index(i).map_err(|e| zip_error(container, e))?;
        let name = entry.name().to_string();
        if name == MIMETYPE_ENTRY {
            continue;
        }
        check_entry_name(&name)?;

        let target = destination.join(&name);
        // Última defensa: la ruta resuelta debe seguir dentro del destino.
        // Protege frente a formas de escape que el análisis del nombre no
        // hubiera previsto.
        if !resolves_inside(&canon_destino, &target) {
            let _ = std::fs::remove_dir_all(&raiz_destino);
            return Err(Error::Container {
                check: "contenedor",
                detail: format!("La entrada «{name}» se resuelve fuera del directorio de destino."),
            });
        }

        if name.ends_with('/') {
            std::fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out = File::create(&target).map_err(|e| Error::io(&target, e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| Error::io(&target, e))?;
        out.sync_all().map_err(|e| Error::io(&target, e))?;
        progress.report(i + 1, total);
    }
    Ok(raiz_destino)
}

fn canonical_prefix(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    dir.canonicalize().map_err(|e| Error::io(dir, e))
}

fn resolves_inside(root: &Path, target: &Path) -> bool {
    // El destino no existe todavía: se normaliza el ascendiente existente más
    // próximo y se comprueba el prefijo.
    let mut existente = target;
    while !existente.exists() {
        match existente.parent() {
            Some(p) => existente = p,
            None => return false,
        }
    }
    match existente.canonicalize() {
        Ok(c) => c.starts_with(root),
        Err(_) => false,
    }
}

/// Progreso que no informa ni cancela. Se emplea en pruebas y en operaciones
/// breves.
pub fn silent_progress() -> (impl FnMut(usize, usize), impl Fn() -> bool) {
    (|_, _| {}, || false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn paquete(dir: &Path, nombre: &str) -> PathBuf {
        let p = dir.join(nombre);
        fs::create_dir_all(p.join("data/content/07_MASTER")).unwrap();
        fs::write(p.join("bagit.txt"), b"BagIt-Version: 1.0\n").unwrap();
        fs::write(p.join("EXCHANGE.yaml"), b"stave:\n  version: \"2.0\"\n").unwrap();
        fs::write(p.join("LEEME.txt"), b"instrucciones\n").unwrap();
        fs::write(p.join("data/content/07_MASTER/m.wav"), b"audio del master").unwrap();
        p
    }

    fn construir(dir: &Path, nombre: &str) -> PathBuf {
        let src = paquete(dir, nombre);
        let destino = dir.join(format!("{nombre}.stave"));
        let (mut prog, canc) = silent_progress();
        let mut p = Progress { on_progress: &mut prog, cancelled: &canc };
        build(&src, &destino, &mut p).unwrap()
    }

    #[test]
    fn la_primera_entrada_es_mimetype_sin_comprimir() {
        let dir = tempfile::tempdir().unwrap();
        let c = construir(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0007");

        let mut zip = ZipArchive::new(File::open(&c).unwrap()).unwrap();
        let mut primera = zip.by_index(0).unwrap();
        assert_eq!(primera.name(), "mimetype");
        assert_eq!(primera.compression(), CompressionMethod::Stored);
        let mut s = String::new();
        primera.read_to_string(&mut s).unwrap();
        assert_eq!(s, "application/vnd.stave.package");
        assert!(!s.ends_with('\n'), "sin terminador de línea");
    }

    #[test]
    fn solo_hay_una_entrada_de_primer_nivel_ademas_de_mimetype() {
        let dir = tempfile::tempdir().unwrap();
        let c = construir(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0007");
        let info = verify_structure(&c).unwrap();
        assert_eq!(info.base_dir, "STAVE-XCHG_2026-08-06_A_B_0007");
        assert_eq!(info.file_count, 4);
    }

    #[test]
    fn rechaza_las_rutas_no_admitidas_del_apartado_32_4_2() {
        assert!(check_entry_name("paquete/data/content/a.wav").is_ok());
        assert!(check_entry_name("/etc/passwd").is_err());
        assert!(check_entry_name("C:/Windows/system.ini").is_err());
        assert!(check_entry_name("../fuera.wav").is_err());
        assert!(check_entry_name("paquete/../../fuera.wav").is_err());
        assert!(check_entry_name("paquete\\data\\a.wav").is_err());
        assert!(check_entry_name("").is_err());
    }

    #[test]
    fn un_contenedor_con_ruta_no_admitida_no_se_extrae() {
        let dir = tempfile::tempdir().unwrap();
        let malicioso = dir.path().join("malicioso.stave");
        {
            let mut zip = ZipWriter::new(File::create(&malicioso).unwrap());
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(MIMETYPE.as_bytes()).unwrap();
            let opts = SimpleFileOptions::default();
            zip.start_file("paquete/../../escapado.txt", opts).unwrap();
            zip.write_all(b"fuera").unwrap();
            zip.finish().unwrap();
        }
        let e = verify_structure(&malicioso).unwrap_err();
        assert_eq!(e.clause(), Some("32.4.2"));
        assert!(e.to_string().contains("directorio superior"), "{e}");

        let destino = dir.path().join("extraccion");
        let (mut prog, canc) = silent_progress();
        let mut p = Progress { on_progress: &mut prog, cancelled: &canc };
        assert!(extract(&malicioso, &destino, &mut p).is_err());
        // Nada se ha escrito fuera del destino ni dentro de él.
        assert!(!dir.path().join("escapado.txt").exists());
        assert!(!destino.join("escapado.txt").exists());
    }

    #[test]
    fn rechaza_un_enlace_simbolico() {
        let dir = tempfile::tempdir().unwrap();
        let c = dir.path().join("enlace.stave");
        {
            let mut zip = ZipWriter::new(File::create(&c).unwrap());
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(MIMETYPE.as_bytes()).unwrap();
            let opts = SimpleFileOptions::default();
            zip.add_symlink("paquete/enlace", "/etc/passwd", opts).unwrap();
            zip.finish().unwrap();
        }
        let e = verify_structure(&c).unwrap_err();
        assert!(e.to_string().contains("enlace simbólico"), "{e}");
    }

    #[test]
    fn rechaza_un_contenedor_sin_mimetype_en_primera_posicion() {
        let dir = tempfile::tempdir().unwrap();
        let c = dir.path().join("desordenado.stave");
        {
            let mut zip = ZipWriter::new(File::create(&c).unwrap());
            let opts = SimpleFileOptions::default();
            zip.start_file("paquete/a.txt", opts).unwrap();
            zip.write_all(b"x").unwrap();
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(MIMETYPE.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        assert!(verify_structure(&c).unwrap_err().to_string().contains("primera entrada"));
    }

    #[test]
    fn la_construccion_es_atomica_frente_a_la_cancelacion() {
        let dir = tempfile::tempdir().unwrap();
        let src = paquete(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0008");
        let destino = dir.path().join("paquete.stave");

        let mut prog = |_: usize, _: usize| {};
        let cancelar = || true;
        let mut p = Progress { on_progress: &mut prog, cancelled: &cancelar };
        assert!(build(&src, &destino, &mut p).is_err());

        // Ni el destino ni el temporal quedan en el sistema de archivos.
        assert!(!destino.exists());
        assert!(atomic::find_abandoned(dir.path()).is_empty());
    }

    #[test]
    fn el_ciclo_de_construccion_y_extraccion_conserva_el_contenido() {
        let dir = tempfile::tempdir().unwrap();
        let c = construir(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0009");
        let destino = dir.path().join("extraido");
        let (mut prog, canc) = silent_progress();
        let mut p = Progress { on_progress: &mut prog, cancelled: &canc };
        let raiz = extract(&c, &destino, &mut p).unwrap();

        assert_eq!(
            fs::read(raiz.join("data/content/07_MASTER/m.wav")).unwrap(),
            b"audio del master"
        );
        assert!(raiz.join("EXCHANGE.yaml").exists());
        assert!(raiz.join("LEEME.txt").exists());
    }

    #[test]
    fn el_progreso_informa_de_cada_archivo() {
        let dir = tempfile::tempdir().unwrap();
        let src = paquete(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0010");
        let destino = dir.path().join("p.stave");
        let mut vistos = Vec::new();
        let canc = || false;
        {
            let mut prog = |hecho: usize, total: usize| vistos.push((hecho, total));
            let mut p = Progress { on_progress: &mut prog, cancelled: &canc };
            build(&src, &destino, &mut p).unwrap();
        }
        assert_eq!(vistos.len(), 4);
        assert_eq!(vistos.last().unwrap(), &(4, 4));
    }

    #[test]
    fn repetir_la_construccion_no_produce_efectos_distintos() {
        let dir = tempfile::tempdir().unwrap();
        let src = paquete(dir.path(), "STAVE-XCHG_2026-08-06_A_B_0011");
        let destino = dir.path().join("p.stave");
        let (mut prog, canc) = silent_progress();
        let mut p = Progress { on_progress: &mut prog, cancelled: &canc };
        build(&src, &destino, &mut p).unwrap();
        let primera = verify_structure(&destino).unwrap();
        build(&src, &destino, &mut p).unwrap();
        let segunda = verify_structure(&destino).unwrap();
        assert_eq!(primera.file_count, segunda.file_count);
        assert_eq!(primera.base_dir, segunda.base_dir);
    }
}
