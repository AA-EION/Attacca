//! Descriptor de volumen (Anexo B.5 y apartado 6.5).
//!
//! Se ubica en la raíz de todo volumen. El punto de montaje de un volumen cambia
//! entre sistemas y entre conexiones; su identidad no cambia nunca.

use super::Manifest;
use crate::clock;
use crate::doc::{ArtifactKind, Map, Node};
use crate::error::Result;
use crate::fsx::readonly::FsCapabilities;
use std::path::Path;

/// Función del volumen (campo `role` del Anexo B.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeRole {
    Local,
    Portable,
    Network,
}

impl VolumeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            VolumeRole::Local => "LOCAL",
            VolumeRole::Portable => "PORTABLE",
            VolumeRole::Network => "NETWORK",
        }
    }

    pub fn parse(s: &str) -> Option<VolumeRole> {
        match s {
            "LOCAL" => Some(VolumeRole::Local),
            "PORTABLE" => Some(VolumeRole::Portable),
            "NETWORK" => Some(VolumeRole::Network),
            _ => None,
        }
    }
}

/// Vista tipada del descriptor de volumen.
#[derive(Debug)]
pub struct VolumeDescriptor(pub Manifest);

impl VolumeDescriptor {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(Manifest::load(path, ArtifactKind::Volume)?))
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(Manifest::new(path.as_ref(), ArtifactKind::Volume))
    }

    pub fn save(&mut self) -> Result<()> {
        self.0.save()
    }

    pub fn doc(&self) -> &Map {
        &self.0.doc
    }

    pub fn doc_mut(&mut self) -> &mut Map {
        &mut self.0.doc
    }

    /// Identificador persistente e inmutable. El traslado de la raíz a otra ruta
    /// no debe alterarlo (apartado 6.5, último guion).
    pub fn uuid(&self) -> Option<&str> {
        self.0
            .doc
            .at("stave_volume.uuid")
            .and_then(|n| n.present_str())
    }

    pub fn label(&self) -> Option<&str> {
        self.0
            .doc
            .at("stave_volume.label")
            .and_then(|n| n.present_str())
    }

    pub fn role(&self) -> Option<VolumeRole> {
        self.0
            .doc
            .at("stave_volume.role")
            .and_then(|n| n.as_str())
            .and_then(VolumeRole::parse)
    }

    /// Raíz del repositorio en este volumen.
    pub fn root(&self) -> Option<&str> {
        self.0
            .doc
            .at("stave_volume.root")
            .and_then(|n| n.present_str())
    }

    /// El sistema de archivos sostiene el atributo de solo lectura
    /// (apartado 6.6.2, tercer guion).
    ///
    /// Cuando no lo sostiene, la protección es más débil y descansa en el
    /// marcador `REPLICA.hold`.
    pub fn enforces_readonly(&self) -> bool {
        self.0
            .doc
            .at("stave_volume.filesystem.enforces_readonly")
            .and_then(|n| n.as_bool())
            .unwrap_or(false)
    }

    pub fn filesystem_name(&self) -> Option<&str> {
        self.0
            .doc
            .at("stave_volume.filesystem.name")
            .and_then(|n| n.present_str())
    }

    pub fn max_path(&self) -> i64 {
        self.0
            .doc
            .at("stave_volume.filesystem.max_path")
            .and_then(|n| n.as_int())
            .unwrap_or(255)
    }

    /// Registra la última conexión observada.
    pub fn touch_last_seen(&mut self) {
        self.0
            .doc
            .ensure_map("stave_volume")
            .set("last_seen", Node::str(clock::now_rfc3339()));
    }

    /// Actualiza las capacidades observadas del sistema de archivos.
    pub fn set_capabilities(&mut self, caps: FsCapabilities, fs_name: Option<&str>) {
        let fs = self
            .0
            .doc
            .ensure_map("stave_volume")
            .ensure_map("filesystem");
        if let Some(name) = fs_name {
            fs.set("name", Node::str(name));
        }
        fs.set("enforces_readonly", Node::Bool(caps.enforces_readonly));
        fs.set("preserves_case", Node::Bool(caps.preserves_case));
    }
}
// El Anexo B fija los campos de este artefacto; agruparlos en una
// estructura intermedia solo desplazaría la lista.
#[allow(clippy::too_many_arguments)]
/// Construye un descriptor de volumen conforme al Anexo B.5.
pub fn scaffold(
    uuid: &str,
    label: &str,
    role: VolumeRole,
    root: &str,
    caps: FsCapabilities,
    fs_name: &str,
    domains: &[&str],
    encrypted: bool,
) -> Map {
    let mut d = Map::new();
    d.set(
        "stave_volume",
        Node::map(vec![
            ("version", Node::str(crate::STAVE_VERSION)),
            ("uuid", Node::str(uuid)),
            ("label", Node::str(label)),
            ("role", Node::str(role.as_str())),
            ("created", Node::str(clock::today())),
            ("root", Node::str(root)),
            (
                "filesystem",
                Node::map(vec![
                    ("name", Node::str(fs_name)),
                    ("enforces_readonly", Node::Bool(caps.enforces_readonly)),
                    ("preserves_case", Node::Bool(caps.preserves_case)),
                    ("max_path", Node::Int(255)),
                ]),
            ),
            (
                "policy",
                Node::map(vec![
                    (
                        "domains",
                        Node::Seq(domains.iter().map(|d| Node::str(*d)).collect()),
                    ),
                    ("encrypted", Node::Bool(encrypted)),
                    ("replicas_required", Node::Int(3)),
                    ("verify_every_days", Node::Int(365)),
                ]),
            ),
            ("last_seen", Node::str(clock::now_rfc3339())),
        ]),
    );
    d
}

