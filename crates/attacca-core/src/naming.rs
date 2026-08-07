//! Nomenclatura (apartado 9).
//!
//! Las once reglas de la Tabla 9 son requisitos, no recomendaciones. Este módulo
//! las aplica al crear cualquier nombre y las comprueba al validar un proyecto.

use crate::error::{Error, Result};
use std::collections::HashMap;
use std::path::Path;

/// Presupuesto de ruta completa, en caracteres (regla 6 de la Tabla 9).
pub const MAX_PATH_CHARS: usize = 200;

/// Longitud recomendada máxima del campo de título (apartado 9.1).
pub const MAX_TITLE_CHARS: usize = 40;

/// Longitud recomendada máxima del campo de destino en `08_DELIVERY`.
pub const MAX_TARGET_CHARS: usize = 24;

/// Nombres de dispositivo reservados (regla 9 de la Tabla 9).
const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Valores del campo TIPO (Tabla 10).
pub const PROJECT_TYPES: &[&str] = &[
    "ORIG", "COVER", "REMIX", "BEAT", "MIX", "MST", "SD", "LIVE", "DEMO", "SYNC",
];

/// Clases de release (Tabla 8).
pub const RELEASE_CLASSES: &[&str] = &["SINGLE", "EP", "ALBUM", "COMP", "LIVE", "SYNC"];

/// Sufijos de etapa para los nombres de sesión (apartado 7.4).
pub const STAGE_SUFFIXES: &[&str] = &["COMP", "TRACK", "EDIT", "MIX", "MST", "SD"];

/// Comprueba un nombre de archivo o de carpeta frente a las reglas 3, 4, 5, 9 y
/// 11 de la Tabla 9.
pub fn is_valid_name(name: &str) -> bool {
    check_name(name).is_ok()
}

/// Comprueba un nombre y devuelve el motivo del rechazo.
pub fn check_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::input(
            "El nombre está vacío. No se ha creado nada. Escribir un nombre.",
        ));
    }
    // Regla 11: ningún nombre termina en espacio. Regla 5: ni en punto.
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(Error::requirement(
            "9.1",
            format!("El nombre «{name}» termina en espacio o en punto. El nombre no se ha aceptado. Suprimir el carácter final."),
        ));
    }
    // Regla 3: sin espacios. Regla 4: conjunto de caracteres admitido.
    for c in name.chars() {
        let admitido = c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
        if !admitido {
            let detalle = if c == ' ' {
                format!("El nombre «{name}» contiene un espacio. El nombre no se ha aceptado. Sustituir los espacios por guiones medios.")
            } else {
                format!("El nombre «{name}» contiene el carácter «{c}», ajeno al conjunto A-Z a-z 0-9 guion bajo guion medio y punto. El nombre no se ha aceptado. Sustituir el carácter.")
            };
            return Err(Error::requirement("9.1", detalle));
        }
    }
    // Regla 9: nombres de dispositivo reservados.
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return Err(Error::requirement(
            "9.1",
            format!("El nombre «{name}» coincide con un nombre de dispositivo reservado del sistema de archivos. El nombre no se ha aceptado. Elegir otro nombre."),
        ));
    }
    Ok(())
}

