//! Identificadores (apartados 9.2 y 32.2).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Alfabeto Crockford base32, sin las letras I, L, O ni U.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Genera el identificador interno inmutable de un proyecto o de un release
/// (apartado 9.2).
///
/// Formato de 26 caracteres: 48 bits de marca temporal en milisegundos, seguidos
/// de 80 bits de aleatoriedad. Cumple los requisitos del apartado 9.2: único, no
/// reutilizable y ajeno al título, al artista y a la fecha legible del proyecto.
/// El orden lexicográfico coincide con el de creación, lo que hace que el
/// catálogo se ordene sin consultar ningún campo adicional.
pub fn new_uid() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
        & 0x0000_FFFF_FFFF_FFFF;

    let mut random = [0u8; 10];
    if getrandom::getrandom(&mut random).is_err() {
        // La ausencia de una fuente de aleatoriedad del sistema no debe impedir
        // crear un proyecto. Se degrada a un contador monótono combinado con la
        // marca temporal, que conserva la unicidad dentro del equipo.
        static FALLBACK: AtomicU64 = AtomicU64::new(0);
        let n = FALLBACK.fetch_add(1, Ordering::Relaxed);
        random[..8].copy_from_slice(&(n ^ ms.rotate_left(17)).to_be_bytes());
    }

    let mut bytes = [0u8; 16];
    bytes[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    bytes[6..].copy_from_slice(&random);
    encode_crockford(&bytes)
}

/// Codifica 16 octetos en 26 caracteres Crockford base32.
fn encode_crockford(bytes: &[u8; 16]) -> String {
    let mut out = String::with_capacity(26);
    // 128 bits no son múltiplo de 5; el primer carácter cubre los 3 bits altos.
    let mut acc: u32 = (bytes[0] >> 5) as u32;
    out.push(CROCKFORD[acc as usize] as char);

    let mut bits: u32 = 5;
    acc = (bytes[0] & 0b0001_1111) as u32;
    for byte in &bytes[1..] {
        acc = (acc << 8) | *byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((acc >> bits) & 0b1_1111) as usize;
            out.push(CROCKFORD[idx] as char);
        }
    }
    debug_assert_eq!(bits, 0);
    debug_assert_eq!(out.len(), 26);
    out
}

/// Comprueba que una cadena tenga la forma de identificador interno.
pub fn is_uid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|b| CROCKFORD.contains(&b))
}

/// Genera un identificador de envío (apartado 32.2).
///
/// El identificador no debe contener datos personales, títulos inéditos ni
/// información clasificada. Se compone de la fecha de emisión y de un sufijo
/// aleatorio, ambos exentos de contenido semántico sobre el material.
pub fn new_shipment_id(date: &str) -> String {
    let mut random = [0u8; 5];
    let _ = getrandom::getrandom(&mut random);
    let mut suffix = String::with_capacity(8);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for byte in random {
        acc = (acc << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            suffix.push(CROCKFORD[((acc >> bits) & 0b1_1111) as usize] as char);
        }
    }
    format!("{}-{}", date.replace('-', ""), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn el_identificador_interno_tiene_forma_estable() {
        let uid = new_uid();
        assert_eq!(uid.len(), 26, "{uid}");
        assert!(is_uid(&uid), "{uid}");
        // Regla 4 de la Tabla 9: solo caracteres del conjunto admitido.
        assert!(uid
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()));
    }

    #[test]
    fn los_identificadores_no_se_repiten() {
        let generados: HashSet<String> = (0..5000).map(|_| new_uid()).collect();
        assert_eq!(generados.len(), 5000);
    }

    #[test]
    fn el_orden_lexicografico_sigue_al_de_creacion() {
        let a = new_uid();
        std::thread::sleep(std::time::Duration::from_millis(3));
        let b = new_uid();
        assert!(a < b, "{a} debería preceder a {b}");
    }

    #[test]
    fn el_identificador_de_envio_no_lleva_datos_del_material() {
        let id = new_shipment_id("2026-08-06");
        assert!(id.starts_with("20260806-"), "{id}");
        assert!(crate::naming::is_valid_name(&id), "{id}");
    }
}
