//! Custodia editorial (apartados 14.3 y 41).
//!
//! En todo instante un proyecto tiene exactamente un titular. Quien cede deja de
//! trabajar en él hasta su retorno. El bloqueo es técnico, no de convención.

use crate::clock;
use crate::doc::Node;
use crate::error::{Error, Result};

/// Estados de custodia (Tabla 15).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyState {
    /// La custodia corresponde a la organización que aloja esta copia.
    Own,
    /// Cedida mediante un envío emitido cuyo acuse aún no se ha obtenido.
    InTransit,
    /// Corresponde a otra parte, que ha acusado recibo.
    Ceded,
    /// La cesión venció y el cedente la recuperó (apartado 41.4).
    Reclaimed,
}

impl CustodyState {
    /// Término normativo del apartado 3. La interfaz emplea exactamente estos.
    pub fn as_str(&self) -> &'static str {
        match self {
            CustodyState::Own => "propia",
            CustodyState::InTransit => "en_transito",
            CustodyState::Ceded => "cedida",
            CustodyState::Reclaimed => "reclamada",
        }
    }

    pub fn parse(s: &str) -> Option<CustodyState> {
        Some(match s {
            "propia" => CustodyState::Own,
            "en_transito" => CustodyState::InTransit,
            "cedida" => CustodyState::Ceded,
            "reclamada" => CustodyState::Reclaimed,
            _ => return None,
        })
    }

    /// Un proyecto solo se modifica cuando su custodia es propia o reclamada
    /// (Tabla 15).
    pub fn allows_write(&self) -> bool {
        matches!(self, CustodyState::Own | CustodyState::Reclaimed)
    }

    /// El marcador `CUSTODY.lock` existe mientras el estado no sea propia ni
    /// reclamada (apartado 14.3.3).
    pub fn requires_lock_marker(&self) -> bool {
        !self.allows_write()
    }

    /// Un proyecto en tránsito o cedido no debe cederse de nuevo
    /// (apartado 41.1, último guion).
    pub fn allows_cession(&self) -> bool {
        matches!(self, CustodyState::Own)
    }

    /// El archivo exige custodia propia o reclamada (apartado 14.3.5).
    pub fn allows_archival(&self) -> bool {
        self.allows_write()
    }
}

/// Actos que generan asiento en la cronología (Tabla 16).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyAction {
    Exported,
    Sent,
    Received,
    Imported,
    CustodyTransferred,
    CustodyAssumed,
    CustodyReturned,
    CustodyReclaimed,
}

impl CustodyAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            CustodyAction::Exported => "exported",
            CustodyAction::Sent => "sent",
            CustodyAction::Received => "received",
            CustodyAction::Imported => "imported",
            CustodyAction::CustodyTransferred => "custody_transferred",
            CustodyAction::CustodyAssumed => "custody_assumed",
            CustodyAction::CustodyReturned => "custody_returned",
            CustodyAction::CustodyReclaimed => "custody_reclaimed",
        }
    }

    pub fn parse(s: &str) -> Option<CustodyAction> {
        Some(match s {
            "exported" => CustodyAction::Exported,
            "sent" => CustodyAction::Sent,
            "received" => CustodyAction::Received,
            "imported" => CustodyAction::Imported,
            "custody_transferred" => CustodyAction::CustodyTransferred,
            "custody_assumed" => CustodyAction::CustodyAssumed,
            "custody_returned" => CustodyAction::CustodyReturned,
            "custody_reclaimed" => CustodyAction::CustodyReclaimed,
            _ => return None,
        })
    }

    /// Nombre normativo del evento correspondiente en el registro (Tabla B.1).
    /// Los cuatro primeros actos se registran con nombres de la familia
    /// `exchange.*`; los de custodia, con la familia `custody.*`.
    pub fn event_name(&self) -> &'static str {
        match self {
            CustodyAction::Exported => "exchange.package.built",
            CustodyAction::Sent => "exchange.package.emitted",
            CustodyAction::Received => "exchange.package.received",
            CustodyAction::Imported => "exchange.package.ingested",
            CustodyAction::CustodyTransferred => "custody.transferred",
            CustodyAction::CustodyAssumed => "custody.assumed",
            CustodyAction::CustodyReturned => "custody.returned",
            CustodyAction::CustodyReclaimed => "custody.reclaimed",
        }
    }
}

