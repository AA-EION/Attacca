//! Puente de C sobre la capa de órdenes.
//!
//! La interfaz nativa de macOS está escrita en Swift y el núcleo en Rust. Entre
//! los dos hay una sola frontera, y es deliberadamente estrecha: una función
//! que recibe el nombre de una orden y sus argumentos en JSON, y devuelve el
//! resultado en JSON.
//!
//! Es la misma forma que `invoke` en la interfaz web. Las dos interfaces
//! llaman a las mismas órdenes con los mismos nombres y los mismos argumentos,
//! de modo que ninguna de las dos puede quedarse atrás sin que se note.
//!
//! # Contrato de memoria
//!
//! Toda cadena que este módulo devuelve se reserva en Rust y debe devolverse a
//! [`attacca_cadena_liberar`]. Liberarla con `free` corrompe el montículo.
//! Las cadenas que recibe se copian antes de volver: quien llama conserva la
//! propiedad de las suyas.
//!
//! # Forma del resultado
//!
//! Siempre un objeto con exactamente una de estas dos claves:
//!
//! ```json
//! { "ok": <valor de la orden> }
//! { "error": "<qué ocurrió, qué consecuencia tiene y qué se puede hacer>" }
//! ```
//!
//! Nunca se devuelve un puntero nulo salvo que la propia reserva de memoria
//! falle, caso en el que no hay nada que decir por ese canal.

use attacca_ordenes::despacho;
use attacca_ordenes::estado::Estado;
use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Sesión abierta contra un repositorio.
///
/// Opaca para quien llama: su contenido cambia entre versiones sin que la
/// frontera se mueva.
pub struct AttaccaSesion {
    estado: Estado,
}

/// Prepara el proceso. Debe llamarse una vez, antes de crear ningún hilo.
///
/// El desplazamiento de zona horaria no se puede obtener de forma fiable en un
/// proceso con varios hilos en algunos sistemas Unix, y de ese valor dependen
/// todas las marcas de tiempo del registro.
///
/// # Safety
///
/// Sin condiciones. Es segura de llamar más de una vez.
#[no_mangle]
pub extern "C" fn attacca_inicializar() {
    attacca_core::clock::init_local_offset();
}

/// Abre una sesión. El resultado se cierra con [`attacca_sesion_cerrar`].
#[no_mangle]
pub extern "C" fn attacca_sesion_nueva() -> *mut AttaccaSesion {
    Box::into_raw(Box::new(AttaccaSesion {
        estado: Estado::default(),
    }))
}

/// Cierra una sesión abierta con [`attacca_sesion_nueva`].
///
/// # Safety
///
/// `sesion` debe proceder de [`attacca_sesion_nueva`] y no haberse cerrado
/// antes. Un puntero nulo se ignora.
#[no_mangle]
pub unsafe extern "C" fn attacca_sesion_cerrar(sesion: *mut AttaccaSesion) {
    if !sesion.is_null() {
        drop(Box::from_raw(sesion));
    }
}

/// Ejecuta una orden sobre la sesión.
///
/// `argumentos_json` es un objeto JSON; nulo o vacío equivale a `{}`.
/// El resultado es una cadena JSON que debe liberarse con
/// [`attacca_cadena_liberar`].
///
/// # Safety
///
/// `sesion` debe ser una sesión viva. `orden` y `argumentos_json` deben ser
/// cadenas terminadas en cero, o nulas.
#[no_mangle]
pub unsafe extern "C" fn attacca_invocar(
    sesion: *mut AttaccaSesion,
    orden: *const c_char,
    argumentos_json: *const c_char,
) -> *mut c_char {
    let Some(sesion) = sesion.as_ref() else {
        return devolver(&fallo(
            "La sesión no está abierta. La orden no se ha ejecutado. \
             Abrir una sesión con attacca_sesion_nueva antes de invocar.",
        ));
    };

    let Some(nombre) = leer(orden) else {
        return devolver(&fallo(
            "El nombre de la orden no es texto válido. No se ha hecho nada. \
             Enviar el nombre como cadena UTF-8 terminada en cero.",
        ));
    };

    let args: Value = match leer(argumentos_json) {
        None | Some("") => json!({}),
        Some(bruto) => match serde_json::from_str(bruto) {
            Ok(v @ Value::Object(_)) => v,
            Ok(_) => {
                return devolver(&fallo(
                    "Los argumentos no son un objeto JSON. La orden no se ha ejecutado. \
                     Enviar un objeto con un campo por argumento.",
                ))
            }
            Err(e) => {
                return devolver(&fallo(&format!(
                    "Los argumentos no son JSON válido: {e}. La orden no se ha ejecutado. \
                     Revisar la construcción del objeto en quien llama."
                )))
            }
        },
    };

    // Un pánico que cruzase la frontera de C sería comportamiento indefinido.
    // En compilación de producción el perfil aborta y esto no llega a actuar;
    // en depuración convierte el pánico en un error con mensaje.
    let salida = catch_unwind(AssertUnwindSafe(|| {
        despacho::invocar(&sesion.estado, nombre, &args)
    }));

    let valor = match salida {
        Ok(Ok(v)) => json!({ "ok": v }),
        Ok(Err(e)) => json!({ "error": e }),
        Err(_) => fallo(
            "La orden terminó de forma anómala. El resultado no es utilizable. \
             Conservar el registro del proceso y comunicar en qué orden ocurrió.",
        ),
    };
    devolver(&valor)
}

