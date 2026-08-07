// Interfaz de Attacca.
//
// Ningún texto visible se escribe aquí: todo procede de `textos.js`, para que
// las reglas de redacción se puedan auditar en un solo archivo.
//
// La decisión central sigue siendo la del apartado 44.2: en cada momento se
// presenta la carpeta correspondiente a la fase activa del proyecto y las
// acciones admisibles en ella, y la fase la deduce el núcleo del manifiesto y
// del contenido real, nunca una preferencia guardada aquí.
//
// Lo que cambia respecto de la versión anterior es el reparto de la atención.
// Antes la pantalla de un proyecto mostraba cinco paneles del mismo peso, uno
// de ellos una lista de apartados de la norma. Ahora muestra una frase con lo
// que toca hacer, un botón para hacerlo, los archivos de la fase, y todo lo
// demás plegado. La información normativa no se ha quitado: está en la sección
// de detalle, entera y con su vocabulario, a un clic de distancia.

import { T, fmt, plural } from "./textos.js";

const invoke = window.__TAURI__.core.invoke;
const dialogo = window.__TAURI__.dialog;

// --- Estado de la interfaz. Nada de esto es normativo. ---
const ui = {
  ruta: "proyectos", // proyectos | proyecto | recibido | historial | ajustes
  proyecto: null,
  carpeta: "",
  catalogo: null,
  reloj: null,
  repositorio: null,
};

// El recorrido habitual de un tema, en orden. Las dos fases que quedan fuera
// —el envío a otro estudio y el material recibido— no pertenecen a esta línea
// y se presentan solas.
const RECORRIDO = [
  "composicion",
  "preproduccion",
  "grabacion",
  "edicion",
  "mezcla",
  "mastering",
  "control_calidad",
  "distribucion",
  "archivo",
];

const $ = (sel) => document.querySelector(sel);
const vista = () => $("#vista");

// --- Construcción de nodos, sin plantillas de cadena ---

function el(etiqueta, atributos = {}, hijos = []) {
  const n = document.createElement(etiqueta);
  for (const [k, v] of Object.entries(atributos)) {
    if (v === null || v === undefined || v === false) continue;
    if (k === "clase") n.className = v;
    else if (k === "texto") n.textContent = v;
    else if (k.startsWith("on")) n.addEventListener(k.slice(2), v);
    else n.setAttribute(k, v === true ? "" : String(v));
  }
  for (const h of [].concat(hijos)) {
    if (h === null || h === undefined || h === false) continue;
    n.appendChild(typeof h === "string" ? document.createTextNode(h) : h);
  }
  return n;
}

function boton(texto, alPulsar, opciones = {}) {
  const hijos = [el("span", { texto })];
  if (opciones.atajo) hijos.push(el("kbd", { texto: opciones.atajo }));
  return el(
    "button",
    {
      clase: opciones.clase || "",
      onclick: alPulsar,
      disabled: opciones.desactivado,
      "aria-pressed": opciones.pulsado === undefined ? null : String(opciones.pulsado),
      "aria-current": opciones.actual ? "page" : null,
    },
    hijos
  );
}

function marca(texto, clase = "") {
  return el("span", { clase: "marca-estado " + clase, texto });
}

function campo(etiqueta, control, nota) {
  const id = "c" + Math.random().toString(36).slice(2, 8);
  control.id = id;
  return el("div", { clase: "campo" }, [
    el("label", { for: id, texto: etiqueta }),
    control,
    nota ? el("p", { clase: "nota", texto: nota }) : null,
  ]);
}

function campoMarca(etiqueta, control, nota) {
  return el("div", { clase: "campo campo-marca" }, [
    el("label", {}, [control, el("span", { texto: etiqueta })]),
    nota ? el("p", { clase: "nota", texto: nota }) : null,
  ]);
}

function seleccion(opciones, valorInicial) {
  const s = el("select");
  for (const [valor, texto] of opciones) {
    s.appendChild(el("option", { value: valor, texto, selected: valor === valorInicial }));
  }
  return s;
}

/// Opciones a partir de un bloque de textos, con el código fuera de la vista.
function opcionesDe(bloque, orden) {
  return orden.map((clave) => [clave, bloque[clave] || clave]);
}

function bytesLegibles(n) {
  if (n < 1024) return n + " B";
  const u = ["kB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i += 1;
  }
  return v.toFixed(v < 10 ? 1 : 0) + " " + u[i];
}

// --- Llamadas al núcleo ---

async function llamar(orden, argumentos) {
  try {
    return await invoke(orden, argumentos);
  } catch (e) {
    avisar({ texto: String(e), tono: "malo" });
    return null;
  }
}

/// Un aviso de una sola frase. El tono decide el color, no el texto.
function avisar({ titulo, texto, tono = "neutro" }) {
  const d = $("#diálogo");
  d.replaceChildren(
    el("h2", { texto: titulo || T.app.nombre }),
    el("div", { clase: "aviso " + (tono === "neutro" ? "bueno" : tono) }, [
      el("p", { texto }),
    ]),
    el("div", { clase: "pie" }, [
      boton(T.accion.cerrar, () => d.close(), { clase: "principal" }),
    ])
  );
  d.showModal();
}

function confirmar({ titulo, aviso, detalle, etiquetaConfirmar, campos = [] }) {
  return new Promise((resolver) => {
    const d = $("#diálogo");
    d.replaceChildren(
      el("h2", { texto: titulo }),
      // Lo irreversible se anuncia en una frase, antes de ejecutarlo.
      aviso ? el("div", { clase: "aviso" }, [el("p", { texto: aviso })]) : null,
      detalle ? el("p", { clase: "nota", texto: detalle }) : null,
      ...campos.map((c) => c.nodo),
      el("div", { clase: "pie" }, [
        boton(T.accion.cancelar, () => {
          d.close();
          resolver(null);
        }),
        boton(
          etiquetaConfirmar || T.irreversible.confirmar,
          () => {
            const valores = {};
            for (const c of campos) valores[c.clave] = c.leer();
            d.close();
            resolver(valores);
          },
          { clase: "principal" }
        ),
      ])
    );
    d.showModal();
    const primero = d.querySelector("input, select, textarea");
    if (primero) primero.focus();
  });
}

// --- Barra permanente ---
//
// El apartado 44.2 exige que la fase, el estado de custodia y el de réplica se
// consulten sin ninguna acción. Aquí están los tres, en llano y sin nada más
// que compita con ellos.

function pintarSituacion() {
  const f = $("#situacion");
  const p = ui.proyecto;
  const partes = [];

  if (p) {
    partes.push(dato(T.rotulo.fase, T.fase[p.etapa] || p.etapa));
    partes.push(dato(T.rotulo.donde, T.donde[p.resumen.custodia] || p.resumen.custodia));
    const rep = p.replicas.find((r) => r.volumen === p.replica_activa);
    partes.push(dato(T.rotulo.disco, rep ? T.disco[rep.estado] : T.disco.activa));
  }

  // El reloj solo se nombra cuando impide algo. Un dato correcto no necesita
  // ocupar sitio permanente.
  if (ui.reloj && !ui.reloj.admite_emision) {
    partes.push(
      el("span", {
        clase: "aviso",
        texto:
          ui.reloj.estado === "unavailable"
            ? T.ajustes.hora_sin
            : fmt(T.ajustes.hora_mal, { s: Math.round(ui.reloj.desviacion_ms / 1000) }),
      })
    );
  }

  f.replaceChildren(...partes);
}