/// Asiento de la cronología de custodia (apartado 14.3.4).
#[derive(Clone, Debug, PartialEq)]
pub struct ChronologyEntry {
    pub seq: i64,
    pub action: CustodyAction,
    pub ts: String,
    pub actor: String,
    pub org: String,
    pub shipment_id: Option<String>,
}

impl ChronologyEntry {
    pub fn from_node(node: &Node) -> Option<ChronologyEntry> {
        let m = node.as_map()?;
        Some(ChronologyEntry {
            seq: m.get("seq")?.as_int()?,
            action: CustodyAction::parse(m.get("action")?.as_str()?)?,
            ts: m.get("ts")?.as_str()?.to_string(),
            actor: m
                .get("actor")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            org: m
                .get("org")
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string(),
            shipment_id: m
                .get("shipment_id")
                .and_then(|n| n.present_str())
                .map(str::to_string),
        })
    }

    pub fn to_node(&self) -> Node {
        Node::map(vec![
            ("seq", Node::Int(self.seq)),
            ("action", Node::str(self.action.as_str())),
            ("ts", Node::str(&self.ts)),
            ("actor", Node::str(&self.actor)),
            ("org", Node::str(&self.org)),
            ("shipment_id", Node::opt_str(self.shipment_id.as_deref())),
        ])
    }
}

/// Resultado de la comprobación de una cronología (verificación 9 del
/// apartado 37.2).
#[derive(Clone, Debug, Default)]
pub struct ChronologyCheck {
    /// Números de secuencia duplicados: no conformidad mayor (apartado 14.3.4).
    pub duplicate_seq: Vec<i64>,
    /// Huecos en la secuencia: indican un asiento suprimido.
    pub missing_seq: Vec<i64>,
    /// Marcas temporales decrecientes. No invalidan la cronología por sí solas:
    /// el orden lo determina el número de secuencia (apartado 14.3.4).
    pub decreasing_timestamps: Vec<i64>,
}

impl ChronologyCheck {
    /// Un asiento suprimido o un número duplicado impiden la aceptación.
    /// Una marca decreciente, no.
    pub fn is_blocking(&self) -> bool {
        !self.duplicate_seq.is_empty() || !self.missing_seq.is_empty()
    }

    pub fn is_clean(&self) -> bool {
        !self.is_blocking() && self.decreasing_timestamps.is_empty()
    }
}

/// Comprueba las propiedades exigibles a una cronología (apartado 14.3.4).
pub fn check_chronology(entries: &[ChronologyEntry]) -> ChronologyCheck {
    let mut check = ChronologyCheck::default();
    if entries.is_empty() {
        return check;
    }
    let mut ordered = entries.to_vec();
    ordered.sort_by_key(|e| e.seq);

    for pair in ordered.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.seq == b.seq {
            check.duplicate_seq.push(a.seq);
        } else {
            for missing in (a.seq + 1)..b.seq {
                check.missing_seq.push(missing);
            }
        }
        // La comparación se hace sobre el instante absoluto: dos marcas con
        // desplazamientos de zona distintos son comparables entre sí.
        if let (Ok(ta), Ok(tb)) = (clock::parse_rfc3339(&a.ts), clock::parse_rfc3339(&b.ts)) {
            if tb < ta {
                check.decreasing_timestamps.push(b.seq);
            }
        }
    }
    // La secuencia empieza en 1 (apartado 14.3.4).
    if ordered[0].seq > 1 {
        for missing in 1..ordered[0].seq {
            check.missing_seq.push(missing);
        }
    }
    check
}

