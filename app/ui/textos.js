// Archivo único de recursos de texto de la interfaz.
//
// Todo el texto que la persona usuaria llega a ver está aquí, para que las
// reglas de redacción se puedan auditar de una sentada, sin recorrer el código.
//
// EL TEXTO TIENE DOS REGISTROS Y NO SE MEZCLAN.
//
// 1. El registro llano. Es todo lo que hay fuera del bloque `tecnico`, y es lo
//    que la persona lee mientras trabaja. Habla de discos, tomas, mezclas y
//    envíos. No nombra la norma, ni sus apartados, ni sus artefactos: quien
//    está grabando no tiene por qué saber qué es un manifiesto para que la
//    aplicación le sirva.
//
// 2. El registro normativo. Es el bloque `tecnico`, y solo aparece en la
//    sección de detalle y en la declaración de conformidad. Ahí el vocabulario
//    del apartado 3 de la norma se emplea literalmente, porque ahí es donde
//    sirve: para citar, para auditar y para discutir con otra implementación.
//
// La aplicación no deja de ser conforme por hablar llano. La conformidad la
// determinan los manifiestos y el árbol, no los rótulos de los botones.
//
// REGLAS DEL REGISTRO LLANO. Prohibido: signos de exclamación; emoji;
// felicitaciones; tuteo entusiasta y segunda persona apelativa; preguntas
// retóricas y frases de relleno; antropomorfismo; metáforas y adjetivos de
// intensidad; explicar lo obvio o repetir en el cuerpo lo que dice el título;
// guiones largos como puntuación frecuente; y el vocabulario normativo, que
// tiene su propio bloque.
//
// Obligatorio: etiquetas de acción en infinitivo; estados en pocas palabras;
// errores con qué ocurrió, qué consecuencia tiene y qué se puede hacer, en ese
// orden; cifras antes que valoraciones; voz impersonal para describir el
// sistema; aviso en una frase antes de una acción irreversible.

