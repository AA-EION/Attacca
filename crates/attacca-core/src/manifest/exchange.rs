//! Manifiesto de intercambio (apartado 33 y Anexo B.2).
//!
//! Un paquete cuyo manifiesto omita cualquier campo exigible no es conforme y
//! debe ser rechazado por la parte receptora (apartado 33).

use super::Manifest;
use crate::doc::{ArtifactKind, Map, Node};
use crate::error::{Error, Result};
use std::path::Path;

/// Perfiles de intercambio (Tabla 28).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Entrega para uso final. No cede la custodia.
    Delivery,
    /// Producción: la parte receptora continúa el trabajo.
    Production,
    /// Custodia: transferencia de custodia o depósito de archivo.
    Archive,
}

impl Profile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Delivery => "E",
            Profile::Production => "P",
            Profile::Archive => "A",
        }
    }

    pub fn parse(s: &str) -> Option<Profile> {
        Some(match s {
            "E" => Profile::Delivery,
            "P" => Profile::Production,
            "A" => Profile::Archive,
            _ => return None,
        })
    }

    /// El perfil `E` no debe ceder la custodia (apartado 41.1, segundo guion).
    pub fn may_transfer_custody(&self) -> bool {
        !matches!(self, Profile::Delivery)
    }

    /// El perfil `P` debe declarar los parámetros de continuación
    /// (apartado 31.3).
    pub fn requires_continuation(&self) -> bool {
        matches!(self, Profile::Production)
    }

    /// El perfil `A` debe incluir la documentación de derechos íntegra; los
    /// perfiles `E` y `P` aplican la minimización del apartado 34.1.
    pub fn requires_full_rights(&self) -> bool {
        matches!(self, Profile::Archive)
    }

    /// Carpeta en la que se congela la copia del paquete emitido
    /// (apartado 32.3).
    pub fn frozen_copy_dir(&self) -> &'static str {
        match self {
            Profile::Delivery => "08_DELIVERY",
            Profile::Production | Profile::Archive => "09_TRANSFER",
        }
    }
}

/// Niveles de clasificación (Tabla 30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Classification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

impl Classification {
    pub fn as_str(&self) -> &'static str {
        match self {
            Classification::Public => "PUBLICO",
            Classification::Internal => "INTERNO",
            Classification::Confidential => "CONFIDENCIAL",
            Classification::Restricted => "RESTRINGIDO",
        }
    }

    pub fn parse(s: &str) -> Option<Classification> {
        Some(match s {
            "PUBLICO" => Classification::Public,
            "INTERNO" => Classification::Internal,
            "CONFIDENCIAL" => Classification::Confidential,
            "RESTRINGIDO" => Classification::Restricted,
            _ => return None,
        })
    }

    /// El manifiesto de integridad debe firmarse en los envíos clasificados
    /// como `CONFIDENCIAL` o `RESTRINGIDO` (apartado 35.1, tercer guion).
    pub fn requires_signature(&self) -> bool {
        *self >= Classification::Confidential
    }

    /// Estos envíos deben cifrarse o transmitirse por canal cifrado de extremo a
    /// extremo (apartado 35.2).
    pub fn requires_encryption(&self) -> bool {
        *self >= Classification::Confidential
    }

    /// El envío debe ser atribuible a un destinatario único (apartado 35.4).
    pub fn requires_unique_recipient(&self) -> bool {
        *self == Classification::Restricted
    }
}

/// Campos exigibles siempre (Tabla 29). Su ausencia obliga al rechazo.
pub const REQUIRED_PATHS: &[&str] = &[
    "stave.version",
    "stave.profile",
    "stave.level",
    "shipment.id",
    "shipment.issued",
    "parties.issuer.org",
    "parties.recipient.org",
    "purpose",
    "scope.file_count",
    "scope.total_bytes",
    "integrity.algorithm",
    "integrity.manifest",
    "classification",
    "usage.permitted",
    "usage.territory",
    "usage.term",
    "usage.sublicensing",
    "usage.forwarding",
    "retention.until",
    "retention.action_on_expiry",
    "personal_data.present",
    "serialization.container",
    "expected_checks",
    "acknowledgement.deadline_hours",
    "acknowledgement.address",
];

/// Vista tipada del manifiesto de intercambio.
#[derive(Debug)]
pub struct ExchangeManifest(pub Manifest);

