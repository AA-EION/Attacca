// Sin consola en Windows en compilación de producción.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod estado;
mod ordenes;

fn main() {
    // El desplazamiento de zona horaria se captura antes de crear ningún hilo:
    // `now_local` no es fiable en procesos con varios hilos en algunos sistemas
    // Unix. De este valor dependen todas las marcas del apartado 22.3.
    attacca_core::clock::init_local_offset();

    let constructor = tauri::Builder::default().plugin(tauri_plugin_dialog::init());

    // La actualización verificable comprueba la firma del paquete descargado
    // contra la clave pública declarada en la configuración. Sin clave no se
    // compila: es preferible no ofrecer actualización a ofrecer una que no
    // verifica nada.
    #[cfg(feature = "actualizador")]
    let constructor = constructor.plugin(tauri_plugin_updater::Builder::new().build());

    constructor
        .manage(estado::Estado::default())
        .invoke_handler(tauri::generate_handler![
            ordenes::raiz_sugerida,
            ordenes::abrir_repositorio,
            ordenes::crear_repositorio,
            ordenes::limpiar_temporales,
            ordenes::identidad,
            ordenes::set_identidad,
            ordenes::comprobar_reloj,
            ordenes::reloj_conocido,
            ordenes::catalogo,
            ordenes::reconstruir_indice,
            ordenes::crear_proyecto,
            ordenes::ver_proyecto,
            ordenes::cambiar_titulo,
            ordenes::cambiar_estado,
            ordenes::registrar_audio,
            ordenes::derivar_proyecto,
            ordenes::crear_sesion,
            ordenes::ruta_exportacion,
            ordenes::incorporar_material,
            ordenes::listar_carpeta,
            ordenes::abrir_en_explorador,
            ordenes::validar,
            ordenes::verificar_integridad,
            ordenes::generar_integridad,
            ordenes::crear_release,
            ordenes::vincular_a_release,
            ordenes::release_listo,
            ordenes::emitir_envio,
            ordenes::cancelar_operacion,
            ordenes::verificar_paquete,
            ordenes::ingerir_paquete,
            ordenes::paquetes_en_cuarentena,
            ordenes::reclamar_retorno,
            ordenes::recuperar_custodia,
            ordenes::derivar_sobre_cedido,
            ordenes::registro,
            ordenes::verificar_registro,
            ordenes::declaracion_conformidad,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la aplicación");
}