/// Nombres de las órdenes que esta biblioteca despacha, como array JSON.
///
/// Permite a la interfaz nativa comprobar al arrancar que la biblioteca con la
/// que se enlazó ofrece lo que espera. El resultado se libera con
/// [`attacca_cadena_liberar`].
#[no_mangle]
pub extern "C" fn attacca_ordenes() -> *mut c_char {
    devolver(&json!({ "ok": despacho::ORDENES }))
}

/// Versión de Attacca y versión de la norma que aplica, como objeto JSON.
///
/// El resultado se libera con [`attacca_cadena_liberar`].
#[no_mangle]
pub extern "C" fn attacca_version() -> *mut c_char {
    devolver(&json!({
        "ok": {
            "attacca": env!("CARGO_PKG_VERSION"),
            "stave": attacca_core::STAVE_VERSION,
        }
    }))
}

/// Libera una cadena devuelta por cualquier función de este módulo.
///
/// # Safety
///
/// `cadena` debe proceder de este módulo y no haberse liberado antes. Un
/// puntero nulo se ignora.
#[no_mangle]
pub unsafe extern "C" fn attacca_cadena_liberar(cadena: *mut c_char) {
    if !cadena.is_null() {
        drop(CString::from_raw(cadena));
    }
}

// --- Auxiliares ---

fn fallo(mensaje: &str) -> Value {
    json!({ "error": mensaje })
}

/// Reserva la cadena de salida.
///
/// Un valor cuyo JSON contuviera un cero interno no podría viajar por una
/// cadena de C. No puede ocurrir —`serde_json` escapa el cero— pero si
/// ocurriese, se prefiere un error legible a un puntero nulo.
fn devolver(valor: &Value) -> *mut c_char {
    let texto = valor.to_string();
    match CString::new(texto) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::new(
            r#"{"error":"El resultado contiene un carácter que no puede cruzar la frontera de C. No se ha devuelto nada. Comunicar en qué orden ocurrió."}"#,
        )
        .expect("la cadena literal no contiene ningún cero")
        .into_raw(),
    }
}

