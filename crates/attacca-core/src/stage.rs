//! Etapa activa y presentación guiada (Anexo I y apartado 44.2).
//!
//! La etapa activa se deduce del manifiesto y del contenido real del proyecto,
//! nunca de una preferencia almacenada en la interfaz. Es lo que exige el
//! apartado 44.2, primer guion, y lo que hace que la vista sea correcta aunque
//! el trabajo se haya hecho desde fuera de Attacca.

use crate::custody::CustodyState;
use crate::doc::Map;
use crate::manifest::project::Status;
use std::path::Path;

/// Etapas del ciclo de vida (Tabla 14 y Tabla I.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Composition,
    PreProduction,
    Recording,
    Editing,
    Mixing,
    Mastering,
    QualityControl,
    Distribution,
    ProductionExchange,
    Reception,
    Archival,
}

impl Stage {
    /// Orden de la etapa dentro del flujo. Determina qué etapas ya concluyeron.
    pub fn index(&self) -> u8 {
        match self {
            Stage::Composition => 0,
            Stage::PreProduction => 1,
            Stage::Recording => 2,
            Stage::Editing => 3,
            Stage::Mixing => 4,
            Stage::Mastering => 5,
            Stage::QualityControl => 6,
            Stage::Distribution => 7,
            Stage::ProductionExchange => 8,
            Stage::Reception => 9,
            Stage::Archival => 10,
        }
    }

    /// Clave estable de la etapa. La interfaz la emplea para localizar su
    /// cadena en el archivo de recursos; no se muestra.
    pub fn key(&self) -> &'static str {
        match self {
            Stage::Composition => "composicion",
            Stage::PreProduction => "preproduccion",
            Stage::Recording => "grabacion",
            Stage::Editing => "edicion",
            Stage::Mixing => "mezcla",
            Stage::Mastering => "mastering",
            Stage::QualityControl => "control_calidad",
            Stage::Distribution => "distribucion",
            Stage::ProductionExchange => "intercambio_produccion",
            Stage::Reception => "recepcion",
            Stage::Archival => "archivo",
        }
    }

    pub fn parse(key: &str) -> Option<Stage> {
        Some(match key {
            "composicion" => Stage::Composition,
            "preproduccion" => Stage::PreProduction,
            "grabacion" => Stage::Recording,
            "edicion" => Stage::Editing,
            "mezcla" => Stage::Mixing,
            "mastering" => Stage::Mastering,
            "control_calidad" => Stage::QualityControl,
            "distribucion" => Stage::Distribution,
            "intercambio_produccion" => Stage::ProductionExchange,
            "recepcion" => Stage::Reception,
            "archivo" => Stage::Archival,
            _ => return None,
        })
    }

    /// Carpetas que la Tabla I.1 indica presentar en primer plano.
    ///
    /// Las carpetas no enumeradas siguen siendo accesibles: la tabla indica cuál
    /// conviene presentar, no cuál es la única alcanzable.
    pub fn folders(&self) -> &'static [&'static str] {
        match self {
            Stage::Composition => &["00_ADMIN/Notes", "01_REF"],
            Stage::PreProduction => &["00_ADMIN"],
            Stage::Recording => &["02_SESSIONS", "03_RECORDINGS"],
            Stage::Editing => &["04_EDIT"],
            Stage::Mixing => &["02_SESSIONS", "05_STEMS", "06_MIX"],
            Stage::Mastering => &["07_MASTER"],
            Stage::QualityControl => &["00_ADMIN/Notes", "07_MASTER"],
            Stage::Distribution => &["08_DELIVERY"],
            Stage::ProductionExchange => &["09_TRANSFER"],
            Stage::Reception => &["40_INBOX"],
            Stage::Archival => &[""],
        }
    }

    /// Acciones admisibles en la etapa (columna 3 de la Tabla I.1).
    ///
    /// Las claves corresponden a las acciones del menú de proyecto y a sus
    /// cadenas en el archivo de recursos de la interfaz.
    pub fn actions(&self) -> &'static [&'static str] {
        match self {
            Stage::Composition => &[
                "registrar_notas",
                "incorporar_referencia",
                "declarar_tempo_tonalidad",
                "crear_sesion",
            ],
            Stage::PreProduction => &[
                "registrar_plan_grabacion",
                "registrar_mapa_tempo",
                "preparar_acuerdo_reparto",
            ],
            Stage::Recording => &[
                "crear_sesion",
                "incorporar_tomas",
                "fijar_vocabulario",
                "congelar_vocabulario",
            ],
            Stage::Editing => &["incorporar_audio_editado", "crear_sesion"],
            Stage::Mixing => &[
                "crear_sesion",
                "exportar_bounce",
                "exportar_stems",
                "generar_hoja_recall",
            ],
            Stage::Mastering => &["incorporar_master", "registrar_mediciones"],
            Stage::QualityControl => &["ejecutar_control_calidad", "firmar_informe"],
            Stage::Distribution => &[
                "constituir_paquete_entrega",
                "asignar_identificadores",
                "emitir_envio",
            ],
            Stage::ProductionExchange => &["ceder_custodia", "constituir_paquete_produccion"],
            Stage::Reception => &["verificar_paquete", "acusar_recibo", "ingerir_paquete"],
            Stage::Archival => &[
                "consolidar_sesion",
                "exportar_stems",
                "generar_manifiesto_integridad",
                "archivar",
            ],
        }
    }

    /// Condición de paso a la etapa siguiente (columna 4 de la Tabla I.1).
    pub fn advance_condition_key(&self) -> &'static str {
        match self {
            Stage::Composition => "maqueta_disponible",
            Stage::PreProduction => "plan_grabacion_cerrado",
            Stage::Recording => "vocabulario_congelado",
            Stage::Editing => "edicion_cerrada",
            Stage::Mixing => "bounce_aprobado",
            Stage::Mastering => "mediciones_registradas",
            Stage::QualityControl => "informe_aprobado",
            Stage::Distribution => "envio_emitido_y_acusado",
            Stage::ProductionExchange => "custodia_cedida_o_retornada",
            Stage::Reception => "cuarentena_vacia",
            Stage::Archival => "proyecto_en_archivo",
        }
    }
}

