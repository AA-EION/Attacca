//! Orden canónico de los campos, derivado del Anexo B.
//!
//! El Anexo B declara que el orden de los campos es indiferente para la
//! conformidad. Attacca fija uno de todos modos: sin un orden estable, dos
//! escrituras del mismo estado producen bytes distintos y las diferencias entre
//! versiones del manifiesto dejan de ser mínimas (apartado 16.2, NOTA 1).
//!
//! El orden reproduce el de los esquemas del Anexo B. Los campos que no
//! aparecen aquí —extensiones con prefijo propio y campos de versiones
//! posteriores de la norma— se conservan al final de su nivel, en su orden
//! relativo original (apartado 44.1).

use super::Map;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// Anexo B.1
    Project,
    /// Anexo B.2
    Exchange,
    /// Anexo B.3
    Receipt,
    /// Anexo B.5
    Volume,
    /// Anexo B.7
    Release,
}

/// Reordena un documento completo conforme al orden canónico de su artefacto.
pub fn canonical_order(doc: &mut Map, kind: ArtifactKind) {
    match kind {
        ArtifactKind::Project => order_project(doc),
        ArtifactKind::Exchange => order_exchange(doc),
        ArtifactKind::Receipt => order_receipt(doc),
        ArtifactKind::Volume => order_volume(doc),
        ArtifactKind::Release => order_release(doc),
    }
}

const STAVE_BLOCK: &[&str] = &["version", "profile", "level", "written_by"];
const CUSTODY_BLOCK: &[&str] = &["state", "holder", "since", "transfer", "history"];
const CUSTODY_TRANSFER: &[&str] = &[
    "shipment_id",
    "to",
    "expected_return",
    "grace_days",
    "onward_allowed",
];
const CUSTODY_ENTRY: &[&str] = &["seq", "action", "ts", "actor", "org", "shipment_id"];
const REPLICATION_BLOCK: &[&str] = &["active_volume", "replicas"];
const REPLICA_ENTRY: &[&str] = &["volume", "path", "state", "last_synced"];
const EXCEPTION_ENTRY: &[&str] = &["clause", "reason", "approved_by", "date"];
const DELIVERY_ENTRY: &[&str] = &["date", "target", "path", "shipment_id", "revision"];
const PARTY: &[&str] = &["org", "contact", "key_id"];

fn order_project(doc: &mut Map) {
    doc.reorder(&[
        "stave",
        "uid",
        "id",
        "id_history",
        "title",
        "artist",
        "type",
        "status",
        "release",
        "lineage",
        "derived",
        "replication",
        "audio",
        "vocabulary",
        "tools",
        "people",
        "sources",
        "deliveries",
        "master",
        "rights",
        "preservation",
        "custody",
        "exceptions",
    ]);
    sub(doc, "stave", STAVE_BLOCK);
    seq_entries(doc, "id_history", &["previous", "changed"]);
    sub(
        doc,
        "lineage",
        &["parent_uid", "parent_id", "derived_at", "reason", "parallel"],
    );
    seq_entries(doc, "derived", &["uid", "id", "derived_at"]);
    sub(doc, "replication", REPLICATION_BLOCK);
    if let Some(r) = doc.get_mut("replication").and_then(|n| n.as_map_mut()) {
        seq_entries(r, "replicas", REPLICA_ENTRY);
    }
    sub(
        doc,
        "audio",
        &[
            "sample_rate",
            "bit_depth",
            "tuning_hz",
            "tempo",
            "key",
            "origin",
        ],
    );
    sub(doc, "tools", &["primary", "sessions", "plugins"]);
    seq_entries(doc, "people", &["name", "role"]);
    seq_entries(
        doc,
        "sources",
        &[
            "path",
            "origin",
            "shipment_id",
            "classification",
            "usage",
            "retention_until",
            "clearance",
        ],
    );
    seq_entries(doc, "deliveries", DELIVERY_ENTRY);
    sub(doc, "master", &["lufs_i", "lra", "true_peak_db"]);
    sub(
        doc,
        "rights",
        &["isrc", "iswc", "split_sheet", "registrations"],
    );
    sub(doc, "preservation", &["manifest", "verified"]);
    order_custody(doc);
    seq_entries(doc, "exceptions", EXCEPTION_ENTRY);
}

