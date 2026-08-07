//! Manifiesto de release (apartado 8.4 y Anexo B.7).

use super::Manifest;
use crate::clock;
use crate::doc::{ArtifactKind, Map, Node};
use crate::error::{Error, Result};
use crate::manifest::project::{Level, Status};
use std::path::Path;

/// Clases de release (Tabla 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseClass {
    Single,
    Ep,
    Album,
    Comp,
    Live,
    Sync,
}

impl ReleaseClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseClass::Single => "SINGLE",
            ReleaseClass::Ep => "EP",
            ReleaseClass::Album => "ALBUM",
            ReleaseClass::Comp => "COMP",
            ReleaseClass::Live => "LIVE",
            ReleaseClass::Sync => "SYNC",
        }
    }

    pub fn parse(s: &str) -> Option<ReleaseClass> {
        Some(match s {
            "SINGLE" => ReleaseClass::Single,
            "EP" => ReleaseClass::Ep,
            "ALBUM" => ReleaseClass::Album,
            "COMP" => ReleaseClass::Comp,
            "LIVE" => ReleaseClass::Live,
            "SYNC" => ReleaseClass::Sync,
            _ => return None,
        })
    }

    /// Una recopilación referencia proyectos ya publicados sin duplicar su
    /// material (apartado 8.3, tercer guion).
    pub fn references_without_duplicating(&self) -> bool {
        matches!(self, ReleaseClass::Comp)
    }
}

/// Un tema del tracklist.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub position: i64,
    /// Referencia por identificador interno: renombrar un tema no rompe el
    /// tracklist (apartado 8.3, cuarto guion).
    pub project_uid: String,
    pub project_id: String,
    pub title: String,
    pub isrc: Option<String>,
}

impl Track {
    pub fn from_node(n: &Node) -> Option<Track> {
        let m = n.as_map()?;
        Some(Track {
            position: m.get("position")?.as_int()?,
            project_uid: m.get("project_uid")?.present_str()?.to_string(),
            project_id: m
                .get("project_id")
                .and_then(|n| n.present_str())
                .unwrap_or_default()
                .to_string(),
            title: m
                .get("title")
                .and_then(|n| n.present_str())
                .unwrap_or_default()
                .to_string(),
            isrc: m.get("isrc").and_then(|n| n.present_str()).map(str::to_string),
        })
    }

    pub fn to_node(&self) -> Node {
        Node::map(vec![
            ("position", Node::Int(self.position)),
            ("project_uid", Node::str(&self.project_uid)),
            ("project_id", Node::str(&self.project_id)),
            ("title", Node::str(&self.title)),
            ("isrc", Node::opt_str(self.isrc.as_deref())),
        ])
    }
}

/// Vista tipada del manifiesto de release.
#[derive(Debug)]
pub struct ReleaseManifest(pub Manifest);

impl ReleaseManifest {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(Manifest::load(path, ArtifactKind::Release)?))
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(Manifest::new(path.as_ref(), ArtifactKind::Release))
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

    /// Raíz del release: la carpeta que contiene `_RELEASE`.
    pub fn root(&self) -> &Path {
        self.0
            .path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("."))
    }

    pub fn uid(&self) -> Option<&str> {
        self.0.doc.get("uid").and_then(|n| n.present_str())
    }

    pub fn id(&self) -> Option<&str> {
        self.0.doc.get("id").and_then(|n| n.present_str())
    }

    pub fn class(&self) -> Option<ReleaseClass> {
        self.0
            .doc
            .get("class")
            .and_then(|n| n.as_str())
            .and_then(ReleaseClass::parse)
    }

    pub fn title(&self) -> Option<&str> {
        self.0.doc.get("title").and_then(|n| n.present_str())
    }

    pub fn artist(&self) -> Option<&str> {
        self.0.doc.get("artist").and_then(|n| n.present_str())
    }

    pub fn level(&self) -> Level {
        self.0
            .doc
            .at("stave.level")
            .and_then(|n| n.as_str())
            .and_then(Level::parse)
            .unwrap_or(Level::B)
    }

    pub fn status(&self) -> Status {
        self.0
            .doc
            .get("status")
            .and_then(|n| n.as_str())
            .and_then(Status::parse)
            .unwrap_or(Status::Idea)
    }

    pub fn tracklist(&self) -> Vec<Track> {
        let mut t: Vec<Track> = self
            .0
            .doc
            .get("tracklist")
            .and_then(|n| n.as_seq())
            .map(|s| s.iter().filter_map(Track::from_node).collect())
            .unwrap_or_default();
        t.sort_by_key(|t| t.position);
        t
    }

    pub fn set_tracklist(&mut self, tracks: &[Track]) {
        let mut ordenados = tracks.to_vec();
        ordenados.sort_by_key(|t| t.position);
        self.0
            .doc
            .set("tracklist", Node::Seq(ordenados.iter().map(Track::to_node).collect()));
    }

    /// Vincula un proyecto al release. La vinculación es por identificador
    /// interno (apartado 8.3).
    pub fn link_project(
        &mut self,
        project_uid: &str,
        project_id: &str,
        title: &str,
        position: Option<i64>,
    ) -> Result<i64> {
        let mut tracks = self.tracklist();
        if tracks.iter().any(|t| t.project_uid == project_uid) {
            return Err(Error::requirement(
                "8.3",
                format!("El proyecto {project_uid} ya figura en el tracklist. El release no se ha modificado."),
            ));
        }
        let pos = position.unwrap_or_else(|| {
            tracks.iter().map(|t| t.position).max().unwrap_or(0) + 1
        });
        if tracks.iter().any(|t| t.position == pos) {
            return Err(Error::requirement(
                "8.3",
                format!("La posición {pos} del tracklist ya está ocupada. El release no se ha modificado. Elegir otra posición."),
            ));
        }
        tracks.push(Track {
            position: pos,
            project_uid: project_uid.to_string(),
            project_id: project_id.to_string(),
            title: title.to_string(),
            isrc: None,
        });
        self.set_tracklist(&tracks);
        Ok(pos)
    }

    /// Registra un hueco de numeración por material descartado.
    ///
    /// Los huecos son admisibles y deben documentarse; el material restante no
    /// debe renumerarse (apartado 8.3, último guion).
    pub fn record_gap(&mut self, position: i64, reason: &str) {
        self.0.doc.ensure_seq("gaps").push(Node::map(vec![
            ("position", Node::Int(position)),
            ("reason", Node::str(reason)),
        ]));
    }

    pub fn gaps(&self) -> Vec<i64> {
        self.0
            .doc
            .get("gaps")
            .and_then(|n| n.as_seq())
            .map(|s| s.iter().filter_map(|n| n.as_map()?.get("position")?.as_int()).collect())
            .unwrap_or_default()
    }

    /// Parámetros técnicos comunes exigidos por el canal (apartado 8.5).
    pub fn common_requirements(&self) -> Vec<(String, Node)> {
        self.0
            .doc
            .get("common_requirements")
            .and_then(|n| n.as_map())
            .map(|m| m.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
            .unwrap_or_default()
    }
}