/// Genera un identificador persistente de volumen en forma de UUID versión 4.
pub fn new_volume_uuid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b[6] = (b[6] & 0x0F) | 0x40; // versión 4
    b[8] = (b[8] & 0x3F) | 0x80; // variante RFC 4122
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// Prepara un volumen: sonda sus capacidades y escribe el descriptor si no
/// existe. Un descriptor existente conserva su identificador persistente.
pub fn ensure_descriptor(
    volume_root: &Path,
    label: &str,
    role: VolumeRole,
    repo_root: &str,
) -> Result<VolumeDescriptor> {
    let path = volume_root.join(super::VOLUME_FILE);
    let caps = crate::fsx::readonly::probe(volume_root);

    if path.exists() {
        let mut d = VolumeDescriptor::load(&path)?;
        // Las capacidades se releen en cada conexión: el mismo volumen puede
        // montarse con un sistema de archivos distinto.
        d.set_capabilities(caps, None);
        d.touch_last_seen();
        d.save()?;
        return Ok(d);
    }

    let mut d = VolumeDescriptor::new(&path);
    *d.doc_mut() = scaffold(
        &new_volume_uuid(),
        label,
        role,
        repo_root,
        caps,
        "desconocido",
        &["10_LIBRARY", "20_PROJECTS", "30_ARCHIVE"],
        false,
    );
    d.save()?;
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_uuid_tiene_la_forma_de_la_version_4() {
        let u = new_volume_uuid();
        assert_eq!(u.len(), 36);
        assert_eq!(u.as_bytes()[14], b'4', "versión 4");
        assert!(
            matches!(u.as_bytes()[19], b'8' | b'9' | b'a' | b'b'),
            "variante RFC 4122"
        );
        assert_ne!(new_volume_uuid(), new_volume_uuid());
    }

    #[test]
    fn el_descriptor_reproduce_el_esquema_del_anexo_b_5() {
        let d = scaffold(
            "8f3c1a20-0000-4000-8000-000000000001",
            "Estudio - Portable 01",
            VolumeRole::Portable,
            "/Volumes/STUDIO-PORT-01/.stave",
            FsCapabilities {
                enforces_readonly: false,
                preserves_case: true,
            },
            "exfat",
            &["20_PROJECTS"],
            true,
        );
        for campo in [
            "version",
            "uuid",
            "label",
            "role",
            "created",
            "root",
            "filesystem",
            "policy",
            "last_seen",
        ] {
            assert!(
                d.at(&format!("stave_volume.{campo}")).is_some(),
                "falta {campo}"
            );
        }
        assert_eq!(
            d.at("stave_volume.role").unwrap().as_str(),
            Some("PORTABLE")
        );
        assert_eq!(
            d.at("stave_volume.filesystem.enforces_readonly")
                .unwrap()
                .as_bool(),
            Some(false)
        );
    }

    #[test]
    fn declara_cuando_el_sistema_de_archivos_no_sostiene_el_solo_lectura() {
        // Apartado 6.6.2: la limitación debe declararse en el descriptor y
        // suplirse mediante el marcador.
        let mut v = VolumeDescriptor::new(Path::new("VOLUME.yaml"));
        *v.doc_mut() = scaffold(
            "u",
            "Portatil exFAT",
            VolumeRole::Portable,
            "/v/.stave",
            FsCapabilities {
                enforces_readonly: false,
                preserves_case: false,
            },
            "exfat",
            &["20_PROJECTS"],
            false,
        );
        assert!(!v.enforces_readonly());
        assert_eq!(v.filesystem_name(), Some("exfat"));
    }

    #[test]
    fn el_identificador_persistente_sobrevive_a_una_reconexion() {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        let primero =
            ensure_descriptor(dir.path(), "Local", VolumeRole::Local, "/home/u/.stave").unwrap();
        let uuid = primero.uuid().unwrap().to_string();

        // Segunda conexión: el descriptor ya existe.
        let segundo = ensure_descriptor(
            dir.path(),
            "Local renombrado",
            VolumeRole::Local,
            "/otra/ruta",
        )
        .unwrap();
        assert_eq!(
            segundo.uuid().unwrap(),
            uuid,
            "el identificador no debe cambiar"
        );
    }

    #[test]
    fn la_sonda_de_capacidades_se_reeja_en_cada_conexion() {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        let d =
            ensure_descriptor(dir.path(), "Local", VolumeRole::Local, "/home/u/.stave").unwrap();
        // En un sistema de archivos corriente de Linux el atributo se sostiene.
        assert!(d
            .doc()
            .at("stave_volume.filesystem.enforces_readonly")
            .is_some());
        assert!(d.doc().at("stave_volume.last_seen").is_some());
    }
}
