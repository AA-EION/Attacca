#!/usr/bin/env python3
"""Auditoría de las reglas de redacción de la interfaz.

Todo el texto visible de Attacca está en `app/ui/textos.js`. Este guion
comprueba de forma automática las prohibiciones y las obligaciones que fija el
apartado 7 del encargo, y el vocabulario normativo del apartado 3 de la norma.

Se ejecuta en cada cambio. Un texto que infrinja una regla detiene la
comprobación.
"""

import pathlib
import re
import sys

RUTA = pathlib.Path(__file__).resolve().parent.parent / "app" / "ui" / "textos.js"

# --- Prohibiciones del apartado 7 ---

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

# --- Vocabulario normativo (apartado 3 de la norma) ---

VOCABULARIO = {
    "custodia": ["propia", "en tránsito", "cedida", "reclamada"],
    "replica": ["activa", "en espera", "desconectada", "divergente"],
}

# Términos que no deben emplearse en lugar de los normativos.
SINONIMOS_PROHIBIDOS = [
    (r"\bbloqueo\b", "custodia"),
    (r"\bpréstamo\b", "custodia"),
    (r"\bprestamo\b", "custodia"),
    (r"\bcheckout\b", "custodia"),
    (r"\bsincronizando\b", "réplica"),
    (r"\bespejo\b", "réplica"),
    (r"\bcarpeta raíz\b", "raíz del repositorio"),
]


def cadenas_de(texto):
    """Cadenas literales del archivo, que es todo lo que la persona ve."""
    return [c for c in re.findall(r'"((?:[^"\\]|\\.)*)"', texto) if len(c) > 1]


def bloque(texto, nombre):
    m = re.search(nombre + r":\s*\{(.*?)\n  \},", texto, re.S)
    return m.group(1) if m else ""


def valores_de(fragmento):
    return re.findall(r':\s*\n?\s*"((?:[^"\\]|\\.)*)"', fragmento)


def main():
    texto = RUTA.read_text(encoding="utf-8")
    cadenas = cadenas_de(texto)
    fallos = []

    # 1. Prohibiciones.
    for motivo, patron in PROHIBIDO:
        for c in cadenas:
            if re.search(patron, c, re.IGNORECASE):
                fallos.append(f"{motivo}: «{c}»")

    # 2. Guion largo como recurso de puntuación frecuente. Se admite en las
    #    opciones de un desplegable, donde separa el código de su significado.
    largos = [c for c in cadenas if "—" in c and " — " not in c]
    for c in largos:
        fallos.append(f"guion largo como puntuación: «{c}»")

    # 3. Etiquetas de acción en infinitivo.
    acciones = valores_de(bloque(texto, "accion"))
    for a in acciones:
        primera = a.split()[0] if a.split() else ""
        if not re.match(r"^[A-ZÁÉÍÓÚÑ][a-záéíóúñ]*(ar|er|ir)$", primera):
            fallos.append(f"etiqueta de acción sin infinitivo: «{a}»")

    # 4. Vocabulario normativo, literal y en el orden de la norma.
    for nombre, esperados in VOCABULARIO.items():
        obtenidos = valores_de(bloque(texto, nombre))
        if obtenidos != esperados:
            fallos.append(
                f"vocabulario de {nombre}: se esperaba {esperados}, hay {obtenidos}"
            )

    # 5. Sinónimos que sustituyen a un término normativo.
    for patron, termino in SINONIMOS_PROHIBIDOS:
        for c in cadenas:
            if re.search(patron, c, re.IGNORECASE):
                fallos.append(
                    f"término no normativo en lugar de «{termino}»: «{c}»"
                )

    # 6. Los mensajes de error llevan tres elementos, en orden: qué ocurrió,
    #    qué consecuencia tiene y qué se puede hacer.
    errores = valores_de(bloque(texto, "error"))
    for e in errores:
        frases = [f for f in re.split(r"(?<=\.)\s+", e) if f.strip()]
        if len(frases) < 3:
            fallos.append(
                f"mensaje de error con menos de tres elementos: «{e[:70]}»"
            )

    # 7. Los estados se expresan en una o dos palabras.
    for nombre in VOCABULARIO:
        for v in valores_de(bloque(texto, nombre)):
            if len(v.split()) > 2:
                fallos.append(f"estado de más de dos palabras: «{v}»")

    print(f"Cadenas revisadas: {len(cadenas)}")
    print(f"Etiquetas de acción: {len(acciones)}")
    print(f"Mensajes de error: {len(errores)}")

    if fallos:
        print(f"\n{len(fallos)} infracciones de las reglas del apartado 7:\n")
        for f in fallos:
            print(f"  {f}")
        return 1

    print("\nSin infracciones.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
