#!/usr/bin/env bash
#
# Comprueba el principio de autodescripcion del apartado 42 empleando
# unicamente las utilidades del sistema operativo.
#
# Attacca produce el paquete. Todo lo demas —extraccion, verificacion de
# integridad y lectura de los manifiestos— se hace con las herramientas que
# cualquier sistema trae de serie. Si este guion pasa, el material no esta
# cautivo de la aplicacion.

set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# El presupuesto de ruta del apartado 9.1 son 200 caracteres contados desde la
# raiz del repositorio, y el directorio temporal de macOS —de la forma
# /var/folders/3s/j8k2m4n90qz5x7cvbn3lp2rw0000gn/T— consume ya 65. Lo que este
# guion comprueba es la autodescripcion del material, no el presupuesto de
# ruta, de modo que parte de la raiz mas corta disponible.
if [ -d /tmp ]; then
  trabajo="$(mktemp -d /tmp/attacca-XXXXXX)"
else
  trabajo="$(mktemp -d)"
fi
# La copia congelada de un envio queda en solo lectura, que es lo correcto, y
# un usuario sin privilegios no puede suprimirla sin restituir antes el
# permiso. El borrado del directorio de trabajo no es parte de lo que se
# comprueba: si fallara, no debe dar por fallida la comprobacion.
trap 'chmod -R u+w "$trabajo" 2>/dev/null || true; rm -rf "$trabajo" || true' EXIT

# `sha256sum` en Linux, `shasum -a 256` en macOS. Windows en CI ejecuta este
# guion bajo bash, que trae ambas.
if command -v sha256sum > /dev/null 2>&1; then
  comprobar_sumas() { sha256sum -c "$1"; }
elif command -v shasum > /dev/null 2>&1; then
  comprobar_sumas() { shasum -a 256 -c "$1"; }
else
  echo "No hay ninguna utilidad de resumen criptografico en el sistema."
  echo "La comprobacion no se ha ejecutado. Instalar coreutils o perl."
  exit 1
fi

if ! command -v unzip > /dev/null 2>&1; then
  echo "No hay utilidad de descompresion en el sistema."
  echo "La comprobacion no se ha ejecutado. Instalar unzip."
  exit 1
fi

echo "== Compilando la linea de ordenes =="
cargo build --release -p attacca-cli --quiet
attacca="$raiz/target/release/attacca"
[ -f "$attacca.exe" ] && attacca="$attacca.exe"

echo "== Creando un repositorio y un proyecto =="
repo="$trabajo/r"
"$attacca" --repo "$repo" --org "Estudio A" --actor "J. Duarte" init > /dev/null
"$attacca" --repo "$repo" --org "Estudio A" --actor "J. Duarte" \
  crear "Tema De Prueba" --artista "Artista" > /dev/null

proyecto="$(find "$repo/20_PROJECTS" -name PROJECT.yaml -print -quit)"
proyecto_dir="$(dirname "$proyecto")"
uid="$("$attacca" --repo "$repo" listar --reconstruir | head -1 | awk '{print $1}')"

echo "== Colocando material y cerrando los parametros =="
mkdir -p "$proyecto_dir/07_MASTER" "$proyecto_dir/00_ADMIN/Credits" \
         "$proyecto_dir/00_ADMIN/Notes"
printf 'audio del master' > "$proyecto_dir/07_MASTER/master.wav"
printf 'rol,nombre\n'     > "$proyecto_dir/00_ADMIN/Credits/creditos.csv"
printf 'control aprobado\n' > "$proyecto_dir/00_ADMIN/Notes/QC_informe.txt"

# El manifiesto es texto plano: se edita con las herramientas del sistema, que
# es precisamente lo que el apartado 42 exige que siga siendo posible.
python3 - "$proyecto" <<'PY'
import sys
ruta = sys.argv[1]
t = open(ruta, encoding="utf-8").read()
t = t.replace("  tempo: null", "  tempo: 96")
t = t.replace("  key: null", '  key: "Db major"')
t = t.replace("  origin: null", '  origin: "00:00:00:00"')
t = t.replace("  lufs_i: null", "  lufs_i: -9.8")
t = t.replace("  lra: null", "  lra: 5.2")
t = t.replace("  true_peak_db: null", "  true_peak_db: -1.0")
open(ruta, "w", encoding="utf-8").write(t)
PY

