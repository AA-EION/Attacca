//! Manifiesto de proyecto (Anexo B.1).

use super::Manifest;
use crate::clock;
use crate::doc::{ArtifactKind, Map, Node};
use crate::error::{Error, Result};
use std::path::Path;

/// Estado del proyecto (campo `status` del Anexo B.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Idea,
    Active,
    OnHold,
    Delivered,
    /// Apartado 14.4.3: proyecto de origen tras una derivación.
    Sealed,
    Archived,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Idea => "idea",
            Status::Active => "active",
            Status::OnHold => "onhold",
            Status::Delivered => "delivered",
            Status::Sealed => "sealed",
            Status::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        Some(match s {
            "idea" => Status::Idea,
            "active" => Status::Active,
            "onhold" => Status::OnHold,
            "delivered" => Status::Delivered,
            "sealed" => Status::Sealed,
            "archived" => Status::Archived,
            _ => return None,
        })
    }

    /// Carpeta de estado dentro de `20_PROJECTS` (apartado 6.2).
    ///
    /// Un proyecto sellado permanece en la carpeta que ocupaba: su condición
    /// consta únicamente en el manifiesto.
    pub fn folder(&self) -> Option<&'static str> {
        Some(match self {
            Status::Idea => "0_IDEAS",
            Status::Active => "1_ACTIVE",
            Status::OnHold => "2_ONHOLD",
            Status::Delivered => "3_DELIVERED",
            Status::Sealed | Status::Archived => return None,
        })
    }

    /// Un proyecto sellado o archivado no admite modificación (apartado 14.4.3).
    pub fn is_frozen(&self) -> bool {
        matches!(self, Status::Sealed | Status::Archived)
    }
}

/// Nivel de conformidad declarado (Tabla 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    A,
    B,
    C,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::A => "A",
            Level::B => "B",
            Level::C => "C",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        Some(match s {
            "A" => Level::A,
            "B" => Level::B,
            "C" => Level::C,
            _ => return None,
        })
    }
}

/// Vista tipada del manifiesto de proyecto.
#[derive(Debug)]
pub struct ProjectManifest(pub Manifest);

