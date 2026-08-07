//! Manifiesto de integridad (apartados 17.1, 35.1 y Tabla 35).
//!
//! Texto plano, resumen criptográfico conforme a ISO/IEC 10118-3 con longitud no
//! inferior a 256 bits, y ruta relativa. El formato es el de las utilidades de
//! resumen incluidas en todos los sistemas operativos, de modo que la
//! verificación pueda hacerse sin Attacca (Anexo H).

use crate::error::{Error, Result};
use crate::fsx::{atomic, walk};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

/// Algoritmo declarado en el manifiesto de intercambio.
pub const ALGORITHM: &str = "SHA-256";

/// Manifiesto de integridad: ruta relativa asociada al resumen de su contenido.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrityManifest {
    /// Orden lexicográfico por ruta, lo que hace el archivo determinista.
    entries: BTreeMap<String, String>,
}

impl IntegrityManifest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn digest_of(&self, path: &str) -> Option<&str> {
        self.entries.get(path).map(String::as_str)
    }

    pub fn insert(&mut self, path: impl Into<String>, digest: impl Into<String>) {
        self.entries.insert(path.into(), digest.into());
    }

    /// Serializa en el formato de `sha256sum`: resumen, dos espacios y ruta.
    ///
    /// Es el formato que `sha256sum -c` y `shasum -a 256 -c` aceptan sin
    /// traducción, lo que satisface el Anexo H: la integridad de un proyecto
    /// archivado debe poder verificarse con las utilidades del sistema
    /// operativo.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.entries.len() * 80);
        for (path, digest) in &self.entries {
            out.push_str(digest);
            out.push_str("  ");
            out.push_str(path);
            out.push('\n');
        }
        out
    }

    pub fn parse(text: &str) -> IntegrityManifest {
        let mut m = IntegrityManifest::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            // El separador es de dos espacios; el modo binario de algunas
            // utilidades emplea espacio y asterisco.
            let (digest, path) = match line.split_once("  ") {
                Some((d, p)) => (d, p),
                None => match line.split_once(" *") {
                    Some((d, p)) => (d, p),
                    None => continue,
                },
            };
            m.insert(path.trim(), digest.trim().to_ascii_lowercase());
        }
        m
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        atomic::write_str(path, &self.render())
    }

    pub fn load(path: &Path) -> Result<IntegrityManifest> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Ok(Self::parse(&text))
    }
}

/// Resumen SHA-256 de un archivo, en minúsculas hexadecimales.
///
/// La lectura es por bloques: un máster de varios gigabytes no debe cargarse
/// entero en memoria.
pub fn digest_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