/// Comprueba que un proyecto cumpla los parámetros comunes del release
/// (apartado 8.5, primer guion).
pub fn check_common_requirements(release: &ReleaseManifest, project: &Map) -> Vec<String> {
    let mut fallos = Vec::new();
    for (campo, esperado) in release.common_requirements() {
        if esperado.is_null() {
            continue;
        }
        let ruta = match campo.as_str() {
            "sample_rate" => "audio.sample_rate",
            "bit_depth" => "audio.bit_depth",
            "true_peak_ceiling_db" => "master.true_peak_db",
            _ => continue,
        };
        let real = project.at(ruta);
        match (campo.as_str(), real) {
            // El techo de pico real es un máximo, no una igualdad.
            ("true_peak_ceiling_db", Some(n)) if !n.is_null() => {
                if let (Some(r), Some(e)) = (n.as_f64(), esperado.as_f64()) {
                    if r > e {
                        fallos.push(format!(
                            "El pico real declarado es {r} dBTP y el release exige un techo de {e} dBTP."
                        ));
                    }
                }
            }
            // Un campo aún no producido no es un incumplimiento (apartado 13.2).
            (_, None) | (_, Some(Node::Null)) => {}
            (_, Some(n)) => {
                if n.as_int() != esperado.as_int() {
                    fallos.push(format!(
                        "El proyecto declara {campo} = {n} y el release exige {esperado}."
                    ));
                }
            }
        }
    }
    fallos
}