function dato(clave, valor) {
  return el("span", { clase: "dato" }, [
    el("span", { clase: "clave", texto: clave }),
    el("span", { clase: "valor", texto: valor }),
  ]);
}

// --- Navegación ---

function pintarNav() {
  const n = $("#nav");
  const pendientes = ui.catalogo ? ui.catalogo.paquetes_en_cuarentena : 0;

  const recibido = boton(T.seccion.recibido, () => ir("recibido"), {
    clase: "tenue",
    actual: ui.ruta === "recibido",
  });
  if (pendientes > 0) {
    recibido.appendChild(el("span", { clase: "cuenta", texto: String(pendientes) }));
  }

  n.replaceChildren(
    boton(T.seccion.proyectos, () => ir("proyectos"), {
      clase: "tenue",
      actual: ui.ruta === "proyectos" || ui.ruta === "proyecto",
    }),
    recibido,
    boton(T.seccion.historial, () => ir("historial"), {
      clase: "tenue",
      actual: ui.ruta === "historial",
    }),
    boton(T.seccion.ajustes, () => ir("ajustes"), {
      clase: "tenue",
      actual: ui.ruta === "ajustes",
    })
  );
}

async function ir(ruta, argumento) {
  ui.ruta = ruta;
  if (ruta === "proyecto" && argumento) {
    ui.proyecto = await llamar("ver_proyecto", { uid: argumento });
    ui.carpeta = ui.proyecto ? ui.proyecto.carpetas_etapa[0] || "" : "";
  }
  if (ruta !== "proyecto") ui.proyecto = null;
  await pintar();
  vista().focus();
}

async function pintar() {
  const v = vista();
  const contenido = await {
    proyectos: pintarProyectos,
    proyecto: pintarProyecto,
    recibido: pintarRecibido,
    historial: pintarHistorial,
    ajustes: pintarAjustes,
  }[ui.ruta]();
  v.replaceChildren(el("div", { clase: "limite" }, [contenido]));
  pintarNav();
  pintarSituacion();
}

// --- Lista de proyectos ---

async function pintarProyectos() {
  ui.catalogo = await llamar("catalogo");
  const c = ui.catalogo;
  const cont = el("div");

  cont.appendChild(
    el("div", { clase: "portada" }, [
      el("div", {}, [el("h1", { texto: T.seccion.proyectos })]),
      el("div", { clase: "derecha" }, [
        boton(T.accion.crear_release, dialogoCrearLanzamiento),
        boton(T.accion.crear_proyecto, dialogoCrearProyecto, {
          clase: "principal",
          atajo: T.atajo.crear_proyecto,
        }),
      ]),
    ])
  );

  if (!c) return cont;

  if (c.atraso_cuarentena > 0) {
    cont.appendChild(
      el("div", { clase: "aviso" }, [
        el("p", { texto: fmt(T.recibido.atraso, { n: c.atraso_cuarentena }) }),
      ])
    );
  }

  // Un manifiesto ilegible no se oculta: es lo único que puede dejar material
  // fuera del alcance de la aplicación.
  if (c.ilegibles.length > 0) {
    cont.appendChild(
      el("div", { clase: "aviso malo" }, [
        el("p", {
          texto: plural(
            c.ilegibles.length,
            "Hay 1 carpeta que no se ha podido leer.",
            "Hay {n} carpetas que no se han podido leer."
          ),
        }),
        el("p", { clase: "mono", texto: c.ilegibles.map(([r]) => r).join("  ") }),
      ])
    );
  }

  if (c.proyectos.length === 0) {
    cont.appendChild(
      el("div", { clase: "vacio" }, [
        el("p", { clase: "principal-texto", texto: T.vacio.sin_proyectos }),
        el("p", { texto: T.vacio.primer_proyecto }),
      ])
    );
    return cont;
  }

  const rejilla = el("div", { clase: "rejilla" });
  for (const p of c.proyectos) {
    rejilla.appendChild(tarjetaProyecto(p));
  }
  cont.appendChild(rejilla);

  if (c.releases.length > 0) {
    cont.appendChild(el("h2", { clase: "rotulo", texto: T.seccion.lanzamientos }));
    const rej = el("div", { clase: "rejilla" });
    for (const r of c.releases) {
      rej.appendChild(
        el("div", { clase: "proyecto" }, [
          el("div", { clase: "nombre", texto: r.titulo }),
          el("div", { clase: "artista", texto: r.artista }),
          el("div", { clase: "pie" }, [
            marca(T.clase_lanzamiento[r.clase] || r.clase, "liso"),
            el("span", {
              clase: "artista cifra",
              texto: plural(r.temas, "1 tema", "{n} temas"),
            }),
          ]),
        ])
      );
    }
    cont.appendChild(rej);
  }

  return cont;
}

function tarjetaProyecto(p) {
  const abrir = () => ir("proyecto", p.uid);
  return el(
    "button",
    {
      clase: "proyecto",
      onclick: abrir,
      title: p.titulo || p.id,
    },
    [
      el("div", { clase: "nombre", texto: p.titulo || p.id }),
      el("div", { clase: "artista", texto: p.artista }),
      el("div", { clase: "pie" }, marcasDeProyecto(p)),
    ]
  );
}

/// Una o dos marcas por proyecto. La primera dice dónde está el trabajo; la
/// segunda solo aparece cuando hay algo que atender.
function marcasDeProyecto(p) {
  const out = [];
  if (p.custodia !== "propia") {
    out.push(marca(T.donde[p.custodia] || p.custodia, "pendiente"));
  } else if (p.estado === "sealed" || p.estado === "archived" || p.estado === "delivered") {
    out.push(marca(T.situacion[p.estado] || p.estado, "liso"));
  } else {
    out.push(marca(T.fase[p.etapa] || p.etapa, "fase"));
  }
  if (p.estado === "onhold") out.push(marca(T.situacion.onhold, "liso"));
  return out;
}

// --- Crear proyecto ---
//
// La calidad de grabación era antes dos desplegables de números. Es un solo
// campo con cuatro opciones nombradas por su uso, porque quien graba elige por
// el uso y no por la cifra. La cifra sigue estando, detrás de la coma.

const CALIDADES = [
  ["48000-24", "calidad_estandar"],
  ["96000-24", "calidad_alta"],
  ["48000-32", "calidad_cine"],
  ["44100-24", "calidad_cd"],
];

