#!/usr/bin/env bash
#
# Produce Attacca.app para macOS, universal.
#
# Universal quiere decir las dos arquitecturas en un solo binario, y eso hay que
# conseguirlo en las dos mitades por separado:
#
#   1. El nucleo en Rust se compila para aarch64 y para x86_64, y las dos
#      bibliotecas estaticas se funden con `lipo`.
#   2. La aplicacion en Swift se compila con `--arch arm64 --arch x86_64`, que
#      produce ya un ejecutable universal, y se enlaza contra la biblioteca
#      fundida del paso anterior.
#
# Al final se comprueba con `lipo -info` que el ejecutable del paquete tiene de
# verdad las dos. Un .app que solo arranca en la mitad de las maquinas es peor
# que uno que no se produjo, porque no se nota hasta que alguien lo abre.
#
# Uso:
#   ./construir.sh                 compila y empaqueta en app/macos/salida
#   ./construir.sh --pruebas       compila y ejecuta las pruebas de AttaccaKit
#   ./construir.sh --firmar        firma con APPLE_SIGNING_IDENTITY

set -euo pipefail

AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RAIZ="$(cd "$AQUI/../.." && pwd)"
SALIDA="$AQUI/salida"
PAQUETE="$SALIDA/Attacca.app"
CONFIG=release

PRUEBAS=0
FIRMAR=0
for arg in "$@"; do
  case "$arg" in
    --pruebas) PRUEBAS=1 ;;
    --firmar)  FIRMAR=1 ;;
    *) echo "Argumento desconocido: $arg" >&2; exit 2 ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Este guion solo produce el paquete en macOS." >&2
  exit 1
fi

# --- El encabezado de C debe ser el del puente ------------------------------
#
# El modulo de Swift incluye una copia. Enlazar contra una biblioteca cuyo
# encabezado ya no coincide falla en el enlazador, con un mensaje que no dice
# cual es el problema. Aqui si lo dice.

ORIGEN_H="$RAIZ/crates/attacca-ffi/include/attacca.h"
COPIA_H="$AQUI/Sources/CAttacca/include/attacca.h"
if ! cmp -s "$ORIGEN_H" "$COPIA_H"; then
  echo "El encabezado de Sources/CAttacca no coincide con el del puente." >&2
  echo "Copiarlo:  cp $ORIGEN_H $COPIA_H" >&2
  diff -u "$COPIA_H" "$ORIGEN_H" || true
  exit 1
fi

# --- 1. El nucleo, para las dos arquitecturas -------------------------------

echo "==> Nucleo en Rust, aarch64 y x86_64"
for objetivo in aarch64-apple-darwin x86_64-apple-darwin; do
  rustup target add "$objetivo" >/dev/null 2>&1 || true
  ( cd "$RAIZ" && cargo build --release -p attacca-ffi --target "$objetivo" )
done

mkdir -p "$SALIDA/lib"
BIBLIOTECA="$SALIDA/lib/libattacca_ffi.a"
lipo -create -output "$BIBLIOTECA" \
  "$RAIZ/target/aarch64-apple-darwin/release/libattacca_ffi.a" \
  "$RAIZ/target/x86_64-apple-darwin/release/libattacca_ffi.a"
echo "    $(lipo -info "$BIBLIOTECA")"

# El puente enlaza contra las bibliotecas del sistema que usa el nucleo:
# Security y CoreFoundation las necesita la verificacion de certificados de la
# consulta de hora, y libresolv la resolucion de nombres.
ENLACE=(
  -Xlinker -L -Xlinker "$SALIDA/lib"
  -Xlinker -lattacca_ffi
  -Xlinker -framework -Xlinker Security
  -Xlinker -framework -Xlinker CoreFoundation
  -Xlinker -framework -Xlinker SystemConfiguration
  -Xlinker -lresolv
)

# --- 2. Las pruebas, si se piden --------------------------------------------

if [ "$PRUEBAS" = 1 ]; then
  echo "==> Pruebas de AttaccaKit"
  # Las pruebas se compilan para la arquitectura de la maquina: comprobar el
  # puente no requiere el binario universal, y compilar una sola arquitectura
  # tarda la mitad.
  ( cd "$AQUI" && swift test "${ENLACE[@]}" )
fi

# --- 3. La aplicacion, universal --------------------------------------------

echo "==> Aplicacion en Swift, universal"
ARQUITECTURAS=(--arch arm64 --arch x86_64)
( cd "$AQUI" && swift build -c "$CONFIG" "${ARQUITECTURAS[@]}" "${ENLACE[@]}" )

# La ruta se consulta con las mismas opciones con las que se compilo: con dos
# arquitecturas SwiftPM deja la salida en otro sitio que con una.
DESTINO="$( cd "$AQUI" && swift build -c "$CONFIG" "${ARQUITECTURAS[@]}" --show-bin-path )"
EJECUTABLE="$DESTINO/Attacca"

if [ ! -f "$EJECUTABLE" ]; then
  echo "No se produjo el ejecutable en $EJECUTABLE" >&2
  exit 1
fi

# --- 4. El paquete ----------------------------------------------------------

echo "==> Paquete"
rm -rf "$PAQUETE"
mkdir -p "$PAQUETE/Contents/MacOS" "$PAQUETE/Contents/Resources"
cp "$EJECUTABLE" "$PAQUETE/Contents/MacOS/Attacca"
cp "$AQUI/Recursos/Info.plist" "$PAQUETE/Contents/Info.plist"

# El icono es el mismo que el de la version de Tauri: una sola identidad.
if [ -f "$RAIZ/app/src-tauri/icons/icon.icns" ]; then
  cp "$RAIZ/app/src-tauri/icons/icon.icns" "$PAQUETE/Contents/Resources/Attacca.icns"
fi

# --- 5. La comprobacion que de verdad importa -------------------------------

echo "==> Comprobar que el paquete es universal"
info=$(lipo -info "$PAQUETE/Contents/MacOS/Attacca")
echo "    $info"
case "$info" in
  *arm64*) ;;
  *) echo "El ejecutable no incluye arm64." >&2; exit 1 ;;
esac
case "$info" in
  *x86_64*) ;;
  *) echo "El ejecutable no incluye x86_64." >&2; exit 1 ;;
esac

# Y que el enlazado quedo resuelto. Un paquete al que le falte un simbolo se
# construye sin quejarse y falla al abrirlo, que es donde peor se nota.
if ! otool -L "$PAQUETE/Contents/MacOS/Attacca" >/dev/null 2>&1; then
  echo "El ejecutable del paquete no se puede inspeccionar." >&2
  exit 1
fi
if nm -u "$PAQUETE/Contents/MacOS/Attacca" 2>/dev/null | grep -q "attacca_invocar"; then
  echo "El puente con el nucleo quedo sin resolver en el ejecutable." >&2
  exit 1
fi

# --- 6. La firma, si hay credenciales ---------------------------------------

if [ "$FIRMAR" = 1 ]; then
  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    echo "Sin APPLE_SIGNING_IDENTITY. El paquete queda sin firmar." >&2
  else
    echo "==> Firma"
    codesign --force --options runtime --timestamp \
      --sign "$APPLE_SIGNING_IDENTITY" "$PAQUETE"
    codesign --verify --deep --strict --verbose=2 "$PAQUETE"
  fi
fi

echo
echo "Paquete en: $PAQUETE"
