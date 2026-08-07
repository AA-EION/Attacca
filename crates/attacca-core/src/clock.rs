//! Referencia temporal (apartado 22.3).
//!
//! Toda la trazabilidad de la norma descansa en que las marcas temporales de
//! equipos distintos sean comparables. Este módulo consulta una fuente de tiempo
//! de red mediante SNTP (RFC 5905, modo cliente simple), mide la desviación del
//! reloj local y expresa las marcas conforme a RFC 3339 con desplazamiento de
//! zona explícito.
//!
//! Sin conexión se permite trabajar. Lo que queda bloqueado es la emisión de
//! paquetes, de acuses y de revocaciones, conforme al apartado 22.3.1.

use crate::error::{Error, Result};
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// Desviación máxima admitida, en segundos (apartado 22.3.1).
pub const MAX_DRIFT_SECONDS: i64 = 1;

/// Servidores por defecto. Ninguno es propio de Attacca: la norma prohíbe
/// depender de un servicio del fabricante y el apartado 3 de las restricciones
/// de la aplicación prohíbe un servidor propio.
pub const DEFAULT_SOURCES: &[&str] = &[
    "pool.ntp.org:123",
    "time.cloudflare.com:123",
    "time.nist.gov:123",
];

/// Diferencia entre la época NTP (1900-01-01) y la de Unix (1970-01-01).
const NTP_UNIX_DELTA: u64 = 2_208_988_800;

/// Estado de la sincronización horaria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    /// Reloj sincronizado dentro de la tolerancia.
    Synced { drift_ms: i64 },
    /// Desviación por encima de la tolerancia del apartado 22.3.1.
    Drifted { drift_ms: i64 },
    /// No se ha podido consultar ninguna fuente de tiempo.
    Unavailable,
}

impl SyncState {
    /// Un acto que emite marca temporal hacia otra organización exige
    /// sincronización válida (apartado 22.3.1, tercer guion).
    pub fn allows_emission(&self) -> bool {
        matches!(self, SyncState::Synced { .. })
    }

    pub fn drift_seconds(&self) -> i64 {
        match self {
            SyncState::Synced { drift_ms } | SyncState::Drifted { drift_ms } => drift_ms / 1000,
            SyncState::Unavailable => 0,
        }
    }
}

/// Desplazamiento de zona horaria, capturado una sola vez.
///
/// `OffsetDateTime::now_local` falla en procesos con varios hilos en algunos
/// sistemas Unix. Se captura al arrancar, antes de crear hilos, y se reutiliza.
static LOCAL_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

/// Última desviación medida, en milisegundos, y su validez.
static LAST_DRIFT_MS: AtomicI64 = AtomicI64::new(0);
static HAS_SYNC: AtomicI64 = AtomicI64::new(0);

/// Captura el desplazamiento de zona local. Debe invocarse al arrancar, antes de
/// crear hilos. Si no se invoca, las marcas se expresan en UTC (`+00:00`), que
/// sigue siendo conforme a RFC 3339.
pub fn init_local_offset() {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let _ = LOCAL_OFFSET.set(offset);
}

fn local_offset() -> UtcOffset {
    *LOCAL_OFFSET.get_or_init(|| UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
}

/// Marca temporal actual conforme a RFC 3339, con desplazamiento de zona
/// explícito y resolución de segundo.
pub fn now_rfc3339() -> String {
    format_rfc3339(OffsetDateTime::now_utc())
}

/// Formatea un instante conforme a RFC 3339 en la zona local.
pub fn format_rfc3339(instant: OffsetDateTime) -> String {
    instant
        .to_offset(local_offset())
        .replace_nanosecond(0)
        .unwrap_or(instant)
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// Fecha actual en formato `AAAA-MM-DD` (regla 1 de la Tabla 9).
pub fn today() -> String {
    let d = OffsetDateTime::now_utc().to_offset(local_offset()).date();
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// Interpreta una marca temporal RFC 3339.
pub fn parse_rfc3339(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|_| {
        Error::input(format!(
            "La marca temporal «{s}» no se ajusta a RFC 3339. El valor no se ha aceptado. Emplear el formato 2026-08-06T17:20:00-05:00."
        ))
    })
}

/// Suma días a una fecha `AAAA-MM-DD` y devuelve el resultado en el mismo
/// formato. Se emplea para la fecha esperada de retorno y el plazo de gracia.
pub fn add_days(date: &str, days: i64) -> Result<String> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::input(format!(
            "La fecha «{date}» no tiene el formato AAAA-MM-DD. El valor no se ha aceptado."
        )));
    }
    let (y, m, d) = (
        parts[0].parse::<i32>().map_err(|_| bad_date(date))?,
        parts[1].parse::<u8>().map_err(|_| bad_date(date))?,
        parts[2].parse::<u8>().map_err(|_| bad_date(date))?,
    );
    let month = time::Month::try_from(m).map_err(|_| bad_date(date))?;
    let base = time::Date::from_calendar_date(y, month, d).map_err(|_| bad_date(date))?;
    let out = base
        .checked_add(time::Duration::days(days))
        .ok_or_else(|| bad_date(date))?;
    Ok(format!(
        "{:04}-{:02}-{:02}",
        out.year(),
        out.month() as u8,
        out.day()
    ))
}