echo "== Emitiendo un paquete de intercambio =="
"$attacca" --repo "$repo" --org "Estudio A" --actor "J. Duarte" emitir "$uid" \
  --perfil E --destinatario "Sello B" --finalidad "Publicacion digital" \
  --control-aprobado > /dev/null

contenedor="$(find "$proyecto_dir/08_DELIVERY" -name '*.stave' -print -quit)"
[ -n "$contenedor" ] || { echo "No se emitio ningun contenedor."; exit 1; }

echo "== A partir de aqui, sin Attacca =="

echo "-- Identificar el contenedor por su contenido --"
# La entrada mimetype va primera y sin comprimir, de modo que el tipo se lee de
# los primeros octetos aunque el archivo haya perdido la extension.
if ! head -c 64 "$contenedor" | grep -q "mimetype"; then
  echo "La primera entrada del contenedor no es mimetype."
  exit 1
fi

echo "-- Renombrar a .zip y extraer con la utilidad del sistema --"
salida="$trabajo/sin-attacca"
mkdir -p "$salida"
cp "$contenedor" "$salida/paquete.zip"
( cd "$salida" && unzip -q paquete.zip )

paquete="$(find "$salida" -maxdepth 1 -type d -name 'STAVE-XCHG_*' -print -quit)"
[ -n "$paquete" ] || { echo "La extraccion no produjo el directorio base."; exit 1; }

echo "-- El contenido exacto de mimetype --"
esperado="application/vnd.stave.package"
real="$(cat "$salida/mimetype")"
if [ "$real" != "$esperado" ]; then
  echo "mimetype contiene «$real» y debe contener «$esperado»."
  exit 1
fi
# Sin terminador de linea.
if [ "$(wc -c < "$salida/mimetype")" -ne "${#esperado}" ]; then
  echo "mimetype lleva terminador de linea y no debe llevarlo."
  exit 1
fi

echo "-- Los archivos del apartado 32.1 --"
for archivo in bagit.txt bag-info.txt EXCHANGE.yaml LEEME.txt \
               manifest-sha256.txt tagmanifest-sha256.txt; do
  [ -f "$paquete/$archivo" ] || { echo "Falta $archivo."; exit 1; }
done
[ -d "$paquete/data/content" ] || { echo "Falta data/content."; exit 1; }

echo "-- Verificar la integridad con la utilidad de resumen del sistema --"
( cd "$paquete" && comprobar_sumas manifest-sha256.txt )
( cd "$paquete" && comprobar_sumas tagmanifest-sha256.txt )

echo "-- El manifiesto de intercambio se lee con un editor de texto --"
grep -q "^stave:" "$paquete/EXCHANGE.yaml"
grep -q "  version: \"2.0\"" "$paquete/EXCHANGE.yaml"
grep -q "^classification:" "$paquete/EXCHANGE.yaml"

echo "-- El LEEME explica el procedimiento a quien no tiene Attacca --"
grep -q "\.zip" "$paquete/LEEME.txt"
grep -q "sha256sum -c" "$paquete/LEEME.txt"

echo "-- El material se abre con cualquier programa --"
contenido="$(cat "$paquete/data/content/07_MASTER/master.wav")"
[ "$contenido" = "audio del master" ] || { echo "El material no coincide."; exit 1; }

echo "== Detectar una alteracion del material =="
printf 'audio manipulado' > "$paquete/data/content/07_MASTER/master.wav"
if ( cd "$paquete" && comprobar_sumas manifest-sha256.txt > /dev/null 2>&1 ); then
  echo "La alteracion del material no se detecto."
  exit 1
fi

echo "== Reconstruir el indice tras borrar la base de datos =="
antes="$("$attacca" --repo "$repo" listar --reconstruir | grep -c "$uid" || true)"
rm -f "$repo/.attacca-index"
despues="$("$attacca" --repo "$repo" listar | grep -c "$uid" || true)"
if [ "$antes" != "$despues" ]; then
  echo "El indice no se reconstruyo por completo: $antes antes, $despues despues."
  exit 1
fi

echo
echo "Comprobacion superada. El material es utilizable sin la aplicacion."
