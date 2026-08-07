import Foundation

/// Todo el texto que se ve en la interfaz nativa.
///
/// Rige lo mismo que en `app/ui/textos.js`, y por la misma razón: las reglas de
/// redacción se auditan en un archivo, no recorriendo vistas. `auditar-redaccion.py`
/// comprueba este archivo con las mismas prohibiciones que el otro.
///
/// El registro es el llano. El vocabulario normativo vive en `Tecnico` y solo
/// aparece en la sección de detalle.
public enum Textos {
    public static let nombre = "Attacca"

    // MARK: Acciones. Siempre en infinitivo.

    public static let accion: [String: String] = [
        "crear_proyecto": "Crear proyecto",
        "crear_release": "Crear lanzamiento",
        "crear_sesion": "Abrir sesión nueva",
        "cambiar_titulo": "Cambiar el título",
        "incorporar_material": "Añadir material de fuera",
        "exportar_stems": "Sacar los stems",
        "exportar_bounce": "Sacar la mezcla",
        "incorporar_master": "Añadir el máster",
        "registrar_mediciones": "Anotar las mediciones",
        "nueva_version": "Empezar una versión nueva",
        "ceder_custodia": "Dejar el proyecto a otro estudio",
        "verificar_paquete": "Revisar lo que ha llegado",
        "acusar_recibo": "Confirmar la recepción",
        "ingerir_paquete": "Guardar lo recibido",
        "reclamar_retorno": "Reclamar la devolución",
        "recuperar_custodia": "Recuperar el proyecto",
        "derivar_sobre_cedido": "Trabajar en una copia aparte",
        "ejecutar_control_calidad": "Hacer la revisión final",
        "firmar_informe": "Firmar la revisión",
        "constituir_paquete_entrega": "Preparar la entrega",
        "emitir_envio": "Enviar a alguien",
        "archivar": "Archivar el proyecto",
        "verificar_integridad": "Comprobar que nada se ha estropeado",
        "abrir_en_explorador": "Abrir la carpeta",
        "volver": "Volver",
        "cancelar": "Cancelar",
        "guardar": "Guardar",
        "cerrar": "Cerrar",
        "registrar_notas": "Apuntar la letra y las notas",
        "incorporar_referencia": "Añadir referencias",
        "declarar_tempo_tonalidad": "Anotar tempo y tonalidad",
        "registrar_plan_grabacion": "Apuntar el plan de grabación",
        "registrar_mapa_tempo": "Apuntar el mapa de tempo",
        "preparar_acuerdo_reparto": "Preparar el reparto de autoría",
        "incorporar_tomas": "Añadir las tomas",
        "fijar_vocabulario": "Poner nombre a las pistas",
        "congelar_vocabulario": "Cerrar los nombres de las pistas",
        "incorporar_audio_editado": "Añadir el audio ya editado",
        "generar_hoja_recall": "Sacar la hoja de recall",
        "asignar_identificadores": "Anotar los códigos de la entrega",
        "constituir_paquete_produccion": "Preparar el envío al otro estudio",
        "consolidar_sesion": "Consolidar la sesión",
        "generar_manifiesto_integridad": "Anotar cómo está todo ahora",
    ]

    // MARK: Fases del trabajo.

    /// Recorrido habitual de un tema, en orden. Las dos fases que quedan fuera
    /// se presentan solas.
    public static let recorrido = [
        "composicion", "preproduccion", "grabacion", "edicion", "mezcla",
        "mastering", "control_calidad", "distribucion", "archivo",
    ]

    public static let fase: [String: String] = [
        "composicion": "Composición",
        "preproduccion": "Preproducción",
        "grabacion": "Grabación",
        "edicion": "Edición",
        "mezcla": "Mezcla",
        "mastering": "Mastering",
        "control_calidad": "Revisión final",
        "distribucion": "Entrega",
        "intercambio_produccion": "Envío a otro estudio",
        "recepcion": "Material recibido",
        "archivo": "Archivo",
    ]