impl ExchangeManifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(Manifest::load(path, ArtifactKind::Exchange)?))
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(Manifest::new(path.as_ref(), ArtifactKind::Exchange))
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

    pub fn shipment_id(&self) -> Option<&str> {
        self.0.doc.at("shipment.id").and_then(|n| n.present_str())
    }

    pub fn issued(&self) -> Option<&str> {
        self.0.doc.at("shipment.issued").and_then(|n| n.present_str())
    }

    pub fn supersedes(&self) -> Option<&str> {
        self.0
            .doc
            .at("shipment.supersedes")
            .and_then(|n| n.present_str())
    }

    pub fn profile(&self) -> Option<Profile> {
        self.0
            .doc
            .at("stave.profile")
            .and_then(|n| n.as_str())
            .and_then(Profile::parse)
    }

    pub fn stave_version(&self) -> Option<&str> {
        self.0.doc.at("stave.version").and_then(|n| n.present_str())
    }

    pub fn classification(&self) -> Option<Classification> {
        self.0
            .doc
            .get("classification")
            .and_then(|n| n.as_str())
            .and_then(Classification::parse)
    }

    pub fn issuer_org(&self) -> Option<&str> {
        self.0
            .doc
            .at("parties.issuer.org")
            .and_then(|n| n.present_str())
    }

    pub fn recipient_org(&self) -> Option<&str> {
        self.0
            .doc
            .at("parties.recipient.org")
            .and_then(|n| n.present_str())
    }

    pub fn file_count(&self) -> Option<i64> {
        self.0.doc.at("scope.file_count").and_then(|n| n.as_int())
    }

    pub fn total_bytes(&self) -> Option<i64> {
        self.0.doc.at("scope.total_bytes").and_then(|n| n.as_int())
    }

    pub fn projects(&self) -> Vec<String> {
        self.0
            .doc
            .at("scope.projects")
            .and_then(|n| n.as_seq())
            .map(|s| s.iter().filter_map(|n| n.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    pub fn signature_name(&self) -> Option<&str> {
        self.0
            .doc
            .at("integrity.signature")
            .and_then(|n| n.present_str())
    }

    /// El envío cede la custodia (apartado 41.1: únicamente cuando lo declara de
    /// forma expresa).
    pub fn transfers_custody(&self) -> bool {
        self.0
            .doc
            .at("custody.transfers")
            .and_then(|n| n.as_bool())
            .unwrap_or(false)
    }

    /// El envío devuelve la custodia (apartado 41.3).
    pub fn returns_custody(&self) -> bool {
        self.0
            .doc
            .at("custody.returns")
            .and_then(|n| n.as_bool())
            .unwrap_or(false)
    }

    pub fn affects_custody(&self) -> bool {
        self.transfers_custody() || self.returns_custody()
    }

    pub fn expected_return(&self) -> Option<&str> {
        self.0
            .doc
            .at("custody.expected_return")
            .and_then(|n| n.present_str())
    }

    pub fn grace_days(&self) -> i64 {
        self.0
            .doc
            .at("custody.grace_days")
            .and_then(|n| n.as_int())
            .unwrap_or(0)
    }

    /// La cesión sucesiva a un tercero requiere autorización expresa
    /// (apartado 41.1, último párrafo).
    pub fn onward_allowed(&self) -> bool {
        self.0
            .doc
            .at("custody.onward_allowed")
            .and_then(|n| n.as_bool())
            .unwrap_or(false)
    }

    pub fn ack_deadline_hours(&self) -> i64 {
        self.0
            .doc
            .at("acknowledgement.deadline_hours")
            .and_then(|n| n.as_int())
            .unwrap_or(72)
    }

    pub fn ack_address(&self) -> Option<&str> {
        self.0
            .doc
            .at("acknowledgement.address")
            .and_then(|n| n.present_str())
    }

    /// El paquete se entrega serializado como contenedor `.stave`
    /// (apartado 32.4.5).
    pub fn is_serialized(&self) -> bool {
        self.0
            .doc
            .at("serialization.container")
            .and_then(|n| n.as_str())
            .map(|s| s == "stave")
            .unwrap_or(true)
    }

    /// Asientos de la cronología generados por la parte emisora para este envío.
    pub fn custody_history(&self) -> Vec<crate::custody::ChronologyEntry> {
        self.0
            .doc
            .at("custody.history")
            .and_then(|n| n.as_seq())
            .map(|s| {
                s.iter()
                    .filter_map(crate::custody::ChronologyEntry::from_node)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn expected_checks(&self) -> Vec<String> {
        self.0
            .doc
            .get("expected_checks")
            .and_then(|n| n.as_seq())
            .map(|s| s.iter().filter_map(|n| n.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    /// Verificación 6 del apartado 37.2: el manifiesto valida frente al esquema
    /// del Anexo B.2 y contiene todos los campos exigibles.
    pub fn validate_schema(&self) -> Result<()> {
        let mut faltan = Vec::new();
        for path in REQUIRED_PATHS {
            match self.0.doc.at(path) {
                None => faltan.push(*path),
                Some(n) if n.is_null() => faltan.push(*path),
                _ => {}
            }
        }
        // Los parámetros de continuación son exigibles en el perfil `P`.
        if self.profile() == Some(Profile::Production) {
            for campo in [
                "continuation.sample_rate",
                "continuation.bit_depth",
                "continuation.tuning_hz",
                "continuation.origin",
                "continuation.vocabulary_frozen",
            ] {
                match self.0.doc.at(campo) {
                    None => faltan.push(campo),
                    Some(n) if n.is_null() => faltan.push(campo),
                    _ => {}
                }
            }
        }
        // Los campos de custodia son exigibles cuando el envío la afecta.
        if self.affects_custody() && self.transfers_custody() {
            for campo in ["custody.expected_return", "custody.grace_days"] {
                match self.0.doc.at(campo) {
                    None => faltan.push(campo),
                    Some(n) if n.is_null() => faltan.push(campo),
                    _ => {}
                }
            }
        }
        if faltan.is_empty() {
            return Ok(());
        }
        Err(Error::requirement(
            "33",
            format!(
                "El manifiesto de intercambio omite {} campos exigibles. El paquete no es conforme y debe rechazarse. Campos ausentes: {}.",
                faltan.len(),
                faltan.join(", ")
            ),
        ))
    }

    /// Coherencia interna del manifiesto, más allá de la presencia de campos.
    pub fn validate_consistency(&self) -> Result<()> {
        let perfil = self.profile().ok_or_else(|| {
            Error::requirement("31.3", "El manifiesto no declara un perfil de intercambio admisible. El paquete debe rechazarse. Los perfiles son E, P y A.")
        })?;
        if self.transfers_custody() && !perfil.may_transfer_custody() {
            return Err(Error::requirement(
                "41.1",
                "El envío declara el perfil E y la cesión de la custodia. El paquete debe rechazarse. Un envío de perfil E es una entrega para uso final y no cede la custodia.",
            ));
        }
        if self.classification().is_none() {
            return Err(Error::requirement(
                "34.2",
                "El manifiesto no declara un nivel de clasificación admisible. El paquete debe rechazarse. Los niveles son PUBLICO, INTERNO, CONFIDENCIAL y RESTRINGIDO.",
            ));
        }
        if let Some(ts) = self.issued() {
            crate::clock::parse_rfc3339(ts)?;
        }
        Ok(())
    }
}

/// Construye el esqueleto de un manifiesto de intercambio con todos los campos
/// del Anexo B.2, en nulo cuando no proceden.
#[allow(clippy::too_many_arguments)]
pub struct ExchangeBuilder {
    pub profile: Profile,
    pub level: String,
    pub shipment_id: String,
    pub issued: String,
    pub supersedes: Option<String>,
    pub revision: String,
    pub issuer_org: String,
    pub issuer_contact: String,
    pub issuer_key_id: Option<String>,
    pub recipient_org: String,
    pub recipient_contact: String,
    pub purpose: String,
    pub projects: Vec<String>,
    pub releases: Vec<String>,
    pub file_count: i64,
    pub total_bytes: i64,
    pub signature: Option<String>,
    pub classification: Classification,
    pub usage_permitted: Vec<String>,
    pub usage_territory: String,
    pub usage_term: String,
    pub sublicensing: bool,
    pub forwarding: bool,
    pub retention_until: String,
    pub destroy_on_expiry: bool,
    pub personal_data: bool,
    pub personal_data_categories: Vec<String>,
    pub serialized: bool,
    pub ack_deadline_hours: i64,
    pub ack_address: String,
}

impl ExchangeBuilder {
    pub fn build(&self) -> Map {
        let mut d = Map::new();
        d.set(
            "stave",
            Node::map(vec![
                ("version", Node::str(crate::STAVE_VERSION)),
                ("profile", Node::str(self.profile.as_str())),
                ("level", Node::str(&self.level)),
                ("written_by", Node::str(crate::written_by())),
            ]),
        );
        d.set(
            "shipment",
            Node::map(vec![
                ("id", Node::str(&self.shipment_id)),
                ("issued", Node::str(&self.issued)),
                ("supersedes", Node::opt_str(self.supersedes.as_deref())),
                ("revision", Node::str(&self.revision)),
            ]),
        );
        d.set(
            "parties",
            Node::map(vec![
                (
                    "issuer",
                    Node::map(vec![
                        ("org", Node::str(&self.issuer_org)),
                        ("contact", Node::str(&self.issuer_contact)),
                        ("key_id", Node::opt_str(self.issuer_key_id.as_deref())),
                    ]),
                ),
                (
                    "recipient",
                    Node::map(vec![
                        ("org", Node::str(&self.recipient_org)),
                        ("contact", Node::str(&self.recipient_contact)),
                    ]),
                ),
            ]),
        );
        d.set("purpose", Node::str(&self.purpose));
        d.set(
            "scope",
            Node::map(vec![
                ("projects", seq_of(&self.projects)),
                ("releases", seq_of(&self.releases)),
                ("file_count", Node::Int(self.file_count)),
                ("total_bytes", Node::Int(self.total_bytes)),
            ]),
        );
        d.set(
            "integrity",
            Node::map(vec![
                ("algorithm", Node::str(crate::integrity::ALGORITHM)),
                ("manifest", Node::str(super::super::package::bagit::MANIFEST_TXT)),
                ("signature", Node::opt_str(self.signature.as_deref())),
            ]),
        );
        d.set("classification", Node::str(self.classification.as_str()));
        d.set(
            "usage",
            Node::map(vec![
                ("permitted", seq_of(&self.usage_permitted)),
                ("territory", Node::str(&self.usage_territory)),
                ("term", Node::str(&self.usage_term)),
                ("sublicensing", Node::Bool(self.sublicensing)),
                ("forwarding", Node::Bool(self.forwarding)),
            ]),
        );
        d.set(
            "retention",
            Node::map(vec![
                ("until", Node::str(&self.retention_until)),
                (
                    "action_on_expiry",
                    Node::str(if self.destroy_on_expiry { "destroy" } else { "keep" }),
                ),
                ("confirmation_required", Node::Bool(self.destroy_on_expiry)),
            ]),
        );
        d.set(
            "personal_data",
            Node::map(vec![
                ("present", Node::Bool(self.personal_data)),
                ("categories", seq_of(&self.personal_data_categories)),
                ("issuer_role", Node::Null),
                ("recipient_role", Node::Null),
            ]),
        );
        d.set(
            "watermark",
            Node::map(vec![
                ("present", Node::Bool(false)),
                ("scope", Node::str("none")),
            ]),
        );
        d.set(
            "serialization",
            Node::map(vec![
                (
                    "container",
                    Node::str(if self.serialized { "stave" } else { "none" }),
                ),
                (
                    "mimetype",
                    Node::str(super::super::package::container::MIMETYPE),
                ),
            ]),
        );
        d.set(
            "expected_checks",
            Node::Seq(
                ["container", "integrity", "schema", "structure"]
                    .iter()
                    .map(|s| Node::str(*s))
                    .collect(),
            ),
        );
        d.set(
            "acknowledgement",
            Node::map(vec![
                ("deadline_hours", Node::Int(self.ack_deadline_hours)),
                ("address", Node::str(&self.ack_address)),
            ]),
        );
        d.set("exceptions", Node::Seq(Vec::new()));
        d
    }
}

fn seq_of(values: &[String]) -> Node {
    Node::Seq(values.iter().map(Node::str).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constructor() -> ExchangeBuilder {
        ExchangeBuilder {
            profile: Profile::Delivery,
            level: "B".into(),
            shipment_id: "20260806-AAAAAAAA".into(),
            issued: "2026-08-06T17:20:00-05:00".into(),
            supersedes: None,
            revision: "r0".into(),
            issuer_org: "Estudio A".into(),
            issuer_contact: "intercambio@a.example".into(),
            issuer_key_id: None,
            recipient_org: "Sello B".into(),
            recipient_contact: "recepcion@b.example".into(),
            purpose: "Publicacion digital del sencillo".into(),
            projects: vec!["01J9ZQ8F3K7N2VYB4T6XM0RSAE".into()],
            releases: vec![],
            file_count: 12,
            total_bytes: 840_000,
            signature: None,
            classification: Classification::Internal,
            usage_permitted: vec!["distribucion digital".into()],
            usage_territory: "mundial".into(),
            usage_term: "indefinido".into(),
            sublicensing: false,
            forwarding: false,
            retention_until: "2031-08-06".into(),
            destroy_on_expiry: true,
            personal_data: false,
            personal_data_categories: vec![],
            serialized: true,
            ack_deadline_hours: 72,
            ack_address: "intercambio@a.example".into(),
        }
    }

    fn manifiesto(b: &ExchangeBuilder) -> ExchangeManifest {
        let mut m = ExchangeManifest::new(Path::new("EXCHANGE.yaml"));
        *m.doc_mut() = b.build();
        m
    }

    #[test]
    fn un_manifiesto_completo_valida() {
        let m = manifiesto(&constructor());
        m.validate_schema().unwrap();
        m.validate_consistency().unwrap();
        assert_eq!(m.profile(), Some(Profile::Delivery));
        assert_eq!(m.classification(), Some(Classification::Internal));
        assert_eq!(m.shipment_id(), Some("20260806-AAAAAAAA"));
    }

    #[test]
    fn la_omision_de_un_campo_exigible_obliga_al_rechazo() {
        let mut m = manifiesto(&constructor());
        m.doc_mut().remove("purpose");
        let e = m.validate_schema().unwrap_err();
        assert_eq!(e.clause(), Some("33"));
        assert!(e.to_string().contains("purpose"), "{e}");
        assert!(e.to_string().contains("debe rechazarse"), "{e}");
    }

    #[test]
    fn el_perfil_p_exige_los_parametros_de_continuacion() {
        let mut b = constructor();
        b.profile = Profile::Production;
        let mut m = manifiesto(&b);
        assert!(m.validate_schema().is_err(), "faltan los parámetros de continuación");

        m.doc_mut().set(
            "continuation",
            Node::map(vec![
                ("sample_rate", Node::Int(48000)),
                ("bit_depth", Node::Int(24)),
                ("tuning_hz", Node::Int(440)),
                ("tempo", Node::Int(96)),
                ("origin", Node::str("00:00:00:00")),
                ("vocabulary_frozen", Node::Bool(true)),
            ]),
        );
        m.validate_schema().unwrap();
    }

    #[test]
    fn el_perfil_e_no_puede_ceder_la_custodia() {
        let mut m = manifiesto(&constructor());
        m.doc_mut().set(
            "custody",
            Node::map(vec![("transfers", Node::Bool(true))]),
        );
        let e = m.validate_consistency().unwrap_err();
        assert_eq!(e.clause(), Some("41.1"));
        assert!(!Profile::Delivery.may_transfer_custody());
        assert!(Profile::Production.may_transfer_custody());
        assert!(Profile::Archive.may_transfer_custody());
    }

    #[test]
    fn la_cesion_exige_fecha_de_retorno_y_plazo_de_gracia() {
        let mut b = constructor();
        b.profile = Profile::Archive;
        let mut m = manifiesto(&b);
        m.doc_mut().set(
            "custody",
            Node::map(vec![
                ("transfers", Node::Bool(true)),
                ("returns", Node::Bool(false)),
            ]),
        );
        let e = m.validate_schema().unwrap_err();
        assert!(e.to_string().contains("custody.expected_return"), "{e}");

        m.doc_mut().ensure_map("custody").set("expected_return", Node::str("2026-08-20"));
        m.doc_mut().ensure_map("custody").set("grace_days", Node::Int(15));
        m.validate_schema().unwrap();
        assert_eq!(m.expected_return(), Some("2026-08-20"));
        assert_eq!(m.grace_days(), 15);
    }

    #[test]
    fn la_clasificacion_determina_firma_y_cifrado() {
        assert!(!Classification::Public.requires_signature());
        assert!(!Classification::Internal.requires_signature());
        assert!(Classification::Confidential.requires_signature());
        assert!(Classification::Restricted.requires_signature());
        assert!(Classification::Confidential.requires_encryption());
        assert!(Classification::Restricted.requires_unique_recipient());
        assert!(!Classification::Confidential.requires_unique_recipient());
    }

    #[test]
    fn la_copia_congelada_va_a_la_carpeta_del_perfil() {
        assert_eq!(Profile::Delivery.frozen_copy_dir(), "08_DELIVERY");
        assert_eq!(Profile::Production.frozen_copy_dir(), "09_TRANSFER");
        assert_eq!(Profile::Archive.frozen_copy_dir(), "09_TRANSFER");
    }

    #[test]
    fn el_reenvio_esta_prohibido_salvo_declaracion_expresa() {
        // Apartado 34.3: salvo declaración explícita en contrario, el reenvío no
        // está permitido. El constructor lo refleja por defecto.
        let d = constructor().build();
        assert_eq!(d.at("usage.forwarding").unwrap().as_bool(), Some(false));
        assert_eq!(d.at("usage.sublicensing").unwrap().as_bool(), Some(false));
    }
}
