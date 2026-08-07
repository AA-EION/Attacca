// Interfaz de Attacca.
//
// Ningún texto visible se escribe aquí: todo procede de `textos.js`, para que
// las reglas de redacción se puedan auditar en un solo archivo.
//
// La vista de contexto único es la decisión central: en cada momento se presenta
// la carpeta correspondiente a la etapa activa del proyecto y las acciones
// admisibles en ella. La etapa la deduce el núcleo del manifiesto y del
// contenido real, nunca de una preferencia guardada aquí.

import { T, fmt, plural } from "./textos.js";

const invoke = window.__TAURI__.core.invoke;
const dialogo = window.__TAURI__.dialog;

// --- Estado de la interfaz. Nada de esto es normativo. ---
const ui = {
  ruta: "catalogo", // catalogo | proyecto | recepcion | registro | preferencias
  proyecto: null, // VistaProyecto
  carpeta: "", // carpeta relativa que muestra el navegador
  catalogo: null,
  reloj: null,
  repositorio: null,
};

const $ = (sel) => document.querySelector(sel);
const vista = () => $("#vista");

// --- Utilidades de construcción sin plantillas de cadena ---

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
    if (h === null || h === undefined) continue;
    n.appendChild(typeof h === "string" ? document.createTextNode(h) : h);
  }
  return n;
}