/// Evidencia observada en el árbol del proyecto. La etapa activa se deduce de
/// ella junto con el manifiesto.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Evidence {
    pub has_sessions: bool,
    pub has_recordings: bool,
    pub has_edit: bool,
    pub has_stems: bool,
    pub has_mix: bool,
    pub has_master: bool,
    pub has_qc_report: bool,
    pub has_delivery: bool,
    pub has_transfer: bool,
    pub inbox_pending: bool,
}

/// Observa el contenido real del proyecto.
pub fn observe(project_root: &Path) -> Evidence {
    let hay = |sub: &str| dir_has_files(&project_root.join(sub));
    Evidence {
        has_sessions: hay("02_SESSIONS"),
        has_recordings: hay("03_RECORDINGS"),
        has_edit: hay("04_EDIT"),
        has_stems: hay("05_STEMS"),
        has_mix: hay("06_MIX"),
        has_master: hay("07_MASTER"),
        has_qc_report: has_qc_report(&project_root.join("00_ADMIN/Notes")),
        has_delivery: hay("08_DELIVERY"),
        has_transfer: hay("09_TRANSFER"),
        inbox_pending: false,
    }
}

fn dir_has_files(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(ft) = entry.file_type() else { continue };
        if crate::fsx::walk::is_regenerable(&name, ft.is_dir()) {
            continue;
        }
        if ft.is_file() {
            return true;
        }
        if ft.is_dir() && dir_has_files(&entry.path()) {
            return true;
        }
    }
    false
}

/// El informe de control de calidad se archiva en `00_ADMIN/Notes`
/// (apartado 15).
fn has_qc_report(notes: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(notes) else {
        return false;
    };
    entries.flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_ascii_uppercase();
        n.contains("QC") || n.contains("CONTROL") || n.contains("CALIDAD")
    })
}

/// Deduce la etapa activa a partir del manifiesto y de la evidencia observada.
///
/// La etapa activa es la más avanzada cuyo trabajo esté en curso: se recorre el
/// flujo en sentido inverso y se devuelve la primera cuya salida obligatoria aún
/// no consta como concluida.
pub fn active_stage(doc: &Map, evidence: &Evidence) -> Stage {
    // La custodia cedida o en tránsito sitúa el proyecto en la etapa de
    // intercambio: es lo único que puede hacerse con él (Tabla 15).
    let custody = doc
        .at("custody.state")
        .and_then(|n| n.as_str())
        .and_then(CustodyState::parse)
        .unwrap_or(CustodyState::Own);
    if matches!(custody, CustodyState::InTransit | CustodyState::Ceded) {
        return Stage::ProductionExchange;
    }

    let status = doc
        .at("status")
        .and_then(|n| n.as_str())
        .and_then(Status::parse)
        .unwrap_or(Status::Idea);
    if status == Status::Archived {
        return Stage::Archival;
    }

    if evidence.inbox_pending {
        return Stage::Reception;
    }

    // Un proyecto entregado sin cambios pasa a la etapa de archivo
    // (apartado 6.2: noventa días en `3_DELIVERED`).
    if evidence.has_delivery && status == Status::Delivered {
        return Stage::Archival;
    }
    if evidence.has_transfer {
        return Stage::ProductionExchange;
    }
    if evidence.has_qc_report {
        return Stage::Distribution;
    }
    if evidence.has_master {
        // Las mediciones registradas concluyen el mastering (Tabla I.1).
        let mediciones = doc
            .at("master.true_peak_db")
            .map(|n| !n.is_null())
            .unwrap_or(false);
        return if mediciones {
            Stage::QualityControl
        } else {
            Stage::Mastering
        };
    }
    if evidence.has_stems || evidence.has_mix {
        return Stage::Mixing;
    }
    if evidence.has_edit {
        return Stage::Editing;
    }
    if evidence.has_recordings || evidence.has_sessions {
        // El vocabulario congelado cierra la grabación (apartado 14.2).
        let congelado = doc
            .at("vocabulary.frozen")
            .and_then(|n| n.as_bool())
            .unwrap_or(false);
        return if congelado { Stage::Editing } else { Stage::Recording };
    }
    // Sin sesiones, la etapa depende de si el plan de grabación consta.
    let plan = doc
        .at("tools.primary")
        .map(|n| !n.is_null())
        .unwrap_or(false);
    if plan {
        Stage::PreProduction
    } else {
        Stage::Composition
    }
}