/// Fusiona los asientos recibidos de otra parte con los propios
/// (apartados 14.3.4 y 41.3, paso 2).
///
/// Los asientos propios no se modifican ni se suprimen, y ninguno de los
/// recibidos se descarta. Los que ya constan —mismo acto, mismo instante y
/// misma organización— no se duplican.
///
/// # Numeración independiente
///
/// Mientras un envío está en tránsito, ambas partes generan asientos sin
/// conocer los de la otra: el cedente registra `custody_transferred` al recibir
/// el acuse, y el cesionario ha registrado ya `received`, `imported` y
/// `custody_assumed`. Ambas series parten del mismo punto y colisionan.
///
/// El apartado 14.3.4 exige tres cosas a la vez: que los números sean
/// consecutivos, que ninguno se duplique y que al recibir un retorno se
/// incorporen los asientos de la otra parte «conservando su orden y sin
/// suprimir ninguno». Con numeración independiente, las tres solo pueden
/// satisfacerse renumerando una de las series. Se renumeran los asientos
/// recibidos, nunca los propios, y se conserva su orden relativo: es la lectura
/// que preserva el requisito de no suprimir ni reordenar, y la única que
/// produce una cronología sin duplicados.
pub fn merge_chronology(
    own: &[ChronologyEntry],
    incoming: &[ChronologyEntry],
) -> Vec<ChronologyEntry> {
    let mut out = own.to_vec();
    out.sort_by_key(|e| e.seq);

    // Un asiento recibido que ya consta no se duplica. La identidad de un
    // asiento la dan el acto, el instante y la organización que lo generó; el
    // número de secuencia no, porque es precisamente lo que puede diferir.
    let mut pendientes: Vec<ChronologyEntry> = Vec::new();
    for entry in incoming {
        let ya_consta = out
            .iter()
            .any(|e| e.action == entry.action && e.ts == entry.ts && e.org == entry.org);
        let repetido_en_lote = pendientes
            .iter()
            .any(|e: &ChronologyEntry| e.action == entry.action && e.ts == entry.ts && e.org == entry.org);
        if !ya_consta && !repetido_en_lote {
            pendientes.push(entry.clone());
        }
    }

    // Los recibidos conservan su orden relativo y se numeran a continuación de
    // los propios.
    pendientes.sort_by_key(|e| e.seq);
    let mut siguiente = out.iter().map(|e| e.seq).max().unwrap_or(0) + 1;
    for mut entry in pendientes {
        entry.seq = siguiente;
        siguiente += 1;
        out.push(entry);
    }
    out
}

/// Vencimiento de una cesión (apartado 41.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    /// Dentro de la fecha esperada de retorno.
    Current,
    /// Vencida la fecha esperada, dentro del plazo de gracia: procede reclamar.
    ClaimDue,
    /// Vencido además el plazo de gracia: procede la recuperación forzosa.
    ReclaimAvailable,
}

/// Determina la situación de una cesión frente a sus plazos.
pub fn expiry_state(expected_return: &str, grace_days: i64) -> Result<Expiry> {
    let limite_gracia = clock::add_days(expected_return, grace_days)?;
    Ok(if clock::is_past(&limite_gracia) {
        Expiry::ReclaimAvailable
    } else if clock::is_past(expected_return) {
        Expiry::ClaimDue
    } else {
        Expiry::Current
    })
}