export const T = {
  app: {
    nombre: "Attacca",
    subtitulo: "Orden en el material de un estudio",
  },

  // --- Acciones. Siempre en infinitivo, siempre en llano. ---
  accion: {
    crear_proyecto: "Crear proyecto",
    crear_release: "Crear lanzamiento",
    crear_sesion: "Abrir sesión nueva",
    abrir_proyecto: "Abrir proyecto",
    cambiar_titulo: "Cambiar el título",
    incorporar_material: "Añadir material de fuera",
    exportar_stems: "Sacar los stems",
    exportar_bounce: "Sacar la mezcla",
    incorporar_master: "Añadir el máster",
    registrar_mediciones: "Anotar las mediciones",
    nueva_version: "Empezar una versión nueva",
    ceder_custodia: "Dejar el proyecto a otro estudio",
    recibir_paquete: "Revisar lo que ha llegado",
    verificar_paquete: "Revisar lo que ha llegado",
    acusar_recibo: "Confirmar la recepción",
    ingerir_paquete: "Guardar lo recibido",
    reclamar_retorno: "Reclamar la devolución",
    recuperar_custodia: "Recuperar el proyecto",
    derivar_sobre_cedido: "Trabajar en una copia aparte",
    ejecutar_control_calidad: "Hacer la revisión final",
    firmar_informe: "Firmar la revisión",
    constituir_paquete_entrega: "Preparar la entrega",
    emitir_envio: "Enviar a alguien",
    archivar: "Archivar el proyecto",
    verificar_integridad: "Comprobar que nada se ha estropeado",
    generar_integridad: "Anotar cómo está todo ahora",
    conmutar_replica: "Cambiar de disco",
    reconciliar: "Juntar las dos copias",
    abrir_en_explorador: "Abrir la carpeta",
    ver_estructura_completa: "Ver todas las carpetas",
    volver: "Volver",
    cancelar: "Cancelar",
    aceptar: "Aceptar",
    cerrar: "Cerrar",
    guardar: "Guardar",
    registrar_notas: "Apuntar la letra y las notas",
    incorporar_referencia: "Añadir referencias",
    declarar_tempo_tonalidad: "Anotar tempo y tonalidad",
    registrar_plan_grabacion: "Apuntar el plan de grabación",
    registrar_mapa_tempo: "Apuntar el mapa de tempo",
    preparar_acuerdo_reparto: "Preparar el reparto de autoría",
    incorporar_tomas: "Añadir las tomas",
    fijar_vocabulario: "Poner nombre a las pistas",
    congelar_vocabulario: "Cerrar los nombres de las pistas",
    incorporar_audio_editado: "Añadir el audio ya editado",
    generar_hoja_recall: "Sacar la hoja de recall",
    asignar_identificadores: "Anotar los códigos de la entrega",
    constituir_paquete_produccion: "Preparar el envío al otro estudio",
    consolidar_sesion: "Consolidar la sesión",
    generar_manifiesto_integridad: "Anotar cómo está todo ahora",
    vincular_release: "Añadir a un lanzamiento",
    sincronizar_reloj: "Comprobar la hora",
    elegir_archivo: "Elegir archivo",
    reconstruir_indice: "Volver a leer las carpetas",
  },

  // --- Atajos de teclado, visibles junto a la acción. ---
  atajo: {
    crear_proyecto: "Ctrl N",
    crear_sesion: "Ctrl S",
    incorporar_material: "Ctrl I",
    abrir_en_explorador: "Ctrl E",
    ver_estructura_completa: "Ctrl T",
    verificar_integridad: "Ctrl K",
    buscar: "Ctrl B",
    volver: "Esc",
  },

  // --- Secciones. ---
  seccion: {
    proyectos: "Proyectos",
    lanzamientos: "Lanzamientos",
    recibido: "Lo que ha llegado",
    historial: "Historial",
    ajustes: "Ajustes",
    archivos: "Archivos",
    ahora: "Ahora toca",
    tambien: "También",
    pendiente: "Antes de seguir",
    detalle: "Detalles",
    otras_carpetas: "Otras carpetas",
  },

  // --- Fases del trabajo. El nombre que se usa en un estudio. ---
  fase: {
    composicion: "Composición",
    preproduccion: "Preproducción",
    grabacion: "Grabación",
    edicion: "Edición",
    mezcla: "Mezcla",
    mastering: "Mastering",
    control_calidad: "Revisión final",
    distribucion: "Entrega",
    intercambio_produccion: "Envío a otro estudio",
    recepcion: "Material recibido",
    archivo: "Archivo",
  },

  // --- Qué se hace en cada fase. Una frase, sin rodeos. ---
  ahora: {
    composicion: "Apuntar la idea y reunir las referencias.",
    preproduccion: "Dejar cerrado qué se graba, con quién y cómo.",
    grabacion: "Grabar las tomas y ponerles nombre.",
    edicion: "Dejar la edición cerrada.",
    mezcla: "Sacar la mezcla y los stems.",
    mastering: "Añadir el máster y anotar las mediciones.",
    control_calidad: "Escuchar el máster y dar el visto bueno.",
    distribucion: "Preparar la entrega y mandarla.",
    intercambio_produccion: "Preparar el envío para que otro estudio siga.",
    recepcion: "Revisar lo que ha llegado y guardarlo.",
    archivo: "Dejarlo todo guardado y cerrado.",
  },

  // --- Qué hace falta para pasar a la fase siguiente. ---
  paso: {
    titulo: "Para pasar a la fase siguiente",
    maqueta_disponible: "Que haya una maqueta.",
    plan_grabacion_cerrado: "Que el plan de grabación esté cerrado.",
    vocabulario_congelado: "Que las tomas tengan su nombre definitivo.",
    edicion_cerrada: "Que la edición esté cerrada.",
    bounce_aprobado: "Que la mezcla esté aprobada y los stems salgan bien.",
    mediciones_registradas: "Que las mediciones estén anotadas.",
    informe_aprobado: "Que la revisión final esté aprobada.",
    envio_emitido_y_acusado: "Que el envío salga y el destinatario confirme.",
    custodia_cedida_o_retornada: "Que el otro estudio lo tenga, o que ya haya vuelto.",
    cuarentena_vacia: "Que no quede nada por revisar.",
    proyecto_en_archivo: "Que el proyecto esté archivado.",
  },

  // --- Dónde está el proyecto. Se ve siempre, sin pedir nada. ---
  donde: {
    propia: "En el estudio",
    en_transito: "De camino",
    cedida: "En otro estudio",
    reclamada: "Devolución reclamada",
    nota_cedida: "Mientras esté fuera, el proyecto no se modifica aquí.",
    nota_en_transito: "Mientras va de camino, no debe modificarlo ninguna de las dos partes.",
  },

  // --- En qué disco está. Se ve siempre, sin pedir nada. ---
  disco: {
    activa: "Este disco",
    en_espera: "Copia en espera",
    desconectada: "Disco desconectado",
    divergente: "Dos copias distintas",
    nota_divergente:
      "Las dos copias tienen cambios que la otra no tiene. Ninguna se sobrescribe. Hay que juntarlas a mano.",
  },

  // --- Situación del proyecto en la lista. ---
  situacion: {
    idea: "Idea",
    active: "En curso",
    onhold: "En pausa",
    delivered: "Entregado",
    sealed: "Cerrado",
    archived: "Archivado",
  },

  // --- Qué falta. Tres cosas que no se confunden. ---
  falta: {
    nada: "No falta nada.",
    pendiente_uno: "Falta 1 dato",
    pendiente_varios: "Faltan {n} datos",
    problema_uno: "1 cosa por arreglar",
    problema_varios: "{n} cosas por arreglar",
    salvedad_una: "1 salvedad anotada",
    salvedad_varias: "{n} salvedades anotadas",
    // Un dato que todavía no toca no es un error, y no se pinta en rojo.
    nota_pendiente:
      "Un dato pendiente es de una fase que aún no ha terminado. No es un fallo.",
    etiqueta_pendiente: "pendiente",
    etiqueta_problema: "por arreglar",
    etiqueta_salvedad: "salvedad",
    ver_tecnico: "Ver el detalle técnico",
  },

  // --- Crear un proyecto. Lo mínimo, y una razón para lo que no cambia. ---
  creacion: {
    titulo: "Crear proyecto",
    campo_artista: "Artista",
    campo_titulo: "Título",
    campo_tipo: "Qué es",
    campo_release: "Lanzamiento",
    campo_release_ninguno: "Ninguno",
    campo_calidad: "Calidad de grabación",
    calidad_estandar: "48 kHz, 24 bits. Lo habitual",
    calidad_alta: "96 kHz, 24 bits. Para grabación de sala",
    calidad_cine: "48 kHz, 32 bits. Para audiovisual",
    calidad_cd: "44,1 kHz, 24 bits. Para masterizar a CD",
    nota_calidad:
      "La calidad se fija ahora y ya no cambia. Todo lo que se grabe después tiene que coincidir con ella.",
    nota_titulo: "El título se puede cambiar mientras el proyecto no se haya enviado ni archivado.",
    nota_resto: "El tempo, la tonalidad y los créditos se anotan cuando toque.",
  },

  // --- Tipos de proyecto. El código va detrás, no delante. ---
  tipo: {
    ORIG: "Tema propio",
    COVER: "Versión de un tema ajeno",
    REMIX: "Remezcla",
    BEAT: "Instrumental",
    MIX: "Solo la mezcla, para otro",
    MST: "Solo el mastering, para otro",
    SD: "Diseño sonoro",
    LIVE: "Directo",
    DEMO: "Maqueta",
    SYNC: "Encargo para imagen",
  },

  clase_lanzamiento: {
    SINGLE: "Sencillo",
    EP: "EP",
    ALBUM: "Álbum",
    COMP: "Recopilación",
    LIVE: "Directo",
    SYNC: "Obra audiovisual",
  },

  sesion: {
    campo_programa: "Programa",
    campo_para: "Para qué es",
    COMP: "Componer",
    TRACK: "Grabar",
    EDIT: "Editar",
    MIX: "Mezclar",
    MST: "Masterizar",
    SD: "Diseño sonoro",
    nombre_sugerido: "Guardar la sesión con este nombre: {nombre}",
  },

  // --- Enviar material a alguien. ---
  envio: {
    titulo: "Enviar a alguien",
    campo_quien: "A quién va",
    campo_contacto: "Correo de contacto",
    campo_para_que: "Para qué",
    campo_hasta: "Hasta cuándo puede conservarlo",
    campo_que_mandar: "Qué se manda",
    manda_E: "El resultado terminado, para publicar o escuchar",
    manda_P: "Todo el proyecto, para que otro siga trabajando",
    manda_A: "Todo, para guardarlo a largo plazo",
    campo_quien_lo_ve: "Quién puede verlo",
    campo_revisado: "La revisión final está aprobada",
    nota_revisado: "Nada sale del estudio sin la revisión final aprobada.",
    resultado: "{archivos} archivos, {tamano}. El paquete está en {ruta}.",
    campo_vuelve: "Cuándo debe volver",
    campo_margen: "Días de margen",
  },

  quien_lo_ve: {
    PUBLICO: "Cualquiera",
    INTERNO: "Solo el estudio",
    CONFIDENCIAL: "Solo quien participa",
    RESTRINGIDO: "Solo quien se nombre",
  },

  // --- Lo que ha llegado. ---
  recibido: {
    titulo: "Lo que ha llegado",
    vacio: "No ha llegado nada.",
    campo_de_quien: "De quién viene",
    campo_condiciones: "Las condiciones de uso sirven para lo que se va a hacer",
    revision: "Revisión antes de guardar",
    accepted: "Se puede guardar",
    accepted_with_reservations: "Se puede guardar, con reparos",
    rejected: "No se puede guardar",
    pass: "bien",
    fail: "mal",
    not_applicable: "no aplica",
    reparos: "Reparos",
    aceptar_custodia: "Hacerse cargo del proyecto que viene en el envío",
    guardado: "{archivos} archivos guardados en {destino}.",
    atraso: "{n} cosas llevan tiempo sin revisar. Conviene revisarlas cada semana.",
  },

  // --- Las catorce comprobaciones de un envío recibido. ---
  comprobacion: {
    provenance: "De quién viene",
    authenticity: "Firma",
    container: "El paquete se abre",
    package_integrity: "El paquete llegó entero",
    content_integrity: "Los archivos llegaron enteros",
    manifest_schema: "Los datos del envío están completos",
    profile_supported: "Es una versión que se entiende",
    custody: "A quién le toca el proyecto",
    chronology: "Las fechas cuadran",
    scope_match: "Viene lo que decía que venía",
    usage_accepted: "Condiciones de uso y plazo",
    personal_data_match: "Datos personales",
    structure: "Las carpetas vienen como deben",
    declared_checks: "Las comprobaciones que declaró el emisor",
  },

  // --- Acciones que no se pueden deshacer. Una frase, sin dramatizar. ---
  irreversible: {
    confirmar: "Confirmar",

    derivar_titulo: "Empezar una versión nueva",
    derivar_aviso:
      "El proyecto actual se cierra y pasa a solo lectura. En la práctica, esto no se deshace.",
    derivar_detalle:
      "Se copia todo: notas, contratos, referencias, sesiones y ajustes. La versión nueva lleva el sufijo _vNN y deja constancia de dónde viene.",
    derivar_campo_motivo: "Por qué",
    derivar_paralelo: "Seguir trabajando en los dos a la vez",
    derivar_paralelo_nota:
      "El proyecto actual no se cierra. Los dos pasan a contar como obras distintas para créditos y derechos.",

    ceder_titulo: "Dejar el proyecto a otro estudio",
    ceder_aviso: "La copia de aquí queda en solo lectura hasta que el proyecto vuelva.",
    ceder_detalle:
      "Mientras el proyecto está fuera, ninguna de las dos partes debe modificarlo.",

    recuperar_titulo: "Recuperar el proyecto",
    recuperar_aviso:
      "Si el otro estudio devuelve algo después, será una versión distinta que habrá que juntar a mano.",
    recuperar_detalle: "Queda anotado como incidencia y se avisa al otro estudio.",

    archivar_titulo: "Archivar el proyecto",
    archivar_aviso: "El proyecto se mueve al archivo y queda en solo lectura.",
  },

  // --- Plazos de un proyecto que está fuera. ---
  plazo: {
    vencido: "Tenía que haber vuelto el {fecha}. Toca reclamar la devolución.",
    margen_vencido:
      "Tenía que haber vuelto el {fecha} y el margen también pasó. Se puede recuperar sin esperar más.",
  },

  // --- Archivos. ---
  archivos: {
    vacia: "Aquí no hay nada todavía.",
    uno: "1 archivo",
    varios: "{n} archivos",
    carpeta: "carpeta",
    subir: "Subir un nivel",
    nota_abrir:
      "Abrir la carpeta no cierra ningún archivo que otro programa tenga abierto.",
  },

  // --- Errores. Qué ocurrió, qué consecuencia, qué se puede hacer. ---
  error: {
    sin_repositorio:
      "No hay ninguna carpeta de trabajo abierta. No se ha cargado nada. Elegir una carpeta de trabajo o crear una.",
    raiz_sincronizada:
      "La carpeta elegida está dentro de {servicio}. No se ha creado nada. Elegir un sitio que no sincronice solo.",
    reloj_sin_fuente:
      "No se ha podido consultar la hora en la red. No se puede enviar ni confirmar nada. Conectar el equipo a la red y volver a comprobar.",
    reloj_desviado:
      "El reloj del equipo va {segundos} s desviado. No se puede enviar ni confirmar nada. Poner el reloj en hora y volver a comprobar.",
    archivos_abiertos:
      "{n} archivos del proyecto están abiertos en otro programa. No se ha empezado nada. Cerrarlos en el programa que los tiene abiertos.",
    ruta_larga:
      "El nombre más largo mediría {actual} de {limite} caracteres. No se ha creado nada. Acortar el título.",
    replica_no_activa:
      "El disco que manda es otro. No se ha escrito nada. Cambiar de disco antes de trabajar aquí.",
    integridad_mal:
      "{fallidos} de {total} archivos han cambiado sin que quede constancia. Lo que se entregue a partir de aquí puede no ser lo aprobado. Revisar estos archivos: {rutas}.",
  },

  // --- Avisos que no son errores. ---
  aviso: {
    integridad_bien: "{n} archivos comprobados. Todos están como estaban.",
    reloj_precision_reducida:
      "La hora se obtuvo con una precisión de un segundo. La comprobación es menos fina de lo normal.",
    nombre_largo: "El nombre más largo del proyecto va por {actual} de {limite} caracteres.",
    volumen_sin_solo_lectura:
      "Este disco no admite marcar archivos como solo lectura. La protección es más débil de lo normal.",
    operacion_a_medias: "{n} operaciones quedaron a medias la última vez.",
    temporales:
      "{n} archivos temporales quedaron sueltos. Los archivos de verdad conservan lo que tenían.",
    sin_carpeta_trabajo:
      "No hay carpeta de trabajo en {ruta}. Una sola por persona, sin espacios en el nombre y fuera de las carpetas que se sincronizan solas.",
    crear_carpeta_trabajo: "Crear la carpeta de trabajo",
  },

  // --- Operaciones largas. ---
  progreso: {
    de: "{hecho} de {total} archivos",
    empaquetando: "Preparando el paquete",
    verificando: "Comprobando los archivos",
    extrayendo: "Abriendo el paquete",
    copiando: "Copiando el material",
    cancelar: "Cancelar",
    cancelado: "Se canceló. No ha quedado nada a medias en el destino.",
  },

  // --- Historial. ---
  historial: {
    titulo: "Historial",
    vacio: "Todavía no hay nada anotado.",
    entradas: "{n} cosas anotadas",
    intacto: "El historial está completo",
    roto: "Al historial le faltan cosas",
    nota_roto:
      "Las anotaciones {cuales} no encajan con el resto. Indica que se borró, se reordenó o se cambió algo.",
    columna_cuando: "Cuándo",
    columna_que: "Qué pasó",
    columna_quien: "Quién",
    columna_proyecto: "Proyecto",
  },

  // --- Ajustes. ---
  ajustes: {
    titulo: "Ajustes",
    aspecto: "Aspecto",
    tema: "Tema",
    tema_sistema: "El del sistema",
    tema_claro: "Claro",
    tema_oscuro: "Oscuro",
    quien_eres: "Quién trabaja aquí",
    organizacion: "Estudio",
    persona: "Persona",
    nota_quien:
      "Este nombre queda anotado en el historial junto a cada cosa que se haga.",
    carpeta_trabajo: "Carpeta de trabajo",
    hora: "Hora",
    hora_bien: "{ms} ms de diferencia con la hora de la red",
    hora_mal: "{s} s de diferencia con la hora de la red",
    hora_sin: "No se ha podido consultar la hora en la red",
    ficha_tecnica: "Ficha técnica",
    nota_ficha:
      "Lo que sigue está en el idioma de la norma, para poder citarlo y compararlo con otras herramientas.",
  },

  // --- Nombres de las carpetas de la norma.
  //
  // En el disco se llaman `05_STEMS` y así deben seguir llamándose: el número
  // ordena el árbol y el nombre en inglés lo hace legible para quien reciba el
  // material fuera de aquí. Lo que no tiene sentido es enseñar ese nombre en un
  // botón. Aquí está la lectura en llano de cada una.
  carpeta: {
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
    Notes: "Notas",
  },

  // --- Rótulos de campos del registro llano. ---
  rotulo: {
    artista: "Artista",
    titulo: "Título",
    tipo: "Qué es",
    fase: "Fase",
    donde: "Dónde está",
    disco: "Disco",
    lanzamiento: "Lanzamiento",
    temas: "Temas",
    calidad: "Calidad",
    tempo: "Tempo",
    tonalidad: "Tonalidad",
    afinacion: "Afinación",
    origen: "Punto de inicio",
    carpeta: "Carpeta",
    situacion: "Situación",
  },

  // --- Nada que mostrar. Sin frases de relleno. ---
  vacio: {
    sin_proyectos: "Todavía no hay ningún proyecto.",
    primer_proyecto: "Crear el primero para empezar.",
    sin_lanzamientos: "Todavía no hay ningún lanzamiento.",
    sin_acciones: "En esta fase no hay nada que hacer desde aquí.",
    sin_carpetas: "No hay más carpetas con contenido.",
  },

  // --- REGISTRO NORMATIVO ---
  //
  // Solo aparece en la sección de detalle y en la ficha técnica. Aquí el
  // vocabulario del apartado 3 de la norma se emplea literalmente, porque aquí
  // es donde sirve: para citar un apartado, para auditar y para entenderse con
  // otra implementación.
  tecnico: {
    titulo: "Detalle técnico",
    nota:
      "Los términos que siguen son los del apartado 3 de la norma. Se emplean literalmente para poder citarlos.",

    custodia: {
      propia: "propia",
      en_transito: "en tránsito",
      cedida: "cedida",
      reclamada: "reclamada",
    },
    replica: {
      activa: "activa",
      en_espera: "en espera",
      desconectada: "desconectada",
      divergente: "divergente",
    },
    etapa: {
      composicion: "Composición",
      preproduccion: "Preproducción",
      grabacion: "Grabación",
      edicion: "Edición",
      mezcla: "Mezcla",
      mastering: "Mastering",
      control_calidad: "Control de calidad",
      distribucion: "Distribución",
      intercambio_produccion: "Intercambio de producción",
      recepcion: "Recepción",
      archivo: "Archivo",
    },
    perfil: {
      E: "Entrega",
      P: "Producción",
      A: "Custodia",
    },
    clasificacion: {
      PUBLICO: "público",
      INTERNO: "interno",
      CONFIDENCIAL: "confidencial",
      RESTRINGIDO: "restringido",
    },

    rotulo: {
      identificador_interno: "Identificador interno",
      identificador_legible: "Identificador legible",
      nivel: "Nivel de conformidad",
      custodia: "Custodia",
      replica: "Réplica",
      etapa: "Etapa activa",
      conformidad: "Conformidad",
      presupuesto_ruta: "Presupuesto de ruta",
      frecuencia: "Frecuencia de muestreo",
      bits: "Profundidad de bits",
      afinacion: "Afinación de referencia",
      origen: "Punto temporal de origen",
      ruta: "Ruta",
      envio: "Envío",
      clausula: "Apartado",
      norma: "Norma aplicada",
    },

    conformidad: {
      conforme: "Conforme",
      pendiente: "pendiente",
      incumplimiento: "incumplimiento",
      excepcion: "excepción",
    },

    presupuesto: "{actual} de {limite} caracteres",
  },
};

/// Sustituye los marcadores {clave} de una cadena.
export function fmt(plantilla, valores = {}) {
  return String(plantilla).replace(/\{(\w+)\}/g, (coincidencia, clave) =>
    Object.prototype.hasOwnProperty.call(valores, clave)
      ? String(valores[clave])
      : coincidencia
  );
}

/// Elige entre singular y plural según la cifra.
export function plural(n, singular, varios) {
  return n === 1 ? singular : fmt(varios, { n });
}
