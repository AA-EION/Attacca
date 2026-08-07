//! Acuse de recibo (apartado 38 y Anexo B.3).
//!
//! La parte receptora emite un acuse dentro del plazo declarado, con
//! independencia de que el resultado sea la aceptación o el rechazo. Un envío no
//! se considera completado hasta que la parte emisora lo recibe.

use super::Manifest;
use crate::doc::{ArtifactKind, Map, Node};
use crate::error::Result;
use std::path::Path;

/// Resultado de una verificación individual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    NotApplicable,
}

impl Outcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::NotApplicable => "not_applicable",
        }
    }

    pub fn parse(s: &str) -> Option<Outcome> {
        Some(match s {
            "pass" => Outcome::Pass,
            "fail" => Outcome::Fail,
            "not_applicable" => Outcome::NotApplicable,
            _ => return None,
        })
    }

    pub fn from_bool(ok: bool) -> Outcome {
        if ok {
            Outcome::Pass
        } else {
            Outcome::Fail
        }
    }

    pub fn failed(&self) -> bool {
        matches!(self, Outcome::Fail)
    }
}

/// Resultado global del acuse (apartado 38).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptResult {
    Accepted,
    AcceptedWithReservations,
    Rejected,
}

impl ReceiptResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReceiptResult::Accepted => "accepted",
            ReceiptResult::AcceptedWithReservations => "accepted_with_reservations",
            ReceiptResult::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Option<ReceiptResult> {
        Some(match s {
            "accepted" => ReceiptResult::Accepted,
            "accepted_with_reservations" => ReceiptResult::AcceptedWithReservations,
            "rejected" => ReceiptResult::Rejected,
            _ => return None,
        })
    }

    /// El material solo se ingiere si el envío no fue rechazado.
    pub fn allows_ingest(&self) -> bool {
        !matches!(self, ReceiptResult::Rejected)
    }
}

/// Las catorce verificaciones del apartado 37.2, en el orden de la Tabla 31.
#[derive(Clone, Debug)]
pub struct VerificationSet {
    pub provenance: Outcome,
    pub authenticity: Outcome,
    pub container: Outcome,
    pub package_integrity: Outcome,
    pub content_integrity: Outcome,
    pub manifest_schema: Outcome,
    pub profile_supported: Outcome,
    pub custody: Outcome,
    pub chronology: Outcome,
    pub scope_match: Outcome,
    pub usage_accepted: Outcome,
    pub personal_data_match: Outcome,
    pub structure: Outcome,
    pub declared_checks: Outcome,
}

impl Default for VerificationSet {
    fn default() -> Self {
        Self {
            provenance: Outcome::NotApplicable,
            authenticity: Outcome::NotApplicable,
            container: Outcome::NotApplicable,
            package_integrity: Outcome::NotApplicable,
            content_integrity: Outcome::NotApplicable,
            manifest_schema: Outcome::NotApplicable,
            profile_supported: Outcome::NotApplicable,
            custody: Outcome::NotApplicable,
            chronology: Outcome::NotApplicable,
            scope_match: Outcome::NotApplicable,
            usage_accepted: Outcome::NotApplicable,
            personal_data_match: Outcome::NotApplicable,
            structure: Outcome::NotApplicable,
            declared_checks: Outcome::NotApplicable,
        }
    }
}

impl VerificationSet {
    /// Verificaciones 1 a 6 de la Tabla 31, cuyo fallo obliga al rechazo íntegro
    /// del paquete (apartado 37.2, último párrafo).
    pub fn blocking_failed(&self) -> bool {
        self.provenance.failed()
            || self.authenticity.failed()
            || self.container.failed()
            || self.package_integrity.failed()
            || self.content_integrity.failed()
            || self.manifest_schema.failed()
    }

    /// Verificaciones 7 a 14, cuyo fallo admite rechazo, consulta o no
    /// conformidad según la Tabla 31.
    pub fn non_blocking_failed(&self) -> bool {
        self.profile_supported.failed()
            || self.custody.failed()
            || self.chronology.failed()
            || self.scope_match.failed()
            || self.usage_accepted.failed()
            || self.personal_data_match.failed()
            || self.structure.failed()
            || self.declared_checks.failed()
    }

    /// Resultado global que corresponde al conjunto de verificaciones.
    pub fn result(&self) -> ReceiptResult {
        if self.blocking_failed() {
            ReceiptResult::Rejected
        } else if self.non_blocking_failed() {
            ReceiptResult::AcceptedWithReservations
        } else {
            ReceiptResult::Accepted
        }
    }

