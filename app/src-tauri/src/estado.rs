//! Estado de la aplicación y proyección de los datos hacia la interfaz.
//!
//! Nada de lo que hay aquí es autoritativo. Todo procede de los manifiestos y
//! del árbol de archivos, y se recalcula al recargar. El apartado 42 lo exige:
//! ninguna información normativa debe existir únicamente en la interfaz.

use attacca_core::clock::SyncState;
use attacca_core::index::Index;
use attacca_core::manifest::project::ProjectManifest;
use attacca_core::repo::Repository;
use attacca_core::stage::Stage;
use attacca_core::validate::{self, Severity};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Identidad de quien opera. Se registra como actor de los eventos.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identidad {
    pub organizacion: String,
    pub persona: String,
}

impl Default for Identidad {
    fn default() -> Self {
        Self {
            organizacion: "Organizacion".into(),
            persona: "Persona".into(),
        }
    }
}

/// Estado compartido entre las órdenes de la interfaz.
#[derive(Default)]
pub struct Estado {
    interno: Mutex<Interno>,
}

#[derive(Default)]
struct Interno {
    repositorio: Option<Repository>,
    identidad: Identidad,
    reloj: Option<SyncState>,
    /// Cancelación de la operación larga en curso.
    cancelar: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Estado {
    pub fn repositorio(&self) -> Result<Repository, String> {
        self.interno
            .lock()
            .unwrap()
            .repositorio
            .clone()
            .ok_or_else(|| "sin_repositorio".to_string())
    }

    pub fn abrir(&self, raiz: PathBuf) -> Result<(), String> {
        let repo = Repository::open(&raiz).map_err(|e| e.to_string())?;
        self.interno.lock().unwrap().repositorio = Some(repo);
        Ok(())
    }

    pub fn crear(&self, raiz: PathBuf) -> Result<(), String> {
        let repo = Repository::create(&raiz).map_err(|e| e.to_string())?;
        self.interno.lock().unwrap().repositorio = Some(repo);
        Ok(())
    }

    pub fn identidad(&self) -> Identidad {
        self.interno.lock().unwrap().identidad.clone()
    }

    pub fn set_identidad(&self, i: Identidad) {
        self.interno.lock().unwrap().identidad = i;
    }

    pub fn actor(&self) -> String {
        self.interno.lock().unwrap().identidad.persona.clone()
    }

    pub fn reloj(&self) -> Option<SyncState> {
        self.interno.lock().unwrap().reloj
    }

    pub fn set_reloj(&self, s: SyncState) {
        self.interno.lock().unwrap().reloj = Some(s);
    }

    pub fn bandera_cancelacion(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.interno.lock().unwrap().cancelar.clone()
    }

    pub fn reiniciar_cancelacion(&self) {
        self.interno
            .lock()
            .unwrap()
            .cancelar
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn cancelar(&self) {
        self.interno
            .lock()
            .unwrap()
            .cancelar
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Resumen de un proyecto para la lista.
#[derive(Clone, Debug, Serialize)]
pub struct ProyectoResumen {
    pub uid: String,
    pub id: String,
    pub titulo: String,
    pub artista: String,
    pub tipo: String,
    pub nivel: String,
    pub estado: String,
    pub custodia: String,
    pub etapa: String,
    pub release_uid: Option<String>,
    pub ruta: String,
}

/// Estado de conformidad, con las tres categorías separadas.
///
/// El requisito 8 de la interfaz visual lo exige: un campo pendiente cuya etapa
/// no ha cerrado, un incumplimiento real y una excepción declarada son tres
/// cosas distintas y no deben confundirse.
#[derive(Clone, Debug, Serialize)]
pub struct Conformidad {
    pub pendientes: usize,
    pub incumplimientos: usize,
    pub excepciones: usize,
    pub conforme: bool,
    pub hallazgos: Vec<Hallazgo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Hallazgo {
    pub severidad: String,
    pub clausula: String,
    pub campo: String,
    pub detalle: String,
}

/// Un archivo o carpeta del navegador propio.
#[derive(Clone, Debug, Serialize)]
pub struct Entrada {
    pub nombre: String,
    pub es_carpeta: bool,
    pub tamano: u64,
    pub ruta: String,
}

/// Vista completa de un proyecto: lo que la interfaz necesita para presentarlo
/// sin volver a consultar nada.
#[derive(Clone, Debug, Serialize)]
pub struct VistaProyecto {
    pub resumen: ProyectoResumen,
    /// Clave de la etapa activa, deducida del manifiesto y del contenido.
    pub etapa: String,
    /// Carpetas que la Tabla I.1 indica presentar en primer plano.
    pub carpetas_etapa: Vec<String>,
    /// Claves de las acciones admisibles en la etapa.
    pub acciones: Vec<String>,
    /// Clave de la condición de paso a la etapa siguiente.
    pub condicion_siguiente: String,
    /// Carpetas del proyecto que existen, para el recorrido fuera del principal.
    pub carpetas_existentes: Vec<String>,
    pub conformidad: Conformidad,
    pub replicas: Vec<ReplicaVista>,
    pub replica_activa: Option<String>,
    /// Vencimiento de la cesión, cuando la custodia está cedida.
    pub vencimiento: Option<Vencimiento>,
    pub presupuesto_ruta: PresupuestoVista,
    pub audio: AudioVista,
    /// El tempo, la tonalidad y la afinación siguen siendo modificables.
    pub audio_modificable: bool,
    pub ruta_absoluta: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReplicaVista {
    pub volumen: String,
    pub ruta: String,
    pub estado: String,
    pub ultima_sync: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Vencimiento {
    /// `current`, `claim_due` o `reclaim_available`.
    pub situacion: String,
    pub fecha: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PresupuestoVista {
    pub actual: usize,
    pub limite: usize,
    pub excedido: bool,
    pub ruta: String,
    pub replica: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AudioVista {
    pub frecuencia: Option<i64>,
    pub bits: Option<i64>,
    pub afinacion: Option<i64>,
    pub tempo: Option<i64>,
    pub tonalidad: Option<String>,
    pub origen: Option<String>,
}

/// Construye el resumen de un proyecto a partir de una entrada del índice.
pub fn resumen_de(entry: &attacca_core::index::ProjectEntry) -> ProyectoResumen {
    ProyectoResumen {
        uid: entry.uid.clone(),
        id: entry.id.clone(),
        titulo: entry.title.clone(),
        artista: entry.artist.clone(),
        tipo: entry.kind.clone(),
        nivel: entry.level.clone(),
        estado: entry.status.clone(),
        custodia: entry.custody.clone(),
        etapa: entry.stage.clone(),
        release_uid: entry.release_uid.clone(),
        ruta: entry
            .path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

/// Construye la vista completa de un proyecto.
pub fn vista_de(m: &ProjectManifest) -> VistaProyecto {
    let raiz = m.root();
    // La etapa se deduce del manifiesto y del contenido real, nunca de una
    // preferencia guardada (apartado 44.2, primer guion).
    let etapa = attacca_core::stage::active_stage_of(m.doc(), raiz);
    let informe = validate::project(m);

    let conformidad = Conformidad {
        pendientes: informe.of(Severity::Pending).len(),
        incumplimientos: informe.of(Severity::Breach).len(),
        excepciones: informe.of(Severity::Exception).len(),
        conforme: informe.is_conformant(),
        hallazgos: informe
            .findings
            .iter()
            .map(|f| Hallazgo {
                severidad: f.severity.key().to_string(),
                clausula: f.clause.clone(),
                campo: f.subject.clone(),
                detalle: f.detail.clone(),
            })
            .collect(),
    };

    let presupuesto = attacca_core::project::path_budget(m);
    let vencimiento = validate::custody_expiry(m).map(|(e, fecha)| Vencimiento {
        situacion: match e {
            attacca_core::custody::Expiry::Current => "current",
            attacca_core::custody::Expiry::ClaimDue => "claim_due",
            attacca_core::custody::Expiry::ReclaimAvailable => "reclaim_available",
        }
        .to_string(),
        fecha,
    });

    VistaProyecto {
        resumen: ProyectoResumen {
            uid: m.uid().unwrap_or_default().to_string(),
            id: m.id().unwrap_or_default().to_string(),
            titulo: m.title().unwrap_or_default().to_string(),
            artista: m.artist().unwrap_or_default().to_string(),
            tipo: m.project_type().unwrap_or_default().to_string(),
            nivel: m.level().as_str().to_string(),
            estado: m.status().as_str().to_string(),
            custodia: m.custody_state().as_str().to_string(),
            etapa: etapa.key().to_string(),
            release_uid: m.release_uid().map(str::to_string),
            ruta: raiz.display().to_string(),
        },
        etapa: etapa.key().to_string(),
        carpetas_etapa: etapa
            .folders()
            .iter()
            .filter(|f| !f.is_empty())
            .map(|f| (*f).to_string())
            .collect(),
        acciones: acciones_admisibles(m, etapa),
        condicion_siguiente: etapa.advance_condition_key().to_string(),
        carpetas_existentes: attacca_core::project::existing_folders(raiz),
        conformidad,
        replicas: attacca_core::replica::replicas_of(m.doc())
            .into_iter()
            .map(|r| ReplicaVista {
                volumen: r.volume,
                ruta: r.path,
                estado: r.state.as_str().to_string(),
                ultima_sync: r.last_synced,
            })
            .collect(),
        replica_activa: m.active_volume().map(str::to_string),
        vencimiento,
        presupuesto_ruta: PresupuestoVista {
            actual: presupuesto.longest,
            limite: presupuesto.limit,
            excedido: presupuesto.exceeded(),
            ruta: presupuesto.longest_path,
            replica: presupuesto.replica,
        },
        audio: AudioVista {
            frecuencia: m.sample_rate(),
            bits: m.bit_depth(),
            afinacion: m.doc().at("audio.tuning_hz").and_then(|n| n.as_int()),
            tempo: m.doc().at("audio.tempo").and_then(|n| n.as_int()),
            tonalidad: m
                .doc()
                .at("audio.key")
                .and_then(|n| n.present_str())
                .map(str::to_string),
            origen: m
                .doc()
                .at("audio.origin")
                .and_then(|n| n.present_str())
                .map(str::to_string),
        },
        // El tempo, la tonalidad y la afinación se pueden modificar mientras no
        // concluya la grabación (apartado 13.2, penúltimo párrafo).
        audio_modificable: !m.audio_params_locked() && m.custody_state().allows_write(),
        ruta_absoluta: raiz.display().to_string(),
    }
}

/// Acciones que la etapa admite, filtradas por el estado real del proyecto.
///
/// El estado de custodia manda: un proyecto cedido no admite modificación
/// (apartado 44.2, penúltimo guion).
fn acciones_admisibles(m: &ProjectManifest, etapa: Stage) -> Vec<String> {
    let custodia = m.custody_state();
    if !custodia.allows_write() {
        // Solo quedan las acciones que el apartado 14.3.3 permite sobre un
        // proyecto cedido, y las del ciclo de custodia del apartado 41.
        let mut acciones = vec![
            "abrir_en_explorador".to_string(),
            "verificar_integridad".to_string(),
            "derivar_sobre_cedido".to_string(),
        ];
        if let Some((situacion, _)) = validate::custody_expiry(m) {
            match situacion {
                attacca_core::custody::Expiry::ClaimDue => {
                    acciones.push("reclamar_retorno".to_string())
                }
                attacca_core::custody::Expiry::ReclaimAvailable => {
                    acciones.push("reclamar_retorno".to_string());
                    acciones.push("recuperar_custodia".to_string());
                }
                attacca_core::custody::Expiry::Current => {}
            }
        }
        return acciones;
    }

    if m.status().is_frozen() {
        // Un proyecto sellado o archivado se consulta, se reproduce y se copia.
        return vec![
            "abrir_en_explorador".to_string(),
            "verificar_integridad".to_string(),
        ];
    }

    let mut acciones: Vec<String> = etapa.actions().iter().map(|a| a.to_string()).collect();
    // Acciones disponibles en cualquier etapa.
    acciones.push("nueva_version".to_string());
    acciones.push("incorporar_material".to_string());
    acciones.push("abrir_en_explorador".to_string());
    acciones
}

/// Carga el índice, reconstruyéndolo si la caché no es utilizable.
pub fn indice(repo: &Repository) -> Result<Index, String> {
    Index::load_or_rebuild(repo, repo.root())
        .map(|(i, _)| i)
        .map_err(|e| e.to_string())
}