/// Comprueba que la acción solicitada sea compatible con el estado de custodia.
pub fn require_writable(state: CustodyState) -> Result<()> {
    if state.allows_write() {
        return Ok(());
    }
    Err(Error::Custody {
        state: state.as_str().to_string(),
        detail: match state {
            CustodyState::InTransit => "El proyecto no se ha modificado. Ninguna de las dos partes debe modificarlo mientras el acuse de recibo no se obtenga. Esperar el acuse, o recuperar la custodia si el plazo venció.".into(),
            _ => "El proyecto no se ha modificado. La copia local permanece en solo lectura hasta que la custodia retorne. Para trabajar ahora, derivar un proyecto conforme al apartado 14.3.3.".into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asiento(seq: i64, action: CustodyAction, ts: &str) -> ChronologyEntry {
        ChronologyEntry {
            seq,
            action,
            ts: ts.to_string(),
            actor: "J. Duarte".into(),
            org: "Estudio A".into(),
            shipment_id: Some("20260806-AAAA".into()),
        }
    }

    #[test]
    fn los_estados_usan_el_vocabulario_normativo() {
        assert_eq!(CustodyState::Own.as_str(), "propia");
        assert_eq!(CustodyState::InTransit.as_str(), "en_transito");
        assert_eq!(CustodyState::Ceded.as_str(), "cedida");
        assert_eq!(CustodyState::Reclaimed.as_str(), "reclamada");
    }

    #[test]
    fn solo_propia_y_reclamada_admiten_escritura() {
        assert!(CustodyState::Own.allows_write());
        assert!(CustodyState::Reclaimed.allows_write());
        assert!(!CustodyState::InTransit.allows_write());
        assert!(!CustodyState::Ceded.allows_write());
        assert!(CustodyState::Ceded.requires_lock_marker());
        assert!(!CustodyState::Reclaimed.requires_lock_marker());
    }

    #[test]
    fn un_proyecto_cedido_no_vuelve_a_cederse() {
        assert!(CustodyState::Own.allows_cession());
        assert!(!CustodyState::InTransit.allows_cession());
        assert!(!CustodyState::Ceded.allows_cession());
    }

    #[test]
    fn una_cronologia_consecutiva_es_limpia() {
        let c = check_chronology(&[
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(2, CustodyAction::Sent, "2026-08-06T10:05:00-05:00"),
            asiento(3, CustodyAction::Received, "2026-08-06T11:00:00-05:00"),
        ]);
        assert!(c.is_clean());
        assert!(!c.is_blocking());
    }

    #[test]
    fn un_asiento_suprimido_bloquea_la_aceptacion() {
        let c = check_chronology(&[
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(3, CustodyAction::Received, "2026-08-06T11:00:00-05:00"),
        ]);
        assert_eq!(c.missing_seq, vec![2]);
        assert!(c.is_blocking());
    }

    #[test]
    fn un_numero_duplicado_bloquea_la_aceptacion() {
        let c = check_chronology(&[
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(1, CustodyAction::Sent, "2026-08-06T10:05:00-05:00"),
        ]);
        assert_eq!(c.duplicate_seq, vec![1]);
        assert!(c.is_blocking());
    }

    #[test]
    fn una_marca_decreciente_no_impide_el_retorno_por_si_sola() {
        // Apartado 14.3.4: el número de secuencia determina el orden; una marca
        // decreciente se trata conforme al apartado 22.3.2 y no bloquea.
        let c = check_chronology(&[
            asiento(1, CustodyAction::Exported, "2026-08-06T12:00:00-05:00"),
            asiento(2, CustodyAction::Sent, "2026-08-06T10:00:00-05:00"),
        ]);
        assert_eq!(c.decreasing_timestamps, vec![2]);
        assert!(!c.is_blocking());
        assert!(!c.is_clean());
    }

    #[test]
    fn compara_marcas_con_desplazamientos_de_zona_distintos() {
        // 15:00Z equivale a 10:00-05:00: no hay decrecimiento.
        let c = check_chronology(&[
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(2, CustodyAction::Sent, "2026-08-06T15:30:00Z"),
        ]);
        assert!(c.decreasing_timestamps.is_empty());
    }

    #[test]
    fn la_fusion_conserva_el_orden_y_no_duplica() {
        let propios = vec![
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(2, CustodyAction::Sent, "2026-08-06T10:05:00-05:00"),
        ];
        let recibidos = vec![
            asiento(2, CustodyAction::Sent, "2026-08-06T10:05:00-05:00"),
            asiento(3, CustodyAction::Received, "2026-08-06T11:00:00-05:00"),
            asiento(4, CustodyAction::Imported, "2026-08-06T11:30:00-05:00"),
        ];
        let fusion = merge_chronology(&propios, &recibidos);
        // El asiento `sent` ya constaba y no se duplica.
        assert_eq!(fusion.len(), 4);
        assert_eq!(fusion.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert!(check_chronology(&fusion).is_clean());
    }

    #[test]
    fn la_fusion_resuelve_la_numeracion_independiente_de_ambas_partes() {
        // Ambas partes numeraron a partir del mismo punto mientras el envío
        // estaba en tránsito: el cedente registró `custody_transferred` con el
        // 3, y el cesionario `received` también con el 3.
        let mut propio_transferido =
            asiento(3, CustodyAction::CustodyTransferred, "2026-08-06T12:00:00-05:00");
        propio_transferido.org = "Estudio A".into();
        let propios = vec![
            asiento(1, CustodyAction::Exported, "2026-08-06T10:00:00-05:00"),
            asiento(2, CustodyAction::Sent, "2026-08-06T10:05:00-05:00"),
            propio_transferido,
        ];

        let recibidos: Vec<ChronologyEntry> = [
            (3, CustodyAction::Received, "2026-08-06T11:00:00-05:00"),
            (4, CustodyAction::Imported, "2026-08-06T11:30:00-05:00"),
            (5, CustodyAction::CustodyAssumed, "2026-08-06T11:40:00-05:00"),
        ]
        .iter()
        .map(|(seq, accion, ts)| {
            let mut e = asiento(*seq, *accion, ts);
            e.org = "Estudio B".into();
            e
        })
        .collect();

        let fusion = merge_chronology(&propios, &recibidos);

        // Ningún asiento se suprime.
        assert_eq!(fusion.len(), 6);
        // La secuencia es consecutiva y sin duplicados.
        assert_eq!(fusion.iter().map(|e| e.seq).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5, 6]);
        assert!(check_chronology(&fusion).is_blocking() == false);
        assert!(check_chronology(&fusion).duplicate_seq.is_empty());
        assert!(check_chronology(&fusion).missing_seq.is_empty());
        // Los asientos propios conservan su número.
        assert_eq!(fusion[2].action, CustodyAction::CustodyTransferred);
        assert_eq!(fusion[2].seq, 3);
        // Los recibidos conservan su orden relativo.
        assert_eq!(fusion[3].action, CustodyAction::Received);
        assert_eq!(fusion[4].action, CustodyAction::Imported);
        assert_eq!(fusion[5].action, CustodyAction::CustodyAssumed);
    }

    #[test]
    fn los_plazos_de_vencimiento_siguen_el_apartado_41_4() {
        let futuro = clock::add_days(&clock::today(), 30).unwrap();
        assert_eq!(expiry_state(&futuro, 15).unwrap(), Expiry::Current);

        let vencido = clock::add_days(&clock::today(), -5).unwrap();
        assert_eq!(expiry_state(&vencido, 15).unwrap(), Expiry::ClaimDue);

        let muy_vencido = clock::add_days(&clock::today(), -40).unwrap();
        assert_eq!(expiry_state(&muy_vencido, 15).unwrap(), Expiry::ReclaimAvailable);
    }

    #[test]
    fn el_error_de_custodia_ofrece_una_salida() {
        let e = require_writable(CustodyState::Ceded).unwrap_err();
        let texto = e.to_string();
        assert!(texto.contains("cedida"));
        assert!(texto.contains("14.3.3"), "debe indicar la salida: {texto}");
        assert!(require_writable(CustodyState::Own).is_ok());
    }
}