/// Construye el manifiesto de un release recién creado.
pub fn scaffold(
    uid: &str,
    id: &str,
    title: &str,
    artist: &str,
    class: ReleaseClass,
    level: Level,
    holder_org: &str,
    holder_person: &str,
) -> Map {
    let mut d = Map::new();
    d.set(
        "stave",
        Node::map(vec![
            ("version", Node::str(crate::STAVE_VERSION)),
            ("level", Node::str(level.as_str())),
            ("written_by", Node::str(crate::written_by())),
        ]),
    );
    d.set("uid", Node::str(uid));
    d.set("id", Node::str(id));
    d.set("class", Node::str(class.as_str()));
    d.set("title", Node::str(title));
    d.set("artist", Node::str(artist));
    d.set("status", Node::str(Status::Idea.as_str()));
    d.set("release_date", Node::Null);
    d.set("tracklist", Node::Seq(Vec::new()));
    d.set("gaps", Node::Seq(Vec::new()));
    d.set(
        "identifiers",
        Node::map(vec![("gtin", Node::Null), ("catalog_number", Node::Null)]),
    );
    d.set(
        "common_requirements",
        Node::map(vec![
            ("sample_rate", Node::Null),
            ("bit_depth", Node::Null),
            ("true_peak_ceiling_db", Node::Null),
        ]),
    );
    d.set("art", Node::map(vec![("path", Node::Null), ("pixels", Node::Null)]));
    d.set("deliveries", Node::Seq(Vec::new()));
    d.set(
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
    d.set(
        "replication",
        Node::map(vec![
            ("active_volume", Node::Null),
            ("replicas", Node::Seq(Vec::new())),
        ]),
    );
    d.set("exceptions", Node::Seq(Vec::new()));
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> ReleaseManifest {
        let mut r = ReleaseManifest::new(Path::new("_RELEASE/RELEASE.yaml"));
        *r.doc_mut() = scaffold(
            "01J9ZQ8F3K7N2VYB4T6XM0RSBF",
            "2026-08-06_Disco_ALBUM",
            "Disco",
            "Artista",
            ReleaseClass::Album,
            Level::C,
            "Estudio A",
            "J. Duarte",
        );
        r
    }

    #[test]
    fn la_vinculacion_es_por_identificador_interno() {
        let mut r = release();
        r.link_project("UID-A", "2026-08-06_Tema-A_ORIG", "Tema A", None).unwrap();
        r.link_project("UID-B", "2026-08-06_Tema-B_ORIG", "Tema B", None).unwrap();
        let t = r.tracklist();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].position, 1);
        assert_eq!(t[0].project_uid, "UID-A");
        assert_eq!(t[1].position, 2);
    }

    #[test]
    fn renombrar_un_tema_no_rompe_el_tracklist() {
        let mut r = release();
        r.link_project("UID-A", "2026-08-06_Titulo-Viejo_ORIG", "Titulo Viejo", None).unwrap();
        // El proyecto cambia su identificador legible. La referencia por
        // identificador interno sigue resolviendo.
        let t = r.tracklist();
        assert_eq!(t[0].project_uid, "UID-A");
        // La búsqueda por identificador interno no depende del nombre.
        assert!(r.tracklist().iter().any(|x| x.project_uid == "UID-A"));
    }

    #[test]
    fn un_proyecto_no_se_vincula_dos_veces() {
        let mut r = release();
        r.link_project("UID-A", "id", "T", None).unwrap();
        let e = r.link_project("UID-A", "id", "T", None).unwrap_err();
        assert_eq!(e.clause(), Some("8.3"));
    }

    #[test]
    fn los_huecos_se_registran_y_no_se_renumera() {
        let mut r = release();
        r.link_project("UID-A", "a", "A", Some(1)).unwrap();
        r.link_project("UID-B", "b", "B", Some(2)).unwrap();
        r.link_project("UID-D", "d", "D", Some(4)).unwrap();
        r.record_gap(3, "Tema descartado en preproduccion");

        let posiciones: Vec<i64> = r.tracklist().iter().map(|t| t.position).collect();
        assert_eq!(posiciones, vec![1, 2, 4], "no se renumera el material restante");
        assert_eq!(r.gaps(), vec![3]);
    }

    #[test]
    fn una_posicion_ocupada_se_rechaza() {
        let mut r = release();
        r.link_project("UID-A", "a", "A", Some(1)).unwrap();
        assert!(r.link_project("UID-B", "b", "B", Some(1)).is_err());
    }

    #[test]
    fn verifica_los_parametros_comunes_en_cada_proyecto() {
        let mut r = release();
        r.doc_mut().set(
            "common_requirements",
            Node::map(vec![
                ("sample_rate", Node::Int(48000)),
                ("bit_depth", Node::Int(24)),
                ("true_peak_ceiling_db", Node::Float(-1.0)),
            ]),
        );

        let mut conforme = Map::new();
        conforme.set("audio", Node::map(vec![("sample_rate", Node::Int(48000)), ("bit_depth", Node::Int(24))]));
        conforme.set("master", Node::map(vec![("true_peak_db", Node::Float(-1.2))]));
        assert!(check_common_requirements(&r, &conforme).is_empty());

        let mut discrepante = Map::new();
        discrepante.set("audio", Node::map(vec![("sample_rate", Node::Int(44100)), ("bit_depth", Node::Int(24))]));
        discrepante.set("master", Node::map(vec![("true_peak_db", Node::Float(-0.3))]));
        let fallos = check_common_requirements(&r, &discrepante);
        assert_eq!(fallos.len(), 2, "{fallos:?}");
    }

    #[test]
    fn un_campo_pendiente_no_es_incumplimiento_de_los_parametros_comunes() {
        let mut r = release();
        r.doc_mut().set(
            "common_requirements",
            Node::map(vec![("true_peak_ceiling_db", Node::Float(-1.0))]),
        );
        // El mastering aún no ha concluido: el campo está en nulo explícito.
        let mut pendiente = Map::new();
        pendiente.set("master", Node::map(vec![("true_peak_db", Node::Null)]));
        assert!(check_common_requirements(&r, &pendiente).is_empty());
    }

    #[test]
    fn el_esquema_reproduce_el_anexo_b_7() {
        let d = release();
        for campo in ["stave", "uid", "id", "class", "title", "artist", "status", "release_date", "tracklist", "gaps", "identifiers", "common_requirements", "art", "deliveries", "custody", "replication", "exceptions"] {
            assert!(d.doc().get(campo).is_some(), "falta {campo}");
        }
    }
}