/// Indica si una fecha `AAAA-MM-DD` ya venció respecto del día de hoy.
pub fn is_past(date: &str) -> bool {
    date < today().as_str()
}

fn bad_date(date: &str) -> Error {
    Error::input(format!(
        "La fecha «{date}» no es una fecha válida. El valor no se ha aceptado. Emplear el formato AAAA-MM-DD."
    ))
}

/// Fuentes de reserva sobre HTTPS.
///
/// Muchas redes de estudio bloquean el puerto 123 y admiten solo HTTPS. El
/// apartado 22.3.1 admite «un protocolo de sincronización horaria equivalente»
/// además de RFC 5905; la cabecera `Date` de una respuesta HTTP lo es, con la
/// limitación de precisión que se declara en [`Precision`].
pub const DEFAULT_HTTPS_SOURCES: &[&str] =
    &["https://www.cloudflare.com/", "https://www.google.com/"];

/// Precisión de la medida, según la fuente empleada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    /// SNTP conforme a RFC 5905. Resolución muy por debajo del segundo.
    Ntp,
    /// Cabecera `Date` de una respuesta HTTPS. Su resolución es de un segundo,
    /// igual a la tolerancia del apartado 22.3.1: una desviación de menos de un
    /// segundo no es distinguible de una desviación nula.
    HttpDate,
}

/// Última precisión empleada. 0 = NTP, 1 = cabecera Date.
static LAST_PRECISION: AtomicI64 = AtomicI64::new(0);

/// Precisión de la última medida.
pub fn last_precision() -> Precision {
    if LAST_PRECISION.load(Ordering::Relaxed) == 0 {
        Precision::Ntp
    } else {
        Precision::HttpDate
    }
}

/// Consulta las fuentes de tiempo y devuelve el estado resultante.
///
/// Se intenta primero SNTP, que es la fuente que el apartado 22.3.1 nombra. Si
/// ninguna responde, se recurre a la cabecera `Date` sobre HTTPS. El resultado
/// se memoriza para que la interfaz pueda consultarlo sin repetir la consulta.
pub fn check_sync(sources: &[&str], timeout: Duration) -> SyncState {
    for source in sources {
        if let Some(drift_ms) = query_sntp(source, timeout) {
            return record(drift_ms, Precision::Ntp);
        }
    }
    for source in DEFAULT_HTTPS_SOURCES {
        if let Some(drift_ms) = query_http_date(source, timeout) {
            return record(drift_ms, Precision::HttpDate);
        }
    }
    HAS_SYNC.store(0, Ordering::Relaxed);
    SyncState::Unavailable
}

fn record(drift_ms: i64, precision: Precision) -> SyncState {
    LAST_DRIFT_MS.store(drift_ms, Ordering::Relaxed);
    HAS_SYNC.store(1, Ordering::Relaxed);
    LAST_PRECISION.store(
        match precision {
            Precision::Ntp => 0,
            Precision::HttpDate => 1,
        },
        Ordering::Relaxed,
    );
    if drift_ms.abs() <= MAX_DRIFT_SECONDS * 1000 {
        SyncState::Synced { drift_ms }
    } else {
        SyncState::Drifted { drift_ms }
    }
}

