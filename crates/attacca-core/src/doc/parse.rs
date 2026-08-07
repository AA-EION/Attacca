//! Lectura de documentos YAML 1.2 al árbol de nodos con orden estable.

use super::{Map, Node};
use std::fmt;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

/// Convierte texto YAML en un árbol de nodos. Un documento vacío produce una
/// aplicación vacía, no un error: un manifiesto recién creado puede no tener
/// todavía ningún campo escrito.
pub fn parse(text: &str) -> Result<Map, ParseError> {
    let docs = YamlLoader::load_from_str(text).map_err(|e| ParseError(e.to_string()))?;
    let Some(first) = docs.into_iter().next() else {
        return Ok(Map::new());
    };
    match convert(first) {
        Node::Map(m) => Ok(m),
        Node::Null => Ok(Map::new()),
        _ => Err(ParseError(
            "el documento no es una aplicación de claves en su nivel superior".into(),
        )),
    }
}

fn convert(y: Yaml) -> Node {
    match y {
        Yaml::Real(s) => s.parse::<f64>().map(Node::Float).unwrap_or(Node::Str(s)),
        Yaml::Integer(i) => Node::Int(i),
        Yaml::String(s) => Node::Str(s),
        Yaml::Boolean(b) => Node::Bool(b),
        Yaml::Array(items) => Node::Seq(items.into_iter().map(convert).collect()),
        Yaml::Hash(h) => Node::Map(
            h.into_iter()
                .map(|(k, v)| (scalar_key(k), convert(v)))
                .collect(),
        ),
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Node::Null,
    }
}

fn scalar_key(k: Yaml) -> String {
    match k {
        Yaml::String(s) => s,
        Yaml::Integer(i) => i.to_string(),
        Yaml::Boolean(b) => b.to_string(),
        Yaml::Real(s) => s,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conserva_el_orden_de_las_claves() {
        let m = parse("zulu: 1\nalfa: 2\nmike: 3\n").unwrap();
        let keys: Vec<&str> = m.keys().collect();
        assert_eq!(keys, vec!["zulu", "alfa", "mike"]);
    }

    #[test]
    fn distingue_nulo_explicito_de_cadena_vacia() {
        let m = parse("a: null\nb: \"\"\n").unwrap();
        assert!(m.get("a").unwrap().is_null());
        assert_eq!(m.get("b").unwrap().as_str(), Some(""));
    }

    #[test]
    fn documento_vacio_produce_aplicacion_vacia() {
        assert!(parse("").unwrap().is_empty());
    }
}
