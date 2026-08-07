//! Emisor YAML determinista.
//!
//! Dos escrituras del mismo estado producen bytes idénticos. El emisor no
//! reordena por sí mismo: recibe el árbol ya ordenado por [`super::order`] y se
//! limita a serializarlo con reglas fijas de sangrado, entrecomillado y
//! representación numérica.

use super::{Map, Node};

/// Serializa un documento. Termina siempre en un único salto de línea.
pub fn emit(root: &Map) -> String {
    let mut out = String::with_capacity(2048);
    emit_map(root, 0, &mut out);
    if out.is_empty() {
        out.push_str("{}\n");
    }
    out
}

fn emit_map(map: &Map, indent: usize, out: &mut String) {
    for (key, value) in map.iter() {
        push_indent(indent, out);
        out.push_str(&quote_key(key));
        out.push(':');
        emit_value(value, indent, out);
    }
}

fn emit_value(value: &Node, indent: usize, out: &mut String) {
    match value {
        Node::Map(m) if !m.is_empty() => {
            out.push('\n');
            emit_map(m, indent + 1, out);
        }
        Node::Map(_) => out.push_str(" {}\n"),
        Node::Seq(items) if !items.is_empty() => {
            out.push('\n');
            for item in items {
                push_indent(indent + 1, out);
                out.push('-');
                match item {
                    // Una aplicación dentro de una secuencia se emite en la
                    // misma línea del guion, con el sangrado desplazado dos
                    // espacios, que es la forma canónica de YAML en bloque.
                    Node::Map(m) if !m.is_empty() => {
                        let mut first = true;
                        for (k, v) in m.iter() {
                            if first {
                                out.push(' ');
                                first = false;
                            } else {
                                push_indent(indent + 2, out);
                            }
                            out.push_str(&quote_key(k));
                            out.push(':');
                            emit_value(v, indent + 2, out);
                        }
                    }
                    Node::Seq(_) | Node::Map(_) => {
                        emit_value(item, indent + 1, out);
                    }
                    scalar => {
                        out.push(' ');
                        out.push_str(&scalar_repr(scalar));
                        out.push('\n');
                    }
                }
            }
        }
        Node::Seq(_) => out.push_str(" []\n"),
        scalar => {
            out.push(' ');
            out.push_str(&scalar_repr(scalar));
            out.push('\n');
        }
    }
}

fn push_indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn scalar_repr(node: &Node) -> String {
    match node {
        Node::Null => "null".to_string(),
        Node::Bool(true) => "true".to_string(),
        Node::Bool(false) => "false".to_string(),
        Node::Int(i) => i.to_string(),
        Node::Float(f) => format_float(*f),
        Node::Str(s) => quote_scalar(s),
        Node::Seq(_) => "[]".to_string(),
        Node::Map(_) => "{}".to_string(),
    }
}

/// Representación estable de un número con parte decimal. `-1.0` se emite como
/// `-1.0` y no como `-1`, para que el tipo declarado en el Anexo B se conserve
/// entre lecturas y escrituras sucesivas.
fn format_float(f: f64) -> String {
    if !f.is_finite() {
        return "null".to_string();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        let mut s = format!("{f}");
        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
            s.push_str(".0");
        }
        s
    }
}

fn quote_key(key: &str) -> String {
    if key.is_empty() || needs_quoting(key) {
        quote_double(key)
    } else {
        key.to_string()
    }
}

fn quote_scalar(s: &str) -> String {
    if needs_quoting(s) {
        quote_double(s)
    } else {
        s.to_string()
    }
}

