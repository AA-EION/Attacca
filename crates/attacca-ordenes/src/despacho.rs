//! Despacho de órdenes por nombre, con argumentos y resultado en JSON.
//!
//! La interfaz web llama a las órdenes por su nombre a través de `invoke`. Para
//! que una interfaz nativa no tenga que reimplementar esa correspondencia, aquí
//! se expone la misma superficie en una sola función: nombre de la orden,
//! objeto de argumentos, valor de retorno.
//!
//! El contrato es deliberadamente estrecho. Una orden nueva se añade en un solo
//! sitio y las tres interfaces la reciben a la vez; una orden que se olvide de
//! añadir aquí falla con un mensaje que la nombra, no con un silencio.

use crate::estado::Estado;
use crate::ordenes;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// Lee un argumento del objeto recibido.
///
/// Un argumento ausente equivale a nulo, que es lo que espera un parámetro
/// opcional. Uno presente pero de otro tipo es un error con nombre propio: el
/// mensaje dice qué argumento y qué se esperaba.
fn arg<T: DeserializeOwned>(args: &Value, clave: &str) -> Result<T, String> {
    let bruto = args.get(clave).cloned().unwrap_or(Value::Null);
    serde_json::from_value(bruto)
        .map_err(|e| format!("El argumento «{clave}» no es del tipo esperado: {e}."))
}

fn a_json<T: Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| format!("El resultado no se pudo serializar: {e}."))
}

macro_rules! despachar {
    (
        $orden:expr, $args:expr, $estado:expr;
        // Órdenes que devuelven `Result` y reciben el estado de la sesión.
        resultado { $( $rn:ident ( $($ra:ident),* ) ),* $(,)? }
        // Órdenes que devuelven un valor y reciben el estado de la sesión.
        directa { $( $dn:ident ( $($da:ident),* ) ),* $(,)? }
        // Órdenes que no consultan el estado de la sesión.
        libre { $( $ln:ident ( $($la:ident),* ) ),* $(,)? }
    ) => {
        match $orden {
            $(
                stringify!($rn) => ordenes::$rn(
                    $estado $(, arg($args, stringify!($ra))?)*
                ).and_then(a_json),
            )*
            $(
                stringify!($dn) => a_json(ordenes::$dn(
                    $estado $(, arg($args, stringify!($da))?)*
                )),
            )*
            $(
                stringify!($ln) => a_json(ordenes::$ln(
                    $(arg($args, stringify!($la))?),*
                )),
            )*
            otra => Err(format!(
                "La orden «{otra}» no existe. No se ha hecho nada. \
                 Comprobar el nombre contra la lista de órdenes de la versión en uso."
            )),
        }
    };
}

/// Ejecuta una orden por su nombre.
pub fn invocar(estado: &Estado, orden: &str, args: &Value) -> Result<Value, String> {
    despachar! {
        orden, args, estado;

        resultado {
            abrir_repositorio(raiz),
            crear_repositorio(raiz),
            limpiar_temporales(),
            catalogo(),
            reconstruir_indice(),
            crear_proyecto(datos),
            ver_proyecto(uid),
            cambiar_titulo(uid, titulo),
            cambiar_estado(uid, nuevo),
            registrar_audio(uid, datos),
            derivar_proyecto(uid, motivo, paralelo),
            crear_sesion(uid, daw, etapa),
            ruta_exportacion(uid, carpeta, nombre, extension),
            incorporar_material(uid, datos),
            listar_carpeta(uid, relativa),
            abrir_en_explorador(uid, relativa),
            validar(uid),
            verificar_integridad(uid),
            generar_integridad(uid),
            crear_release(datos),
            vincular_a_release(release_uid, proyecto_uid, posicion),
            release_listo(release_uid),
            emitir_envio(uid, datos),
            verificar_paquete(datos),
            ingerir_paquete(datos),
            paquetes_en_cuarentena(),
            reclamar_retorno(uid),
            recuperar_custodia(uid),
            derivar_sobre_cedido(uid),
            registro(ultimas),
            verificar_registro(),
        }

        directa {
            identidad(),
            set_identidad(identidad),
            comprobar_reloj(),
            reloj_conocido(),
            cancelar_operacion(),
        }

        libre {
            raiz_sugerida(),
            declaracion_conformidad(),
        }
    }
}

/// Nombres de todas las órdenes, en el orden en que se despachan.
///
/// La interfaz nativa la emplea para comprobar al arrancar que la biblioteca
/// con la que se enlazó ofrece lo que espera, en lugar de descubrirlo al pulsar
/// un botón.
pub const ORDENES: &[&str] = &[
    "abrir_repositorio",
    "crear_repositorio",
    "limpiar_temporales",
    "catalogo",
    "reconstruir_indice",
    "crear_proyecto",
    "ver_proyecto",
    "cambiar_titulo",
    "cambiar_estado",
    "registrar_audio",
    "derivar_proyecto",
    "crear_sesion",
    "ruta_exportacion",
    "incorporar_material",
    "listar_carpeta",
    "abrir_en_explorador",
    "validar",
    "verificar_integridad",
    "generar_integridad",
    "crear_release",
    "vincular_a_release",
    "release_listo",
    "emitir_envio",
    "verificar_paquete",
    "ingerir_paquete",
    "paquetes_en_cuarentena",
    "reclamar_retorno",
    "recuperar_custodia",
    "derivar_sobre_cedido",
    "registro",
    "verificar_registro",
    "identidad",
    "set_identidad",
    "comprobar_reloj",
    "reloj_conocido",
    "cancelar_operacion",
    "raiz_sugerida",
    "declaracion_conformidad",
];

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn toda_orden_declarada_se_despacha() {
        // Una orden en la lista que el despacho no conozca daría el mensaje de
        // orden inexistente. Se comprueba con el estado vacío: las órdenes que
        // necesitan repositorio fallan por eso, no por no existir.
        let estado = Estado::default();
        for orden in ORDENES {
            let r = invocar(&estado, orden, &Value::Object(Default::default()));
            if let Err(e) = r {
                assert!(
                    !e.contains("no existe. No se ha hecho nada"),
                    "la orden «{orden}» figura en la lista y el despacho no la conoce"
                );
            }
        }
    }

    #[test]
    fn una_orden_desconocida_se_nombra_en_el_error() {
        let estado = Estado::default();
        let e = invocar(&estado, "inventada", &Value::Object(Default::default())).unwrap_err();
        assert!(e.contains("inventada"));
    }

    #[test]
    fn el_estado_sin_repositorio_no_es_un_fallo_del_despacho() {
        let estado = Estado::default();
        let e = invocar(&estado, "catalogo", &Value::Object(Default::default())).unwrap_err();
        assert_eq!(e, "sin_repositorio");
    }

    #[test]
    fn un_argumento_de_otro_tipo_se_nombra() {
        let estado = Estado::default();
        let args = serde_json::json!({ "uid": 7 });
        let e = invocar(&estado, "ver_proyecto", &args).unwrap_err();
        assert!(e.contains("uid"), "el mensaje no nombra el argumento: {e}");
    }
}