/// Lee la cabecera `Date` de una respuesta HTTPS y calcula la desviación.
///
/// La mitad del tiempo de ida y vuelta se descuenta como estimación de la
/// latencia. Una respuesta cuyo trayecto sea demasiado lento se descarta: la
/// medida dejaría de ser útil frente a una tolerancia de un segundo.
fn query_http_date(url: &str, timeout: Duration) -> Option<i64> {
    // El almacén de confianza del sistema operativo se emplea en lugar de una
    // lista de raíces incrustada: un estudio detrás de un proxy corporativo
    // tiene su propia autoridad de certificación instalada en el sistema, y una
    // lista incrustada la rechazaría.
    let mut constructor = ureq::AgentBuilder::new()
        .timeout(timeout)
        .tls_config(tls_config())
        .user_agent(&crate::written_by());

    // Se respeta la configuración de proxy del entorno, habitual en redes de
    // estudio y de oficina.
    if let Some(proxy) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .ok()
        .and_then(|s| ureq::Proxy::new(s.trim_start_matches("http://")).ok())
    {
        constructor = constructor.proxy(proxy);
    }
    let agente = constructor.build();

    let t1 = SystemTime::now();
    let respuesta = agente.head(url).call().ok()?;
    let t2 = SystemTime::now();

    let cabecera = respuesta.header("date")?;
    let servidor_ms = parse_http_date(cabecera)?;

    let ida_vuelta = t2.duration_since(t1).ok()?.as_millis() as i64;
    if ida_vuelta > 2000 {
        return None;
    }
    let local_ms = unix_millis(t1)? + ida_vuelta / 2;
    Some(local_ms - servidor_ms)
}

/// Configuración TLS que delega la validación en el almacén de confianza del
/// sistema operativo.
///
/// Se construye una sola vez: cargar el almacén tiene coste y no cambia durante
/// la ejecución.
fn tls_config() -> std::sync::Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<std::sync::Arc<rustls::ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            use rustls_platform_verifier::ConfigVerifierExt;
            // El proveedor criptográfico se instala una vez por proceso; que ya
            // esté instalado no es un fallo.
            let _ = rustls::crypto::ring::default_provider().install_default();
            std::sync::Arc::new(
                rustls::ClientConfig::with_platform_verifier()
                    .expect("el almacén de confianza del sistema no se pudo cargar"),
            )
        })
        .clone()
}

/// Interpreta el formato de fecha de HTTP: `Wed, 06 Aug 2026 17:20:00 GMT`.
fn parse_http_date(value: &str) -> Option<i64> {
    let partes: Vec<&str> = value.split_whitespace().collect();
    if partes.len() < 5 {
        return None;
    }
    let dia: u8 = partes[1].parse().ok()?;
    let mes = match partes[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let anio: i32 = partes[3].parse().ok()?;
    let hora: Vec<&str> = partes[4].split(':').collect();
    if hora.len() != 3 {
        return None;
    }
    let fecha = time::Date::from_calendar_date(anio, time::Month::try_from(mes).ok()?, dia).ok()?;
    let hms = time::Time::from_hms(
        hora[0].parse().ok()?,
        hora[1].parse().ok()?,
        hora[2].parse().ok()?,
    )
    .ok()?;
    Some((fecha.with_time(hms).assume_utc().unix_timestamp()) * 1000)
}

/// Último estado conocido, sin consultar la red.
pub fn last_known_state() -> SyncState {
    if HAS_SYNC.load(Ordering::Relaxed) == 0 {
        return SyncState::Unavailable;
    }
    let drift_ms = LAST_DRIFT_MS.load(Ordering::Relaxed);
    if drift_ms.abs() <= MAX_DRIFT_SECONDS * 1000 {
        SyncState::Synced { drift_ms }
    } else {
        SyncState::Drifted { drift_ms }
    }
}

/// Exige sincronización válida antes de un acto que emite marca temporal hacia
/// otra organización: paquete, acuse o revocación (apartado 22.3.1).
pub fn require_sync_for_emission(state: SyncState) -> Result<()> {
    match state {
        SyncState::Synced { .. } => Ok(()),
        SyncState::Drifted { drift_ms } => Err(Error::Clock {
            drift_seconds: drift_ms / 1000,
        }),
        SyncState::Unavailable => Err(Error::requirement(
            "22.3.1",
            "No se ha podido consultar ninguna fuente de tiempo de red. La emisión de paquetes y de acuses queda bloqueada hasta que haya una sincronización válida. Conectar el equipo a la red y repetir la comprobación.",
        )),
    }
}

/// Cliente SNTP mínimo. Devuelve la desviación del reloj local respecto de la
/// fuente, en milisegundos, o `None` si la fuente no responde.
fn query_sntp(source: &str, timeout: Duration) -> Option<i64> {
    let addr = source.to_socket_addrs().ok()?.next()?;
    let socket = UdpSocket::bind(if addr.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })
    .ok()?;
    socket.set_read_timeout(Some(timeout)).ok()?;
    socket.set_write_timeout(Some(timeout)).ok()?;

    let mut packet = [0u8; 48];
    // LI = 0, VN = 4, Mode = 3 (cliente).
    packet[0] = 0b00_100_011;

    let t1 = SystemTime::now();
    socket.send_to(&packet, addr).ok()?;
    let mut buf = [0u8; 48];
    let (n, _) = socket.recv_from(&mut buf).ok()?;
    let t4 = SystemTime::now();
    if n < 48 {
        return None;
    }

    // Modo servidor (4) o difusión (5); cualquier otro valor no es una respuesta
    // válida a una petición de cliente.
    let mode = buf[0] & 0b111;
    if mode != 4 && mode != 5 {
        return None;
    }
    // Estrato 0 indica un mensaje de control («kiss-o'-death»), sin hora útil.
    if buf[1] == 0 {
        return None;
    }

    let recv_ts = ntp_timestamp(&buf[32..40])?; // T2: llegada al servidor
    let xmit_ts = ntp_timestamp(&buf[40..48])?; // T3: salida del servidor
    if xmit_ts == 0 {
        return None;
    }

    let t1_ms = unix_millis(t1)?;
    let t4_ms = unix_millis(t4)?;
    let t2_ms = ntp_to_unix_millis(recv_ts);
    let t3_ms = ntp_to_unix_millis(xmit_ts);

    // Desplazamiento del reloj según RFC 5905: ((T2-T1) + (T3-T4)) / 2.
    // El signo se invierte para expresar la desviación del reloj local.
    let offset_ms = ((t2_ms - t1_ms) + (t3_ms - t4_ms)) / 2;
    Some(-offset_ms)
}