async function dialogoCrearProyecto() {
  const artista = el("input", { type: "text", autofocus: true });
  const titulo = el("input", { type: "text" });
  const tipo = seleccion(
    opcionesDe(T.tipo, [
      "ORIG",
      "COVER",
      "REMIX",
      "BEAT",
      "MIX",
      "MST",
      "SD",
      "LIVE",
      "DEMO",
      "SYNC",
    ]),
    "ORIG"
  );
  const calidad = seleccion(
    CALIDADES.map(([v, clave]) => [v, T.creacion[clave]]),
    "48000-24"
  );
  const releases = ui.catalogo ? ui.catalogo.releases : [];
  const release = seleccion(
    [["", T.creacion.campo_release_ninguno]].concat(releases.map((r) => [r.uid, r.titulo])),
    ""
  );

  const campos = [
    { clave: "artista", nodo: campo(T.creacion.campo_artista, artista), leer: () => artista.value },
    {
      clave: "titulo",
      nodo: campo(T.creacion.campo_titulo, titulo, T.creacion.nota_titulo),
      leer: () => titulo.value,
    },
    { clave: "tipo", nodo: campo(T.creacion.campo_tipo, tipo), leer: () => tipo.value },
    {
      clave: "calidad",
      nodo: campo(T.creacion.campo_calidad, calidad, T.creacion.nota_calidad),
      leer: () => calidad.value,
    },
  ];
  // El lanzamiento solo se pregunta cuando hay alguno al que vincular.
  if (releases.length > 0) {
    campos.push({
      clave: "release",
      nodo: campo(T.creacion.campo_release, release),
      leer: () => release.value,
    });
  }

  const r = await confirmar({
    titulo: T.creacion.titulo,
    detalle: T.creacion.nota_resto,
    etiquetaConfirmar: T.accion.crear_proyecto,
    campos,
  });
  if (!r) return;

  const [frecuencia, bits] = r.calidad.split("-").map(Number);
  const p = await llamar("crear_proyecto", {
    datos: {
      titulo: r.titulo,
      artista: r.artista,
      tipo: r.tipo,
      nivel: "B",
      frecuencia,
      bits,
      release_uid: r.release || null,
    },
  });
  if (p) {
    ui.proyecto = p;
    ui.carpeta = p.carpetas_etapa[0] || "";
    ui.ruta = "proyecto";
    await pintar();
  }
}

async function dialogoCrearLanzamiento() {
  const artista = el("input", { type: "text", autofocus: true });
  const titulo = el("input", { type: "text" });
  const clase = seleccion(
    opcionesDe(T.clase_lanzamiento, ["SINGLE", "EP", "ALBUM", "COMP", "LIVE", "SYNC"]),
    "SINGLE"
  );
  const r = await confirmar({
    titulo: T.accion.crear_release,
    etiquetaConfirmar: T.accion.crear_release,
    campos: [
      { clave: "artista", nodo: campo(T.creacion.campo_artista, artista), leer: () => artista.value },
      { clave: "titulo", nodo: campo(T.creacion.campo_titulo, titulo), leer: () => titulo.value },
      { clave: "clase", nodo: campo(T.rotulo.tipo, clase), leer: () => clase.value },
    ],
  });
  if (!r) return;
  const uid = await llamar("crear_release", {
    datos: { titulo: r.titulo, artista: r.artista, clase: r.clase },
  });
  if (uid) await pintar();
}

// --- Vista de un proyecto ---

async function pintarProyecto() {
  const p = ui.proyecto;
  if (!p) return el("p", { clase: "sin-nada", texto: T.vacio.sin_proyectos });

  const cont = el("div");

  cont.appendChild(
    el("div", { clase: "portada" }, [
      el("div", {}, [
        el("h1", { texto: p.resumen.titulo || p.resumen.id }),
        el("p", { clase: "sub", texto: p.resumen.artista }),
      ]),
      el("div", { clase: "derecha" }, [
        boton(T.accion.volver, () => ir("proyectos"), { clase: "tenue", atajo: T.atajo.volver }),
        boton(
          T.accion.abrir_en_explorador,
          () => llamar("abrir_en_explorador", { uid: p.resumen.uid, relativa: ui.carpeta }),
          { atajo: T.atajo.abrir_en_explorador }
        ),
      ]),
    ])
  );

  cont.appendChild(recorridoDeFases(p));

  const avisoPlazo = avisoDePlazo(p);
  if (avisoPlazo) cont.appendChild(avisoPlazo);

  const izquierda = el("div");
  izquierda.appendChild(panelAhora(p));
  izquierda.appendChild(await panelArchivos(p));

  const derecha = el("div");
  const falta = panelFalta(p);
  if (falta) derecha.appendChild(falta);
  derecha.appendChild(panelOtrasCarpetas(p));

  cont.appendChild(el("div", { clase: "doble" }, [izquierda, derecha]));
  cont.appendChild(detalleTecnico(p));
  return cont;
}

/// La fase, como un recorrido. Reemplaza a la palabra suelta «Etapa: mezcla»:
/// dice también qué ha quedado atrás y qué viene.
function recorridoDeFases(p) {
  const cont = el("div", { clase: "recorrido" });
  const actual = RECORRIDO.indexOf(p.etapa);

  if (actual < 0) {
    // Fase fuera del recorrido habitual: se presenta sola.
    cont.appendChild(
      el("span", { clase: "hito actual" }, [
        el("span", { clase: "punto" }),
        el("span", { texto: T.fase[p.etapa] || p.etapa }),
      ])
    );
    return cont;
  }

  RECORRIDO.forEach((clave, i) => {
    if (i > 0) cont.appendChild(el("span", { clase: "union" }));
    const estado = i < actual ? "hecho" : i === actual ? "actual" : "";
    cont.appendChild(
      el("span", { clase: "hito " + estado }, [
        el("span", { clase: "punto" }),
        el("span", { texto: T.fase[clave] }),
      ])
    );
  });
  return cont;
}

function avisoDePlazo(p) {
  if (!p.vencimiento || p.vencimiento.situacion === "current") return null;
  const texto =
    p.vencimiento.situacion === "reclaim_available" ? T.plazo.margen_vencido : T.plazo.vencido;
  return el("div", { clase: "aviso" }, [
    el("p", { texto: fmt(texto, { fecha: p.vencimiento.fecha }) }),
  ]);
}

/// Lo que la fase pide, con la acción principal debajo y el resto en voz baja.
function panelAhora(p) {
  const principal = p.acciones[0];
  const resto = p.acciones.slice(1).filter((a) => T.accion[a]);

  const secundarias = el("div", { clase: "tambien" });
  for (const clave of resto) {
    secundarias.appendChild(
      boton(T.accion[clave], () => ejecutar(clave, p), {
        clase: "tenue",
        atajo: T.atajo[clave],
      })
    );
  }

  return el("section", { clase: "tarjeta ahora" }, [
    el("h2", { clase: "rotulo", texto: T.seccion.ahora }),
    el("p", { clase: "frase", texto: T.ahora[p.etapa] || "" }),
    principal && T.accion[principal]
      ? boton(T.accion[principal], () => ejecutar(principal, p), {
          clase: "principal grande",
          atajo: T.atajo[principal],
        })
      : el("p", { clase: "sin-nada", texto: T.vacio.sin_acciones }),
    resto.length > 0 ? secundarias : null,
    el("p", {
      clase: "nota",
      texto: T.paso.titulo + ": " + (T.paso[p.condicion_siguiente] || p.condicion_siguiente),
    }),
  ]);
}