    /// Nombre de la primera verificación que falló, para el acuse de rechazo
    /// (apartado 39.1, primer guion).
    pub fn first_failure(&self) -> Option<&'static str> {
        for (name, outcome) in self.as_pairs() {
            if outcome.failed() {
                return Some(name);
            }
        }
        None
    }

    /// Las catorce verificaciones con su resultado, en el orden de la Tabla 31.
    pub fn as_pairs(&self) -> [(&'static str, Outcome); 14] {
        [
            ("provenance", self.provenance),
            ("authenticity", self.authenticity),
            ("container", self.container),
            ("package_integrity", self.package_integrity),
            ("content_integrity", self.content_integrity),
            ("manifest_schema", self.manifest_schema),
            ("profile_supported", self.profile_supported),
            ("custody", self.custody),
            ("chronology", self.chronology),
            ("scope_match", self.scope_match),
            ("usage_accepted", self.usage_accepted),
            ("personal_data_match", self.personal_data_match),
            ("structure", self.structure),
            ("declared_checks", self.declared_checks),
        ]
    }

    pub fn to_node(&self) -> Node {
        Node::map(
            self.as_pairs()
                .iter()
                .map(|(k, v)| (*k, Node::str(v.as_str())))
                .collect(),
        )
    }
}

/// Vista tipada del acuse de recibo.
#[derive(Debug)]
pub struct Receipt(pub Manifest);

impl Receipt {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(Manifest::load(path, ArtifactKind::Receipt)?))
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(Manifest::new(path.as_ref(), ArtifactKind::Receipt))
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
        self.0
            .doc
            .at("receipt.shipment_id")
            .and_then(|n| n.present_str())
    }

    pub fn result(&self) -> Option<ReceiptResult> {
        self.0
            .doc
            .get("result")
            .and_then(|n| n.as_str())
            .and_then(ReceiptResult::parse)
    }

    /// Resumen del manifiesto de integridad recibido: acredita qué se recibió
    /// exactamente (apartado 38, cuarto guion).
    pub fn manifest_digest(&self) -> Option<&str> {
        self.0
            .doc
            .at("receipt.manifest_digest")
            .and_then(|n| n.present_str())
    }

    /// Aceptación expresa de la custodia (apartado 41.2, último párrafo).
    pub fn custody_accepted(&self) -> Option<bool> {
        self.0.doc.at("custody.accepted").and_then(|n| n.as_bool())
    }

    pub fn assumed_at(&self) -> Option<&str> {
        self.0.doc.at("custody.assumed_at").and_then(|n| n.present_str())
    }
}

/// Construye un acuse de recibo conforme al Anexo B.3.
#[allow(clippy::too_many_arguments)]
pub fn build(
    shipment_id: &str,
    received: &str,
    issued: &str,
    manifest_digest: &str,
    recipient_org: &str,
    officer: &str,
    key_id: Option<&str>,
    checks: &VerificationSet,
    discrepancies: &[(String, String)],
    custody: Option<CustodyAcceptance>,
    retention_until: Option<&str>,
    destination_class: &str,
) -> Map {
    let result = checks.result();
    let mut d = Map::new();
    d.set(
        "stave",
        Node::map(vec![
            ("version", Node::str(crate::STAVE_VERSION)),
            ("written_by", Node::str(crate::written_by())),
        ]),
    );
    d.set(
        "receipt",
        Node::map(vec![
            ("shipment_id", Node::str(shipment_id)),
            ("received", Node::str(received)),
            ("issued", Node::str(issued)),
            ("manifest_digest", Node::str(manifest_digest)),
        ]),
    );
    d.set(
        "recipient",
        Node::map(vec![
            ("org", Node::str(recipient_org)),
            ("officer", Node::str(officer)),
            ("key_id", Node::opt_str(key_id)),
        ]),
    );
    d.set("verification", checks.to_node());
    d.set("result", Node::str(result.as_str()));
    if let Some(c) = custody {
        d.set(
            "custody",
            Node::map(vec![
                ("accepted", Node::Bool(c.accepted)),
                ("assumed_at", Node::opt_str(c.assumed_at.as_deref())),
                ("expected_return", Node::opt_str(c.expected_return.as_deref())),
            ]),
        );
    }
    d.set(
        "discrepancies",
        Node::Seq(
            discrepancies
                .iter()
                .map(|(check, detail)| {
                    Node::map(vec![
                        ("check", Node::str(check)),
                        ("detail", Node::str(detail)),
                    ])
                })
                .collect(),
        ),
    );
    d.set(
        "acceptance",
        Node::map(vec![
            (
                "usage_terms",
                Node::str(if checks.usage_accepted.failed() || result == ReceiptResult::Rejected {
                    "rejected"
                } else {
                    "accepted"
                }),
            ),
            ("retention_until", Node::opt_str(retention_until)),
        ]),
    );
    d.set("destination_class", Node::str(destination_class));
    d
}

/// Aceptación o rechazo expresos de la custodia.
#[derive(Clone, Debug)]
pub struct CustodyAcceptance {
    pub accepted: bool,
    pub assumed_at: Option<String>,
    pub expected_return: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todas(o: Outcome) -> VerificationSet {
        VerificationSet {
            provenance: o,
            authenticity: o,
            container: o,
            package_integrity: o,
            content_integrity: o,
            manifest_schema: o,
            profile_supported: o,
            custody: o,
            chronology: o,
            scope_match: o,
            usage_accepted: o,
            personal_data_match: o,
            structure: o,
            declared_checks: o,
        }
    }

