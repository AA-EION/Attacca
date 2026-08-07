//! Documento estructurado con orden estable.
//!
//! Los manifiestos de STAVE (Anexo B) se manipulan como un árbol de nodos cuyo
//! orden de claves se conserva. Esta representación satisface dos requisitos que
//! no pueden obtenerse de una deserialización a estructuras fijas:
//!
//! - apartado 44.1: los campos no comprendidos se preservan al reescribir, en
//!   cualquier nivel de anidamiento y sin necesidad de declararlos;
//! - escritura determinista: el emisor ordena las claves conocidas conforme a un
//!   orden canónico y conserva las desconocidas en su orden relativo original,
//!   de modo que dos escrituras del mismo estado producen bytes idénticos.

mod emit;
mod order;
mod parse;

pub use emit::emit;
pub use order::{canonical_order, ArtifactKind};
pub use parse::{parse, ParseError};

use std::fmt;

/// Nodo de un documento estructurado.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// Nulo explícito. El apartado 13.2 exige que los campos pendientes se
    /// registren con nulo explícito y no se omitan.
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<Node>),
    Map(Map),
}

/// Aplicación clave-valor que conserva el orden de inserción.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Map {
    entries: Vec<(String, Node)>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Node)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Node> {
        self.entries
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Inserta o sustituye conservando la posición original de la clave.
    pub fn set(&mut self, key: &str, value: Node) {
        match self.entries.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = value,
            None => self.entries.push((key.to_string(), value)),
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Node> {
        let idx = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(idx).1)
    }

    /// Devuelve el nodo situado en una ruta separada por puntos.
    pub fn at(&self, path: &str) -> Option<&Node> {
        let mut cur = self.get(path.split('.').next()?)?;
        for seg in path.split('.').skip(1) {
            cur = cur.as_map()?.get(seg)?;
        }
        Some(cur)
    }

    /// Asegura que exista una aplicación en la clave indicada y la devuelve.
    pub fn ensure_map(&mut self, key: &str) -> &mut Map {
        if !matches!(self.get(key), Some(Node::Map(_))) {
            self.set(key, Node::Map(Map::new()));
        }
        match self.get_mut(key) {
            Some(Node::Map(m)) => m,
            _ => unreachable!("ensure_map acaba de establecer una aplicación"),
        }
    }

    /// Asegura que exista una secuencia en la clave indicada y la devuelve.
    pub fn ensure_seq(&mut self, key: &str) -> &mut Vec<Node> {
        if !matches!(self.get(key), Some(Node::Seq(_))) {
            self.set(key, Node::Seq(Vec::new()));
        }
        match self.get_mut(key) {
            Some(Node::Seq(s)) => s,
            _ => unreachable!("ensure_seq acaba de establecer una secuencia"),
        }
    }

    /// Reordena las claves conforme a un orden canónico. Las claves ausentes del
    /// orden canónico —las extensiones y los campos de versiones posteriores de
    /// la norma— se conservan al final, en su orden relativo original.
    pub fn reorder(&mut self, order: &[&str]) {
        let mut rest = std::mem::take(&mut self.entries);
        let mut out: Vec<(String, Node)> = Vec::with_capacity(rest.len());
        for key in order {
            if let Some(pos) = rest.iter().position(|(k, _)| k == key) {
                out.push(rest.remove(pos));
            }
        }
        out.extend(rest);
        self.entries = out;
    }
}

impl FromIterator<(String, Node)> for Map {
    fn from_iter<T: IntoIterator<Item = (String, Node)>>(iter: T) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

impl Node {
    pub fn str(s: impl Into<String>) -> Node {
        Node::Str(s.into())
    }

    /// Cadena o nulo explícito, según la opción.
    pub fn opt_str(s: Option<impl Into<String>>) -> Node {
        match s {
            Some(v) => Node::Str(v.into()),
            None => Node::Null,
        }
    }

    pub fn opt_int(v: Option<i64>) -> Node {
        match v {
            Some(v) => Node::Int(v),
            None => Node::Null,
        }
    }

    pub fn map(entries: Vec<(&str, Node)>) -> Node {
        Node::Map(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Node::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        match self {
            Node::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Node]> {
        match self {
            Node::Seq(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Node::Int(i) => Some(*i),
            Node::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Node::Float(f) => Some(*f),
            Node::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Node::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Node::Null)
    }

    /// Cadena presente y no vacía. Un campo pendiente conforme al apartado 13.2
    /// se registra como nulo y no como cadena vacía.
    pub fn present_str(&self) -> Option<&str> {
        self.as_str().filter(|s| !s.is_empty())
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Null => f.write_str(""),
            Node::Bool(b) => write!(f, "{b}"),
            Node::Int(i) => write!(f, "{i}"),
            Node::Float(v) => write!(f, "{v}"),
            Node::Str(s) => f.write_str(s),
            Node::Seq(_) | Node::Map(_) => f.write_str("<estructura>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_conserva_claves_desconocidas_al_final() {
        let mut m = Map::new();
        m.set("z_ext", Node::str("uno"));
        m.set("id", Node::str("dos"));
        m.set("a_ext", Node::str("tres"));
        m.set("uid", Node::str("cuatro"));
        m.reorder(&["uid", "id"]);
        let keys: Vec<&str> = m.keys().collect();
        assert_eq!(keys, vec!["uid", "id", "z_ext", "a_ext"]);
    }

    #[test]
    fn set_conserva_la_posicion_original() {
        let mut m = Map::new();
        m.set("a", Node::Int(1));
        m.set("b", Node::Int(2));
        m.set("a", Node::Int(9));
        let keys: Vec<&str> = m.keys().collect();
        assert_eq!(keys, vec!["a", "b"]);
        assert_eq!(m.get("a").unwrap().as_int(), Some(9));
    }

    #[test]
    fn at_recorre_rutas_anidadas() {
        let mut root = Map::new();
        root.set("custody", Node::map(vec![("state", Node::str("propia"))]));
        assert_eq!(root.at("custody.state").unwrap().as_str(), Some("propia"));
        assert!(root.at("custody.holder").is_none());
    }
}