    public static let ahora: [String: String] = [
        "composicion": "Apuntar la idea y reunir las referencias.",
        "preproduccion": "Dejar cerrado qué se graba, con quién y cómo.",
        "grabacion": "Grabar las tomas y ponerles nombre.",
        "edicion": "Dejar la edición cerrada.",
        "mezcla": "Sacar la mezcla y los stems.",
        "mastering": "Añadir el máster y anotar las mediciones.",
        "control_calidad": "Escuchar el máster y dar el visto bueno.",
        "distribucion": "Preparar la entrega y mandarla.",
        "intercambio_produccion": "Preparar el envío para que otro estudio siga.",
        "recepcion": "Revisar lo que ha llegado y guardarlo.",
        "archivo": "Dejarlo todo guardado y cerrado.",
    ]

    public static let paso: [String: String] = [
        "maqueta_disponible": "Que haya una maqueta.",
        "plan_grabacion_cerrado": "Que el plan de grabación esté cerrado.",
        "vocabulario_congelado": "Que las tomas tengan su nombre definitivo.",
        "edicion_cerrada": "Que la edición esté cerrada.",
        "bounce_aprobado": "Que la mezcla esté aprobada y los stems salgan bien.",
        "mediciones_registradas": "Que las mediciones estén anotadas.",
        "informe_aprobado": "Que la revisión final esté aprobada.",
        "envio_emitido_y_acusado": "Que el envío salga y el destinatario confirme.",
        "custodia_cedida_o_retornada": "Que el otro estudio lo tenga, o que ya haya vuelto.",
        "cuarentena_vacia": "Que no quede nada por revisar.",
        "proyecto_en_archivo": "Que el proyecto esté archivado.",
    ]

    // MARK: Dónde está y en qué disco. Se ven siempre, sin pedir nada.

    public static let donde: [String: String] = [
        "propia": "En el estudio",
        "en_transito": "De camino",
        "cedida": "En otro estudio",
        "reclamada": "Devolución reclamada",
    ]

    public static let disco: [String: String] = [
        "activa": "Este disco",
        "en_espera": "Copia en espera",
        "desconectada": "Disco desconectado",
        "divergente": "Dos copias distintas",
    ]

    public static let situacion: [String: String] = [
        "idea": "Idea",
        "active": "En curso",
        "onhold": "En pausa",
        "delivered": "Entregado",
        "sealed": "Cerrado",
        "archived": "Archivado",
    ]

    // MARK: Carpetas de la norma, leídas en llano.

    public static let carpeta: [String: String] = [
        "00_ADMIN": "Papeles",
        "00_SYSTEM": "Sistema",
        "01_REF": "Referencias",
        "02_SESSIONS": "Sesiones",
        "03_RECORDINGS": "Tomas",
        "04_EDIT": "Edición",
        "05_STEMS": "Stems",
        "06_MIX": "Mezclas",
        "07_MASTER": "Másters",
        "08_DELIVERY": "Entrega",
        "09_TRANSFER": "Envíos",
        "10_PROJECTS": "Proyectos",
        "20_RELEASES": "Lanzamientos",
        "30_ARCHIVE": "Archivo",
        "40_INBOX": "Lo que ha llegado",
        "Notes": "Notas",
    ]

    // MARK: Secciones y rótulos.

    public enum Seccion {
        public static let proyectos = "Proyectos"
        public static let lanzamientos = "Lanzamientos"
        public static let recibido = "Lo que ha llegado"
        public static let historial = "Historial"
        public static let ajustes = "Ajustes"
        public static let archivos = "Archivos"
        public static let ahora = "Ahora toca"
        public static let tambien = "También"
        public static let pendiente = "Antes de seguir"
        public static let detalle = "Detalles"
        public static let otrasCarpetas = "Otras carpetas"
    }

    public enum Rotulo {
        public static let artista = "Artista"
        public static let fase = "Fase"
        public static let donde = "Dónde está"
        public static let disco = "Disco"
        public static let tempo = "Tempo"
        public static let tonalidad = "Tonalidad"
        public static let pasoSiguiente = "Para pasar a la fase siguiente"
    }

    public enum Falta {
        public static let nada = "No falta nada."
        public static let pendiente = "pendiente"
        public static let problema = "por arreglar"
        public static let salvedad = "salvedad"
        public static let notaPendiente =
            "Un dato pendiente es de una fase que aún no ha terminado. No es un fallo."
    }

