//! Estructura del paquete conforme a RFC 8493 (apartado 32.1).
//!
//! El paquete es autodescriptivo: la totalidad de la información necesaria para
//! verificarlo, interpretarlo e ingerirlo está en el propio paquete. Quien lo
//! recibe no necesita Attacca ni ninguna otra implementación de la norma.

use crate::error::{Error, Result};
use crate::fsx::atomic;
use crate::integrity::{self, IntegrityManifest};
use std::path::{Path, PathBuf};

pub const BAGIT_TXT: &str = "bagit.txt";
pub const BAG_INFO_TXT: &str = "bag-info.txt";
pub const MANIFEST_TXT: &str = "manifest-sha256.txt";
pub const TAGMANIFEST_TXT: &str = "tagmanifest-sha256.txt";
pub const TAGMANIFEST_SIG: &str = "tagmanifest-sha256.txt.asc";
pub const CUSTODY_TXT: &str = "CUSTODY.txt";
pub const LEEME_TXT: &str = "LEEME.txt";
pub const DATA_DIR: &str = "data";

/// Archivos de etiqueta cubiertos por `tagmanifest-sha256.txt`.
///
/// El apartado 32.1 exige que cubra la totalidad de los archivos de etiqueta,
/// incluidos `EXCHANGE.yaml`, `CUSTODY.txt` y `manifest-sha256.txt`.
pub const TAG_FILES: &[&str] = &[
    BAGIT_TXT,
    BAG_INFO_TXT,
    crate::manifest::EXCHANGE_FILE,
    CUSTODY_TXT,
    LEEME_TXT,
    MANIFEST_TXT,
];

/// Nombre del directorio base del paquete (apartado 32.1).
pub fn package_name(date: &str, issuer: &str, recipient: &str, shipment_id: &str) -> Result<String> {
    let name = format!(
        "STAVE-XCHG_{date}_{}_{}_{}",
        crate::naming::slugify(issuer),
        crate::naming::slugify(recipient),
        crate::naming::slugify(shipment_id)
    );
    crate::naming::check_name(&name)?;
    Ok(name)
}

/// Contenido de `bagit.txt` (RFC 8493).
pub fn bagit_txt() -> String {
    "BagIt-Version: 1.0\nTag-File-Character-Encoding: UTF-8\n".to_string()
}

/// Contenido de `bag-info.txt`: metadatos del contenedor.
pub fn bag_info_txt(
    issuer: &str,
    date: &str,
    payload_bytes: u64,
    file_count: usize,
    shipment_id: &str,
) -> String {
    format!(
        "Source-Organization: {issuer}\n\
         Bagging-Date: {date}\n\
         Bag-Software-Agent: {}\n\
         External-Identifier: {shipment_id}\n\
         Payload-Oxum: {payload_bytes}.{file_count}\n",
        crate::written_by()
    )
}