async function panelArchivos(p) {
  const entradas =
    (await llamar("listar_carpeta", { uid: p.resumen.uid, relativa: ui.carpeta })) || [];

  const pestanas = el("div", { clase: "pestanas" });
  for (const carpeta of p.carpetas_etapa) {
    pestanas.appendChild(
      boton(
        nombreLlanoDeCarpeta(carpeta),
        () => {
          ui.carpeta = carpeta;
          pintar();
        },
        { pulsado: ui.carpeta === carpeta }
      )
    );
  }

  const lista = el("ul", { clase: "navegador" });
  if (ui.carpeta && ui.carpeta.includes("/")) {
    lista.appendChild(
      el(
        "li",
        {
          tabindex: "0",
          onclick: subirUnNivel,
          onkeydown: (e) => e.key === "Enter" && subirUnNivel(),
        },
        [
          el("span", { clase: "icono", texto: "↑" }),
          el("span", { clase: "nombre", texto: T.archivos.subir }),
        ]
      )
    );
  }
  for (const e of entradas) {
    lista.appendChild(
      el(
        "li",
        e.es_carpeta
          ? {
              tabindex: "0",
              onclick: () => {
                ui.carpeta = e.ruta;
                pintar();
              },
              onkeydown: (ev) => {
                if (ev.key !== "Enter") return;
                ui.carpeta = e.ruta;
                pintar();
              },
            }
          : {},
        [
          el("span", { clase: "icono", texto: e.es_carpeta ? "▸" : "" }),
          el("span", { clase: "nombre", texto: e.nombre }),
          el("span", {
            clase: "tamano",
            texto: e.es_carpeta ? "" : bytesLegibles(e.tamano),
          }),
        ]
      )
    );
  }

  return el("section", { clase: "tarjeta" }, [
    el("h2", { clase: "rotulo", texto: T.seccion.archivos }),
    p.carpetas_etapa.length > 1 ? pestanas : null,
    entradas.length === 0 ? el("p", { clase: "sin-nada", texto: T.archivos.vacia }) : lista,
    entradas.length > 0
      ? el("p", {
          clase: "nota",
          texto: plural(entradas.length, T.archivos.uno, T.archivos.varios),
        })
      : null,
  ]);
}

function subirUnNivel() {
  ui.carpeta = ui.carpeta.split("/").slice(0, -1).join("/");
  pintar();
}

/// Las carpetas de la norma se llaman `05_STEMS` en el disco, y así deben
/// seguir llamándose. Lo que se enseña es su lectura en llano; la que no esté
/// en la tabla se muestra sin el número, que es lo único que sobra al leerla.
function nombreLlanoDeCarpeta(ruta) {
  return ruta
    .split("/")
    .map((t) => T.carpeta[t] || t.replace(/^\d+_/, "").replace(/_/g, " "))
    .join(" › ");
}

/// Lo que falta, en llano. Los apartados de la norma quedan en el detalle.
function panelFalta(p) {
  const c = p.conformidad;
  if (c.hallazgos.length === 0) return null;

  const lista = el("ul", { clase: "lista-falta" });
  for (const h of c.hallazgos) {
    const clase =
      h.severidad === "pendiente"
        ? "pendiente"
        : h.severidad === "excepcion"
          ? "salvedad"
          : "problema";
    const etiqueta =
      h.severidad === "pendiente"
        ? T.falta.etiqueta_pendiente
        : h.severidad === "excepcion"
          ? T.falta.etiqueta_salvedad
          : T.falta.etiqueta_problema;
    lista.appendChild(
      el("li", {}, [marca(etiqueta, clase), el("span", { clase: "texto", texto: h.detalle })])
    );
  }

  return el("section", { clase: "tarjeta" }, [
    el("h2", { clase: "rotulo", texto: T.seccion.pendiente }),
    lista,
    c.pendientes > 0 ? el("p", { clase: "nota", texto: T.falta.nota_pendiente }) : null,
  ]);
}

/// El resto de la estructura sigue alcanzable, fuera del recorrido principal,
/// como exige el apartado 44.2.
function panelOtrasCarpetas(p) {
  const otras = p.carpetas_existentes.filter((c) => !p.carpetas_etapa.includes(c));
  const cont = el("div", { clase: "pila" });
  for (const carpeta of otras) {
    cont.appendChild(
      boton(nombreLlanoDeCarpeta(carpeta), () => {
        ui.carpeta = carpeta;
        pintar();
      })
    );
  }
  return el("section", { clase: "tarjeta" }, [
    el("h2", { clase: "rotulo", texto: T.seccion.otras_carpetas }),
    otras.length === 0 ? el("p", { clase: "sin-nada", texto: T.vacio.sin_carpetas }) : cont,
    el("p", { clase: "nota", texto: T.archivos.nota_abrir }),
  ]);
}

// --- Detalle técnico ---
//
// Nada de lo que la versión anterior enseñaba se ha perdido: está aquí, con el
// vocabulario del apartado 3 de la norma, para poder citarlo y compararlo con
// otra implementación. Lo que ha cambiado es que ya no compite por la atención
// de quien solo quiere grabar.

function detalleTecnico(p) {
  const K = T.tecnico;
  const filas = [
    [K.rotulo.identificador_legible, p.resumen.id, "mono"],
    [K.rotulo.identificador_interno, p.resumen.uid, "mono"],
    [K.rotulo.etapa, K.etapa[p.etapa] || p.etapa],
    [K.rotulo.custodia, K.custodia[p.resumen.custodia] || p.resumen.custodia],
    [K.rotulo.nivel, p.resumen.nivel],
    [
      K.rotulo.presupuesto_ruta,
      fmt(K.presupuesto, {
        actual: p.presupuesto_ruta.actual,
        limite: p.presupuesto_ruta.limite,
      }),
    ],
    [K.rotulo.frecuencia, p.audio.frecuencia ? p.audio.frecuencia + " Hz" : "—"],
    [K.rotulo.bits, p.audio.bits ? p.audio.bits + " bits" : "—"],
    [T.rotulo.tempo, p.audio.tempo != null ? String(p.audio.tempo) : "—"],
    [T.rotulo.tonalidad, p.audio.tonalidad || "—"],
    [K.rotulo.afinacion, p.audio.afinacion ? p.audio.afinacion + " Hz" : "—"],
    [K.rotulo.origen, p.audio.origen || "—"],
    [K.rotulo.ruta, p.ruta_absoluta, "mono"],
  ];

  for (const r of p.replicas) {
    filas.push([K.rotulo.replica, (K.replica[r.estado] || r.estado) + "  " + r.ruta]);
  }

  const tbody = el("tbody");
  for (const [clave, valor, clase] of filas) {
    tbody.appendChild(
      el("tr", {}, [el("th", { texto: clave }), el("td", { clase: clase || "", texto: valor })])
    );
  }

  const cuerpo = el("div", { clase: "cuerpo" }, [
    el("p", { clase: "nota", texto: K.nota }),
    el("table", {}, [tbody]),
  ]);

  // Los hallazgos con su apartado, que es la forma en que la norma los nombra.
  if (p.conformidad.hallazgos.length > 0) {
    const tb = el("tbody");
    for (const h of p.conformidad.hallazgos) {
      tb.appendChild(
        el("tr", {}, [
          el("th", { clase: "mono", texto: h.clausula }),
          el("td", {}, [
            marca(K.conformidad[h.severidad] || h.severidad, "liso"),
            el("span", { texto: " " + h.detalle }),
          ]),
        ])
      );
    }
    cuerpo.appendChild(el("h2", { clase: "rotulo", texto: K.rotulo.conformidad }));
    cuerpo.appendChild(el("table", {}, [tb]));
  }

  // Acciones que solo tienen sentido con el detalle a la vista.
  const acciones = el("div", { clase: "tambien" }, [
    boton(T.accion.verificar_integridad, () => ejecutar("verificar_integridad", p), {
      atajo: T.atajo.verificar_integridad,
    }),
    boton(T.accion.cambiar_titulo, () => dialogoCambiarTitulo(p)),
  ]);
  if (p.audio_modificable) {
    acciones.appendChild(
      boton(T.accion.declarar_tempo_tonalidad, () => dialogoAudio(p))
    );
  }
  cuerpo.appendChild(acciones);

  return el("details", { clase: "detalle" }, [
    el("summary", { texto: T.seccion.detalle }),
    cuerpo,
  ]);
}

