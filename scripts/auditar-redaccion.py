#!/usr/bin/env python3
"""Auditoría de las reglas de redacción de la interfaz.

Todo el texto visible de Attacca está en `app/ui/textos.js`, y ese archivo
tiene dos registros que no deben mezclarse:

  - el registro llano, que es todo lo que hay fuera del bloque `tecnico` y es
    lo que la persona lee mientras trabaja;
  - el registro normativo, que es el bloque `tecnico` y solo aparece en la
    sección de detalle, donde el vocabulario del apartado 3 de la norma se
    emplea literalmente para poder citarlo.

La versión anterior de este guion exigía el vocabulario normativo en todas
partes. Esa exigencia es la que llenó los botones de «constituir», «emitir» y
«conmutar». Ahora la comprobación va en el sentido contrario: el vocabulario
normativo se exige donde sirve y se prohíbe donde estorba.

Se ejecuta en cada cambio. Un texto que infrinja una regla detiene la
comprobación. No necesita más que python3.
"""

import json
import pathlib
import re
import sys

RAIZ = pathlib.Path(__file__).resolve().parent.parent
TEXTOS = RAIZ / "app" / "ui" / "textos.js"
INTERFAZ = RAIZ / "app" / "ui" / "main.js"
TEXTOS_SWIFT = RAIZ / "app" / "macos" / "Sources" / "AttaccaKit" / "Textos.swift"
ETAPAS = RAIZ / "crates" / "attacca-core" / "src" / "stage.rs"
VISTA = RAIZ / "crates" / "attacca-ordenes" / "src" / "estado.rs"

# --- Prohibiciones, en los dos registros ---

PROHIBIDO = [
    ("signo de exclamación", r"[!¡]"),
    ("emoji", r"[\U0001F000-\U0001FAFF☀-➿]"),
    (
        "felicitación o celebración",
        r"\b(listo|perfecto|genial|estupendo|excelente|enhorabuena|"
        r"todo en orden|ya está|conseguido)\b",
    ),
    (
        "tuteo entusiasta o segunda persona apelativa",
        r"\b(vamos a|puedes|podrás|podras|quieres|deseas|tu proyecto|"
        r"tus proyectos|tu repositorio|te )\b",
    ),
    (
        "pregunta retórica o frase de relleno",
        r"(aquí puedes|aqui puedes|como puedes|como ves|ya sabes|"
        r"no te preocupes|sin más)",
    ),
    (
        "antropomorfismo",
        r"\b(estoy |he encontrado|he detectado|he creado|pensando|"
        r"analizando\.\.\.|buscando por ti|voy a )\b",
    ),
    (
        "metáfora o adjetivo de intensidad",
        r"\b(potente|increíble|increible|sin esfuerzo|mágico|magico|"
        r"rápidamente|rapidamente|fácilmente|facilmente|simplemente|"
        r"sencillamente|al instante)\b",
    ),
]

# --- Vocabulario normativo. Literal, y en el orden de la norma. ---
#
# Solo dentro de `tecnico`. Es lo que permite citar el apartado 3 y entenderse
# con otra implementación.

VOCABULARIO = {
    "custodia": ["propia", "en tránsito", "cedida", "reclamada"],
    "replica": ["activa", "en espera", "desconectada", "divergente"],
}

# --- Jerga que no debe aparecer en el registro llano ---
#
# Cada uno de estos términos es correcto en la norma y estaba en un botón o en
# un rótulo de la versión anterior. Ninguno significa nada para quien está
# grabando, y para todos hay una forma llana que dice lo mismo.