/// Contenido de `LEEME.txt` (apartados 32.1 y 32.4.2).
///
/// Se redacta para quien no tiene Attacca ni lo tendrá. Indica cómo extraer el
/// contenedor, cómo verificar la firma y los manifiestos con utilidades de línea
/// de órdenes, y a qué dirección dirigir el acuse de recibo.
pub fn leeme_txt(
    package_name: &str,
    shipment_id: &str,
    issuer: &str,
    ack_address: &str,
    ack_deadline_hours: i64,
    signed: bool,
) -> String {
    let firma = if signed {
        format!(
            "\n2. Verificar la firma del manifiesto de etiquetas.\n\
             \n\
             Junto al contenedor se transmite el archivo de firma separada\n\
             {TAGMANIFEST_SIG}. Con GnuPG:\n\
             \n\
             \x20   gpg --verify {package_name}/{TAGMANIFEST_SIG} {package_name}/{TAGMANIFEST_TXT}\n\
             \n\
             La clave publica de {issuer} es la declarada en el acuerdo de\n\
             intercambio entre ambas partes. Si la firma no es valida, o la\n\
             identidad no es la declarada, el envio debe rechazarse sin extraer\n\
             ni utilizar el material.\n"
        )
    } else {
        String::new()
    };

    format!(
        "PAQUETE DE INTERCAMBIO STAVE 2.0\n\
         =================================\n\
         \n\
         Envio:    {shipment_id}\n\
         Emisor:   {issuer}\n\
         Paquete:  {package_name}\n\
         \n\
         Este archivo describe como abrir y verificar el paquete sin ninguna\n\
         herramienta especifica. Todo lo que sigue puede hacerse con las\n\
         utilidades incluidas en cualquier sistema operativo.\n\
         \n\
         \n\
         1. Extraer el contenedor.\n\
         \n\
         El archivo con extension .stave es un archivo comprimido ZIP corriente.\n\
         Basta renombrarlo con extension .zip y descomprimirlo con la utilidad\n\
         del sistema, o abrirlo directamente con cualquier programa de\n\
         descompresion. No hace falta ningun programa especifico.\n\
         \n\
             mv {package_name}.stave {package_name}.zip\n\
             unzip {package_name}.zip\n\
         {firma}\n\
         {}. Verificar la integridad del material.\n\
         \n\
         El archivo {MANIFEST_TXT} asocia a cada archivo su resumen SHA-256.\n\
         Desde el directorio {package_name} :\n\
         \n\
             sha256sum -c {MANIFEST_TXT}      (Linux)\n\
             shasum -a 256 -c {MANIFEST_TXT}  (macOS)\n\
             certutil -hashfile <archivo> SHA256   (Windows, archivo por archivo)\n\
         \n\
         El archivo {TAGMANIFEST_TXT} hace lo mismo con los documentos que\n\
         describen el envio. Se verifica igual.\n\
         \n\
         Si cualquiera de las comprobaciones falla, el envio no debe utilizarse.\n\
         Comunicarlo al emisor indicando el identificador de envio.\n\
         \n\
         \n\
         {}. Leer que contiene el envio.\n\
         \n\
         EXCHANGE.yaml es un archivo de texto que se abre con cualquier editor.\n\
         Declara la finalidad del envio, su alcance, las condiciones de uso, el\n\
         periodo de retencion y, si procede, la cesion de la custodia editorial.\n\
         \n\
         El material esta en data/content/ con la estructura de carpetas de la\n\
         norma. Los archivos de audio se abren con cualquier programa.\n\
         \n\
         \n\
         {}. Acusar recibo.\n\
         \n\
         Debe emitirse un acuse de recibo dentro de las {ack_deadline_hours} horas\n\
         siguientes a la recepcion, tanto si el envio se acepta como si se\n\
         rechaza.\n\
         \n\
         Dirigirlo a: {ack_address}\n\
         \n\
         El acuse debe indicar el identificador de envio {shipment_id}, la fecha\n\
         y hora de recepcion, el resultado de las comprobaciones anteriores y si\n\
         se aceptan las condiciones de uso y el periodo de retencion declarados.\n\
         El apartado 38 de la norma STAVE 2.0 detalla su contenido.\n\
         \n\
         \n\
         Norma aplicada: STAVE 2.0, Parte 4.\n\
         Paquete generado por {}.\n",
        if signed { 3 } else { 2 },
        if signed { 4 } else { 3 },
        if signed { 5 } else { 4 },
        crate::written_by()
    )
}

/// Un paquete en construcción sobre el sistema de archivos.
pub struct PackageDir {
    root: PathBuf,
}

impl PackageDir {
    /// Crea la estructura de directorios del apartado 32.1.
    pub fn create(parent: &Path, package_name: &str) -> Result<PackageDir> {
        crate::naming::check_name(package_name)?;
        let root = parent.join(package_name);
        for sub in ["data/content", "data/rights", "data/log"] {
            let p = root.join(sub);
            std::fs::create_dir_all(&p).map_err(|e| Error::io(&p, e))?;
        }
        Ok(PackageDir { root })
    }

