//! Marcador de custodia `CUSTODY.lock` (Anexo B.4).
//!
//! Texto plano en la raíz del proyecto, legible sin ninguna herramienta. Existe
//! únicamente mientras el estado de custodia no sea propia ni reclamada. El
//! manifiesto prevalece sobre este archivo en caso de discrepancia.

use crate::custody::CustodyState;
use crate::error::{Error, Result};
use crate::fsx::atomic;
use std::path::Path;

/// Contenido del marcador.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustodyLock {
    pub state: CustodyState,
    /// Titular actual: organización y persona.
    pub holder: String,
    /// Parte que cedió la custodia.
    pub ceded_by: String,
    pub shipment_id: String,
    /// Instante de la cesión, conforme a RFC 3339.
    pub ceded_at: String,
    /// Fecha esperada de retorno, `AAAA-MM-DD`.
    pub expected_return: String,
    pub grace_days: i64,
}

impl CustodyLock {
    /// Serializa el marcador. El formato reproduce el del Anexo B.4.
    pub fn render(&self) -> String {
        format!(
            "STAVE CUSTODY LOCK\n\
             estado:            {}\n\
             titular:           {}\n\
             cedida-por:        {}\n\
             envio:             {}\n\
             cedida-el:         {}\n\
             retorno-esperado:  {}\n\
             plazo-de-gracia:   {} dias\n\
             \n\
             Este proyecto no debe modificarse hasta que la custodia retorne.\n\
             El manifiesto PROJECT.yaml prevalece sobre este archivo.\n",
            self.state.as_str(),
            self.holder,
            self.ceded_by,
            self.shipment_id,
            self.ceded_at,
            self.expected_return,
            self.grace_days,
        )
    }

    /// Interpreta un marcador escrito por cualquier implementación.
    pub fn parse(text: &str) -> Result<CustodyLock> {
        let mut fields = std::collections::HashMap::new();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim();
                let value = v.trim();
                if !value.is_empty() {
                    fields.insert(key.to_string(), value.to_string());
                }
            }
        }
        let get = |k: &str| fields.get(k).cloned().unwrap_or_default();
        let estado = get("estado");
        let state = CustodyState::parse(&estado).ok_or_else(|| {
            Error::input(format!(
                "El marcador de custodia declara el estado «{estado}», ajeno a la Tabla 15. El marcador no se ha interpretado. Los estados admisibles son propia, en_transito, cedida y reclamada."
            ))
        })?;
        Ok(CustodyLock {
            state,
            holder: get("titular"),
            ceded_by: get("cedida-por"),
            shipment_id: get("envio"),
            ceded_at: get("cedida-el"),
            expected_return: get("retorno-esperado"),
            grace_days: get("plazo-de-gracia")
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        })
    }

    pub fn load(path: &Path) -> Result<CustodyLock> {
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
        Self::parse(&text)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        atomic::write_str(path, &self.render())
    }
}

/// Escribe el marcador en la raíz del proyecto.
pub fn write_marker(project_root: &Path, lock: &CustodyLock) -> Result<()> {
    // La raíz suele estar ya bloqueada cuando se escribe el marcador: la cesión
    // bloquea y a continuación marca.
    crate::fsx::readonly::with_writable_dir(project_root, || {
        lock.write(&project_root.join(super::CUSTODY_LOCK_FILE))
    })
}

/// Suprime el marcador. Se invoca cuando el estado vuelve a ser propia o
/// reclamada (apartado 14.3.3, tercer guion).
pub fn remove_marker(project_root: &Path) -> Result<()> {
    let path = project_root.join(super::CUSTODY_LOCK_FILE);
    if !path.exists() {
        return Ok(());
    }
    // El marcador pudo quedar en solo lectura junto con el resto del proyecto,
    // y suprimirlo exige además escritura sobre el directorio que lo contiene.
    let _ = crate::fsx::readonly::set_file_readonly(&path, false);
    crate::fsx::readonly::with_writable_dir(project_root, || {
        std::fs::remove_file(&path).map_err(|e| Error::io(&path, e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marcador() -> CustodyLock {
        CustodyLock {
            state: CustodyState::Ceded,
            holder: "Estudio B / M. Rivas".into(),
            ceded_by: "Estudio A / J. Duarte".into(),
            shipment_id: "0007".into(),
            ceded_at: "2026-08-06T17:20:00-05:00".into(),
            expected_return: "2026-08-20".into(),
            grace_days: 15,
        }
    }

    #[test]
    fn reproduce_el_formato_del_anexo_b_4() {
        let texto = marcador().render();
        assert!(texto.starts_with("STAVE CUSTODY LOCK\n"));
        assert!(texto.contains("estado:            cedida"));
        assert!(texto.contains("titular:           Estudio B / M. Rivas"));
        assert!(texto.contains("plazo-de-gracia:   15 dias"));
        assert!(texto.contains("El manifiesto PROJECT.yaml prevalece sobre este archivo."));
    }

    #[test]
    fn la_lectura_recupera_lo_escrito() {
        let original = marcador();
        let leido = CustodyLock::parse(&original.render()).unwrap();
        assert_eq!(leido, original);
    }

    #[test]
    fn rechaza_un_estado_ajeno_a_la_tabla_15() {
        let e = CustodyLock::parse("STAVE CUSTODY LOCK\nestado: prestado\n").unwrap_err();
        assert!(e.to_string().contains("Tabla 15"));
    }

    #[test]
    fn el_marcador_se_escribe_y_se_suprime() {
        let dir = crate::pruebas::raiz_temporal().unwrap();
        write_marker(dir.path(), &marcador()).unwrap();
        let ruta = dir.path().join(super::super::CUSTODY_LOCK_FILE);
        assert!(ruta.exists());
        // Aunque quede bloqueado con el resto del proyecto, debe poder suprimirse
        // al retornar la custodia.
        crate::fsx::readonly::set_file_readonly(&ruta, true).unwrap();
        remove_marker(dir.path()).unwrap();
        assert!(!ruta.exists());
        // Suprimir dos veces no es un error.
        remove_marker(dir.path()).unwrap();
    }
}