JERGA = [
    (r"\bconstituir\b", "preparar"),
    (r"\bconmutar\b", "cambiar"),
    (r"\bemitir\b", "enviar"),
    (r"\bingerir\b|\bingesta\b", "guardar lo recibido"),
    (r"\bacuse\b", "confirmación de recepción"),
    (r"\bcuarentena\b", "lo que ha llegado"),
    (r"\bcustodia\b", "dónde está el proyecto"),
    (r"\bréplica\b|\breplica\b", "disco"),
    (r"\bmanifiesto\b", "datos del proyecto"),
    (r"\bconformidad\b|\bconforme\b", "lo que falta"),
    (r"\bincumplimiento\b|\bno conformidad\b", "cosa por arreglar"),
    (r"\bcláusula\b|\bclausula\b|\bapartado\b", "sin cita normativa"),
    (r"\bhallazgo\b", "lo que falta"),
    (r"\bpresupuesto de ruta\b", "longitud del nombre"),
    (r"\bidentificador\b", "sin identificadores a la vista"),
    (r"\bperfil\b", "qué se manda"),
    (r"\bclasificación\b", "quién puede verlo"),
    (r"\bprocedencia\b", "de dónde viene"),
    (r"\bretención\b", "hasta cuándo"),
    (r"\bserializar\b", "sin detalle de empaquetado"),
    (r"\bencadenamiento\b", "historial completo"),
    (r"\betapa\b", "fase"),
    (r"\bartefacto\b", "paquete"),
    (r"\badmisible\b", "que se puede hacer"),
    (r"\bnormativ", "sin apelar a la norma"),
    (r"\btrazabilidad\b", "historial"),
    (r"\bexcepción declarada\b", "salvedad"),
]

# --- Términos que no deben sustituir a uno normativo dentro de `tecnico` ---

SINONIMOS_PROHIBIDOS = [
    (r"\bbloqueo\b", "custodia"),
    (r"\bpréstamo\b", "custodia"),
    (r"\bprestamo\b", "custodia"),
    (r"\bcheckout\b", "custodia"),
    (r"\bsincronizando\b", "réplica"),
    (r"\bespejo\b", "réplica"),
    (r"\bcarpeta raíz\b", "raíz del repositorio"),
]


# --- Lectura de textos.js ---------------------------------------------------


def objeto_de(fuente):
    """Convierte el objeto literal `T` de textos.js en un diccionario.

    Se recorre carácter a carácter en lugar de con una expresión regular
    porque hay llaves dentro de las cadenas —los marcadores `{n}`— y una
    expresión regular las confundiría con el final de un bloque.
    """
    inicio = fuente.index("export const T = {") + len("export const T = ")
    salida = []
    profundidad = 0
    i = inicio
    n = len(fuente)

    while i < n:
        c = fuente[i]

        if c == '"':
            j = i + 1
            while j < n:
                if fuente[j] == "\\":
                    j += 2
                    continue
                if fuente[j] == '"':
                    break
                j += 1
            salida.append(fuente[i : j + 1])
            i = j + 1
            continue

        if c == "/" and i + 1 < n and fuente[i + 1] == "/":
            i = fuente.find("\n", i)
            if i < 0:
                break
            continue

        if c == "{":
            profundidad += 1
        elif c == "}":
            profundidad -= 1
            salida.append(c)
            i += 1
            if profundidad == 0:
                break
            continue

        salida.append(c)
        i += 1

    texto = "".join(salida)
    # Claves sin comillas y comas sobrantes: lo que separa a un objeto de
    # JavaScript de un documento JSON.
    texto = re.sub(r"([{,]\s*)([A-Za-z_]\w*)\s*:", r'\1"\2":', texto)
    texto = re.sub(r",(\s*[}\]])", r"\1", texto)
    return json.loads(texto)


def cadenas(nodo, camino=""):
    """Todas las cadenas del árbol, con la ruta en la que aparecen."""
    if isinstance(nodo, str):
        yield camino, nodo
    elif isinstance(nodo, dict):
        for k, v in nodo.items():
            yield from cadenas(v, f"{camino}.{k}" if camino else k)
    elif isinstance(nodo, list):
        for i, v in enumerate(nodo):
            yield from cadenas(v, f"{camino}[{i}]")


# --- Comprobaciones ---------------------------------------------------------


def comprobar_prohibiciones(todas, fallos):
    for motivo, patron in PROHIBIDO:
        for camino, c in todas:
            if re.search(patron, c, re.IGNORECASE):
                fallos.append(f"{motivo} en {camino}: «{c}»")


