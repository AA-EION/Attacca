//! Declaración de conformidad (apartados 44 y 45).
//!
//! La declaración indica la versión de la norma, la clase declarada, las partes
//! implementadas y las extensiones propias. Se expone aquí en forma de datos
//! para que la interfaz y la línea de órdenes la presenten sin duplicarla.

/// Clases de conformidad de implementación (Tabla 36).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// Lee y valida un repositorio sin modificarlo.
    Reader,
    /// Crea y modifica proyectos conformes.
    Writer,
    /// Administra el ciclo de vida completo.
    Manager,
}

impl Class {
    pub fn as_str(&self) -> &'static str {
        match self {
            Class::Reader => "R",
            Class::Writer => "W",
            Class::Manager => "M",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Class::Reader => "Lectora",
            Class::Writer => "Escritora",
            Class::Manager => "Gestora",
        }
    }
}

/// Clase declarada por Attacca.
pub const DECLARED_CLASS: Class = Class::Manager;

/// Una extensión propia, documentada conforme al apartado 44.1, quinto guion.
#[derive(Clone, Copy, Debug)]
pub struct Extension {
    /// Nombre completo del campo, con el prefijo propio.
    pub field: &'static str,
    /// Artefacto en el que aparece.
    pub artifact: &'static str,
    /// Para qué sirve.
    pub purpose: &'static str,
}

/// Extensiones propias de Attacca.
///
/// Todas llevan el prefijo `x_attacca_`. Ninguna contiene información
/// normativa: su supresión no altera la conformidad del proyecto ni impide
/// interpretarlo. La documentación pública figura en `docs/EXTENSIONES.md`.
pub const EXTENSIONS: &[Extension] = &[
    Extension {
        field: "x_attacca_daw_hints",
        artifact: "Manifiesto de proyecto",
        purpose: "Nombre de la estación de trabajo elegida al crear cada subcarpeta de 02_SESSIONS, para volver a ofrecerla por defecto. El apartado 13.1 ya recoge el programa principal en el grupo Herramientas; este campo solo evita repetir la elección.",
    },
    Extension {
        field: "x_attacca_last_opened",
        artifact: "Manifiesto de proyecto",
        purpose: "Marca temporal de la última apertura en Attacca, para ordenar la lista de proyectos por uso reciente. No participa en ninguna comprobación de conformidad.",
    },
];

/// Partes de la norma implementadas.
pub const IMPLEMENTED_PARTS: &[(&str, &str)] = &[
    ("Parte 1", "Estructura, nomenclatura, formatos, metadatos, derechos y ciclo de vida"),
    ("Parte 2", "Registro de eventos, no conformidades, trazabilidad y referencia temporal"),
    ("Parte 3", "Control de acceso por medios técnicos, protección en reposo y atribución de copias"),
    ("Parte 4", "Intercambio completo: paquete, manifiesto, control de datos, emisión, recepción, acuse, rechazo, revocación y custodia"),
    ("Parte 5", "Autodescripción, requisitos de formato y conformidad de implementación"),
];

/// Requisitos que Attacca no ejecuta por sí misma y en qué se apoya.
///
/// Se declaran de forma expresa: el apartado 21.3 establece que un requisito
/// inaplicable en la práctica se documenta y se comunica, no se elude.
pub const DELEGATED: &[(&str, &str)] = &[
    (
        "35.1, 35.3",
        "La firma criptográfica de manifiestos y acuses se delega en una herramienta OpenPGP externa. Attacca declara la firma en el manifiesto, sitúa el archivo de firma separada en el paquete e indica en LEEME.txt cómo verificarla. La verificación criptográfica y su resultado se aportan a la comprobación de autenticidad.",
    ),
    (
        "15",
        "Las once verificaciones del control de calidad exigen escucha y juicio humanos. Attacca registra su resultado y bloquea la emisión sin informe aprobado; no las ejecuta.",
    ),
    (
        "10.2",
        "La medición de sonoridad y de pico real conforme a ITU-R BS.1770 se realiza con un medidor externo. Attacca registra los valores en el manifiesto y comprueba el techo declarado frente a los parámetros comunes del release.",
    ),
    (
        "29.2",
        "La marca inaudible por destinatario se aplica con una herramienta externa. Attacca declara su existencia y su ámbito en el manifiesto de intercambio, conforme al apartado 35.4.",
    ),
    (
        "17.1, paso 5",
        "La exportación de un archivo de intercambio conforme a AES31-3 depende de la estación de trabajo. El apartado lo exige como mejor esfuerzo; Attacca registra en las notas técnicas qué elementos no sobrevivieron a la exportación.",
    ),
];

