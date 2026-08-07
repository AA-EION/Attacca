// Archivo único de recursos de texto de la interfaz.
//
// Todo el texto que la persona usuaria llega a ver está aquí. La finalidad es
// que las reglas de redacción se puedan auditar de una sentada, sin recorrer el
// código.
//
// REGLAS. Prohibido: signos de exclamación; emoji; felicitaciones; tuteo
// entusiasta y segunda persona apelativa; preguntas retóricas y frases de
// relleno; antropomorfismo; metáforas y adjetivos de intensidad; explicar lo
// obvio o repetir en el cuerpo lo que dice el título; guiones largos como
// puntuación frecuente.
//
// Obligatorio: etiquetas de acción en infinitivo; terminología idéntica a la
// del apartado 3 de la norma; estados en una o dos palabras con el vocabulario
// normativo; errores con qué ocurrió, qué consecuencia tiene y qué se puede
// hacer, en ese orden; cifras antes que valoraciones; voz impersonal para
// describir el sistema; aviso en una frase antes de una acción irreversible.

export const T = {
  app: {
    nombre: "Attacca",
    subtitulo: "Gestión conforme a STAVE 2.0",
  },

  // --- Acciones. Siempre en infinitivo. ---
  accion: {
    crear_proyecto: "Crear proyecto",
    crear_release: "Crear release",
    crear_sesion: "Crear sesión",
    abrir_proyecto: "Abrir proyecto",
    cambiar_titulo: "Cambiar título",
    incorporar_material: "Incorporar material externo",
    exportar_stems: "Exportar stems",
    exportar_bounce: "Exportar bounce",
    incorporar_master: "Incorporar máster",
    registrar_mediciones: "Registrar mediciones",
    nueva_version: "Derivar versión nueva",
    ceder_custodia: "Ceder custodia",
    recibir_paquete: "Recibir paquete",
    verificar_paquete: "Verificar paquete",
    acusar_recibo: "Acusar recibo",
    ingerir_paquete: "Ingerir paquete",
    reclamar_retorno: "Reclamar retorno",
    recuperar_custodia: "Recuperar custodia",
    derivar_sobre_cedido: "Derivar proyecto sobre material cedido",
    control_calidad: "Ejecutar control de calidad",
    firmar_informe: "Firmar informe",
    constituir_paquete: "Constituir paquete de entrega",
    emitir_envio: "Emitir envío",
    archivar: "Archivar proyecto",
    verificar_integridad: "Verificar integridad",
    generar_integridad: "Generar manifiesto de integridad",
    conmutar_replica: "Conmutar réplica activa",
    reconciliar: "Reconciliar réplica",
    abrir_en_explorador: "Abrir en el explorador del sistema",
    ver_estructura_completa: "Ver estructura completa",
    volver: "Volver",
    cancelar: "Cancelar",
    aceptar: "Aceptar",
    cerrar: "Cerrar",
    registrar_notas: "Registrar notas",
    incorporar_referencia: "Incorporar referencia",
    declarar_tempo_tonalidad: "Declarar tempo y tonalidad",
    registrar_plan_grabacion: "Registrar plan de grabación",
    registrar_mapa_tempo: "Registrar mapa de tempo",
    preparar_acuerdo_reparto: "Preparar acuerdo de reparto",
    incorporar_tomas: "Incorporar tomas",
    fijar_vocabulario: "Fijar vocabulario de pistas",
    congelar_vocabulario: "Congelar vocabulario",
    incorporar_audio_editado: "Incorporar audio editado",
    generar_hoja_recall: "Generar hoja de recall",
    asignar_identificadores: "Asignar identificadores",
    constituir_paquete_produccion: "Constituir paquete de producción",
    consolidar_sesion: "Consolidar sesión",
    generar_manifiesto_integridad: "Generar manifiesto de integridad",
    vincular_release: "Vincular a release",
    sincronizar_reloj: "Comprobar reloj",
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

  // --- Estados. Vocabulario normativo del apartado 3. ---
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
  estado_proyecto: {
    idea: "idea",
    active: "en curso",
    onhold: "en pausa",
    delivered: "entregado",
    sealed: "sellado",
    archived: "archivado",
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

  // --- Rótulos permanentes de la barra de estado. ---
  rotulo: {
    etapa: "Etapa",
    custodia: "Custodia",
    replica: "Réplica",
    conformidad: "Conformidad",
    nivel: "Nivel",
    reloj: "Reloj",
    artista: "Artista",
    titulo: "Título",
    tipo: "Tipo",
    release: "Release",
    frecuencia: "Frecuencia de muestreo",
    bits: "Profundidad de bits",
    tempo: "Tempo",
    tonalidad: "Tonalidad",
    afinacion: "Afinación de referencia",
    origen: "Punto temporal de origen",
    identificador_interno: "Identificador interno",
    identificador_legible: "Identificador legible",
    ruta: "Ruta",
    presupuesto_ruta: "Presupuesto de ruta",
    envio: "Envío",
    destinatario: "Destinatario",
    retorno_esperado: "Retorno esperado",
    plazo_gracia: "Plazo de gracia",
    posicion: "Posición",
    clase: "Clase",
    tracklist: "Tracklist",
    acciones: "Acciones",
    carpeta: "Carpeta",
    contenido: "Contenido",
  },

  // --- Indicador de conformidad. Tres categorías que no se confunden. ---
  conformidad: {
    conforme: "Conforme",
    pendiente_uno: "1 campo pendiente",
    pendiente_varios: "{n} campos pendientes",
    incumplimiento_uno: "1 incumplimiento",
    incumplimiento_varios: "{n} incumplimientos",
    excepcion_una: "1 excepción declarada",
    excepcion_varias: "{n} excepciones declaradas",
    // Un campo pendiente no es un error y no se presenta en rojo.
    nota_pendiente:
      "Un campo pendiente corresponde a una etapa que no ha concluido. No es un incumplimiento.",
  },

  // --- Creación de proyecto. Cinco campos, nada más. ---
  creacion: {
    titulo: "Crear proyecto",
    campo_artista: "Artista",
    campo_titulo: "Título",
    campo_tipo: "Tipo",
    campo_release: "Release",
    campo_release_ninguno: "Ninguno",
    campo_release_nuevo: "Crear release",
    campo_frecuencia: "Frecuencia de muestreo",
    campo_bits: "Profundidad de bits",
    nota_audio:
      "La frecuencia de muestreo y la profundidad de bits quedan fijadas para todo el ciclo de vida del proyecto y no pueden modificarse después.",
    nota_titulo:
      "El título puede cambiarse mientras el proyecto no se haya enviado ni archivado.",
    nota_campos_pendientes:
      "El tempo, la tonalidad y los créditos se registran cuando se producen.",
  },

  // --- Menú de acciones del proyecto. ---
  menu: {
    titulo: "Acciones",
    sin_acciones: "La etapa activa no admite ninguna acción en este momento.",
    otras_carpetas: "Otras carpetas del proyecto",
    nota_otras_carpetas:
      "La estructura completa del proyecto está accesible desde aquí.",
  },

  // --- Diálogos de acción irreversible. Una frase, sin dramatizar. ---
  irreversible: {
    derivar_titulo: "Derivar versión nueva",
    derivar_aviso:
      "El proyecto de origen pasa a sellado y a solo lectura. La operación es irreversible en la práctica.",
    derivar_detalle:
      "Se copia el proyecto completo, incluidas notas, contratos, referencias, sesiones y parámetros. El proyecto nuevo recibe identificador interno propio y el sufijo _vNN, y declara su ascendencia.",
    derivar_campo_motivo: "Motivo",
    derivar_paralelo: "Continuar el trabajo sobre ambos proyectos en paralelo",
    derivar_paralelo_nota:
      "El proyecto de origen no se sella. Ambos pasan a tratarse como obras independientes a efectos de créditos y de derechos.",

    ceder_titulo: "Ceder custodia",
    ceder_aviso:
      "La copia local queda en solo lectura hasta que la custodia retorne.",
    ceder_detalle:
      "Mientras el estado sea en tránsito, ninguna de las dos partes debe modificar el proyecto.",

    recuperar_titulo: "Recuperar custodia",
    recuperar_aviso:
      "Todo retorno posterior será una versión divergente que habrá que reconciliar a mano.",
    recuperar_detalle:
      "La recuperación abre una no conformidad mayor y notifica al cesionario.",

    archivar_titulo: "Archivar proyecto",
    archivar_aviso:
      "El proyecto se traslada a 30_ARCHIVE y queda en solo lectura.",

    confirmar: "Confirmar",
  },

  // --- Vista de contexto único. ---
  contexto: {
    carpeta_presentada: "Carpeta de la etapa activa",
    vacia: "La carpeta no contiene ningún archivo.",
    archivos_uno: "1 archivo",
    archivos_varios: "{n} archivos",
    condicion_siguiente: "Para pasar a la etapa siguiente",
    nota_cambio_vista:
      "Cambiar de vista no cierra ningún archivo que otro programa tenga abierto.",
  },

  // --- Condiciones de paso de etapa (columna 4 de la Tabla I.1). ---
  condicion: {
    maqueta_disponible: "Maqueta disponible",
    plan_grabacion_cerrado: "Plan de grabación cerrado",
    vocabulario_congelado: "Vocabulario congelado y tomas nombradas",
    edicion_cerrada: "Edición cerrada",
    bounce_aprobado: "Bounce aprobado y stems conformes",
    mediciones_registradas: "Mediciones registradas",
    informe_aprobado: "Informe aprobado",
    envio_emitido_y_acusado: "Envío emitido y acuse recibido",
    custodia_cedida_o_retornada: "Custodia cedida o retornada",
    cuarentena_vacia: "Cuarentena vacía",
    proyecto_en_archivo: "Proyecto en 30_ARCHIVE",
  },

  // --- Mensajes de error. Qué ocurrió, qué consecuencia, qué se puede hacer. ---
  error: {
    // El núcleo redacta sus propios mensajes con los tres elementos. Estos
    // cubren los casos que solo la interfaz conoce.
    sin_repositorio:
      "No hay ningún repositorio abierto. No se ha cargado nada. Elegir una raíz de repositorio o crear una.",
    raiz_sincronizada:
      "La raíz elegida está dentro de una carpeta sincronizada con {servicio}. La raíz no se ha creado. Elegir una ubicación fuera de los servicios de almacenamiento de uso personal.",
    reloj_sin_fuente:
      "No se ha podido consultar ninguna fuente de tiempo de red. La emisión de paquetes y de acuses queda bloqueada. Conectar el equipo a la red y repetir la comprobación.",
    reloj_desviado:
      "El reloj presenta una desviación de {segundos} s respecto de la fuente de tiempo de red. La emisión de paquetes y de acuses queda bloqueada. Corregir el reloj del sistema y repetir la comprobación.",
    archivos_abiertos:
      "{n} archivos del proyecto están abiertos para escritura por otro proceso. La operación no se ha iniciado. Cerrarlos en el programa que los mantiene abiertos.",
    ruta_larga:
      "La ruta más larga mediría {actual} de {limite} caracteres. El nombre no se ha creado. Acortar el título.",
    replica_no_activa:
      "La réplica activa reside en otro volumen. No se ha escrito nada. Conmutar la réplica activa antes de trabajar aquí.",
  },

  // --- Avisos que no son errores. ---
  aviso: {
    reloj_precision_reducida:
      "La hora se obtuvo de una cabecera HTTPS, cuya resolución es de un segundo. La comprobación de la tolerancia es menos precisa que con una fuente de tiempo de red.",
    presupuesto_ruta:
      "{actual} de {limite} caracteres.",
    volumen_sin_solo_lectura:
      "El sistema de archivos de este volumen no sostiene el atributo de solo lectura. La protección descansa en el archivo marcador y es más débil.",
    cuarentena_con_atraso:
      "{n} elementos llevan en la cuarentena sin clasificar. La cuarentena debe vaciarse al menos una vez por semana.",
    operacion_a_medias:
      "{n} operaciones quedaron sin concluir en el cierre anterior.",
    replica_divergente:
      "La réplica reconectada contiene {n} archivos con cambios no presentes en la activa. La réplica queda marcada divergente y no se sobrescribe. La reconciliación es manual.",
    cesion_vencida:
      "La cesión venció el {fecha}. Procede reclamar el retorno.",
    cesion_gracia_vencida:
      "La cesión venció el {fecha} y su plazo de gracia. Procede la recuperación forzosa.",
  },

  // --- Progreso de operaciones largas. ---
  progreso: {
    de: "{hecho} de {total} archivos",
    empaquetando: "Constituyendo el paquete",
    verificando: "Verificando la integridad",
    extrayendo: "Extrayendo el contenedor",
    copiando: "Copiando el material",
    cancelar: "Cancelar",
    cancelado:
      "La operación se canceló. No se ha dejado ningún archivo en el destino.",
  },

  // --- Recepción de paquetes. ---
  recepcion: {
    titulo: "Recepción",
    cuarentena: "Cuarentena",
    sin_paquetes: "La cuarentena no contiene ningún paquete.",
    resultado_accepted: "Aceptado",
    resultado_accepted_with_reservations: "Aceptado con reservas",
    resultado_rejected: "Rechazado",
    verificacion: "Verificaciones previas a la ingesta",
    pass: "conforme",
    fail: "no conforme",
    not_applicable: "no procede",
    discrepancias: "Discrepancias",
    aceptar_custodia: "Aceptar la custodia que el envío cede",
  },

  // --- Nombres de las catorce verificaciones (Tabla 31). ---
  verificacion: {
    provenance: "Procedencia",
    authenticity: "Autenticidad",
    container: "Contenedor",
    package_integrity: "Integridad del empaquetado",
    content_integrity: "Integridad del contenido",
    manifest_schema: "Validez del manifiesto",
    profile_supported: "Perfil y versión",
    custody: "Custodia",
    chronology: "Cronología",
    scope_match: "Correspondencia del alcance",
    usage_accepted: "Condiciones de uso y retención",
    personal_data_match: "Datos personales",
    structure: "Conformidad estructural",
    declared_checks: "Verificaciones declaradas",
  },

  // --- Preferencias. ---
  preferencias: {
    titulo: "Preferencias",
    tema: "Tema",
    tema_sistema: "Del sistema",
    tema_claro: "Claro",
    tema_oscuro: "Oscuro",
    identidad: "Identidad",
    organizacion: "Organización",
    persona: "Persona",
    raiz_local: "Raíz local",
    conformidad: "Declaración de conformidad",
  },

  // --- Vacíos. Sin frases de relleno. ---
  vacio: {
    sin_proyectos: "El repositorio no contiene ningún proyecto.",
    sin_releases: "El repositorio no contiene ningún release.",
    sin_eventos: "El registro no contiene ninguna entrada.",
    sin_hallazgos: "La validación no encontró ningún hallazgo.",
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