// --- Ejecución de acciones ---

async function ejecutar(clave, p) {
  const uid = p.resumen.uid;
  switch (clave) {
    case "abrir_en_explorador":
      return llamar("abrir_en_explorador", { uid, relativa: ui.carpeta });

    case "crear_sesion":
      return dialogoCrearSesion(p);

    case "nueva_version":
      return dialogoDerivar(p);

    case "ceder_custodia":
      return dialogoDejarProyecto(p);

    case "recuperar_custodia":
      return dialogoRecuperar(p);

    case "reclamar_retorno": {
      const r = await llamar("reclamar_retorno", { uid });
      if (r) await ir("proyecto", uid);
      return;
    }

    case "derivar_sobre_cedido": {
      const d = await llamar("derivar_sobre_cedido", { uid });
      if (d) await ir("proyecto", d.uid_derivado);
      return;
    }

    case "verificar_integridad": {
      const r = await llamar("verificar_integridad", { uid });
      if (!r) return;
      if (r.fallidos === 0) {
        avisar({ texto: fmt(T.aviso.integridad_bien, { n: r.comprobados }), tono: "bueno" });
      } else {
        avisar({
          texto: fmt(T.error.integridad_mal, {
            fallidos: r.fallidos,
            total: r.comprobados,
            rutas: r.rutas.join(", "),
          }),
          tono: "malo",
        });
      }
      return;
    }

    case "generar_manifiesto_integridad": {
      const n = await llamar("generar_integridad", { uid });
      if (n !== null) await ir("proyecto", uid);
      return;
    }

    case "incorporar_material":
      return dialogoAnadirMaterial(p);

    case "exportar_stems":
      return dialogoSacar(p, "05_STEMS");
    case "exportar_bounce":
      return dialogoSacar(p, "06_MIX");
    case "incorporar_master":
      return dialogoSacar(p, "07_MASTER");

    case "constituir_paquete_entrega":
    case "emitir_envio":
    case "constituir_paquete_produccion":
      return dialogoEnviar(p);

    case "declarar_tempo_tonalidad":
      return dialogoAudio(p);

    default: {
      // Las acciones que consisten en dejar archivos en su sitio abren la
      // carpeta correspondiente y no piden nada más.
      const carpeta = p.carpetas_etapa[0] || "";
      ui.carpeta = carpeta;
      await pintar();
      return llamar("abrir_en_explorador", { uid, relativa: carpeta });
    }
  }
}

async function dialogoCambiarTitulo(p) {
  const titulo = el("input", { type: "text", value: p.resumen.titulo, autofocus: true });
  const r = await confirmar({
    titulo: T.accion.cambiar_titulo,
    detalle: T.creacion.nota_titulo,
    etiquetaConfirmar: T.accion.guardar,
    campos: [{ clave: "titulo", nodo: campo(T.rotulo.titulo, titulo), leer: () => titulo.value }],
  });
  if (!r) return;
  const v = await llamar("cambiar_titulo", { uid: p.resumen.uid, titulo: r.titulo });
  if (v) {
    ui.proyecto = v;
    await pintar();
  }
}

async function dialogoAudio(p) {
  const tempo = el("input", { type: "number", value: p.audio.tempo ?? "", min: "1" });
  const tonalidad = el("input", { type: "text", value: p.audio.tonalidad ?? "" });
  const afinacion = el("input", { type: "number", value: p.audio.afinacion ?? 440 });
  const r = await confirmar({
    titulo: T.accion.declarar_tempo_tonalidad,
    detalle: T.creacion.nota_resto,
    etiquetaConfirmar: T.accion.guardar,
    campos: [
      {
        clave: "tempo",
        nodo: campo(T.rotulo.tempo, tempo),
        leer: () => (tempo.value ? Number(tempo.value) : null),
      },
      {
        clave: "tonalidad",
        nodo: campo(T.rotulo.tonalidad, tonalidad),
        leer: () => tonalidad.value || null,
      },
      {
        clave: "afinacion",
        nodo: campo(T.rotulo.afinacion, afinacion),
        leer: () => (afinacion.value ? Number(afinacion.value) : null),
      },
    ],
  });
  if (!r) return;
  const v = await llamar("registrar_audio", {
    uid: p.resumen.uid,
    datos: { ...r, origen: p.audio.origen },
  });
  if (v) {
    ui.proyecto = v;
    await pintar();
  }
}

async function dialogoCrearSesion(p) {
  const daw = el("input", { type: "text", value: "Reaper", autofocus: true });
  const para = seleccion(
    opcionesDe(T.sesion, ["COMP", "TRACK", "EDIT", "MIX", "MST", "SD"]),
    "TRACK"
  );
  const r = await confirmar({
    titulo: T.accion.crear_sesion,
    etiquetaConfirmar: T.accion.crear_sesion,
    campos: [
      { clave: "daw", nodo: campo(T.sesion.campo_programa, daw), leer: () => daw.value },
      { clave: "etapa", nodo: campo(T.sesion.campo_para, para), leer: () => para.value },
    ],
  });
  if (!r) return;
  const c = await llamar("crear_sesion", { uid: p.resumen.uid, daw: r.daw, etapa: r.etapa });
  if (!c) return;
  // Se abre esa carpeta, y solo esa, para guardar ahí desde el programa.
  await llamar("abrir_en_explorador", {
    uid: p.resumen.uid,
    relativa: "02_SESSIONS/" + r.daw,
  });
  avisar({ texto: fmt(T.sesion.nombre_sugerido, { nombre: c.nombre_sugerido }), tono: "bueno" });
  await ir("proyecto", p.resumen.uid);
}