def comprobar_guion_largo(todas, fallos):
    # Se admite entre espacios, donde separa un código de su significado.
    for camino, c in todas:
        if "—" in c and " — " not in c:
            fallos.append(f"guion largo como puntuación en {camino}: «{c}»")


def comprobar_jerga(llanas, fallos):
    """La jerga de la norma no aparece fuera del bloque `tecnico`."""
    for patron, alternativa in JERGA:
        for camino, c in llanas:
            if re.search(patron, c, re.IGNORECASE):
                fallos.append(
                    f"jerga en el registro llano ({camino}), decir «{alternativa}»: «{c}»"
                )


def comprobar_infinitivos(T, fallos):
    for etiqueta in T.get("accion", {}).values():
        primera = etiqueta.split()[0] if etiqueta.split() else ""
        if not re.match(r"^[A-ZÁÉÍÓÚÑ][a-záéíóúñ]*(ar|er|ir)$", primera):
            fallos.append(f"etiqueta de acción sin infinitivo: «{etiqueta}»")


def comprobar_vocabulario(T, fallos):
    tecnico = T.get("tecnico", {})
    for nombre, esperados in VOCABULARIO.items():
        obtenidos = list(tecnico.get(nombre, {}).values())
        if obtenidos != esperados:
            fallos.append(
                f"vocabulario normativo de {nombre} en `tecnico`: "
                f"se esperaba {esperados}, hay {obtenidos}"
            )
        for v in obtenidos:
            if len(v.split()) > 2:
                fallos.append(f"estado de más de dos palabras: «{v}»")


def comprobar_sinonimos(todas, fallos):
    for patron, termino in SINONIMOS_PROHIBIDOS:
        for camino, c in todas:
            if re.search(patron, c, re.IGNORECASE):
                fallos.append(
                    f"término no normativo en lugar de «{termino}» ({camino}): «{c}»"
                )


def comprobar_errores(T, fallos):
    """Qué ocurrió, qué consecuencia tiene y qué se puede hacer, en ese orden."""
    for clave, e in T.get("error", {}).items():
        frases = [f for f in re.split(r"(?<=\.)\s+", e) if f.strip()]
        if len(frases) < 3:
            fallos.append(
                f"mensaje de error con menos de tres elementos (error.{clave}): «{e[:70]}»"
            )


def acciones_del_nucleo():
    """Claves de acción que el núcleo puede emitir hacia la interfaz.

    Son las de la Tabla I.1, en `stage.rs`, más las que la capa de órdenes
    añade a cualquier etapa.
    """
    claves = set()
    fuente = ETAPAS.read_text(encoding="utf-8")
    cuerpo = fuente[fuente.index("pub fn actions(") :]
    cuerpo = cuerpo[: cuerpo.index("pub fn advance_condition_key")]
    claves.update(re.findall(r'"([a-z_]+)"', cuerpo))

    vista = VISTA.read_text(encoding="utf-8")
    cuerpo = vista[vista.index("fn acciones_admisibles(") :]
    claves.update(re.findall(r'"([a-z_]+)"\.to_string\(\)', cuerpo))
    return claves


def comprobar_cobertura_de_acciones(T, fallos):
    """Toda acción que el núcleo ofrezca debe tener etiqueta.

    Sin esta comprobación, una acción sin etiqueta desaparece de la pantalla en
    silencio: la interfaz la salta y nadie se entera de que falta un botón.
    """
    etiquetadas = set(T.get("accion", {}))
    for clave in sorted(acciones_del_nucleo() - etiquetadas):
        fallos.append(f"el núcleo ofrece la acción «{clave}» y no tiene etiqueta")


