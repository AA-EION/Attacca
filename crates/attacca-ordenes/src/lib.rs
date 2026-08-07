//! Capa de órdenes de Attacca, sin plataforma gráfica.
//!
//! Aquí vive todo lo que una interfaz necesita para operar sobre un
//! repositorio: el estado de la sesión, la proyección de los manifiestos hacia
//! estructuras presentables y las órdenes que modifican el árbol.
//!
//! Ninguna de las tres interfaces que existen —la de Tauri, la nativa de macOS
//! y la línea de órdenes— toma decisiones normativas. Las tres consultan estas
//! funciones, y estas funciones consultan al núcleo. Que la capa sea una sola
//! es lo que permite que una interfaz nueva no reabra ninguna cuestión ya
//! resuelta.
//!
//! El acceso desde Swift se hace a través de [`despacho`], que expone la misma
//! superficie por nombre y con argumentos en JSON. Es la misma forma que emplea
//! `invoke` en la interfaz web, de modo que las dos interfaces llaman a lo
//! mismo con los mismos nombres.

pub mod despacho;
pub mod estado;
pub mod ordenes;

pub use estado::{
    Conformidad, Entrada, Estado, Hallazgo, Identidad, ProyectoResumen, VistaProyecto,
};
pub use ordenes::*;