fn order_release(doc: &mut Map) {
    doc.reorder(&[
        "stave",
        "uid",
        "id",
        "class",
        "title",
        "artist",
        "status",
        "release_date",
        "tracklist",
        "gaps",
        "identifiers",
        "common_requirements",
        "art",
        "deliveries",
        "custody",
        "replication",
        "exceptions",
    ]);
    sub(doc, "stave", STAVE_BLOCK);
    seq_entries(
        doc,
        "tracklist",
        &["position", "project_uid", "project_id", "title", "isrc"],
    );
    seq_entries(doc, "gaps", &["position", "reason"]);
    sub(doc, "identifiers", &["gtin", "catalog_number"]);
    sub(
        doc,
        "common_requirements",
        &["sample_rate", "bit_depth", "true_peak_ceiling_db"],
    );
    sub(doc, "art", &["path", "pixels"]);
    seq_entries(doc, "deliveries", DELIVERY_ENTRY);
    order_custody(doc);
    sub(doc, "replication", REPLICATION_BLOCK);
    if let Some(r) = doc.get_mut("replication").and_then(|n| n.as_map_mut()) {
        seq_entries(r, "replicas", REPLICA_ENTRY);
    }
    seq_entries(doc, "exceptions", EXCEPTION_ENTRY);
}

fn order_exchange(doc: &mut Map) {
    doc.reorder(&[
        "stave",
        "shipment",
        "parties",
        "purpose",
        "scope",
        "integrity",
        "classification",
        "usage",
        "retention",
        "personal_data",
        "rights",
        "watermark",
        "continuation",
        "custody",
        "serialization",
        "expected_checks",
        "acknowledgement",
        "exceptions",
    ]);
    sub(doc, "stave", STAVE_BLOCK);
    sub(doc, "shipment", &["id", "issued", "supersedes", "revision"]);
    sub(doc, "parties", &["issuer", "recipient"]);
    if let Some(p) = doc.get_mut("parties").and_then(|n| n.as_map_mut()) {
        sub(p, "issuer", PARTY);
        sub(p, "recipient", PARTY);
    }
    sub(
        doc,
        "scope",
        &["projects", "releases", "file_count", "total_bytes"],
    );
    sub(doc, "integrity", &["algorithm", "manifest", "signature"]);
    sub(
        doc,
        "usage",
        &["permitted", "territory", "term", "sublicensing", "forwarding"],
    );
    sub(
        doc,
        "retention",
        &["until", "action_on_expiry", "confirmation_required"],
    );
    sub(
        doc,
        "personal_data",
        &["present", "categories", "issuer_role", "recipient_role"],
    );
    sub(
        doc,
        "rights",
        &["isrc", "iswc", "clearances", "pending", "split_sheet"],
    );
    sub(doc, "watermark", &["present", "scope"]);
    sub(
        doc,
        "continuation",
        &[
            "sample_rate",
            "bit_depth",
            "tuning_hz",
            "tempo",
            "origin",
            "vocabulary_frozen",
        ],
    );
    sub(
        doc,
        "custody",
        &[
            "transfers",
            "returns",
            "supersedes_transfer",
            "expected_return",
            "grace_days",
            "onward_allowed",
            "history",
        ],
    );
    if let Some(c) = doc.get_mut("custody").and_then(|n| n.as_map_mut()) {
        seq_entries(c, "history", CUSTODY_ENTRY);
    }
    sub(doc, "serialization", &["container", "mimetype"]);
    sub(doc, "acknowledgement", &["deadline_hours", "address"]);
    seq_entries(doc, "exceptions", EXCEPTION_ENTRY);
}