/// Lee una cadena de C sin tomar su propiedad.
unsafe fn leer<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    /// Invoca como lo haría Swift y devuelve el JSON ya interpretado.
    fn invocar(sesion: *mut AttaccaSesion, orden: &str, args: &str) -> Value {
        let o = CString::new(orden).unwrap();
        let a = CString::new(args).unwrap();
        unsafe {
            let p = attacca_invocar(sesion, o.as_ptr(), a.as_ptr());
            assert!(!p.is_null());
            let v: Value = serde_json::from_str(CStr::from_ptr(p).to_str().unwrap()).unwrap();
            attacca_cadena_liberar(p);
            v
        }
    }

    #[test]
    fn el_ciclo_de_una_sesion_no_pierde_memoria_observable() {
        let s = attacca_sesion_nueva();
        assert!(!s.is_null());
        unsafe { attacca_sesion_cerrar(s) };
        // Cerrar un puntero nulo no debe hacer nada.
        unsafe { attacca_sesion_cerrar(std::ptr::null_mut()) };
    }

    #[test]
    fn una_orden_sin_repositorio_devuelve_error_y_no_ok() {
        let s = attacca_sesion_nueva();
        let v = invocar(s, "catalogo", "{}");
        assert_eq!(v["error"], "sin_repositorio");
        assert!(v.get("ok").is_none());
        unsafe { attacca_sesion_cerrar(s) };
    }

    #[test]
    fn una_orden_que_no_necesita_repositorio_responde_ok() {
        let s = attacca_sesion_nueva();
        let v = invocar(s, "declaracion_conformidad", "{}");
        assert!(v["ok"].as_str().unwrap().contains("STAVE"));
        unsafe { attacca_sesion_cerrar(s) };
    }

    #[test]
    fn los_argumentos_vacios_equivalen_al_objeto_vacio() {
        let s = attacca_sesion_nueva();
        for args in ["", "{}"] {
            let v = invocar(s, "raiz_sugerida", args);
            assert!(v.get("ok").is_some(), "con argumentos «{args}»: {v}");
        }
        unsafe { attacca_sesion_cerrar(s) };
    }

    #[test]
    fn un_argumento_que_no_es_objeto_se_rechaza_con_mensaje() {
        let s = attacca_sesion_nueva();
        let v = invocar(s, "catalogo", "[1,2]");
        assert!(v["error"].as_str().unwrap().contains("objeto JSON"));
        unsafe { attacca_sesion_cerrar(s) };
    }

    #[test]
    fn la_sesion_nula_se_rechaza_en_lugar_de_desreferenciarse() {
        let v = invocar(std::ptr::null_mut(), "catalogo", "{}");
        assert!(v["error"]
            .as_str()
            .unwrap()
            .contains("sesión no está abierta"));
    }

    #[test]
    fn la_lista_de_ordenes_coincide_con_la_del_despacho() {
        unsafe {
            let p = attacca_ordenes();
            let v: Value = serde_json::from_str(CStr::from_ptr(p).to_str().unwrap()).unwrap();
            attacca_cadena_liberar(p);
            let nombres: Vec<&str> = v["ok"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_str().unwrap())
                .collect();
            assert_eq!(nombres, despacho::ORDENES);
        }
    }

    #[test]
    fn la_version_declara_la_norma_que_aplica() {
        unsafe {
            let p = attacca_version();
            let v: Value = serde_json::from_str(CStr::from_ptr(p).to_str().unwrap()).unwrap();
            attacca_cadena_liberar(p);
            assert_eq!(v["ok"]["stave"], attacca_core::STAVE_VERSION);
        }
    }
}

#[cfg(test)]
mod encabezado {
    /// El encabezado de C y las funciones exportadas deben declarar lo mismo.
    ///
    /// Es una comprobación de texto, no de enlazado: basta para detectar la
    /// omisión que de verdad ocurre, que es añadir una función al puente y
    /// olvidar el encabezado que la interfaz nativa incluye.
    #[test]
    fn declara_las_mismas_funciones_que_el_puente() {
        let fuente = include_str!("lib.rs");
        let h = include_str!("../include/attacca.h");

        let mut exportadas: Vec<&str> = fuente
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.strip_prefix("unsafe ").or(Some(l)))
            .filter_map(|l| l.strip_prefix("extern \"C\" fn "))
            .filter_map(|l| l.split('(').next())
            .collect();
        exportadas.sort_unstable();
        assert!(
            !exportadas.is_empty(),
            "no se detectó ninguna función exportada"
        );

        for nombre in &exportadas {
            assert!(
                h.contains(&format!("{nombre}(")),
                "el encabezado no declara «{nombre}»"
            );
        }

        // Y a la inversa: una función que el encabezado prometa y el puente no
        // exporte deja la aplicación nativa sin enlazar.
        for linea in h.lines() {
            let Some(inicio) = linea.find("attacca_") else {
                continue;
            };
            if !linea.contains('(') || linea.trim_start().starts_with('*') {
                continue;
            }
            let resto = &linea[inicio..];
            let Some(nombre) = resto.split('(').next() else {
                continue;
            };
            assert!(
                exportadas.contains(&nombre),
                "el encabezado declara «{nombre}» y el puente no lo exporta"
            );
        }
    }
}