/// Deduce la etapa a partir del manifiesto únicamente, sin observar el árbol.
/// Se emplea cuando el proyecto no está montado.
pub fn active_stage_from_doc(doc: &Map) -> Option<Stage> {
    Some(active_stage(doc, &Evidence::default()))
}

/// Deduce la etapa observando el proyecto en disco.
pub fn active_stage_of(doc: &Map, project_root: &Path) -> Stage {
    active_stage(doc, &observe(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Node;
    use std::fs;

    fn doc_base() -> Map {
        let mut d = Map::new();
        d.set("status", Node::str("active"));
        d.set("custody", Node::map(vec![("state", Node::str("propia"))]));
        d
    }

    #[test]
    fn un_proyecto_vacio_esta_en_composicion() {
        assert_eq!(active_stage(&doc_base(), &Evidence::default()), Stage::Composition);
    }

    #[test]
    fn las_sesiones_situan_el_proyecto_en_grabacion() {
        let ev = Evidence { has_sessions: true, ..Default::default() };
        assert_eq!(active_stage(&doc_base(), &ev), Stage::Recording);
    }

    #[test]
    fn el_vocabulario_congelado_cierra_la_grabacion() {
        let mut d = doc_base();
        d.set("vocabulary", Node::map(vec![("frozen", Node::Bool(true))]));
        let ev = Evidence { has_sessions: true, ..Default::default() };
        assert_eq!(active_stage(&d, &ev), Stage::Editing);
    }

    #[test]
    fn las_mediciones_registradas_cierran_el_mastering() {
        let ev = Evidence { has_master: true, ..Default::default() };
        assert_eq!(active_stage(&doc_base(), &ev), Stage::Mastering);

        let mut d = doc_base();
        d.set("master", Node::map(vec![("true_peak_db", Node::Float(-1.0))]));
        assert_eq!(active_stage(&d, &ev), Stage::QualityControl);
    }

    #[test]
    fn la_custodia_cedida_prevalece_sobre_el_contenido() {
        let mut d = doc_base();
        d.set("custody", Node::map(vec![("state", Node::str("cedida"))]));
        let ev = Evidence { has_master: true, has_qc_report: true, ..Default::default() };
        assert_eq!(active_stage(&d, &ev), Stage::ProductionExchange);
    }

    #[test]
    fn la_etapa_no_procede_de_una_preferencia_guardada() {
        // El manifiesto declara una etapa que no corresponde al contenido: se
        // ignora. El apartado 44.2 exige deducirla del manifiesto y del
        // contenido, no de un valor almacenado.
        let mut d = doc_base();
        d.set("x_attacca_ui_stage", Node::str("distribucion"));
        assert_eq!(active_stage(&d, &Evidence::default()), Stage::Composition);
    }

    #[test]
    fn observa_el_contenido_real_del_proyecto() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("02_SESSIONS/Reaper")).unwrap();
        fs::create_dir_all(dir.path().join("05_STEMS")).unwrap();
        // Una carpeta con solo regenerables cuenta como vacía.
        fs::write(dir.path().join("05_STEMS/.DS_Store"), b"x").unwrap();
        fs::write(dir.path().join("02_SESSIONS/Reaper/s.rpp"), b"sesion").unwrap();

        let ev = observe(dir.path());
        assert!(ev.has_sessions);
        assert!(!ev.has_stems, "una carpeta con solo regenerables está vacía");
        assert_eq!(active_stage(&doc_base(), &ev), Stage::Recording);
    }

    #[test]
    fn cada_etapa_declara_carpetas_y_acciones() {
        for etapa in [
            Stage::Composition, Stage::PreProduction, Stage::Recording, Stage::Editing,
            Stage::Mixing, Stage::Mastering, Stage::QualityControl, Stage::Distribution,
            Stage::ProductionExchange, Stage::Reception, Stage::Archival,
        ] {
            assert!(!etapa.folders().is_empty(), "{:?}", etapa);
            assert!(!etapa.actions().is_empty(), "{:?}", etapa);
            assert_eq!(Stage::parse(etapa.key()), Some(etapa));
        }
    }
}