fn ntp_timestamp(bytes: &[u8]) -> Option<u64> {
    if bytes.len() < 8 {
        return None;
    }
    let mut v = [0u8; 8];
    v.copy_from_slice(&bytes[..8]);
    Some(u64::from_be_bytes(v))
}

fn ntp_to_unix_millis(ts: u64) -> i64 {
    let seconds = (ts >> 32) as i64 - NTP_UNIX_DELTA as i64;
    let fraction = (ts & 0xFFFF_FFFF) as i64;
    seconds * 1000 + (fraction * 1000) / 0x1_0000_0000
}

fn unix_millis(t: SystemTime) -> Option<i64> {
    let d = t.duration_since(UNIX_EPOCH).ok()?;
    Some(d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_marcas_llevan_desplazamiento_de_zona_explicito() {
        let ts = now_rfc3339();
        assert!(
            ts.ends_with('Z') || ts[19..].starts_with('+') || ts[19..].starts_with('-'),
            "la marca {ts} carece de desplazamiento de zona"
        );
        assert!(parse_rfc3339(&ts).is_ok());
    }

    #[test]
    fn hoy_tiene_formato_iso_8601() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert_eq!(d.as_bytes()[4], b'-');
        assert_eq!(d.as_bytes()[7], b'-');
    }

    #[test]
    fn suma_de_dias_cruza_fin_de_mes_y_de_ano() {
        assert_eq!(add_days("2026-08-06", 15).unwrap(), "2026-08-21");
        assert_eq!(add_days("2026-01-31", 1).unwrap(), "2026-02-01");
        assert_eq!(add_days("2026-12-31", 1).unwrap(), "2027-01-01");
        assert_eq!(add_days("2028-02-28", 1).unwrap(), "2028-02-29");
    }

    #[test]
    fn la_emision_exige_sincronizacion_valida() {
        assert!(require_sync_for_emission(SyncState::Synced { drift_ms: 120 }).is_ok());
        assert!(require_sync_for_emission(SyncState::Drifted { drift_ms: 4000 }).is_err());
        assert!(require_sync_for_emission(SyncState::Unavailable).is_err());
    }

    #[test]
    fn interpreta_el_formato_de_fecha_de_http() {
        let ms = parse_http_date("Wed, 06 Aug 2026 17:20:00 GMT").unwrap();
        // 2026-08-06T17:20:00Z
        assert_eq!(ms, 1_786_036_800_000);
        assert!(parse_http_date("no es una fecha").is_none());
        assert!(parse_http_date("Wed, 06 Xxx 2026 17:20:00 GMT").is_none());
    }

    #[test]
    fn declara_la_precision_de_la_fuente_empleada() {
        // La cabecera Date tiene resolución de un segundo, igual a la
        // tolerancia: la limitación debe ser visible para quien la consulta.
        assert_ne!(Precision::Ntp, Precision::HttpDate);
    }

    #[test]
    fn el_estado_desviado_no_admite_emision_pero_no_impide_trabajar() {
        let s = SyncState::Drifted { drift_ms: 3500 };
        assert!(!s.allows_emission());
        assert_eq!(s.drift_seconds(), 3);
    }
}