    #[test]
    fn todas_las_verificaciones_conformes_producen_aceptacion() {
        assert_eq!(todas(Outcome::Pass).result(), ReceiptResult::Accepted);
    }

    #[test]
    fn el_fallo_de_una_verificacion_bloqueante_produce_rechazo() {
        for modificar in [
            |v: &mut VerificationSet| v.provenance = Outcome::Fail,
            |v: &mut VerificationSet| v.authenticity = Outcome::Fail,
            |v: &mut VerificationSet| v.container = Outcome::Fail,
            |v: &mut VerificationSet| v.package_integrity = Outcome::Fail,
            |v: &mut VerificationSet| v.content_integrity = Outcome::Fail,
            |v: &mut VerificationSet| v.manifest_schema = Outcome::Fail,
        ] {
            let mut v = todas(Outcome::Pass);
            modificar(&mut v);
            assert_eq!(v.result(), ReceiptResult::Rejected);
            assert!(v.blocking_failed());
        }
    }

    #[test]
    fn el_fallo_de_una_verificacion_no_bloqueante_produce_reservas() {
        let mut v = todas(Outcome::Pass);
        v.scope_match = Outcome::Fail;
        assert_eq!(v.result(), ReceiptResult::AcceptedWithReservations);
        assert!(!v.blocking_failed());
        assert!(v.result().allows_ingest());
    }

    #[test]
    fn el_acuse_indica_la_verificacion_que_fallo() {
        let mut v = todas(Outcome::Pass);
        v.content_integrity = Outcome::Fail;
        assert_eq!(v.first_failure(), Some("content_integrity"));
        assert_eq!(todas(Outcome::Pass).first_failure(), None);
    }

    #[test]
    fn el_acuse_reproduce_el_esquema_del_anexo_b_3() {
        let d = build(
            "20260806-AAAA",
            "2026-08-07T09:12:00-05:00",
            "2026-08-07T10:00:00-05:00",
            "abc123",
            "Estudio B",
            "M. Rivas",
            None,
            &todas(Outcome::Pass),
            &[],
            Some(CustodyAcceptance {
                accepted: true,
                assumed_at: Some("2026-08-07T10:00:00-05:00".into()),
                expected_return: Some("2026-08-20".into()),
            }),
            Some("2031-08-06"),
            "project",
        );
        for campo in ["stave", "receipt", "recipient", "verification", "result", "custody", "discrepancies", "acceptance", "destination_class"] {
            assert!(d.get(campo).is_some(), "falta {campo}");
        }
        assert_eq!(d.get("result").unwrap().as_str(), Some("accepted"));
        assert_eq!(d.at("custody.accepted").unwrap().as_bool(), Some(true));
        assert_eq!(d.at("acceptance.usage_terms").unwrap().as_str(), Some("accepted"));
        // Las catorce verificaciones constan por separado.
        assert_eq!(d.get("verification").unwrap().as_map().unwrap().len(), 14);
    }

    #[test]
    fn un_acuse_de_rechazo_no_acepta_las_condiciones_de_uso() {
        let mut v = todas(Outcome::Pass);
        v.package_integrity = Outcome::Fail;
        let d = build(
            "20260806-AAAA", "2026-08-07T09:12:00-05:00", "2026-08-07T10:00:00-05:00",
            "abc", "Estudio B", "M. Rivas", None, &v,
            &[("package_integrity".into(), "3 de 128 archivos no coinciden".into())],
            None, None, "01_REF",
        );
        assert_eq!(d.get("result").unwrap().as_str(), Some("rejected"));
        assert_eq!(d.at("acceptance.usage_terms").unwrap().as_str(), Some("rejected"));
        assert_eq!(d.get("discrepancies").unwrap().as_seq().unwrap().len(), 1);
    }

    #[test]
    fn el_acuse_se_relee_desde_el_disco() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("RECEIPT.yaml");
        let mut r = Receipt::new(&ruta);
        *r.doc_mut() = build(
            "20260806-AAAA", "2026-08-07T09:12:00-05:00", "2026-08-07T10:00:00-05:00",
            "resumen", "Estudio B", "M. Rivas", None, &todas(Outcome::Pass), &[],
            Some(CustodyAcceptance { accepted: true, assumed_at: Some("2026-08-07T10:00:00-05:00".into()), expected_return: None }),
            None, "project",
        );
        r.save().unwrap();

        let leido = Receipt::load(&ruta).unwrap();
        assert_eq!(leido.shipment_id(), Some("20260806-AAAA"));
        assert_eq!(leido.result(), Some(ReceiptResult::Accepted));
        assert_eq!(leido.manifest_digest(), Some("resumen"));
        assert_eq!(leido.custody_accepted(), Some(true));
    }
}