/// Declaración completa.
#[derive(Clone, Debug)]
pub struct Declaration {
    pub implementation: String,
    pub version: String,
    pub stave_version: String,
    pub class: Class,
}

/// Devuelve la declaración de conformidad de esta compilación.
pub fn declaration() -> Declaration {
    Declaration {
        implementation: crate::IMPL_NAME.to_string(),
        version: crate::IMPL_VERSION.to_string(),
        stave_version: crate::STAVE_VERSION.to_string(),
        class: DECLARED_CLASS,
    }
}

/// Declaración en texto plano, para la línea de órdenes y el panel de la
/// aplicación.
pub fn render() -> String {
    let d = declaration();
    let mut s = String::new();
    s.push_str("DECLARACION DE CONFORMIDAD\n");
    s.push_str("==========================\n\n");
    s.push_str(&format!("Implementacion:    {} {}\n", d.implementation, d.version));
    s.push_str(&format!("Norma aplicada:    STAVE {}\n", d.stave_version));
    s.push_str(&format!(
        "Clase declarada:   {} — {}\n",
        d.class.as_str(),
        d.class.name()
    ));
    s.push_str("Procedimiento:     apartado 45\n\n");

    s.push_str("PARTES IMPLEMENTADAS\n\n");
    for (parte, contenido) in IMPLEMENTED_PARTS {
        s.push_str(&format!("  {parte}: {contenido}\n"));
    }

    s.push_str("\nEXTENSIONES PROPIAS (apartado 44.1)\n\n");
    if EXTENSIONS.is_empty() {
        s.push_str("  Ninguna.\n");
    } else {
        for e in EXTENSIONS {
            s.push_str(&format!("  {}\n", e.field));
            s.push_str(&format!("    Artefacto: {}\n", e.artifact));
            s.push_str(&format!("    Finalidad: {}\n", e.purpose));
        }
    }

    s.push_str("\nREQUISITOS DELEGADOS EN HERRAMIENTAS EXTERNAS\n\n");
    for (clausula, detalle) in DELEGATED {
        s.push_str(&format!("  Apartado {clausula}\n    {detalle}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declara_la_clase_gestora() {
        let d = declaration();
        assert_eq!(d.class, Class::Manager);
        assert_eq!(d.class.as_str(), "M");
        assert_eq!(d.stave_version, "2.0");
        assert_eq!(d.implementation, "Attacca");
    }

    #[test]
    fn la_clase_gestora_incluye_las_anteriores() {
        assert!(Class::Manager > Class::Writer);
        assert!(Class::Writer > Class::Reader);
    }

    #[test]
    fn toda_extension_lleva_el_prefijo_propio() {
        for e in EXTENSIONS {
            assert!(
                e.field.starts_with(crate::EXT_PREFIX),
                "{} carece del prefijo {}",
                e.field,
                crate::EXT_PREFIX
            );
            assert!(!e.purpose.is_empty(), "{} no está documentada", e.field);
        }
    }

    #[test]
    fn la_declaracion_indica_lo_que_exige_el_apartado_45() {
        let t = render();
        // Versión de la norma, clase declarada, partes implementadas y
        // extensiones propias.
        assert!(t.contains("STAVE 2.0"));
        assert!(t.contains("M — Gestora"));
        assert!(t.contains("Parte 1"));
        assert!(t.contains("Parte 5"));
        assert!(t.contains("EXTENSIONES PROPIAS"));
        assert!(t.contains("x_attacca_"));
    }

    #[test]
    fn los_requisitos_delegados_se_declaran_de_forma_expresa() {
        let t = render();
        assert!(t.contains("REQUISITOS DELEGADOS"));
        assert!(t.contains("OpenPGP"));
        assert!(!DELEGATED.is_empty());
    }
}
