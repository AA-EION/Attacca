//! Envoltura de Tauri sobre la capa de órdenes.
//!
//! Aquí no hay lógica. Cada función declara la orden ante Tauri y llama a la
//! del crate `attacca-ordenes`, que es la misma que emplea la interfaz nativa
//! de macOS a través del puente de C. Una orden implementada dos veces acabaría
//! comportándose de dos maneras; implementada una sola vez, no puede.

use attacca_ordenes::estado::{Conformidad, Entrada, Estado, Identidad, VistaProyecto};
use attacca_ordenes::ordenes as nucleo;
use tauri::State;

pub use attacca_ordenes::ordenes::{
    AperturaRepositorio, CarpetaSesion, Catalogo, DatosAudio, DatosEnvio, DatosIncorporacion,
    DatosIngesta, DatosProyecto, DatosRecepcion, DatosRelease, EntradaRegistro, EstadoRegistro,
    EstadoReloj, ResultadoDerivacion, ResultadoEmision, ResultadoIngesta, ResultadoIntegridad,
    ResultadoVerificacion,
};

type R<T> = Result<T, String>;

/// Declara una orden ante Tauri delegando en la capa portable.
///
/// La forma `-> T` distingue las órdenes que devuelven un valor de las que
/// devuelven `Result`; Tauri las trata igual, pero la firma ha de coincidir con
/// la de la función a la que se delega.
macro_rules! ordenes {
    ( $( $(#[$meta:meta])* fn $nombre:ident ( $($arg:ident : $tipo:ty),* ) -> $ret:ty; )* ) => {
        $(
            $(#[$meta])*
            #[tauri::command]
            pub fn $nombre(estado: State<Estado> $(, $arg: $tipo)*) -> $ret {
                nucleo::$nombre(&estado $(, $arg)*)
            }
        )*
    };
}

ordenes! {
    fn abrir_repositorio(raiz: String) -> R<AperturaRepositorio>;
    fn crear_repositorio(raiz: String) -> R<AperturaRepositorio>;
    fn limpiar_temporales() -> R<usize>;
    fn identidad() -> Identidad;
    fn set_identidad(identidad: Identidad) -> ();
    fn comprobar_reloj() -> EstadoReloj;
    fn reloj_conocido() -> EstadoReloj;
    fn catalogo() -> R<Catalogo>;
    fn reconstruir_indice() -> R<usize>;
    fn crear_proyecto(datos: DatosProyecto) -> R<VistaProyecto>;
    fn ver_proyecto(uid: String) -> R<VistaProyecto>;
    fn cambiar_titulo(uid: String, titulo: String) -> R<VistaProyecto>;
    fn cambiar_estado(uid: String, nuevo: String) -> R<VistaProyecto>;
    fn registrar_audio(uid: String, datos: DatosAudio) -> R<VistaProyecto>;
    fn derivar_proyecto(uid: String, motivo: String, paralelo: bool) -> R<ResultadoDerivacion>;
    fn crear_sesion(uid: String, daw: String, etapa: String) -> R<CarpetaSesion>;
    fn ruta_exportacion(
        uid: String,
        carpeta: String,
        nombre: String,
        extension: String
    ) -> R<String>;
    fn incorporar_material(uid: String, datos: DatosIncorporacion) -> R<VistaProyecto>;
    fn listar_carpeta(uid: String, relativa: String) -> R<Vec<Entrada>>;
    fn abrir_en_explorador(uid: String, relativa: String) -> R<()>;
    fn validar(uid: String) -> R<Conformidad>;
    fn verificar_integridad(uid: String) -> R<ResultadoIntegridad>;
    fn generar_integridad(uid: String) -> R<usize>;
    fn crear_release(datos: DatosRelease) -> R<String>;
    fn vincular_a_release(
        release_uid: String,
        proyecto_uid: String,
        posicion: Option<i64>
    ) -> R<i64>;
    fn release_listo(release_uid: String) -> R<Vec<String>>;
    fn emitir_envio(uid: String, datos: DatosEnvio) -> R<ResultadoEmision>;
    fn cancelar_operacion() -> ();
    fn verificar_paquete(datos: DatosRecepcion) -> R<ResultadoVerificacion>;
    fn ingerir_paquete(datos: DatosIngesta) -> R<ResultadoIngesta>;
    fn paquetes_en_cuarentena() -> R<Vec<String>>;
    fn reclamar_retorno(uid: String) -> R<String>;
    fn recuperar_custodia(uid: String) -> R<VistaProyecto>;
    fn derivar_sobre_cedido(uid: String) -> R<ResultadoDerivacion>;
    fn registro(ultimas: usize) -> R<Vec<EntradaRegistro>>;
    fn verificar_registro() -> R<EstadoRegistro>;
}

// Las dos que no consultan el estado de la sesión.

#[tauri::command]
pub fn raiz_sugerida() -> Option<String> {
    nucleo::raiz_sugerida()
}

#[tauri::command]
pub fn declaracion_conformidad() -> String {
    nucleo::declaracion_conformidad()
}