/// Convierte un texto libre en un campo de nombre conforme a la Tabla 9.
///
/// Los caracteres acentuados se transliteran en lugar de suprimirse: la regla 4
/// existe porque se corrompen al intercambiar soportes, no porque el texto sea
/// irrelevante. El título legible se conserva íntegro en el campo `title` del
/// manifiesto.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_sep = false;
    for c in text.chars() {
        let mapped = transliterate(c);
        if mapped.is_empty() {
            if !out.is_empty() && !prev_sep {
                out.push('-');
                prev_sep = true;
            }
            continue;
        }
        for m in mapped.chars() {
            if m.is_ascii_alphanumeric() {
                out.push(m);
                prev_sep = false;
            } else if m == '-' && !out.is_empty() && !prev_sep {
                out.push('-');
                prev_sep = true;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn transliterate(c: char) -> String {
    match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => "a".into(),
        'é' | 'è' | 'ë' | 'ê' => "e".into(),
        'í' | 'ì' | 'ï' | 'î' => "i".into(),
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => "o".into(),
        'ú' | 'ù' | 'ü' | 'û' => "u".into(),
        'ñ' => "n".into(),
        'ç' => "c".into(),
        'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'Å' => "A".into(),
        'É' | 'È' | 'Ë' | 'Ê' => "E".into(),
        'Í' | 'Ì' | 'Ï' | 'Î' => "I".into(),
        'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => "O".into(),
        'Ú' | 'Ù' | 'Ü' | 'Û' => "U".into(),
        'Ñ' => "N".into(),
        'Ç' => "C".into(),
        'ß' => "ss".into(),
        c if c.is_ascii_alphanumeric() => c.to_string(),
        _ => String::new(),
    }
}

/// Construye el identificador legible de un proyecto (apartado 9.2):
/// `AAAA-MM-DD_Titulo-Con-Guiones_TIPO`.
pub fn project_id(date: &str, title: &str, kind: &str) -> Result<String> {
    if !PROJECT_TYPES.contains(&kind) {
        return Err(Error::requirement(
            "9.2",
            format!("El tipo «{kind}» no figura en la Tabla 10. El proyecto no se ha creado. Elegir uno de: {}.", PROJECT_TYPES.join(", ")),
        ));
    }
    let slug = slugify(title);
    let slug = if slug.is_empty() { "Sin-Titulo".to_string() } else { slug };
    let slug = truncate_chars(&slug, MAX_TITLE_CHARS);
    let id = format!("{date}_{slug}_{kind}");
    check_name(&id)?;
    Ok(id)
}

/// Construye el identificador legible de un release (apartado 8.1):
/// `AAAA-MM-DD_Titulo_CLASE`.
pub fn release_id(date: &str, title: &str, class: &str) -> Result<String> {
    if !RELEASE_CLASSES.contains(&class) {
        return Err(Error::requirement(
            "8.2",
            format!("La clase «{class}» no figura en la Tabla 8. El release no se ha creado. Elegir una de: {}.", RELEASE_CLASSES.join(", ")),
        ));
    }
    let slug = slugify(title);
    let slug = if slug.is_empty() { "Sin-Titulo".to_string() } else { slug };
    let id = format!("{date}_{}_{class}", truncate_chars(&slug, MAX_TITLE_CHARS));
    check_name(&id)?;
    Ok(id)
}

/// Añade el sufijo de versión `_vNN` (apartado 14.4.2 y regla 7 de la Tabla 9).
pub fn with_version_suffix(id: &str, version: u32) -> String {
    format!("{id}_v{version:02}")
}

/// Extrae el número de versión de un identificador terminado en `_vNN`.
pub fn version_suffix_of(id: &str) -> Option<u32> {
    let (_, tail) = id.rsplit_once("_v")?;
    if tail.len() == 2 && tail.bytes().all(|b| b.is_ascii_digit()) {
        tail.parse().ok()
    } else {
        None
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    let out: String = s.chars().take(max).collect();
    out.trim_end_matches('-').to_string()
}

/// Resultado de la comprobación del presupuesto de ruta (regla 6 de la Tabla 9).
#[derive(Clone, Debug)]
pub struct PathBudget {
    /// Longitud de la ruta más larga encontrada.
    pub longest: usize,
    /// Ruta que produce esa longitud, relativa a la raíz del repositorio.
    pub longest_path: String,
    /// Etiqueta de la réplica que produce la ruta más larga.
    pub replica: String,
    /// Presupuesto total.
    pub limit: usize,
}

impl PathBudget {
    pub fn exceeded(&self) -> bool {
        self.longest > self.limit
    }

    pub fn remaining(&self) -> i64 {
        self.limit as i64 - self.longest as i64
    }
}

/// Comprueba el presupuesto de ruta contra un conjunto de raíces de réplica.
///
/// El apartado 9.1 exige medir contra el volumen cuya raíz produzca la ruta más
/// larga entre todas las réplicas, no contra la local.
pub fn check_path_budget(
    relative_paths: &[String],
    replica_roots: &HashMap<String, String>,
) -> PathBudget {
    let mut budget = PathBudget {
        longest: 0,
        longest_path: String::new(),
        replica: String::new(),
        limit: MAX_PATH_CHARS,
    };
    // Sin réplicas declaradas, la medida se toma sobre la ruta relativa, que es
    // el mínimo que cualquier réplica añadirá.
    if replica_roots.is_empty() {
        for p in relative_paths {
            let len = p.chars().count();
            if len > budget.longest {
                budget.longest = len;
                budget.longest_path = p.clone();
            }
        }
        return budget;
    }
    for (label, root) in replica_roots {
        let root_len = root.chars().count() + usize::from(!root.ends_with('/'));
        for p in relative_paths {
            let len = root_len + p.chars().count();
            if len > budget.longest {
                budget.longest = len;
                budget.longest_path = p.clone();
                budget.replica = label.clone();
            }
        }
    }
    budget
}

/// Detecta colisiones al comparar sin distinguir mayúsculas de minúsculas
/// (regla 10 de la Tabla 9).
pub fn case_collisions(names: &[String]) -> Vec<(String, String)> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut out = Vec::new();
    for name in names {
        let key = name.to_lowercase();
        match seen.get(&key) {
            Some(prev) => out.push((prev.clone(), name.clone())),
            None => {
                seen.insert(key, name.clone());
            }
        }
    }
    out
}

/// Nombre de archivo de audio interno (apartado 9.3):
/// `Titulo_ROL_descriptor_vNN.wav`.
pub fn internal_audio_name(
    title: &str,
    role: &str,
    descriptor: &str,
    version: u32,
    ext: &str,
) -> String {
    let mut parts = vec![slugify(title), role.to_string()];
    let d = slugify(descriptor);
    if !d.is_empty() {
        parts.push(d);
    }
    format!("{}_v{version:02}.{ext}", parts.join("_"))
}

/// Nombre de entregable (apartado 9.3):
/// `Artista - Titulo (Version) [Formato].wav`.
///
/// Este patrón admite espacios: la regla 3 los excluye de los nombres técnicos,
/// y este patrón prioriza la lectura por el cliente o el distribuidor.
pub fn deliverable_audio_name(
    artist: &str,
    title: &str,
    version: Option<&str>,
    format: &str,
    ext: &str,
) -> String {
    let mut name = format!("{artist} - {title}");
    if let Some(v) = version {
        name.push_str(&format!(" ({v})"));
    }
    name.push_str(&format!(" [{format}].{ext}"));
    name
}

/// Comprueba las reglas de nomenclatura sobre cada componente de una ruta
/// relativa.
pub fn check_relative_path(path: &Path) -> Result<()> {
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::Normal(part) => {
                let name = part.to_string_lossy();
                check_name(&name)?;
            }
            Component::ParentDir => {
                return Err(Error::requirement(
                    "32.4.2",
                    format!("La ruta «{}» contiene una referencia al directorio superior. La ruta no se ha aceptado.", path.display()),
                ))
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(Error::requirement(
                    "32.4.2",
                    format!("La ruta «{}» es absoluta. La ruta no se ha aceptado. Emplear una ruta relativa.", path.display()),
                ))
            }
            Component::CurDir => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rechaza_los_nombres_contrarios_a_la_tabla_9() {
        assert!(check_name("Cancion De Ejemplo").is_err(), "regla 3");
        assert!(check_name("Canción").is_err(), "regla 4");
        assert!(check_name("mezcla.").is_err(), "regla 5");
        assert!(check_name("CON").is_err(), "regla 9");
        assert!(check_name("com1.wav").is_err(), "regla 9");
        assert!(check_name("mezcla ").is_err(), "regla 11");
        assert!(check_name("").is_err());
        assert!(check_name("2026-08-06_Cancion-De-Ejemplo_ORIG").is_ok());
    }

    #[test]
    fn translitera_en_lugar_de_suprimir() {
        assert_eq!(slugify("Canción de Cuna"), "Cancion-de-Cuna");
        assert_eq!(slugify("Mañana, ¿qué?"), "Manana-que");
        assert_eq!(slugify("   "), "");
        assert_eq!(slugify("A--B"), "A-B");
    }

    #[test]
    fn construye_el_identificador_legible() {
        let id = project_id("2026-08-06", "Canción de Ejemplo", "ORIG").unwrap();
        assert_eq!(id, "2026-08-06_Cancion-de-Ejemplo_ORIG");
        assert!(is_valid_name(&id));
        assert!(project_id("2026-08-06", "X", "NOEXISTE").is_err());
    }

    #[test]
    fn respeta_el_limite_de_titulo() {
        let largo = "a".repeat(120);
        let id = project_id("2026-08-06", &largo, "ORIG").unwrap();
        let campo = id.split('_').nth(1).unwrap();
        assert_eq!(campo.chars().count(), MAX_TITLE_CHARS);
    }

    #[test]
    fn el_sufijo_de_version_lleva_dos_digitos() {
        assert_eq!(with_version_suffix("2026-08-06_T_ORIG", 2), "2026-08-06_T_ORIG_v02");
        assert_eq!(version_suffix_of("2026-08-06_T_ORIG_v02"), Some(2));
        assert_eq!(version_suffix_of("2026-08-06_T_ORIG"), None);
        assert_eq!(version_suffix_of("2026-08-06_T_ORIG_v2"), None, "regla 7");
    }

    #[test]
    fn el_presupuesto_se_mide_contra_la_replica_mas_larga() {
        let rutas = vec!["20_PROJECTS/1_ACTIVE/2026-08-06_Tema_ORIG/08_DELIVERY/a.wav".to_string()];
        let mut replicas = HashMap::new();
        replicas.insert("local".into(), "/home/u/.stave".to_string());
        replicas.insert("portatil".into(), "/Volumes/STUDIO-PORT-01/STAVE".to_string());
        let b = check_path_budget(&rutas, &replicas);
        assert_eq!(b.replica, "portatil");
        assert!(!b.exceeded());
        assert!(b.remaining() > 0);
    }

    #[test]
    fn detecta_el_exceso_de_presupuesto() {
        let rutas = vec![format!("20_PROJECTS/{}/a.wav", "x".repeat(190))];
        let b = check_path_budget(&rutas, &HashMap::new());
        assert!(b.exceeded());
        assert!(b.remaining() < 0);
    }

    #[test]
    fn detecta_colisiones_sin_distincion_de_mayusculas() {
        let nombres = vec!["Mezcla.wav".to_string(), "mezcla.WAV".to_string(), "otro.wav".to_string()];
        let c = case_collisions(&nombres);
        assert_eq!(c.len(), 1);
        assert!(case_collisions(&["a.wav".to_string(), "b.wav".to_string()]).is_empty());
    }

    #[test]
    fn los_dos_patrones_de_audio_del_apartado_9_3() {
        assert_eq!(
            internal_audio_name("Canción", "LEAD-VOX", "comp", 3, "wav"),
            "Cancion_LEAD-VOX_comp_v03.wav"
        );
        assert_eq!(
            deliverable_audio_name("Artista", "Cancion De Ejemplo", Some("Radio Edit"), "WAV-24-48", "wav"),
            "Artista - Cancion De Ejemplo (Radio Edit) [WAV-24-48].wav"
        );
    }

    #[test]
    fn rechaza_rutas_no_admitidas() {
        assert!(check_relative_path(Path::new("data/content/00_ADMIN/a.txt")).is_ok());
        assert!(check_relative_path(Path::new("../fuera.txt")).is_err());
        assert!(check_relative_path(Path::new("/etc/passwd")).is_err());
    }
}