impl ProjectManifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(Manifest::load(path, ArtifactKind::Project)?))
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(Manifest::new(path.as_ref(), ArtifactKind::Project))
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

    pub fn path(&self) -> &Path {
        &self.0.path
    }

    /// Carpeta del proyecto: el directorio que contiene el manifiesto.
    pub fn root(&self) -> &Path {
        self.0.path.parent().unwrap_or(Path::new("."))
    }

    // --- Identidad ---

    pub fn uid(&self) -> Option<&str> {
        self.0.doc.get("uid").and_then(|n| n.present_str())
    }

    pub fn id(&self) -> Option<&str> {
        self.0.doc.get("id").and_then(|n| n.present_str())
    }

    pub fn title(&self) -> Option<&str> {
        self.0.doc.get("title").and_then(|n| n.present_str())
    }

    pub fn artist(&self) -> Option<&str> {
        self.0.doc.get("artist").and_then(|n| n.present_str())
    }

    pub fn project_type(&self) -> Option<&str> {
        self.0.doc.get("type").and_then(|n| n.present_str())
    }

    pub fn release_uid(&self) -> Option<&str> {
        self.0.doc.get("release").and_then(|n| n.present_str())
    }

    pub fn status(&self) -> Status {
        self.0
            .doc
            .get("status")
            .and_then(|n| n.as_str())
            .and_then(Status::parse)
            .unwrap_or(Status::Idea)
    }

    pub fn set_status(&mut self, status: Status) {
        self.0.doc.set("status", Node::str(status.as_str()));
    }

    pub fn level(&self) -> Level {
        self.0
            .doc
            .at("stave.level")
            .and_then(|n| n.as_str())
            .and_then(Level::parse)
            .unwrap_or(Level::B)
    }

    /// Eleva el nivel de conformidad. El apartado 5.1 admite elevarlo en
    /// cualquier momento y prohíbe reducirlo.
    pub fn raise_level(&mut self, level: Level) -> Result<()> {
        let actual = self.level();
        if level < actual {
            return Err(Error::requirement(
                "5.1",
                format!(
                    "El nivel declarado es {} y se solicitó {}. El nivel no se ha modificado. El nivel puede elevarse en cualquier momento; no puede reducirse.",
                    actual.as_str(),
                    level.as_str()
                ),
            ));
        }
        self.0.doc.ensure_map("stave").set("level", Node::str(level.as_str()));
        Ok(())
    }

    // --- Audio (apartado 10.1) ---

    pub fn sample_rate(&self) -> Option<i64> {
        self.0.doc.at("audio.sample_rate").and_then(|n| n.as_int())
    }

    pub fn bit_depth(&self) -> Option<i64> {
        self.0.doc.at("audio.bit_depth").and_then(|n| n.as_int())
    }

    /// La grabación cerrada fija tempo, tonalidad y afinación (apartado 13.2).
    pub fn audio_params_locked(&self) -> bool {
        matches!(
            crate::stage::active_stage_from_doc(&self.0.doc),
            Some(s) if s.index() > crate::stage::Stage::Recording.index()
        )
    }

    // --- Cambio de identificador legible (apartado 9.2) ---

    /// Registra un cambio de identificador legible con su valor anterior y su
    /// marca temporal. El identificador interno no se toca.
    pub fn record_id_change(&mut self, previous: &str) {
        let entrada = Node::map(vec![
            ("previous", Node::str(previous)),
            ("changed", Node::str(clock::now_rfc3339())),
        ]);
        self.0.doc.ensure_seq("id_history").push(entrada);
    }

    /// El identificador legible no debe modificarse una vez emitido un envío que
    /// incluya el proyecto, ni archivado el proyecto (apartado 9.2).
    pub fn id_is_frozen(&self) -> bool {
        let entregado = self
            .0
            .doc
            .get("deliveries")
            .and_then(|n| n.as_seq())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        entregado || matches!(self.status(), Status::Archived)
    }

    // --- Custodia (apartados 14.3 y 41) ---

    pub fn custody_state(&self) -> crate::custody::CustodyState {
        self.0
            .doc
            .at("custody.state")
            .and_then(|n| n.as_str())
            .and_then(crate::custody::CustodyState::parse)
            .unwrap_or(crate::custody::CustodyState::Own)
    }

    /// Número de secuencia siguiente de la cronología de custodia.
    ///
    /// Los números deben ser consecutivos (apartado 14.3.4).
    pub fn next_custody_seq(&self) -> i64 {
        self.0
            .doc
            .at("custody.history")
            .and_then(|n| n.as_seq())
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|e| e.as_map()?.get("seq")?.as_int())
                    .max()
                    .unwrap_or(0)
                    + 1
            })
            .unwrap_or(1)
    }

    /// Asientos de la cronología, en el orden que determina el número de
    /// secuencia (apartado 14.3.4).
    pub fn custody_history(&self) -> Vec<crate::custody::ChronologyEntry> {
        let mut entries: Vec<_> = self
            .0
            .doc
            .at("custody.history")
            .and_then(|n| n.as_seq())
            .map(|s| s.iter().filter_map(crate::custody::ChronologyEntry::from_node).collect())
            .unwrap_or_default();
        entries.sort_by_key(|e| e.seq);
        entries
    }

    // --- Réplicas (apartado 6.6) ---

    pub fn active_volume(&self) -> Option<&str> {
        self.0
            .doc
            .at("replication.active_volume")
            .and_then(|n| n.present_str())
    }

    // --- Entregas (apartado 36, paso 13) ---

    pub fn record_delivery(
        &mut self,
        date: &str,
        target: &str,
        path: &str,
        shipment_id: &str,
        revision: &str,
    ) {
        let entrada = Node::map(vec![
            ("date", Node::str(date)),
            ("target", Node::str(target)),
            ("path", Node::str(path)),
            ("shipment_id", Node::str(shipment_id)),
            ("revision", Node::str(revision)),
        ]);
        self.0.doc.ensure_seq("deliveries").push(entrada);
    }

    /// Registra una fuente externa incorporada (grupo Fuentes de la Tabla 13).
    #[allow(clippy::too_many_arguments)]
    pub fn record_source(
        &mut self,
        path: &str,
        origin: &str,
        shipment_id: Option<&str>,
        classification: &str,
        usage: Option<&str>,
        retention_until: Option<&str>,
        clearance: &str,
    ) {
        let entrada = Node::map(vec![
            ("path", Node::str(path)),
            ("origin", Node::str(origin)),
            ("shipment_id", Node::opt_str(shipment_id)),
            ("classification", Node::str(classification)),
            ("usage", Node::opt_str(usage)),
            ("retention_until", Node::opt_str(retention_until)),
            ("clearance", Node::str(clearance)),
        ]);
        self.0.doc.ensure_seq("sources").push(entrada);
    }

    /// Registra una excepción justificada (apartado 13.1).
    pub fn record_exception(&mut self, clause: &str, reason: &str, approved_by: &str) {
        let entrada = Node::map(vec![
            ("clause", Node::str(clause)),
            ("reason", Node::str(reason)),
            ("approved_by", Node::str(approved_by)),
            ("date", Node::str(clock::today())),
        ]);
        self.0.doc.ensure_seq("exceptions").push(entrada);
    }

    /// Cláusulas con excepción declarada.
    pub fn declared_exceptions(&self) -> Vec<String> {
        self.0
            .doc
            .get("exceptions")
            .and_then(|n| n.as_seq())
            .map(|s| {
                s.iter()
                    .filter_map(|e| Some(e.as_map()?.get("clause")?.as_str()?.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Construye el manifiesto de un proyecto recién creado (apartado 13.2).
///
/// Solo se exigen los campos que la persona conoce en ese instante. Todos los
/// demás se escriben con nulo explícito, no se omiten: la validación debe poder
/// distinguir un campo pendiente de un campo incumplido.
#[allow(clippy::too_many_arguments)]
pub fn scaffold(
    uid: &str,
    id: &str,
    title: &str,
    artist: &str,
    kind: &str,
    level: Level,
    release_uid: Option<&str>,
    sample_rate: i64,
    bit_depth: i64,
    holder_org: &str,
    holder_person: &str,
    active_volume: Option<&str>,
) -> Map {
    let mut doc = Map::new();

    doc.set(
        "stave",
        Node::map(vec![
            ("version", Node::str(crate::STAVE_VERSION)),
            ("level", Node::str(level.as_str())),
            ("written_by", Node::str(crate::written_by())),
        ]),
    );
    doc.set("uid", Node::str(uid));
    doc.set("id", Node::str(id));
    doc.set("id_history", Node::Seq(Vec::new()));
    doc.set("title", Node::str(title));
    doc.set("artist", Node::str(artist));
    doc.set("type", Node::str(kind));
    doc.set("status", Node::str(Status::Idea.as_str()));
    doc.set("release", Node::opt_str(release_uid));

    doc.set(
        "replication",
        Node::map(vec![
            ("active_volume", Node::opt_str(active_volume)),
            ("replicas", Node::Seq(Vec::new())),
        ]),
    );

    // Frecuencia de muestreo y profundidad de bits son exigibles en la creación
    // (Tabla 11B). El resto del grupo Audio es exigible al concluir la
    // composición y se registra en nulo.
    doc.set(
        "audio",
        Node::map(vec![
            ("sample_rate", Node::Int(sample_rate)),
            ("bit_depth", Node::Int(bit_depth)),
            ("tuning_hz", Node::Int(440)),
            ("tempo", Node::Null),
            ("key", Node::Null),
            ("origin", Node::Null),
        ]),
    );

    doc.set(
        "tools",
        Node::map(vec![
            ("primary", Node::Null),
            ("sessions", Node::Seq(Vec::new())),
        ]),
    );
    doc.set("people", Node::Seq(Vec::new()));
    doc.set("sources", Node::Seq(Vec::new()));
    doc.set("deliveries", Node::Seq(Vec::new()));
    doc.set(
        "master",
        Node::map(vec![
            ("lufs_i", Node::Null),
            ("lra", Node::Null),
            ("true_peak_db", Node::Null),
        ]),
    );
    doc.set(
        "rights",
        Node::map(vec![
            ("isrc", Node::Null),
            ("iswc", Node::Null),
            ("split_sheet", Node::Null),
            ("registrations", Node::Seq(Vec::new())),
        ]),
    );
    doc.set(
        "preservation",
        Node::map(vec![
            ("manifest", Node::Null),
            ("verified", Node::Null),
        ]),
    );

    // El grupo Custodia es exigible en la creación (Tabla 11B).
    doc.set(
        "custody",
        Node::map(vec![
            ("state", Node::str("propia")),
            (
                "holder",
                Node::map(vec![
                    ("org", Node::str(holder_org)),
                    ("person", Node::str(holder_person)),
                ]),
            ),
            ("since", Node::str(clock::now_rfc3339())),
            ("history", Node::Seq(Vec::new())),
        ]),
    );
    doc.set("exceptions", Node::Seq(Vec::new()));
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifiesto_base() -> Map {
        scaffold(
            "01J9ZQ8F3K7N2VYB4T6XM0RSAE",
            "2026-08-06_Tema_ORIG",
            "Tema",
            "Artista",
            "ORIG",
            Level::B,
            None,
            48000,
            24,
            "Estudio A",
            "J. Duarte",
            Some("vol-1"),
        )
    }

    #[test]
    fn la_creacion_escribe_los_pendientes_en_nulo_explicito() {
        let doc = manifiesto_base();
        // Exigibles en la creación.
        assert_eq!(doc.get("uid").unwrap().as_str(), Some("01J9ZQ8F3K7N2VYB4T6XM0RSAE"));
        assert_eq!(doc.at("audio.sample_rate").unwrap().as_int(), Some(48000));
        assert_eq!(doc.at("custody.state").unwrap().as_str(), Some("propia"));
        // Pendientes: presentes y nulos, nunca ausentes (apartado 13.2).
        for ruta in ["audio.tempo", "audio.key", "audio.origin", "master.lufs_i", "rights.isrc", "preservation.manifest"] {
            let n = doc.at(ruta).unwrap_or_else(|| panic!("{ruta} está ausente"));
            assert!(n.is_null(), "{ruta} debería ser nulo explícito");
        }
    }

    #[test]
    fn el_nivel_se_eleva_pero_no_se_reduce() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        m.0.doc = manifiesto_base();
        assert_eq!(m.level(), Level::B);
        m.raise_level(Level::C).unwrap();
        assert_eq!(m.level(), Level::C);
        let e = m.raise_level(Level::A).unwrap_err();
        assert_eq!(e.clause(), Some("5.1"));
        assert_eq!(m.level(), Level::C);
    }

    #[test]
    fn la_secuencia_de_custodia_es_consecutiva() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        m.0.doc = manifiesto_base();
        assert_eq!(m.next_custody_seq(), 1);
        let hist = m.0.doc.ensure_map("custody").ensure_seq("history");
        hist.push(Node::map(vec![("seq", Node::Int(1))]));
        hist.push(Node::map(vec![("seq", Node::Int(2))]));
        assert_eq!(m.next_custody_seq(), 3);
    }

    #[test]
    fn el_identificador_se_congela_tras_el_primer_envio() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        m.0.doc = manifiesto_base();
        assert!(!m.id_is_frozen());
        m.record_delivery("2026-08-06", "Sello", "08_DELIVERY/x", "20260806-AAA", "r0");
        assert!(m.id_is_frozen());
    }

    #[test]
    fn el_cambio_de_identificador_deja_el_valor_anterior_con_marca_temporal() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = ProjectManifest::new(dir.path().join("PROJECT.yaml"));
        m.0.doc = manifiesto_base();
        m.record_id_change("2026-08-06_Tema-Viejo_ORIG");
        let hist = m.doc().get("id_history").unwrap().as_seq().unwrap();
        assert_eq!(hist.len(), 1);
        let e = hist[0].as_map().unwrap();
        assert_eq!(e.get("previous").unwrap().as_str(), Some("2026-08-06_Tema-Viejo_ORIG"));
        assert!(crate::clock::parse_rfc3339(e.get("changed").unwrap().as_str().unwrap()).is_ok());
    }

    #[test]
    fn el_proyecto_sellado_permanece_en_su_carpeta_de_estado() {
        assert_eq!(Status::Active.folder(), Some("1_ACTIVE"));
        assert_eq!(Status::Sealed.folder(), None, "apartado 6.2");
        assert!(Status::Sealed.is_frozen());
        assert!(!Status::Delivered.is_frozen());
    }
}