async function dialogoDerivar(p) {
  const motivo = el("input", { type: "text", autofocus: true });
  const paralelo = el("input", { type: "checkbox" });
  const r = await confirmar({
    titulo: T.irreversible.derivar_titulo,
    aviso: T.irreversible.derivar_aviso,
    detalle: T.irreversible.derivar_detalle,
    etiquetaConfirmar: T.accion.nueva_version,
    campos: [
      {
        clave: "motivo",
        nodo: campo(T.irreversible.derivar_campo_motivo, motivo),
        leer: () => motivo.value,
      },
      {
        clave: "paralelo",
        nodo: campoMarca(
          T.irreversible.derivar_paralelo,
          paralelo,
          T.irreversible.derivar_paralelo_nota
        ),
        leer: () => paralelo.checked,
      },
    ],
  });
  if (!r) return;
  const d = await llamar("derivar_proyecto", {
    uid: p.resumen.uid,
    motivo: r.motivo,
    paralelo: r.paralelo,
  });
  if (d) await ir("proyecto", d.uid_derivado);
}

async function dialogoAnadirMaterial(p) {
  const origen = el("input", { type: "text", autofocus: true });
  const procedencia = el("input", { type: "text" });
  const autorizacion = seleccion(
    [
      ["cleared", "Sí, hay permiso"],
      ["pending", "Todavía no"],
      ["not_required", "No hace falta"],
    ],
    "cleared"
  );
  const quienLoVe = seleccion(
    opcionesDe(T.quien_lo_ve, ["PUBLICO", "INTERNO", "CONFIDENCIAL", "RESTRINGIDO"]),
    "INTERNO"
  );

  const elegir = boton(T.accion.elegir_archivo, async () => {
    const ruta = await dialogo.open({ multiple: false });
    if (ruta) origen.value = ruta;
  });

  const r = await confirmar({
    titulo: T.accion.incorporar_material,
    detalle:
      "Todo lo que viene de fuera pasa primero por la bandeja de entrada. La copia se guarda aparte y en solo lectura.",
    etiquetaConfirmar: T.accion.incorporar_material,
    campos: [
      {
        clave: "origen",
        nodo: el("div", { clase: "campo" }, [
          el("label", { texto: "Archivo" }),
          origen,
          el("div", { clase: "tambien" }, [elegir]),
        ]),
        leer: () => origen.value,
      },
      {
        clave: "procedencia",
        nodo: campo("De dónde viene", procedencia),
        leer: () => procedencia.value,
      },
      {
        clave: "autorizacion",
        nodo: campo("Permiso para usarlo", autorizacion),
        leer: () => autorizacion.value,
      },
      {
        clave: "clasificacion",
        nodo: campo(T.envio.campo_quien_lo_ve, quienLoVe),
        leer: () => quienLoVe.value,
      },
    ],
  });
  if (!r) return;
  const v = await llamar("incorporar_material", {
    uid: p.resumen.uid,
    datos: {
      origen_en_cuarentena: r.origen,
      procedencia: r.procedencia,
      autorizacion: r.autorizacion,
      clasificacion: r.clasificacion,
    },
  });
  if (v) {
    ui.proyecto = v;
    await pintar();
  }
}

/// Cada acción decide dónde va el archivo. La persona no elige ubicación.
async function dialogoSacar(p, carpeta) {
  const etiqueta =
    carpeta === "05_STEMS"
      ? T.accion.exportar_stems
      : carpeta === "06_MIX"
        ? T.accion.exportar_bounce
        : T.accion.incorporar_master;
  const nombre = el("input", {
    type: "text",
    value: (p.resumen.titulo || "Tema").replace(/[^A-Za-z0-9-]/g, "-"),
    autofocus: true,
  });
  const r = await confirmar({
    titulo: etiqueta,
    detalle: "El número de versión sube solo. Lo que ya se sacó no se sobrescribe.",
    etiquetaConfirmar: T.accion.aceptar,
    campos: [{ clave: "nombre", nodo: campo("Nombre", nombre), leer: () => nombre.value }],
  });
  if (!r) return;
  const ruta = await llamar("ruta_exportacion", {
    uid: p.resumen.uid,
    carpeta,
    nombre: r.nombre,
    extension: "wav",
  });
  if (!ruta) return;
  ui.carpeta = carpeta;
  await pintar();
  await llamar("abrir_en_explorador", { uid: p.resumen.uid, relativa: carpeta });
  avisar({ texto: "Guardar el archivo aquí: " + ruta, tono: "bueno" });
}

// --- Enviar y dejar el proyecto ---
//
// El diálogo de envío pedía antes siete campos, dos de ellos con vocabulario
// de la norma. Pide cinco, y lo que se manda se elige por lo que es, no por la
// letra del perfil.

async function dialogoEnviar(p) {
  const quien = el("input", { type: "text", autofocus: true });
  const contacto = el("input", { type: "email" });
  const paraQue = el("input", { type: "text" });
  const hasta = el("input", { type: "date" });
  const queMandar = seleccion(
    [
      ["E", T.envio.manda_E],
      ["P", T.envio.manda_P],
      ["A", T.envio.manda_A],
    ],
    "E"
  );
  const quienLoVe = seleccion(
    opcionesDe(T.quien_lo_ve, ["PUBLICO", "INTERNO", "CONFIDENCIAL", "RESTRINGIDO"]),
    "INTERNO"
  );
  const revisado = el("input", { type: "checkbox" });

  const r = await confirmar({
    titulo: T.envio.titulo,
    etiquetaConfirmar: T.accion.emitir_envio,
    campos: [
      { clave: "quien", nodo: campo(T.envio.campo_quien, quien), leer: () => quien.value },
      {
        clave: "contacto",
        nodo: campo(T.envio.campo_contacto, contacto),
        leer: () => contacto.value,
      },
      {
        clave: "que",
        nodo: campo(T.envio.campo_que_mandar, queMandar),
        leer: () => queMandar.value,
      },
      { clave: "paraQue", nodo: campo(T.envio.campo_para_que, paraQue), leer: () => paraQue.value },
      {
        clave: "quienLoVe",
        nodo: campo(T.envio.campo_quien_lo_ve, quienLoVe),
        leer: () => quienLoVe.value,
      },
      { clave: "hasta", nodo: campo(T.envio.campo_hasta, hasta), leer: () => hasta.value },
      {
        clave: "revisado",
        nodo: campoMarca(T.envio.campo_revisado, revisado, T.envio.nota_revisado),
        leer: () => revisado.checked,
      },
    ],
  });
  if (!r) return;

  const e = await llamar("emitir_envio", {
    uid: p.resumen.uid,
    datos: {
      perfil: r.que,
      clasificacion: r.quienLoVe,
      destinatario: r.quien,
      contacto: r.contacto,
      finalidad: r.paraQue,
      retencion: r.hasta,
      acuse: r.contacto,
      ceder_custodia: false,
      retorno_esperado: null,
      gracia: 0,
      serializar: true,
      control_aprobado: r.revisado,
    },
  });
  if (!e) return;
  avisar({
    texto: fmt(T.envio.resultado, {
      archivos: e.archivos,
      tamano: bytesLegibles(e.bytes),
      ruta: e.artefacto,
    }),
    tono: "bueno",
  });
  await ir("proyecto", p.resumen.uid);
}