/// Resumen SHA-256 de una secuencia de octetos.
pub fn digest_bytes(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Calcula el manifiesto de integridad de un árbol, aplicando las exclusiones
/// del Anexo C.
///
/// El manifiesto de integridad no se incluye a sí mismo.
pub fn compute(root: &Path) -> Result<IntegrityManifest> {
    let mut m = IntegrityManifest::new();
    for entry in walk::conserved_files(root) {
        if entry.relative == crate::manifest::INTEGRITY_FILE {
            continue;
        }
        m.insert(entry.relative.clone(), digest_file(&entry.path)?);
    }
    Ok(m)
}

/// Resultado de una verificación de integridad.
#[derive(Clone, Debug, Default)]
pub struct Verification {
    /// Número de archivos comprobados.
    pub checked: usize,
    /// Archivos cuyo resumen no coincide.
    pub mismatched: Vec<String>,
    /// Archivos declarados en el manifiesto que no están presentes.
    pub missing: Vec<String>,
    /// Archivos presentes que el manifiesto no declara (apartado 37.2,
    /// verificación 5: «sin archivos ausentes ni no declarados»).
    pub undeclared: Vec<String>,
}

impl Verification {
    pub fn is_ok(&self) -> bool {
        self.mismatched.is_empty() && self.missing.is_empty() && self.undeclared.is_empty()
    }

    pub fn failed_paths(&self) -> Vec<String> {
        let mut out = self.mismatched.clone();
        out.extend(self.missing.iter().cloned());
        out.extend(self.undeclared.iter().cloned());
        out.sort();
        out
    }

    /// Número de archivos con fallo, para el mensaje «falló en N de M».
    pub fn failed_count(&self) -> usize {
        self.mismatched.len() + self.missing.len() + self.undeclared.len()
    }
}

/// Verifica un árbol frente a un manifiesto de integridad.
pub fn verify_against(root: &Path, expected: &IntegrityManifest) -> Result<Verification> {
    let mut v = Verification {
        checked: expected.len(),
        ..Default::default()
    };
    let presentes = walk::conserved_files(root);
    let mut vistos = std::collections::HashSet::new();

    for entry in &presentes {
        if entry.relative == crate::manifest::INTEGRITY_FILE {
            continue;
        }
        match expected.digest_of(&entry.relative) {
            Some(esperado) => {
                vistos.insert(entry.relative.clone());
                let real = digest_file(&entry.path)?;
                if real != esperado {
                    v.mismatched.push(entry.relative.clone());
                }
            }
            None => v.undeclared.push(entry.relative.clone()),
        }
    }
    for (path, _) in expected.entries() {
        if !vistos.contains(path) {
            v.missing.push(path.to_string());
        }
    }
    v.mismatched.sort();
    v.missing.sort();
    v.undeclared.sort();
    Ok(v)
}

/// Verifica un árbol frente al manifiesto de integridad situado en su raíz.
pub fn verify_tree(root: &Path) -> Result<Verification> {
    let manifest_path = root.join(crate::manifest::INTEGRITY_FILE);
    let expected = IntegrityManifest::load(&manifest_path)?;
    verify_against(root, &expected)
}

/// Genera y escribe el manifiesto de integridad en la raíz del árbol.
pub fn generate(root: &Path) -> Result<IntegrityManifest> {
    let m = compute(root)?;
    m.write(&root.join(crate::manifest::INTEGRITY_FILE))?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn arbol() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("07_MASTER")).unwrap();
        fs::write(dir.path().join("PROJECT.yaml"), b"uid: A\n").unwrap();
        fs::write(dir.path().join("07_MASTER/m.wav"), b"audio del master").unwrap();
        dir
    }

    #[test]
    fn el_resumen_coincide_con_el_valor_conocido() {
        // Vector de prueba de SHA-256 para la cadena vacía.
        assert_eq!(
            digest_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn el_formato_es_el_de_las_utilidades_del_sistema() {
        let dir = arbol();
        let m = compute(dir.path()).unwrap();
        let texto = m.render();
        for linea in texto.lines() {
            let (resumen, ruta) = linea.split_once("  ").expect("separador de dos espacios");
            assert_eq!(resumen.len(), 64, "SHA-256 en hexadecimal");
            assert!(resumen.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(!ruta.starts_with('/'), "ruta relativa");
        }
        assert_eq!(IntegrityManifest::parse(&texto), m);
    }

    #[test]
    fn el_manifiesto_es_determinista() {
        let dir = arbol();
        assert_eq!(
            compute(dir.path()).unwrap().render(),
            compute(dir.path()).unwrap().render()
        );
    }

    #[test]
    fn verifica_un_arbol_intacto() {
        let dir = arbol();
        generate(dir.path()).unwrap();
        let v = verify_tree(dir.path()).unwrap();
        assert!(v.is_ok(), "{v:?}");
        assert_eq!(v.checked, 2);
        assert_eq!(v.failed_count(), 0);
    }

    #[test]
    fn detecta_alteracion_ausencia_y_archivo_no_declarado() {
        let dir = arbol();
        let esperado = compute(dir.path()).unwrap();

        fs::write(dir.path().join("07_MASTER/m.wav"), b"audio alterado").unwrap();
        let v = verify_against(dir.path(), &esperado).unwrap();
        assert_eq!(v.mismatched, vec!["07_MASTER/m.wav"]);

        fs::remove_file(dir.path().join("07_MASTER/m.wav")).unwrap();
        let v = verify_against(dir.path(), &esperado).unwrap();
        assert_eq!(v.missing, vec!["07_MASTER/m.wav"]);

        fs::write(dir.path().join("intruso.wav"), b"no declarado").unwrap();
        let v = verify_against(dir.path(), &esperado).unwrap();
        assert_eq!(v.undeclared, vec!["intruso.wav"]);
        assert!(!v.is_ok());
    }

    #[test]
    fn los_regenerables_del_anexo_c_no_entran_en_el_manifiesto() {
        let dir = arbol();
        fs::write(dir.path().join(".DS_Store"), b"basura").unwrap();
        fs::write(dir.path().join("07_MASTER/m.wav.asd"), b"analisis").unwrap();
        let m = compute(dir.path()).unwrap();
        assert_eq!(m.len(), 2);
        assert!(m.digest_of(".DS_Store").is_none());
        assert!(m.digest_of("07_MASTER/m.wav.asd").is_none());
        // Y su presencia no produce falso positivo en la verificación.
        generate(dir.path()).unwrap();
        assert!(verify_tree(dir.path()).unwrap().is_ok());
    }

    #[test]
    fn el_mensaje_de_fallo_lleva_las_cifras() {
        let e = Error::Integrity {
            checked: 128,
            failed: vec!["a".into(), "b".into(), "c".into()],
        };
        let t = e.to_string();
        assert!(t.contains("falló en 3 de 128 archivos"), "{t}");
        assert!(t.contains("no se ha completado"), "{t}");
    }
}
