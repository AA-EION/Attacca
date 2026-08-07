//! Errores del núcleo.
//!
//! Cada variante se redacta con los tres elementos que exige la interfaz: qué
//! ocurrió, qué consecuencia tiene y qué se puede hacer. El texto de esta capa
//! es el que se muestra cuando no existe una cadena específica en el archivo de
//! recursos de la interfaz.

use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Fallo de entrada y salida sobre una ruta concreta.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Un manifiesto no se pudo interpretar.
    Manifest { path: PathBuf, detail: String },
    /// Un requisito de la norma impide continuar.
    Requirement {
        clause: &'static str,
        detail: String,
    },
    /// La entrada de la persona usuaria no es admisible.
    Input(String),
    /// Verificación de integridad fallida.
    Integrity { checked: usize, failed: Vec<String> },
    /// Estado de custodia incompatible con la acción solicitada.
    Custody { state: String, detail: String },
    /// El reloj no está sincronizado y la acción exige marca temporal fiable.
    Clock { drift_seconds: i64 },
    /// Espacio insuficiente en el volumen de destino.
    Space { required: u64, available: u64 },
    /// Archivos abiertos para escritura por otro proceso.
    OpenFiles(Vec<PathBuf>),
    /// El contenedor recibido no es admisible.
    Container { check: &'static str, detail: String },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn requirement(clause: &'static str, detail: impl Into<String>) -> Self {
        Error::Requirement {
            clause,
            detail: detail.into(),
        }
    }

    pub fn input(detail: impl Into<String>) -> Self {
        Error::Input(detail.into())
    }

    /// Cláusula de la norma que motiva el error, cuando procede. Permite a la
    /// interfaz enlazar el mensaje con el apartado correspondiente.
    pub fn clause(&self) -> Option<&'static str> {
        match self {
            Error::Requirement { clause, .. } => Some(clause),
            Error::Custody { .. } => Some("14.3"),
            Error::Clock { .. } => Some("22.3"),
            Error::Integrity { .. } => Some("37.2"),
            Error::Container { .. } => Some("32.4.2"),
            Error::OpenFiles(_) => Some("14.3.3"),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(
                f,
                "No se pudo acceder a {}. La operación no se ha completado. Comprobar que el volumen está conectado y que la ruta admite escritura. Detalle del sistema: {source}",
                path.display()
            ),
            Error::Manifest { path, detail } => write!(
                f,
                "El manifiesto {} no se pudo interpretar. El proyecto no se ha cargado. Revisar el archivo con un editor de texto. Detalle: {detail}",
                path.display()
            ),
            Error::Requirement { clause, detail } => {
                write!(f, "{detail} Apartado {clause} de la norma.")
            }
            Error::Input(detail) => f.write_str(detail),
            Error::Integrity { checked, failed } => write!(
                f,
                "La verificación de integridad falló en {} de {checked} archivos. La operación no se ha completado. Ver los archivos afectados: {}",
                failed.len(),
                failed.join(", ")
            ),
            Error::Custody { state, detail } => write!(
                f,
                "El estado de custodia del proyecto es {state}. {detail}"
            ),
            Error::Clock { drift_seconds } => write!(
                f,
                "El reloj presenta una desviación de {drift_seconds} s respecto de la fuente de tiempo de red. La emisión de paquetes y de acuses queda bloqueada. Sincronizar el reloj y repetir la operación."
            ),
            Error::Space { required, available } => write!(
                f,
                "El volumen de destino dispone de {available} bytes y la operación requiere {required}. La operación no se ha iniciado. Liberar espacio o elegir otro volumen."
            ),
            Error::OpenFiles(paths) => write!(
                f,
                "{} archivos del proyecto están abiertos para escritura por otro proceso. La operación no se ha iniciado. Cerrarlos en la aplicación que los mantiene abiertos: {}",
                paths.len(),
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Error::Container { check, detail } => write!(
                f,
                "El contenedor no superó la verificación de {check}. El paquete no se ha extraído. {detail}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<crate::doc::ParseError> for Error {
    fn from(e: crate::doc::ParseError) -> Self {
        Error::Manifest {
            path: PathBuf::new(),
            detail: e.0,
        }
    }
}