async function dialogoDejarProyecto(p) {
  const quien = el("input", { type: "text", autofocus: true });
  const contacto = el("input", { type: "email" });
  const paraQue = el("input", { type: "text" });
  const vuelve = el("input", { type: "date" });
  const margen = el("input", { type: "number", value: "15", min: "1" });

  const r = await confirmar({
    titulo: T.irreversible.ceder_titulo,
    aviso: T.irreversible.ceder_aviso,
    detalle: T.irreversible.ceder_detalle,
    etiquetaConfirmar: T.accion.ceder_custodia,
    campos: [
      { clave: "quien", nodo: campo(T.envio.campo_quien, quien), leer: () => quien.value },
      {
        clave: "contacto",
        nodo: campo(T.envio.campo_contacto, contacto),
        leer: () => contacto.value,
      },
      { clave: "paraQue", nodo: campo(T.envio.campo_para_que, paraQue), leer: () => paraQue.value },
      { clave: "vuelve", nodo: campo(T.envio.campo_vuelve, vuelve), leer: () => vuelve.value },
      { clave: "margen", nodo: campo(T.envio.campo_margen, margen), leer: () => Number(margen.value) },
    ],
  });
  if (!r) return;

  const e = await llamar("emitir_envio", {
    uid: p.resumen.uid,
    datos: {
      perfil: "P",
      clasificacion: "CONFIDENCIAL",
      destinatario: r.quien,
      contacto: r.contacto,
      finalidad: r.paraQue,
      retencion: r.vuelve,
      acuse: r.contacto,
      ceder_custodia: true,
      retorno_esperado: r.vuelve,
      gracia: r.margen,
      serializar: true,
      control_aprobado: true,
    },
  });
  if (e) await ir("proyecto", p.resumen.uid);
}

async function dialogoRecuperar(p) {
  const r = await confirmar({
    titulo: T.irreversible.recuperar_titulo,
    aviso: T.irreversible.recuperar_aviso,
    detalle: T.irreversible.recuperar_detalle,
    etiquetaConfirmar: T.accion.recuperar_custodia,
  });
  if (!r) return;
  const v = await llamar("recuperar_custodia", { uid: p.resumen.uid });
  if (v) {
    ui.proyecto = v;
    await pintar();
  }
}

// --- Lo que ha llegado ---

async function pintarRecibido() {
  const paquetes = (await llamar("paquetes_en_cuarentena")) || [];
  const cont = el("div");
  cont.appendChild(
    el("div", { clase: "portada" }, [el("h1", { texto: T.recibido.titulo })])
  );

  if (paquetes.length === 0) {
    cont.appendChild(
      el("div", { clase: "vacio" }, [
        el("p", { clase: "principal-texto", texto: T.recibido.vacio }),
      ])
    );
    return cont;
  }

  const lista = el("ul", { clase: "navegador" });
  for (const ruta of paquetes) {
    lista.appendChild(
      el("li", {}, [
        el("span", { clase: "nombre", texto: ruta.split(/[\\/]/).pop() }),
        boton(T.accion.verificar_paquete, () => dialogoRevisar(ruta), { clase: "principal" }),
      ])
    );
  }
  cont.appendChild(el("section", { clase: "tarjeta" }, [lista]));
  return cont;
}

async function dialogoRevisar(ruta) {
  const emisor = el("input", { type: "text", autofocus: true });
  const condiciones = el("input", { type: "checkbox", checked: true });
  const r = await confirmar({
    titulo: T.accion.verificar_paquete,
    etiquetaConfirmar: T.accion.verificar_paquete,
    campos: [
      {
        clave: "emisor",
        nodo: campo(T.recibido.campo_de_quien, emisor),
        leer: () => emisor.value,
      },
      {
        clave: "condiciones",
        nodo: campoMarca(T.recibido.campo_condiciones, condiciones),
        leer: () => condiciones.checked,
      },
    ],
  });
  if (!r) return;
  const v = await llamar("verificar_paquete", {
    datos: { paquete: ruta, emisor: r.emisor, condiciones_aceptables: r.condiciones },
  });
  if (v) mostrarRevision(v, ruta, r.emisor, r.condiciones);
}

function mostrarRevision(v, ruta, emisor, condiciones) {
  const tono =
    v.resultado === "rejected" ? "malo" : v.resultado === "accepted" ? "bueno" : "";

  // Solo se enumeran las comprobaciones que no salieron bien. Catorce filas
  // verdes no informan de nada; las que fallan, sí.
  const problemas = v.verificaciones.filter(([, res]) => res !== "pass");
  const lista = el("ul", { clase: "lista-falta" });
  for (const [clave, res] of problemas) {
    lista.appendChild(
      el("li", {}, [
        marca(T.recibido[res] || res, res === "fail" ? "problema" : "pendiente"),
        el("span", { clase: "texto", texto: T.comprobacion[clave] || clave }),
      ])
    );
  }
  for (const [clave, detalle] of v.discrepancias) {
    lista.appendChild(
      el("li", {}, [
        marca(T.recibido.reparos, "problema"),
        el("span", { clase: "texto", texto: (T.comprobacion[clave] || clave) + ": " + detalle }),
      ])
    );
  }

  const d = $("#diálogo");
  d.replaceChildren(
    el("h2", { texto: T.recibido.revision }),
    el("div", { clase: "aviso " + tono }, [
      el("p", { texto: T.recibido[v.resultado] || v.resultado }),
    ]),
    problemas.length + v.discrepancias.length === 0
      ? el("p", { clase: "nota", texto: T.falta.nada })
      : lista,
    el("div", { clase: "pie" }, [
      boton(T.accion.cerrar, () => d.close()),
      v.admite_ingesta
        ? boton(
            T.accion.ingerir_paquete,
            async () => {
              d.close();
              const res = await llamar("ingerir_paquete", {
                datos: {
                  paquete: ruta,
                  emisor,
                  condiciones_aceptables: condiciones,
                  proyecto_destino: null,
                  aceptar_custodia: v.cede_custodia,
                },
              });
              if (res) {
                avisar({
                  texto: fmt(T.recibido.guardado, {
                    archivos: res.archivos,
                    destino: res.destino,
                  }),
                  tono: "bueno",
                });
                await pintar();
              }
            },
            { clase: "principal" }
          )
        : null,
    ])
  );
  d.showModal();
}

// --- Historial ---