fn order_receipt(doc: &mut Map) {
    doc.reorder(&[
        "stave",
        "receipt",
        "recipient",
        "verification",
        "result",
        "custody",
        "discrepancies",
        "acceptance",
        "destination_class",
    ]);
    sub(doc, "stave", STAVE_BLOCK);
    sub(
        doc,
        "receipt",
        &["shipment_id", "received", "issued", "manifest_digest"],
    );
    sub(doc, "recipient", &["org", "officer", "key_id"]);
    sub(
        doc,
        "verification",
        &[
            "provenance",
            "authenticity",
            "container",
            "package_integrity",
            "content_integrity",
            "manifest_schema",
            "profile_supported",
            "custody",
            "chronology",
            "scope_match",
            "usage_accepted",
            "personal_data_match",
            "structure",
            "declared_checks",
        ],
    );
    sub(doc, "custody", &["accepted", "assumed_at", "expected_return"]);
    seq_entries(doc, "discrepancies", &["check", "detail"]);
    sub(doc, "acceptance", &["usage_terms", "retention_until"]);
}

fn order_volume(doc: &mut Map) {
    doc.reorder(&["stave_volume"]);
    sub(
        doc,
        "stave_volume",
        &[
            "version",
            "uuid",
            "label",
            "role",
            "created",
            "root",
            "filesystem",
            "policy",
            "last_seen",
        ],
    );
    if let Some(v) = doc.get_mut("stave_volume").and_then(|n| n.as_map_mut()) {
        sub(
            v,
            "filesystem",
            &["name", "enforces_readonly", "preserves_case", "max_path"],
        );
        sub(
            v,
            "policy",
            &[
                "domains",
                "encrypted",
                "replicas_required",
                "verify_every_days",
            ],
        );
    }
}

fn order_custody(doc: &mut Map) {
    sub(doc, "custody", CUSTODY_BLOCK);
    let Some(c) = doc.get_mut("custody").and_then(|n| n.as_map_mut()) else {
        return;
    };
    sub(c, "holder", &["org", "person"]);
    sub(c, "transfer", CUSTODY_TRANSFER);
    if let Some(t) = c.get_mut("transfer").and_then(|n| n.as_map_mut()) {
        sub(t, "to", &["org", "person"]);
    }
    seq_entries(c, "history", CUSTODY_ENTRY);
}

fn sub(parent: &mut Map, key: &str, order: &[&str]) {
    if let Some(m) = parent.get_mut(key).and_then(|n| n.as_map_mut()) {
        m.reorder(order);
    }
}

fn seq_entries(parent: &mut Map, key: &str, order: &[&str]) {
    if let Some(super::Node::Seq(items)) = parent.get_mut(key) {
        for item in items.iter_mut() {
            if let Some(m) = item.as_map_mut() {
                m.reorder(order);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{emit, parse};
    use super::*;

    #[test]
    fn ordena_el_manifiesto_de_proyecto_y_conserva_las_extensiones() {
        let mut doc = parse(
            "x_otrofab_campo: valor\ncustody:\n  history: []\n  state: propia\nid: 2026-08-06_T_ORIG\nuid: ABC\nstave:\n  level: B\n  version: \"2.0\"\n",
        )
        .unwrap();
        canonical_order(&mut doc, ArtifactKind::Project);
        let keys: Vec<&str> = doc.keys().collect();
        assert_eq!(keys, vec!["stave", "uid", "id", "custody", "x_otrofab_campo"]);
        assert_eq!(
            doc.get("stave").unwrap().as_map().unwrap().keys().collect::<Vec<_>>(),
            vec!["version", "level"]
        );
        assert!(emit(&doc).contains("x_otrofab_campo: valor"));
    }

    #[test]
    fn el_orden_es_estable_frente_a_la_entrada() {
        let a = "uid: X\nid: Y\ntitle: T\n";
        let b = "title: T\nid: Y\nuid: X\n";
        let mut da = parse(a).unwrap();
        let mut db = parse(b).unwrap();
        canonical_order(&mut da, ArtifactKind::Project);
        canonical_order(&mut db, ArtifactKind::Project);
        assert_eq!(emit(&da), emit(&db));
    }
}