def comprobar_referencias(T, fallos):
    """Toda ruta `T.algo.otro` que la interfaz use debe existir en los textos."""
    fuente = INTERFAZ.read_text(encoding="utf-8")
    for bloque, clave in re.findall(r"\bT\.([A-Za-z_]\w*)\.([A-Za-z_]\w*)\b", fuente):
        if bloque not in T:
            fallos.append(f"la interfaz usa T.{bloque} y no existe")
        elif isinstance(T[bloque], dict) and clave not in T[bloque]:
            fallos.append(f"la interfaz usa T.{bloque}.{clave} y no existe")
    for bloque in re.findall(r"\bT\.([A-Za-z_]\w*)\[", fuente):
        if bloque not in T:
            fallos.append(f"la interfaz usa T.{bloque} y no existe")


def cadenas_de_swift():
    """Textos visibles de la interfaz nativa, separados por registro.

    Se recogen solo los valores: el lado derecho de un par `"clave": "valor"` y
    lo que sigue a un `=`. Las claves son identificadores internos y no las lee
    nadie.

    El bloque `Tecnico` se localiza contando llaves, del mismo modo que en
    textos.js y por el mismo motivo.
    """
    if not TEXTOS_SWIFT.exists():
        return [], []

    fuente = TEXTOS_SWIFT.read_text(encoding="utf-8")
    marca = "public enum Tecnico {"
    inicio = fuente.find(marca)
    if inicio < 0:
        tecnico, resto = "", fuente
    else:
        i = inicio + len(marca)
        profundidad = 1
        while i < len(fuente) and profundidad > 0:
            if fuente[i] == "{":
                profundidad += 1
            elif fuente[i] == "}":
                profundidad -= 1
            i += 1
        tecnico = fuente[inicio:i]
        resto = fuente[:inicio] + fuente[i:]

    cadena = r'"(?:[^"\\\n]|\\.)*"'
    par = re.compile(rf"{cadena}\s*:\s*({cadena})")
    asignacion = re.compile(rf"=\s*({cadena})", re.S)
    # Las cadenas de varias líneas se escriben con tres comillas.
    larga = re.compile(r'"""(.*?)"""', re.S)

    def valores(texto, etiqueta):
        out = []
        for m in par.finditer(texto):
            out.append((etiqueta, m.group(1).strip('"')))
        for m in asignacion.finditer(texto):
            out.append((etiqueta, m.group(1).strip('"')))
        for m in larga.finditer(texto):
            # El escape de fin de línea une los tramos: se deshace para leerlo.
            out.append((etiqueta, m.group(1).replace("\\\n", " ")))
        return out

    return valores(resto, "swift"), valores(tecnico, "swift.tecnico")


def main():
    T = objeto_de(TEXTOS.read_text(encoding="utf-8"))

    todas = list(cadenas(T))
    llanas = [(c, s) for c, s in todas if not c.startswith("tecnico")]
    tecnicas = [(c, s) for c, s in todas if c.startswith("tecnico")]

    # La interfaz nativa tiene su propio archivo de textos y rigen las mismas
    # reglas. Un registro que se relaje en una de las dos interfaces deja de
    # ser una regla y pasa a ser una costumbre de un archivo.
    llanas_swift, tecnicas_swift = cadenas_de_swift()
    llanas += llanas_swift
    tecnicas += tecnicas_swift
    todas = todas + llanas_swift + tecnicas_swift

    fallos = []
    comprobar_prohibiciones(todas, fallos)
    comprobar_guion_largo(todas, fallos)
    comprobar_jerga(llanas, fallos)
    comprobar_infinitivos(T, fallos)
    comprobar_vocabulario(T, fallos)
    comprobar_sinonimos(llanas, fallos)
    comprobar_errores(T, fallos)
    comprobar_cobertura_de_acciones(T, fallos)
    comprobar_referencias(T, fallos)

    print(f"Cadenas en el registro llano:     {len(llanas)}")
    print(f"Cadenas en el registro normativo: {len(tecnicas)}")
    print(f"Etiquetas de acción:              {len(T.get('accion', {}))}")
    print(f"Mensajes de error:                {len(T.get('error', {}))}")

    if fallos:
        print(f"\n{len(fallos)} infracciones de las reglas de redacción:\n")
        for f in fallos:
            print(f"  {f}")
        return 1

    print("\nSin infracciones.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