async function pintarHistorial() {
  const entradas = (await llamar("registro", { ultimas: 200 })) || [];
  const estado = await llamar("verificar_registro");
  const cont = el("div");
  cont.appendChild(el("div", { clase: "portada" }, [el("h1", { texto: T.historial.titulo })]));

  if (estado) {
    const roto = !estado.intacto;
    cont.appendChild(
      el("div", { clase: "aviso " + (roto ? "malo" : "bueno") }, [
        el("p", {
          texto:
            (roto ? T.historial.roto : T.historial.intacto) +
            ". " +
            fmt(T.historial.entradas, { n: estado.total }) +
            ".",
        }),
        roto
          ? el("p", {
              texto: fmt(T.historial.nota_roto, {
                cuales: estado.sin_encadenar.concat(estado.alteradas).join(", "),
              }),
            })
          : null,
      ])
    );
  }

  if (entradas.length === 0) {
    cont.appendChild(el("p", { clase: "sin-nada", texto: T.historial.vacio }));
    return cont;
  }

  const tbody = el("tbody");
  for (const e of entradas.slice().reverse()) {
    tbody.appendChild(
      el("tr", {}, [
        el("td", { clase: "mono", texto: e.marca }),
        el("td", { texto: e.evento }),
        el("td", { texto: e.actor }),
        el("td", { clase: "mono", texto: e.proyecto || "" }),
      ])
    );
  }
  cont.appendChild(
    el("section", { clase: "tarjeta" }, [
      el("table", {}, [
        el("thead", {}, [
          el("tr", {}, [
            el("th", { texto: T.historial.columna_cuando }),
            el("th", { texto: T.historial.columna_que }),
            el("th", { texto: T.historial.columna_quien }),
            el("th", { texto: T.historial.columna_proyecto }),
          ]),
        ]),
        tbody,
      ]),
    ])
  );
  return cont;
}

// --- Ajustes ---

async function pintarAjustes() {
  const identidad = (await llamar("identidad")) || { organizacion: "", persona: "" };
  const declaracion = (await llamar("declaracion_conformidad")) || "";

  const org = el("input", { type: "text", value: identidad.organizacion });
  const persona = el("input", { type: "text", value: identidad.persona });
  const tema = seleccion(
    [
      ["sistema", T.ajustes.tema_sistema],
      ["claro", T.ajustes.tema_claro],
      ["oscuro", T.ajustes.tema_oscuro],
    ],
    localStorage.getItem("tema") || "sistema"
  );
  tema.addEventListener("change", () => aplicarTema(tema.value));

  const horaTexto = () => {
    const r = ui.reloj;
    if (!r || r.estado === "unavailable") return T.ajustes.hora_sin;
    return r.estado === "synced"
      ? fmt(T.ajustes.hora_bien, { ms: r.desviacion_ms })
      : fmt(T.ajustes.hora_mal, { s: Math.round(r.desviacion_ms / 1000) });
  };
  const hora = el("p", { clase: "nota", texto: horaTexto() });

  return el("div", {}, [
    el("div", { clase: "portada" }, [el("h1", { texto: T.ajustes.titulo })]),

    el("section", { clase: "tarjeta" }, [
      el("h2", { texto: T.ajustes.quien_eres }),
      campo(T.ajustes.organizacion, org),
      campo(T.ajustes.persona, persona, T.ajustes.nota_quien),
      el("div", { clase: "tambien" }, [
        boton(
          T.accion.guardar,
          async () => {
            await llamar("set_identidad", {
              identidad: { organizacion: org.value, persona: persona.value },
            });
          },
          { clase: "principal" }
        ),
      ]),
    ]),

    el("section", { clase: "tarjeta" }, [
      el("h2", { texto: T.ajustes.aspecto }),
      campo(T.ajustes.tema, tema),
    ]),

    el("section", { clase: "tarjeta" }, [
      el("h2", { texto: T.ajustes.hora }),
      hora,
      el("div", { clase: "tambien" }, [
        boton(T.accion.sincronizar_reloj, async () => {
          ui.reloj = await llamar("comprobar_reloj");
          hora.textContent = horaTexto();
          pintarSituacion();
          if (ui.reloj && ui.reloj.precision === "http_date") {
            avisar({ texto: T.aviso.reloj_precision_reducida });
          }
        }),
        boton(T.accion.reconstruir_indice, async () => {
          await llamar("reconstruir_indice");
          await pintar();
        }),
      ]),
    ]),

    el("section", { clase: "tarjeta" }, [
      el("h2", { texto: T.ajustes.carpeta_trabajo }),
      el("p", { clase: "mono", texto: ui.repositorio || "" }),
    ]),

    el("details", { clase: "detalle" }, [
      el("summary", { texto: T.ajustes.ficha_tecnica }),
      el("div", { clase: "cuerpo" }, [
        el("p", { clase: "nota", texto: T.ajustes.nota_ficha }),
        el("pre", { clase: "mono", texto: declaracion, style: "white-space: pre-wrap;" }),
      ]),
    ]),
  ]);
}

function aplicarTema(valor) {
  localStorage.setItem("tema", valor);
  if (valor === "sistema") document.documentElement.removeAttribute("data-tema");
  else document.documentElement.setAttribute("data-tema", valor);
}

// --- Atajos de teclado ---

document.addEventListener("keydown", (e) => {
  if (e.target.matches("input, select, textarea")) return;
  const ctrl = e.ctrlKey || e.metaKey;
  if (ctrl && e.key === "n") {
    e.preventDefault();
    dialogoCrearProyecto();
  } else if (ctrl && e.key === "e" && ui.proyecto) {
    e.preventDefault();
    llamar("abrir_en_explorador", { uid: ui.proyecto.resumen.uid, relativa: ui.carpeta });
  } else if (ctrl && e.key === "k" && ui.proyecto) {
    e.preventDefault();
    ejecutar("verificar_integridad", ui.proyecto);
  } else if (ctrl && e.key === "s" && ui.proyecto) {
    e.preventDefault();
    dialogoCrearSesion(ui.proyecto);
  } else if (e.key === "Escape" && ui.ruta === "proyecto" && !$("#diálogo").open) {
    ir("proyectos");
  }
});

// --- Arranque ---

async function arrancar() {
  aplicarTema(localStorage.getItem("tema") || "sistema");

  const sugerida = await invoke("raiz_sugerida");
  let apertura = null;
  try {
    apertura = await invoke("abrir_repositorio", { raiz: sugerida });
  } catch {
    // La carpeta de trabajo no existe todavía: se ofrece crearla donde toca.
    const r = await confirmar({
      titulo: T.app.nombre,
      detalle: fmt(T.aviso.sin_carpeta_trabajo, { ruta: sugerida }),
      etiquetaConfirmar: T.aviso.crear_carpeta_trabajo,
    });
    if (r) apertura = await llamar("crear_repositorio", { raiz: sugerida });
  }

  if (apertura) {
    ui.repositorio = apertura.raiz;
    if (apertura.aviso_sincronizacion) {
      avisar({
        texto: fmt(T.error.raiz_sincronizada, { servicio: apertura.aviso_sincronizacion }),
        tono: "malo",
      });
    } else if (apertura.operaciones_a_medias > 0 || apertura.temporales_abandonados > 0) {
      // Recuperación tras un cierre inesperado.
      const r = await confirmar({
        titulo: T.app.nombre,
        aviso: fmt(T.aviso.operacion_a_medias, { n: apertura.operaciones_a_medias }),
        detalle: fmt(T.aviso.temporales, { n: apertura.temporales_abandonados }),
        etiquetaConfirmar: T.accion.aceptar,
      });
      if (r) await llamar("limpiar_temporales");
    }
  }

  // La hora se consulta al arrancar (apartado 22.3.1).
  ui.reloj = await llamar("reloj_conocido");
  await pintar();
  invoke("comprobar_reloj").then((r) => {
    ui.reloj = r;
    pintarSituacion();
  });
}

arrancar();