function boton(texto, alPulsar, opciones = {}) {
  const hijos = [el("span", { texto })];
  if (opciones.atajo) hijos.push(el("kbd", { texto: opciones.atajo }));
  return el(
    "button",
    { clase: opciones.clase || "", onclick: alPulsar, disabled: opciones.desactivado },
    hijos
  );
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

function seleccion(opciones, valorInicial) {
  const s = el("select");
  for (const [valor, texto] of opciones) {
    s.appendChild(el("option", { value: valor, texto, selected: valor === valorInicial }));
  }
  return s;
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

// --- Errores. El núcleo los redacta con los tres elementos. ---

async function llamar(orden, argumentos) {
  try {
    return await invoke(orden, argumentos);
  } catch (e) {
    mostrarError(String(e));
    return null;
  }
}

function mostrarError(texto) {
  const d = $("#diálogo");
  d.replaceChildren(
    el("h2", { texto: T.app.nombre }),
    el("div", { clase: "aviso-error" }, [el("p", { texto })]),
    el("div", { clase: "pie" }, [boton(T.accion.cerrar, () => d.close(), { clase: "principal" })])
  );
  d.showModal();
}

function confirmar({ titulo, aviso, detalle, etiquetaConfirmar, campos = [] }) {
  return new Promise((resolver) => {
    const d = $("#diálogo");
    const controles = campos.map((c) => c.nodo);
    d.replaceChildren(
      el("h2", { texto: titulo }),
      // La acción irreversible se anuncia en una frase, antes de ejecutarla.
      aviso ? el("div", { clase: "aviso-irreversible" }, [el("p", { texto: aviso })]) : null,
      detalle ? el("p", { clase: "nota", texto: detalle }) : null,
      ...controles,
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
  });
}

// --- Barra de estado permanente ---

function pintarEstado() {
  const f = $("#estado");
  const partes = [];
  const p = ui.proyecto;

  if (p) {
    partes.push(dato(T.rotulo.etapa, T.etapa[p.etapa] || p.etapa));
    partes.push(dato(T.rotulo.custodia, T.custodia[p.resumen.custodia] || p.resumen.custodia));
    const rep = p.replicas.find((r) => r.volumen === p.replica_activa);
    partes.push(dato(T.rotulo.replica, rep ? T.replica[rep.estado] : T.replica.activa));
    partes.push(dato(T.rotulo.nivel, p.resumen.nivel));
    partes.push(distintivoConformidad(p.conformidad));
    // Cifras antes que valoraciones.
    partes.push(
      dato(
        T.rotulo.presupuesto_ruta,
        fmt(T.aviso.presupuesto_ruta, {
          actual: p.presupuesto_ruta.actual,
          limite: p.presupuesto_ruta.limite,
        }),
        p.presupuesto_ruta.excedido ? "incumplimiento" : null
      )
    );
  }

  if (ui.reloj) {
    const r = ui.reloj;
    const valor =
      r.estado === "synced"
        ? r.desviacion_ms + " ms"
        : r.estado === "drifted"
          ? Math.round(r.desviacion_ms / 1000) + " s"
          : "sin fuente";
    partes.push(dato(T.rotulo.reloj, valor, r.admite_emision ? null : "pendiente"));
  }

  f.replaceChildren(...partes);
}

function dato(clave, valor, clase) {
  return el("span", { clase: "dato" }, [
    el("span", { clase: "clave", texto: clave }),
    el("span", { clase: "valor" + (clase ? " distintivo " + clase : ""), texto: valor }),
  ]);
}

function distintivoConformidad(c) {
  // Tres cosas que no se confunden: pendiente, incumplimiento y excepción.
  if (c.incumplimientos > 0) {
    return dato(
      T.rotulo.conformidad,
      plural(c.incumplimientos, T.conformidad.incumplimiento_uno, T.conformidad.incumplimiento_varios),
      "incumplimiento"
    );
  }
  if (c.pendientes > 0) {
    return dato(
      T.rotulo.conformidad,
      plural(c.pendientes, T.conformidad.pendiente_uno, T.conformidad.pendiente_varios),
      "pendiente"
    );
  }
  if (c.excepciones > 0) {
    return dato(
      T.rotulo.conformidad,
      plural(c.excepciones, T.conformidad.excepcion_una, T.conformidad.excepcion_varias),
      "excepcion"
    );
  }
  return dato(T.rotulo.conformidad, T.conformidad.conforme, "conforme");
}

// --- Navegación ---

function pintarNav() {
  const n = $("#nav");
  n.replaceChildren(
    boton("Proyectos", () => ir("catalogo")),
    boton(T.recepcion.titulo, () => ir("recepcion")),
    boton("Registro", () => ir("registro")),
    boton(T.preferencias.titulo, () => ir("preferencias"))
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
}

async function pintar() {
  pintarNav();
  const v = vista();
  switch (ui.ruta) {
    case "catalogo":
      v.replaceChildren(await pintarCatalogo());
      break;
    case "proyecto":
      v.replaceChildren(await pintarProyecto());
      break;
    case "recepcion":
      v.replaceChildren(await pintarRecepcion());
      break;
    case "registro":
      v.replaceChildren(await pintarRegistro());
      break;
    case "preferencias":
      v.replaceChildren(await pintarPreferencias());
      break;
  }
  pintarEstado();
}

// --- Catálogo ---

async function pintarCatalogo() {
  ui.catalogo = await llamar("catalogo");
  const c = ui.catalogo;
  const cont = el("div");

  cont.appendChild(
    el("div", { clase: "etapa-titulo" }, [
      el("h1", { texto: "Proyectos" }),
      boton(T.accion.crear_proyecto, dialogoCrearProyecto, {
        clase: "principal",
        atajo: T.atajo.crear_proyecto,
      }),
      boton(T.accion.crear_release, dialogoCrearRelease),
    ])
  );

  if (!c) return cont;

  if (c.atraso_cuarentena > 0) {
    cont.appendChild(
      el("div", { clase: "panel" }, [
        el("p", { texto: fmt(T.aviso.cuarentena_con_atraso, { n: c.atraso_cuarentena }) }),
      ])
    );
  }

  if (c.ilegibles.length > 0) {
    const lista = el("ul", { clase: "hallazgos" });
    for (const [ruta, motivo] of c.ilegibles) {
      lista.appendChild(
        el("li", {}, [
          el("span", { clase: "distintivo incumplimiento", texto: "no legible" }),
          el("span", { clase: "mono", texto: ruta }),
          el("span", { texto: motivo }),
        ])
      );
    }
    cont.appendChild(el("section", { clase: "panel" }, [el("h2", { texto: "Manifiestos" }), lista]));
  }

  if (c.proyectos.length === 0) {
    cont.appendChild(el("p", { clase: "vacio", texto: T.vacio.sin_proyectos }));
    return cont;
  }

  const tbody = el("tbody");
  for (const p of c.proyectos) {
    tbody.appendChild(
      el(
        "tr",
        { tabindex: "0", onclick: () => ir("proyecto", p.uid), onkeydown: (e) => e.key === "Enter" && ir("proyecto", p.uid) },
        [
          el("td", {}, [el("div", { texto: p.titulo || p.id }), el("div", { clase: "mono", texto: p.id })]),
          el("td", { texto: p.artista }),
          el("td", { texto: T.etapa[p.etapa] || p.etapa }),
          el("td", {}, [
            el("span", {
              clase: "distintivo " + (p.custodia === "propia" || p.custodia === "reclamada" ? "neutro" : "pendiente"),
              texto: T.custodia[p.custodia] || p.custodia,
            }),
          ]),
          el("td", { texto: T.estado_proyecto[p.estado] || p.estado }),
          el("td", { clase: "cifra", texto: p.nivel }),
        ]
      )
    );
  }

  cont.appendChild(
    el("section", { clase: "panel" }, [
      el("table", {}, [
        el("thead", {}, [
          el("tr", {}, [
            el("th", { texto: T.rotulo.titulo }),
            el("th", { texto: T.rotulo.artista }),
            el("th", { texto: T.rotulo.etapa }),
            el("th", { texto: T.rotulo.custodia }),
            el("th", { texto: "Estado" }),
            el("th", { texto: T.rotulo.nivel }),
          ]),
        ]),
        tbody,
      ]),
    ])
  );

  if (c.releases.length > 0) {
    const rb = el("tbody");
    for (const r of c.releases) {
      rb.appendChild(
        el("tr", {}, [
          el("td", {}, [el("div", { texto: r.titulo }), el("div", { clase: "mono", texto: r.id })]),
          el("td", { texto: r.artista }),
          el("td", { texto: r.clase }),
          el("td", { clase: "cifra", texto: String(r.temas) }),
        ])
      );
    }
    cont.appendChild(
      el("section", { clase: "panel" }, [
        el("h2", { texto: "Releases" }),
        el("table", {}, [
          el("thead", {}, [
            el("tr", {}, [
              el("th", { texto: T.rotulo.titulo }),
              el("th", { texto: T.rotulo.artista }),
              el("th", { texto: T.rotulo.clase }),
              el("th", { texto: T.rotulo.tracklist }),
            ]),
          ]),
          rb,
        ]),
      ])
    );
  }

  return cont;
}

// --- Creación de proyecto. Cinco campos y nada más. ---

async function dialogoCrearProyecto() {
  const artista = el("input", { type: "text", autofocus: true });
  const titulo = el("input", { type: "text" });
  const tipo = seleccion(
    [
      ["ORIG", "ORIG — obra original"],
      ["COVER", "COVER — versión de obra ajena"],
      ["REMIX", "REMIX — remezcla"],
      ["BEAT", "BEAT — pista instrumental"],
      ["MIX", "MIX — solo mezcla para terceros"],
      ["MST", "MST — solo mastering"],
      ["SD", "SD — diseño sonoro"],
      ["LIVE", "LIVE — registro en directo"],
      ["DEMO", "DEMO — maqueta"],
      ["SYNC", "SYNC — encargo audiovisual"],
    ],
    "ORIG"
  );
  const releases = ui.catalogo ? ui.catalogo.releases : [];
  const release = seleccion(
    [["", T.creacion.campo_release_ninguno]].concat(releases.map((r) => [r.uid, r.titulo])),
    ""
  );
  const frecuencia = seleccion(
    [44100, 48000, 88200, 96000, 176400, 192000].map((v) => [String(v), v + " Hz"]),
    "48000"
  );
  const bits = seleccion([["24", "24 bits"], ["32", "32 bits"]], "24");

  const r = await confirmar({
    titulo: T.creacion.titulo,
    detalle: T.creacion.nota_campos_pendientes,
    etiquetaConfirmar: T.accion.crear_proyecto,
    campos: [
      { clave: "artista", nodo: campo(T.creacion.campo_artista, artista), leer: () => artista.value },
      {
        clave: "titulo",
        nodo: campo(T.creacion.campo_titulo, titulo, T.creacion.nota_titulo),
        leer: () => titulo.value,
      },
      { clave: "tipo", nodo: campo(T.creacion.campo_tipo, tipo), leer: () => tipo.value },
      { clave: "release", nodo: campo(T.creacion.campo_release, release), leer: () => release.value },
      {
        clave: "frecuencia",
        nodo: campo(T.creacion.campo_frecuencia, frecuencia),
        leer: () => Number(frecuencia.value),
      },
      {
        clave: "bits",
        // La razón de que no se puedan cambiar después, en una línea.
        nodo: campo(T.creacion.campo_bits, bits, T.creacion.nota_audio),
        leer: () => Number(bits.value),
      },
    ],
  });
  if (!r) return;

  const p = await llamar("crear_proyecto", {
    datos: {
      titulo: r.titulo,
      artista: r.artista,
      tipo: r.tipo,
      nivel: "B",
      frecuencia: r.frecuencia,
      bits: r.bits,
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

async function dialogoCrearRelease() {
  const artista = el("input", { type: "text", autofocus: true });
  const titulo = el("input", { type: "text" });
  const clase = seleccion(
    [
      ["SINGLE", "SINGLE — sencillo"],
      ["EP", "EP — extended play"],
      ["ALBUM", "ALBUM — álbum"],
      ["COMP", "COMP — recopilación"],
      ["LIVE", "LIVE — registro en directo"],
      ["SYNC", "SYNC — obra audiovisual"],
    ],
    "SINGLE"
  );
  const r = await confirmar({
    titulo: T.accion.crear_release,
    etiquetaConfirmar: T.accion.crear_release,
    campos: [
      { clave: "artista", nodo: campo(T.creacion.campo_artista, artista), leer: () => artista.value },
      { clave: "titulo", nodo: campo(T.creacion.campo_titulo, titulo), leer: () => titulo.value },
      { clave: "clase", nodo: campo(T.rotulo.clase, clase), leer: () => clase.value },
    ],
  });
  if (!r) return;
  const uid = await llamar("crear_release", {
    datos: { titulo: r.titulo, artista: r.artista, clase: r.clase },
  });
  if (uid) await pintar();
}

// --- Vista de contexto único ---

async function pintarProyecto() {
  const p = ui.proyecto;
  if (!p) return el("p", { clase: "vacio", texto: T.vacio.sin_proyectos });

  const cont = el("div");
  cont.appendChild(
    el("div", { clase: "etapa-titulo" }, [
      el("h1", { texto: p.resumen.titulo || p.resumen.id }),
      el("span", { clase: "distintivo neutro", texto: T.etapa[p.etapa] || p.etapa }),
      boton(T.accion.volver, () => ir("catalogo"), { atajo: T.atajo.volver }),
    ])
  );

  const izquierda = el("div");
  const derecha = el("div");

  // Carpeta de la etapa activa. Al cambiar de contexto se cierra la vista
  // anterior; ningún archivo abierto por otro programa se toca.
  izquierda.appendChild(await panelCarpeta(p));
  izquierda.appendChild(panelIdentidad(p));
  izquierda.appendChild(panelConformidad(p));

  derecha.appendChild(panelAcciones(p));
  derecha.appendChild(panelOtrasCarpetas(p));

  cont.appendChild(el("div", { clase: "contexto" }, [izquierda, derecha]));
  return cont;
}

async function panelCarpeta(p) {
  const entradas = (await llamar("listar_carpeta", { uid: p.resumen.uid, relativa: ui.carpeta })) || [];

  const selector = el("div", { clase: "acciones" });
  const fila = el("div", { clase: "etapa-titulo" }, [
    el("h2", { texto: T.contexto.carpeta_presentada }),
  ]);

  const pestanas = el("div", {}, []);
  for (const carpeta of p.carpetas_etapa) {
    pestanas.appendChild(
      boton(carpeta, () => {
        ui.carpeta = carpeta;
        pintar();
      }, { clase: ui.carpeta === carpeta ? "principal" : "" })
    );
  }

  const lista = el("ul", { clase: "navegador" });
  if (ui.carpeta) {
    lista.appendChild(
      el(
        "li",
        {
          tabindex: "0",
          onclick: () => {
            ui.carpeta = ui.carpeta.split("/").slice(0, -1).join("/");
            pintar();
          },
        },
        [el("span", { clase: "tipo", texto: "" }), el("span", { clase: "nombre", texto: ".." })]
      )
    );
  }
  for (const e of entradas) {
    const nodo = el(
      "li",
      e.es_carpeta
        ? {
            tabindex: "0",
            onclick: () => {
              ui.carpeta = e.ruta;
              pintar();
            },
          }
        : {},
      [
        el("span", { clase: "tipo", texto: e.es_carpeta ? "carpeta" : "" }),
        el("span", { clase: "nombre", texto: e.nombre }),
        el("span", { clase: "tamano", texto: e.es_carpeta ? "" : bytesLegibles(e.tamano) }),
      ]
    );
    lista.appendChild(nodo);
  }

  const panel = el("section", { clase: "panel" }, [
    fila,
    pestanas,
    el("p", { clase: "ruta-actual", texto: p.ruta_absoluta + (ui.carpeta ? "/" + ui.carpeta : "") }),
    entradas.length === 0
      ? el("p", { clase: "vacio", texto: T.contexto.vacia })
      : lista,
    el("p", {
      clase: "nota",
      texto: plural(entradas.length, T.contexto.archivos_uno, T.contexto.archivos_varios),
    }),
    // El apartado 44.2 exige esta acción en todo momento.
    boton(
      T.accion.abrir_en_explorador,
      () => llamar("abrir_en_explorador", { uid: p.resumen.uid, relativa: ui.carpeta }),
      { atajo: T.atajo.abrir_en_explorador }
    ),
    el("p", { clase: "nota", texto: T.contexto.nota_cambio_vista }),
  ]);
  panel.appendChild(selector);
  return panel;
}

function panelIdentidad(p) {
  const filas = [
    [T.rotulo.identificador_legible, p.resumen.id, "mono"],
    [T.rotulo.identificador_interno, p.resumen.uid, "mono"],
    [T.rotulo.artista, p.resumen.artista],
    [T.rotulo.tipo, p.resumen.tipo],
    [T.rotulo.frecuencia, p.audio.frecuencia ? p.audio.frecuencia + " Hz" : "—"],
    [T.rotulo.bits, p.audio.bits ? p.audio.bits + " bits" : "—"],
    [T.rotulo.tempo, p.audio.tempo != null ? String(p.audio.tempo) : "—"],
    [T.rotulo.tonalidad, p.audio.tonalidad || "—"],
    [T.rotulo.afinacion, p.audio.afinacion ? p.audio.afinacion + " Hz" : "—"],
    [T.rotulo.origen, p.audio.origen || "—"],
  ];
  const tbody = el("tbody");
  for (const [clave, valor, clase] of filas) {
    tbody.appendChild(
      el("tr", {}, [el("th", { texto: clave }), el("td", { clase: clase || "", texto: valor })])
    );
  }
  const acciones = [];
  // El título se puede cambiar mientras el proyecto no se haya enviado ni
  // archivado. No se presenta como definitivo cuando no lo es.
  acciones.push(boton(T.accion.cambiar_titulo, () => dialogoCambiarTitulo(p)));
  if (p.audio_modificable) {
    acciones.push(boton(T.accion.declarar_tempo_tonalidad, () => dialogoAudio(p)));
  }
  return el("section", { clase: "panel" }, [
    el("h2", { texto: "Identidad" }),
    el("table", {}, [tbody]),
    el("div", { clase: "acciones" }, acciones),
  ]);
}

function panelConformidad(p) {
  const c = p.conformidad;
  const lista = el("ul", { clase: "hallazgos" });
  for (const h of c.hallazgos) {
    lista.appendChild(
      el("li", {}, [
        el("span", { clase: "distintivo " + h.severidad, texto: etiquetaSeveridad(h.severidad) }),
        el("span", { clase: "clausula", texto: h.clausula }),
        el("span", { texto: h.detalle }),
      ])
    );
  }
  return el("section", { clase: "panel" }, [
    el("h2", { texto: T.rotulo.conformidad }),
    c.hallazgos.length === 0 ? el("p", { clase: "vacio", texto: T.vacio.sin_hallazgos }) : lista,
    c.pendientes > 0 ? el("p", { clase: "nota", texto: T.conformidad.nota_pendiente }) : null,
  ]);
}

function etiquetaSeveridad(s) {
  if (s === "pendiente") return "pendiente";
  if (s === "excepcion") return "excepción";
  return "incumplimiento";
}

function panelAcciones(p) {
  const cont = el("div", { clase: "acciones" });
  for (const clave of p.acciones) {
    const etiqueta = T.accion[clave];
    if (!etiqueta) continue;
    cont.appendChild(boton(etiqueta, () => ejecutar(clave, p), { atajo: T.atajo[clave] }));
  }
  return el("section", { clase: "panel" }, [
    el("h2", { texto: T.menu.titulo }),
    p.acciones.length === 0 ? el("p", { clase: "vacio", texto: T.menu.sin_acciones }) : cont,
    el("h3", { texto: T.contexto.condicion_siguiente }),
    el("p", { clase: "nota", texto: T.condicion[p.condicion_siguiente] || p.condicion_siguiente }),
    p.vencimiento && p.vencimiento.situacion !== "current"
      ? el("div", { clase: "aviso-irreversible" }, [
          el("p", {
            texto: fmt(
              p.vencimiento.situacion === "reclaim_available"
                ? T.aviso.cesion_gracia_vencida
                : T.aviso.cesion_vencida,
              { fecha: p.vencimiento.fecha }
            ),
          }),
        ])
      : null,
  ]);
}

// El resto de la estructura sigue siendo alcanzable, fuera del recorrido
// principal, en dos acciones como máximo.
function panelOtrasCarpetas(p) {
  const cont = el("div", { clase: "acciones" });
  for (const carpeta of p.carpetas_existentes) {
    if (p.carpetas_etapa.includes(carpeta)) continue;
    cont.appendChild(
      boton(carpeta, () => {
        ui.carpeta = carpeta;
        pintar();
      })
    );
  }
  return el("section", { clase: "panel" }, [
    el("h2", { texto: T.menu.otras_carpetas }),
    cont,
    el("p", { clase: "nota", texto: T.menu.nota_otras_carpetas }),
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
      return dialogoCederCustodia(p);

    case "recuperar_custodia":
      return dialogoRecuperarCustodia(p);

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
        mostrarError(r.comprobados + " archivos verificados. Todos coinciden.");
      } else {
        mostrarError(
          "La verificación de integridad falló en " +
            r.fallidos +
            " de " +
            r.comprobados +
            " archivos. Ver los archivos afectados: " +
            r.rutas.join(", ")
        );
      }
      return;
    }

    case "generar_manifiesto_integridad": {
      const n = await llamar("generar_integridad", { uid });
      if (n !== null) await ir("proyecto", uid);
      return;
    }

    case "incorporar_material":
      return dialogoIncorporar(p);

    case "exportar_stems":
      return dialogoExportar(p, "05_STEMS");
    case "exportar_bounce":
      return dialogoExportar(p, "06_MIX");
    case "incorporar_master":
      return dialogoExportar(p, "07_MASTER");

    case "constituir_paquete_entrega":
    case "emitir_envio":
    case "constituir_paquete_produccion":
      return dialogoEmitir(p);

    case "declarar_tempo_tonalidad":
      return dialogoAudio(p);

    default: {
      // Las acciones que consisten en colocar material abren su carpeta.
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
    etiquetaConfirmar: T.accion.cambiar_titulo,
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
  const origen = el("input", { type: "text", value: p.audio.origen ?? "00:00:00:00" });
  const r = await confirmar({
    titulo: T.accion.declarar_tempo_tonalidad,
    detalle: "Estos valores quedan fijados al cerrar la etapa de grabación.",
    etiquetaConfirmar: T.accion.aceptar,
    campos: [
      { clave: "tempo", nodo: campo(T.rotulo.tempo, tempo), leer: () => (tempo.value ? Number(tempo.value) : null) },
      { clave: "tonalidad", nodo: campo(T.rotulo.tonalidad, tonalidad), leer: () => tonalidad.value || null },
      { clave: "afinacion", nodo: campo(T.rotulo.afinacion, afinacion), leer: () => (afinacion.value ? Number(afinacion.value) : null) },
      { clave: "origen", nodo: campo(T.rotulo.origen, origen), leer: () => origen.value || null },
    ],
  });
  if (!r) return;
  const v = await llamar("registrar_audio", { uid: p.resumen.uid, datos: r });
  if (v) {
    ui.proyecto = v;
    await pintar();
  }
}

async function dialogoCrearSesion(p) {
  const daw = el("input", { type: "text", value: "Reaper", autofocus: true });
  const etapa = seleccion(
    [
      ["COMP", "COMP — composición"],
      ["TRACK", "TRACK — grabación"],
      ["EDIT", "EDIT — edición"],
      ["MIX", "MIX — mezcla"],
      ["MST", "MST — mastering"],
      ["SD", "SD — diseño sonoro"],
    ],
    "TRACK"
  );
  const r = await confirmar({
    titulo: T.accion.crear_sesion,
    etiquetaConfirmar: T.accion.crear_sesion,
    campos: [
      { clave: "daw", nodo: campo("Estación de trabajo", daw), leer: () => daw.value },
      { clave: "etapa", nodo: campo("Sufijo de etapa", etapa), leer: () => etapa.value },
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
  mostrarError("Nombre sugerido para la sesión: " + c.nombre_sugerido);
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
      { clave: "motivo", nodo: campo(T.irreversible.derivar_campo_motivo, motivo), leer: () => motivo.value },
      {
        clave: "paralelo",
        nodo: el("div", { clase: "campo" }, [
          el("label", {}, [paralelo, " " + T.irreversible.derivar_paralelo]),
          el("p", { clase: "nota", texto: T.irreversible.derivar_paralelo_nota }),
        ]),
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

async function dialogoIncorporar(p) {
  const paquetes = (await llamar("paquetes_en_cuarentena")) || [];
  const origen = el("input", { type: "text", placeholder: "", autofocus: true });
  const procedencia = el("input", { type: "text" });
  const autorizacion = seleccion(
    [
      ["cleared", "resuelta"],
      ["pending", "pendiente"],
      ["not_required", "no procede"],
    ],
    "cleared"
  );
  const clasificacion = seleccion(
    [
      ["PUBLICO", T.clasificacion.PUBLICO],
      ["INTERNO", T.clasificacion.INTERNO],
      ["CONFIDENCIAL", T.clasificacion.CONFIDENCIAL],
      ["RESTRINGIDO", T.clasificacion.RESTRINGIDO],
    ],
    "INTERNO"
  );

  const elegir = boton("Elegir archivo de la cuarentena", async () => {
    const ruta = await dialogo.open({ multiple: false });
    if (ruta) origen.value = ruta;
  });

  const r = await confirmar({
    titulo: T.accion.incorporar_material,
    detalle:
      "Todo material procedente del exterior se deposita primero en 40_INBOX. La copia se sitúa en 01_REF, que se mantiene en solo lectura.",
    etiquetaConfirmar: T.accion.incorporar_material,
    campos: [
      {
        clave: "origen",
        nodo: el("div", { clase: "campo" }, [
          el("label", { texto: "Archivo en la cuarentena" }),
          origen,
          elegir,
          paquetes.length > 0
            ? el("p", { clase: "nota", texto: paquetes.length + " paquetes en la cuarentena" })
            : null,
        ]),
        leer: () => origen.value,
      },
      { clave: "procedencia", nodo: campo("Origen", procedencia), leer: () => procedencia.value },
      { clave: "autorizacion", nodo: campo("Estado de autorización", autorizacion), leer: () => autorizacion.value },
      { clave: "clasificacion", nodo: campo("Clasificación", clasificacion), leer: () => clasificacion.value },
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

// Cada acción decide dónde va el archivo. La persona no elige ubicación.
async function dialogoExportar(p, carpeta) {
  const nombre = el("input", {
    type: "text",
    value: (p.resumen.titulo || "Tema").replace(/[^A-Za-z0-9-]/g, "-"),
    autofocus: true,
  });
  const r = await confirmar({
    titulo: carpeta === "05_STEMS" ? T.accion.exportar_stems : carpeta === "06_MIX" ? T.accion.exportar_bounce : T.accion.incorporar_master,
    detalle: "El número de versión se incrementa. Lo ya exportado no se sobrescribe.",
    etiquetaConfirmar: T.accion.aceptar,
    campos: [{ clave: "nombre", nodo: campo("Nombre base", nombre), leer: () => nombre.value }],
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
  mostrarError("Exportar a: " + ruta);
}

async function dialogoCederCustodia(p) {
  const destinatario = el("input", { type: "text", autofocus: true });
  const contacto = el("input", { type: "text" });
  const finalidad = el("input", { type: "text" });
  const retorno = el("input", { type: "date" });
  const gracia = el("input", { type: "number", value: "15", min: "1" });

  const r = await confirmar({
    titulo: T.accion.ceder_custodia,
    aviso: T.irreversible.ceder_aviso,
    detalle: T.irreversible.ceder_detalle,
    etiquetaConfirmar: T.accion.ceder_custodia,
    campos: [
      { clave: "destinatario", nodo: campo(T.rotulo.destinatario, destinatario), leer: () => destinatario.value },
      { clave: "contacto", nodo: campo("Contacto", contacto), leer: () => contacto.value },
      { clave: "finalidad", nodo: campo("Finalidad del envío", finalidad), leer: () => finalidad.value },
      { clave: "retorno", nodo: campo(T.rotulo.retorno_esperado, retorno), leer: () => retorno.value },
      { clave: "gracia", nodo: campo(T.rotulo.plazo_gracia, gracia), leer: () => Number(gracia.value) },
    ],
  });
  if (!r) return;
  const e = await llamar("emitir_envio", {
    uid: p.resumen.uid,
    datos: {
      perfil: "P",
      clasificacion: "CONFIDENCIAL",
      destinatario: r.destinatario,
      contacto: r.contacto,
      finalidad: r.finalidad,
      retencion: r.retorno,
      acuse: r.contacto,
      ceder_custodia: true,
      retorno_esperado: r.retorno,
      gracia: r.gracia,
      serializar: true,
      control_aprobado: true,
    },
  });
  if (e) await ir("proyecto", p.resumen.uid);
}

async function dialogoEmitir(p) {
  const destinatario = el("input", { type: "text", autofocus: true });
  const contacto = el("input", { type: "text" });
  const finalidad = el("input", { type: "text" });
  const retencion = el("input", { type: "date" });
  const perfil = seleccion(
    [
      ["E", "E — " + T.perfil.E],
      ["P", "P — " + T.perfil.P],
      ["A", "A — " + T.perfil.A],
    ],
    "E"
  );
  const clasificacion = seleccion(
    [
      ["PUBLICO", T.clasificacion.PUBLICO],
      ["INTERNO", T.clasificacion.INTERNO],
      ["CONFIDENCIAL", T.clasificacion.CONFIDENCIAL],
      ["RESTRINGIDO", T.clasificacion.RESTRINGIDO],
    ],
    "INTERNO"
  );
  const control = el("input", { type: "checkbox" });

  const r = await confirmar({
    titulo: T.accion.emitir_envio,
    detalle:
      "Ningún paquete debe entregarse ni incluirse en un envío sin informe de control de calidad aprobado.",
    etiquetaConfirmar: T.accion.emitir_envio,
    campos: [
      { clave: "perfil", nodo: campo("Perfil", perfil), leer: () => perfil.value },
      { clave: "clasificacion", nodo: campo("Clasificación", clasificacion), leer: () => clasificacion.value },
      { clave: "destinatario", nodo: campo(T.rotulo.destinatario, destinatario), leer: () => destinatario.value },
      { clave: "contacto", nodo: campo("Contacto", contacto), leer: () => contacto.value },
      { clave: "finalidad", nodo: campo("Finalidad del envío", finalidad), leer: () => finalidad.value },
      { clave: "retencion", nodo: campo("Retención en destino", retencion), leer: () => retencion.value },
      {
        clave: "control",
        nodo: el("div", { clase: "campo" }, [
          el("label", {}, [control, " El informe de control de calidad está aprobado"]),
        ]),
        leer: () => control.checked,
      },
    ],
  });
  if (!r) return;
  const e = await llamar("emitir_envio", {
    uid: p.resumen.uid,
    datos: {
      perfil: r.perfil,
      clasificacion: r.clasificacion,
      destinatario: r.destinatario,
      contacto: r.contacto,
      finalidad: r.finalidad,
      retencion: r.retencion,
      acuse: r.contacto,
      ceder_custodia: false,
      retorno_esperado: null,
      gracia: 0,
      serializar: true,
      control_aprobado: r.control,
    },
  });
  if (e) {
    mostrarError(
      "Envío " + e.envio + ". " + e.archivos + " archivos, " + bytesLegibles(e.bytes) + ". Paquete en " + e.artefacto
    );
    await ir("proyecto", p.resumen.uid);
  }
}

async function dialogoRecuperarCustodia(p) {
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

// --- Recepción ---

async function pintarRecepcion() {
  const paquetes = (await llamar("paquetes_en_cuarentena")) || [];
  const cont = el("div");
  cont.appendChild(el("div", { clase: "etapa-titulo" }, [el("h1", { texto: T.recepcion.titulo })]));

  if (paquetes.length === 0) {
    cont.appendChild(el("p", { clase: "vacio", texto: T.recepcion.sin_paquetes }));
    return cont;
  }

  const lista = el("ul", { clase: "navegador" });
  for (const ruta of paquetes) {
    lista.appendChild(
      el("li", {}, [
        el("span", { clase: "nombre mono", texto: ruta }),
        boton(T.accion.verificar_paquete, () => dialogoVerificar(ruta)),
      ])
    );
  }
  cont.appendChild(
    el("section", { clase: "panel" }, [el("h2", { texto: T.recepcion.cuarentena }), lista])
  );
  return cont;
}

async function dialogoVerificar(ruta) {
  const emisor = el("input", { type: "text", autofocus: true });
  const condiciones = el("input", { type: "checkbox", checked: true });
  const r = await confirmar({
    titulo: T.accion.verificar_paquete,
    etiquetaConfirmar: T.accion.verificar_paquete,
    campos: [
      { clave: "emisor", nodo: campo("Organización emisora", emisor), leer: () => emisor.value },
      {
        clave: "condiciones",
        nodo: el("div", { clase: "campo" }, [
          el("label", {}, [
            condiciones,
            " Las condiciones de uso declaradas son compatibles con la finalidad prevista",
          ]),
        ]),
        leer: () => condiciones.checked,
      },
    ],
  });
  if (!r) return;
  const v = await llamar("verificar_paquete", {
    datos: { paquete: ruta, emisor: r.emisor, condiciones_aceptables: r.condiciones },
  });
  if (!v) return;
  mostrarVerificacion(v, ruta, r.emisor, r.condiciones);
}

function mostrarVerificacion(v, ruta, emisor, condiciones) {
  const tbody = el("tbody");
  for (const [clave, resultado] of v.verificaciones) {
    tbody.appendChild(
      el("tr", {}, [
        el("td", { texto: T.verificacion[clave] || clave }),
        el("td", {}, [
          el("span", {
            clase: "distintivo " + (resultado === "fail" ? "incumplimiento" : resultado === "pass" ? "conforme" : "neutro"),
            texto: T.recepcion[resultado] || resultado,
          }),
        ]),
      ])
    );
  }
  const disc = el("ul", { clase: "hallazgos" });
  for (const [comprobacion, detalle] of v.discrepancias) {
    disc.appendChild(
      el("li", {}, [
        el("span", { clase: "distintivo incumplimiento", texto: T.verificacion[comprobacion] || comprobacion }),
        el("span", { texto: "" }),
        el("span", { texto: detalle }),
      ])
    );
  }

  const d = $("#diálogo");
  const pie = el("div", { clase: "pie" }, [
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
              mostrarError(
                res.archivos + " archivos incorporados en " + res.destino + ". Acuse en " + res.ruta_acuse
              );
              await pintar();
            }
          },
          { clase: "principal" }
        )
      : null,
  ]);

  d.replaceChildren(
    el("h2", { texto: T.recepcion.verificacion }),
    el("p", {}, [
      el("span", { texto: T.rotulo.envio + " " + v.envio + ". " }),
      el("span", {
        clase: "distintivo " + (v.resultado === "rejected" ? "incumplimiento" : v.resultado === "accepted" ? "conforme" : "pendiente"),
        texto: T.recepcion["resultado_" + v.resultado] || v.resultado,
      }),
    ]),
    el("table", {}, [tbody]),
    v.discrepancias.length > 0 ? el("h3", { texto: T.recepcion.discrepancias }) : null,
    v.discrepancias.length > 0 ? disc : null,
    pie
  );
  d.showModal();
}

// --- Registro ---

async function pintarRegistro() {
  const entradas = (await llamar("registro", { ultimas: 200 })) || [];
  const estado = await llamar("verificar_registro");
  const cont = el("div");
  cont.appendChild(el("div", { clase: "etapa-titulo" }, [el("h1", { texto: "Registro de eventos" })]));

  if (estado) {
    cont.appendChild(
      el("section", { clase: "panel" }, [
        el("p", {}, [
          el("span", { clase: "cifra", texto: String(estado.total) }),
          el("span", { texto: " entradas. " }),
          el("span", {
            clase: "distintivo " + (estado.intacto ? "conforme" : "incumplimiento"),
            texto: estado.intacto ? "encadenamiento intacto" : "encadenamiento roto",
          }),
        ]),
        !estado.intacto
          ? el("p", {
              clase: "nota",
              texto:
                "Las entradas " +
                estado.sin_encadenar.concat(estado.alteradas).join(", ") +
                " no corresponden a la cadena. Indica supresión, reordenación o alteración.",
            })
          : null,
      ])
    );
  }

  if (entradas.length === 0) {
    cont.appendChild(el("p", { clase: "vacio", texto: T.vacio.sin_eventos }));
    return cont;
  }

  const tbody = el("tbody");
  for (const e of entradas.slice().reverse()) {
    tbody.appendChild(
      el("tr", {}, [
        el("td", { clase: "mono", texto: e.marca }),
        el("td", { clase: "mono", texto: e.evento }),
        el("td", { texto: e.actor }),
        el("td", { clase: "mono", texto: e.proyecto || "" }),
      ])
    );
  }
  cont.appendChild(el("section", { clase: "panel" }, [el("table", {}, [tbody])]));
  return cont;
}

// --- Preferencias ---

async function pintarPreferencias() {
  const identidad = (await llamar("identidad")) || { organizacion: "", persona: "" };
  const declaracion = (await llamar("declaracion_conformidad")) || "";

  const org = el("input", { type: "text", value: identidad.organizacion });
  const persona = el("input", { type: "text", value: identidad.persona });
  const tema = seleccion(
    [
      ["sistema", T.preferencias.tema_sistema],
      ["claro", T.preferencias.tema_claro],
      ["oscuro", T.preferencias.tema_oscuro],
    ],
    localStorage.getItem("tema") || "sistema"
  );
  tema.addEventListener("change", () => aplicarTema(tema.value));

  return el("div", {}, [
    el("div", { clase: "etapa-titulo" }, [el("h1", { texto: T.preferencias.titulo })]),
    el("section", { clase: "panel" }, [
      el("h2", { texto: T.preferencias.identidad }),
      campo(T.preferencias.organizacion, org),
      campo(T.preferencias.persona, persona),
      boton(
        T.accion.aceptar,
        async () => {
          await llamar("set_identidad", {
            identidad: { organizacion: org.value, persona: persona.value },
          });
        },
        { clase: "principal" }
      ),
    ]),
    el("section", { clase: "panel" }, [
      el("h2", { texto: T.preferencias.tema }),
      campo(T.preferencias.tema, tema),
    ]),
    el("section", { clase: "panel" }, [
      el("h2", { texto: T.rotulo.reloj }),
      boton(T.accion.sincronizar_reloj, async () => {
        ui.reloj = await llamar("comprobar_reloj");
        pintarEstado();
        if (ui.reloj && ui.reloj.precision === "http_date") {
          mostrarError(T.aviso.reloj_precision_reducida);
        }
      }),
    ]),
    el("section", { clase: "panel" }, [
      el("h2", { texto: T.preferencias.conformidad }),
      el("pre", { clase: "mono", texto: declaracion, style: "white-space: pre-wrap;" }),
    ]),
  ]);
}

function aplicarTema(valor) {
  localStorage.setItem("tema", valor);
  if (valor === "sistema") document.documentElement.removeAttribute("data-tema");
  else document.documentElement.setAttribute("data-tema", valor);
}

// --- Atajos de teclado. Toda acción frecuente es accesible por teclado. ---

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
    ir("catalogo");
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
    // El repositorio no existe todavía: se ofrece crearlo en la ruta sugerida.
    const r = await confirmar({
      titulo: T.app.nombre,
      detalle:
        "No hay ningún repositorio en " +
        sugerida +
        ". Una sola raíz local por persona, sin espacios y fuera de carpetas sincronizadas con servicios personales.",
      etiquetaConfirmar: "Crear repositorio",
    });
    if (r) apertura = await llamar("crear_repositorio", { raiz: sugerida });
  }

  if (apertura) {
    ui.repositorio = apertura.raiz;
    if (apertura.aviso_sincronizacion) {
      mostrarError(fmt(T.error.raiz_sincronizada, { servicio: apertura.aviso_sincronizacion }));
    } else if (apertura.operaciones_a_medias > 0 || apertura.temporales_abandonados > 0) {
      // Recuperación tras cierre inesperado.
      const r = await confirmar({
        titulo: T.app.nombre,
        aviso: fmt(T.aviso.operacion_a_medias, { n: apertura.operaciones_a_medias }),
        detalle:
          apertura.temporales_abandonados +
          " archivos temporales de escritura atómica quedaron abandonados. Los archivos de destino conservan su contenido anterior.",
        etiquetaConfirmar: "Suprimir los temporales",
      });
      if (r) await llamar("limpiar_temporales");
    }
  }

  // El reloj se consulta al arrancar (apartado 22.3.1).
  ui.reloj = await llamar("reloj_conocido");
  pintar();
  invoke("comprobar_reloj").then((r) => {
    ui.reloj = r;
    pintarEstado();
  });
}

arrancar();