/// Una cadena se entrecomilla cuando sin comillas se leería como otro tipo, o
/// cuando contiene caracteres con significado sintáctico en YAML en bloque.
fn needs_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.trim() != s {
        return true;
    }
    // Escalares que YAML 1.2 interpreta como no cadena.
    const RESERVADAS: [&str; 11] = [
        "null", "Null", "NULL", "~", "true", "True", "TRUE", "false", "False", "FALSE", "-",
    ];
    if RESERVADAS.contains(&s) {
        return true;
    }
    if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        return true;
    }
    // Fechas y marcas temporales. YAML 1.2 las trata como cadenas, pero las
    // implementaciones de YAML 1.1 tienen tipo temporal propio y las
    // reinterpretan. Entrecomillarlas conserva el valor exacto entre
    // implementaciones, y es la forma en que aparecen en el Anexo B.
    if looks_temporal(s) {
        return true;
    }
    let first = s.as_bytes()[0];
    if matches!(
        first,
        b'-' | b'?' | b':' | b',' | b'[' | b']' | b'{' | b'}' | b'#' | b'&' | b'*' | b'!'
            | b'|' | b'>' | b'\'' | b'"' | b'%' | b'@' | b'`'
    ) {
        return true;
    }
    // Dos puntos seguidos de espacio abren una aplicación; la almohadilla
    // precedida de espacio abre un comentario.
    if s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return true;
    }
    s.chars()
        .any(|c| c.is_control() || c == '\n' || c == '\t')
}

/// Reconoce `AAAA-MM-DD` y las marcas RFC 3339 que empiezan por esa forma.
///
/// Un identificador legible de proyecto empieza igual —`2026-08-06_Titulo_ORIG`—
/// y no es una marca temporal: el separador que sigue a la fecha distingue
/// ambos casos.
fn looks_temporal(s: &str) -> bool {
    let b = s.as_bytes();
    let fecha = b.len() >= 10
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit);
    fecha && (b.len() == 10 || b[10] == b'T' || b[10] == b't' || b[10] == b' ')
}

fn quote_double(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::*;

    #[test]
    fn la_escritura_es_idempotente() {
        let fuente = "stave:\n  version: \"2.0\"\n  level: B\nuid: 01J9ZQ8F3K7N2VYB4T6XM0RSAE\naudio:\n  sample_rate: 48000\n  tuning_hz: 440\n  tempo: null\nmaster:\n  true_peak_db: -1.0\npeople:\n  - name: A. Ruiz\n    role: mezcla\n";
        let doc = parse(fuente).unwrap();
        let primera = emit(&doc);
        let segunda = emit(&parse(&primera).unwrap());
        assert_eq!(primera, segunda, "la reescritura debe ser byte a byte igual");
        assert_eq!(primera, fuente);
    }

    #[test]
    fn entrecomilla_las_cadenas_ambiguas() {
        let mut m = Map::new();
        m.set("version", Node::str("2.0"));
        m.set("state", Node::str("propia"));
        m.set("vacio", Node::str(""));
        m.set("si", Node::str("true"));
        assert_eq!(
            emit(&m),
            "version: \"2.0\"\nstate: propia\nvacio: \"\"\nsi: \"true\"\n"
        );
    }

    #[test]
    fn el_nulo_explicito_se_escribe_y_no_se_omite() {
        let mut m = Map::new();
        m.set("tempo", Node::Null);
        assert_eq!(emit(&m), "tempo: null\n");
        assert!(parse(&emit(&m)).unwrap().get("tempo").unwrap().is_null());
    }

    #[test]
    fn conserva_el_decimal_de_las_mediciones() {
        let doc = parse("master:\n  true_peak_db: -1.0\n  lufs_i: -9.8\n").unwrap();
        assert_eq!(emit(&doc), "master:\n  true_peak_db: -1.0\n  lufs_i: -9.8\n");
    }

    #[test]
    fn secuencias_de_aplicaciones_se_releen_igual() {
        let fuente = "custody:\n  history:\n    - seq: 1\n      action: exported\n      ts: \"2026-08-06T17:20:00-05:00\"\n    - seq: 2\n      action: sent\n      ts: \"2026-08-06T17:21:00-05:00\"\n";
        let doc = parse(fuente).unwrap();
        assert_eq!(emit(&doc), fuente);
        let hist = doc.at("custody.history").unwrap().as_seq().unwrap();
        assert_eq!(hist.len(), 2);
        assert_eq!(
            hist[1].as_map().unwrap().get("action").unwrap().as_str(),
            Some("sent")
        );
    }
}