    pub fn open(root: impl Into<PathBuf>) -> PackageDir {
        PackageDir { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// `data/content/`: el material, con la estructura de la Parte 1.
    pub fn content(&self) -> PathBuf {
        self.root.join("data/content")
    }

    /// `data/rights/`: documentación de derechos, según perfil y minimización.
    pub fn rights(&self) -> PathBuf {
        self.root.join("data/rights")
    }

    /// `data/log/EXCHANGE.jsonl`: extracto del registro de eventos del envío.
    pub fn log_file(&self) -> PathBuf {
        self.root.join("data/log/EXCHANGE.jsonl")
    }

    pub fn exchange_manifest(&self) -> PathBuf {
        self.root.join(crate::manifest::EXCHANGE_FILE)
    }

    pub fn custody_txt(&self) -> PathBuf {
        self.root.join(CUSTODY_TXT)
    }

    /// Genera `manifest-sha256.txt`, que cubre la totalidad de `data/`, con las
    /// rutas relativas a la raíz del directorio base (apartado 32.1).
    pub fn write_payload_manifest(&self) -> Result<IntegrityManifest> {
        let data = self.root.join(DATA_DIR);
        let mut m = IntegrityManifest::new();
        for entry in crate::fsx::walk::conserved_files(&data) {
            m.insert(
                format!("{DATA_DIR}/{}", entry.relative),
                integrity::digest_file(&entry.path)?,
            );
        }
        m.write(&self.root.join(MANIFEST_TXT))?;
        Ok(m)
    }

    /// Genera `tagmanifest-sha256.txt`, que cubre la totalidad de los archivos
    /// de etiqueta presentes.
    pub fn write_tag_manifest(&self) -> Result<IntegrityManifest> {
        let mut m = IntegrityManifest::new();
        for name in TAG_FILES {
            let p = self.root.join(name);
            if p.is_file() {
                m.insert(*name, integrity::digest_file(&p)?);
            }
        }
        m.write(&self.root.join(TAGMANIFEST_TXT))?;
        Ok(m)
    }

    pub fn write_bagit(&self) -> Result<()> {
        atomic::write_str(&self.root.join(BAGIT_TXT), &bagit_txt())
    }

    pub fn write_bag_info(
        &self,
        issuer: &str,
        date: &str,
        payload_bytes: u64,
        file_count: usize,
        shipment_id: &str,
    ) -> Result<()> {
        atomic::write_str(
            &self.root.join(BAG_INFO_TXT),
            &bag_info_txt(issuer, date, payload_bytes, file_count, shipment_id),
        )
    }

    pub fn write_leeme(
        &self,
        shipment_id: &str,
        issuer: &str,
        ack_address: &str,
        ack_deadline_hours: i64,
        signed: bool,
    ) -> Result<()> {
        atomic::write_str(
            &self.root.join(LEEME_TXT),
            &leeme_txt(
                &self.name(),
                shipment_id,
                issuer,
                ack_address,
                ack_deadline_hours,
                signed,
            ),
        )
    }

    /// Verifica el empaquetado (verificaciones 4 y 5 del apartado 37.2).
    pub fn verify(&self) -> Result<PackageVerification> {
        let mut v = PackageVerification::default();

        // Archivos de etiqueta exigidos por el apartado 32.1.
        for name in [BAGIT_TXT, crate::manifest::EXCHANGE_FILE, LEEME_TXT, MANIFEST_TXT] {
            if !self.root.join(name).is_file() {
                v.missing_tag_files.push((*name).to_string());
            }
        }

        // Integridad del empaquetado: los archivos de etiqueta frente a
        // `tagmanifest-sha256.txt`.
        let tag_path = self.root.join(TAGMANIFEST_TXT);
        if tag_path.is_file() {
            let esperado = IntegrityManifest::load(&tag_path)?;
            for (name, digest) in esperado.entries() {
                let p = self.root.join(name);
                if !p.is_file() {
                    v.tag_failures.push(name.to_string());
                } else if integrity::digest_file(&p)? != digest {
                    v.tag_failures.push(name.to_string());
                }
            }
            v.tag_checked = esperado.len();
        } else {
            v.missing_tag_files.push(TAGMANIFEST_TXT.to_string());
        }

        // Integridad del contenido frente a `manifest-sha256.txt`.
        let manifest_path = self.root.join(MANIFEST_TXT);
        if manifest_path.is_file() {
            let esperado = IntegrityManifest::load(&manifest_path)?;
            v.payload_checked = esperado.len();
            let mut vistos = std::collections::HashSet::new();
            for (rel, digest) in esperado.entries() {
                let p = self.root.join(rel);
                vistos.insert(rel.to_string());
                if !p.is_file() {
                    v.payload_missing.push(rel.to_string());
                } else if integrity::digest_file(&p)? != digest {
                    v.payload_mismatched.push(rel.to_string());
                }
            }
            // Archivos presentes que el manifiesto no declara.
            let data = self.root.join(DATA_DIR);
            for entry in crate::fsx::walk::conserved_files(&data) {
                let rel = format!("{DATA_DIR}/{}", entry.relative);
                if !vistos.contains(&rel) {
                    v.payload_undeclared.push(rel);
                }
            }
        }

        v.tag_failures.sort();
        v.payload_missing.sort();
        v.payload_mismatched.sort();
        v.payload_undeclared.sort();
        Ok(v)
    }
}

/// Resultado de la verificación del empaquetado.
#[derive(Clone, Debug, Default)]
pub struct PackageVerification {
    pub missing_tag_files: Vec<String>,
    pub tag_checked: usize,
    pub tag_failures: Vec<String>,
    pub payload_checked: usize,
    pub payload_missing: Vec<String>,
    pub payload_mismatched: Vec<String>,
    pub payload_undeclared: Vec<String>,
}

impl PackageVerification {
    /// Verificación 4 del apartado 37.2: archivos de etiqueta y sumas de
    /// verificación de RFC 8493.
    pub fn package_integrity_ok(&self) -> bool {
        self.missing_tag_files.is_empty() && self.tag_failures.is_empty()
    }

    /// Verificación 5: todos los archivos coinciden, sin ausentes ni no
    /// declarados.
    pub fn content_integrity_ok(&self) -> bool {
        self.payload_missing.is_empty()
            && self.payload_mismatched.is_empty()
            && self.payload_undeclared.is_empty()
    }

    pub fn failed_paths(&self) -> Vec<String> {
        let mut out = self.tag_failures.clone();
        out.extend(self.payload_mismatched.iter().cloned());
        out.extend(self.payload_missing.iter().cloned());
        out.extend(self.payload_undeclared.iter().cloned());
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn paquete(dir: &Path) -> PackageDir {
        let p = PackageDir::create(dir, "STAVE-XCHG_2026-08-06_EstudioA_EstudioB_0007").unwrap();
        fs::create_dir_all(p.content().join("07_MASTER")).unwrap();
        fs::write(p.content().join("07_MASTER/m.wav"), b"audio del master").unwrap();
        fs::write(p.rights().join("split.csv"), b"nombre,porcentaje\n").unwrap();
        fs::write(p.log_file(), b"{\"event\":\"exchange.package.built\"}\n").unwrap();
        atomic::write_str(&p.exchange_manifest(), "stave:\n  version: \"2.0\"\n").unwrap();
        p.write_bagit().unwrap();
        p.write_bag_info("Estudio A", "2026-08-06", 16, 1, "0007").unwrap();
        p.write_leeme("0007", "Estudio A", "intercambio@estudioa.example", 72, false).unwrap();
        p.write_payload_manifest().unwrap();
        p.write_tag_manifest().unwrap();
        p
    }

    #[test]
    fn la_estructura_reproduce_el_apartado_32_1() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());
        for archivo in [BAGIT_TXT, BAG_INFO_TXT, "EXCHANGE.yaml", LEEME_TXT, MANIFEST_TXT, TAGMANIFEST_TXT] {
            assert!(p.root().join(archivo).is_file(), "falta {archivo}");
        }
        assert!(p.content().is_dir());
        assert!(p.rights().is_dir());
        assert!(p.log_file().is_file());
    }

    #[test]
    fn el_manifiesto_de_carga_expresa_rutas_relativas_a_la_raiz_del_paquete() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());
        let m = IntegrityManifest::load(&p.root().join(MANIFEST_TXT)).unwrap();
        assert!(m.digest_of("data/content/07_MASTER/m.wav").is_some());
        assert!(m.digest_of("data/rights/split.csv").is_some());
        // No cubre los archivos de etiqueta: eso es cometido del otro manifiesto.
        assert!(m.digest_of("EXCHANGE.yaml").is_none());
    }

    #[test]
    fn el_manifiesto_de_etiquetas_cubre_el_manifiesto_de_carga() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());
        let t = IntegrityManifest::load(&p.root().join(TAGMANIFEST_TXT)).unwrap();
        // Apartado 32.1: debe cubrir EXCHANGE.yaml, CUSTODY.txt y
        // manifest-sha256.txt.
        assert!(t.digest_of("EXCHANGE.yaml").is_some());
        assert!(t.digest_of(MANIFEST_TXT).is_some());
        assert!(t.digest_of(BAGIT_TXT).is_some());
    }

    #[test]
    fn un_paquete_intacto_supera_la_verificacion() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());
        let v = p.verify().unwrap();
        assert!(v.package_integrity_ok(), "{v:?}");
        assert!(v.content_integrity_ok(), "{v:?}");
        assert_eq!(v.payload_checked, 3);
    }

    #[test]
    fn detecta_la_alteracion_del_material_y_la_del_manifiesto() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());

        fs::write(p.content().join("07_MASTER/m.wav"), b"audio alterado").unwrap();
        let v = p.verify().unwrap();
        assert!(!v.content_integrity_ok());
        assert_eq!(v.payload_mismatched, vec!["data/content/07_MASTER/m.wav"]);
        // Una alteración del material no afecta a los archivos de etiqueta.
        assert!(v.package_integrity_ok());

        // Una alteración del manifiesto de intercambio sí es detectable de forma
        // independiente, que es el motivo de separar ambos manifiestos.
        fs::write(p.exchange_manifest(), "stave:\n  version: \"9.9\"\n").unwrap();
        let v = p.verify().unwrap();
        assert!(!v.package_integrity_ok());
        assert_eq!(v.tag_failures, vec!["EXCHANGE.yaml"]);
    }

    #[test]
    fn detecta_un_archivo_no_declarado_en_la_carga() {
        let dir = tempfile::tempdir().unwrap();
        let p = paquete(dir.path());
        fs::write(p.content().join("intruso.wav"), b"no declarado").unwrap();
        let v = p.verify().unwrap();
        assert_eq!(v.payload_undeclared, vec!["data/content/intruso.wav"]);
        assert!(!v.content_integrity_ok());
    }

    #[test]
    fn el_leeme_explica_como_extraer_sin_attacca() {
        let t = leeme_txt("STAVE-XCHG_2026-08-06_A_B_0007", "0007", "Estudio A", "correo@a.example", 72, false);
        assert!(t.contains(".zip"), "debe indicar el renombrado a .zip");
        assert!(t.contains("unzip"));
        assert!(t.contains("sha256sum -c"));
        assert!(t.contains("shasum -a 256 -c"));
        assert!(t.contains("correo@a.example"), "debe indicar la dirección del acuse");
        assert!(t.contains("72 horas"));
        assert!(t.is_ascii(), "texto plano sin caracteres que dependan de la codificación");
    }

    #[test]
    fn el_leeme_firmado_explica_la_verificacion_de_la_firma() {
        let t = leeme_txt("P", "0007", "Estudio A", "c@a.example", 72, true);
        assert!(t.contains("gpg --verify"));
        assert!(t.contains(TAGMANIFEST_SIG));
    }

    #[test]
    fn el_nombre_del_paquete_cumple_la_nomenclatura() {
        let n = package_name("2026-08-06", "Estudio A", "Estudio B", "0007").unwrap();
        assert_eq!(n, "STAVE-XCHG_2026-08-06_Estudio-A_Estudio-B_0007");
        assert!(crate::naming::is_valid_name(&n));
    }
}