    public enum Vacio {
        public static let sinProyectos = "Todavía no hay ningún proyecto."
        public static let primerProyecto = "Crear el primero para empezar."
        public static let sinArchivos = "Aquí no hay nada todavía."
        public static let sinAcciones = "En esta fase no hay nada que hacer desde aquí."
        public static let sinCarpetas = "No hay más carpetas con contenido."
    }

    public enum Aviso {
        public static let notaAbrir =
            "Abrir la carpeta no cierra ningún archivo que otro programa tenga abierto."
        public static let integridadBien = "%d archivos comprobados. Todos están como estaban."
        public static let integridadMal = """
            %1$d de %2$d archivos han cambiado sin que quede constancia. \
            Lo que se entregue a partir de aquí puede no ser lo aprobado. \
            Revisar estos archivos: %3$@.
            """
        public static let horaDesviada = "%d s de diferencia con la hora de la red"
    }

    public enum Plazo {
        public static let vencido =
            "Tenía que haber vuelto el %@. Toca reclamar la devolución."
        public static let margenVencido = """
            Tenía que haber vuelto el %@ y el margen también pasó. \
            Se puede recuperar sin esperar más.
            """
    }

    public enum Cuenta {
        public static let unArchivo = "1 archivo"
        public static let variosArchivos = "%d archivos"
        public static let unTema = "1 tema"
        public static let variosTemas = "%d temas"
    }

    // MARK: Registro normativo.
    //
    // Solo en la sección de detalle. Aquí el vocabulario del apartado 3 de la
    // norma se emplea literalmente, porque aquí es donde sirve.

    public enum Tecnico {
        public static let titulo = "Detalle técnico"
        public static let nota =
            "Los términos que siguen son los del apartado 3 de la norma. Se emplean literalmente para poder citarlos."

        public static let custodia: [String: String] = [
            "propia": "propia",
            "en_transito": "en tránsito",
            "cedida": "cedida",
            "reclamada": "reclamada",
        ]

        public static let replica: [String: String] = [
            "activa": "activa",
            "en_espera": "en espera",
            "desconectada": "desconectada",
            "divergente": "divergente",
        ]

        /// Nombres de etapa del apartado 3. Difieren del registro llano en dos:
        /// «Control de calidad» y «Distribución», que en llano son «Revisión
        /// final» y «Entrega».
        public static let etapa: [String: String] = [
            "composicion": "Composición",
            "preproduccion": "Preproducción",
            "grabacion": "Grabación",
            "edicion": "Edición",
            "mezcla": "Mezcla",
            "mastering": "Mastering",
            "control_calidad": "Control de calidad",
            "distribucion": "Distribución",
            "intercambio_produccion": "Intercambio de producción",
            "recepcion": "Recepción",
            "archivo": "Archivo",
        ]

        public static let severidad: [String: String] = [
            "pendiente": "pendiente",
            "incumplimiento": "incumplimiento",
            "excepcion": "excepción",
        ]

        public static let identificadorLegible = "Identificador legible"
        public static let identificadorInterno = "Identificador interno"
        public static let etapaActiva = "Etapa activa"
        public static let custodiaRotulo = "Custodia"
        public static let replicaRotulo = "Réplica"
        public static let nivel = "Nivel de conformidad"
        public static let presupuestoRuta = "Presupuesto de ruta"
        public static let frecuencia = "Frecuencia de muestreo"
        public static let bits = "Profundidad de bits"
        public static let afinacion = "Afinación de referencia"
        public static let ruta = "Ruta"
        public static let conformidad = "Conformidad"
    }

    // MARK: Auxiliares

    /// Lectura en llano de una ruta de carpeta de la norma.
    public static func nombreDeCarpeta(_ ruta: String) -> String {
        ruta.split(separator: "/")
            .map { tramo -> String in
                let t = String(tramo)
                if let llano = carpeta[t] { return llano }
                return t.replacingOccurrences(
                    of: "^[0-9]+_", with: "", options: .regularExpression
                ).replacingOccurrences(of: "_", with: " ")
            }
            .joined(separator: " › ")
    }
}
