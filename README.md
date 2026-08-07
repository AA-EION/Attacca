# Attacca

Aplicación de escritorio que aplica la norma [STAVE 2.0](https://github.com/AA-EION/STAVE)
sobre el árbol de archivos, para macOS, Windows y Linux.

Clase de conformidad declarada: **`M` — Gestora**. La declaración completa está
en [`docs/CONFORMIDAD.md`](docs/CONFORMIDAD.md).

---

## Qué resuelve

Un equipo de producción musical pierde trabajo porque el material queda
inaccesible: sesiones que no abren, masters que nadie sabe si son el aprobado,
proyectos que dos personas editan a la vez en dos discos distintos. STAVE define
la estructura que lo evita. Attacca la aplica sin que la persona usuaria tenga
que conocerla.

El criterio rector: la norma y la aplicación deben interponerse lo mínimo
posible entre la persona y su trabajo.

## Qué no hace

- No interpone ninguna capa entre el material y el sistema de archivos.
- No exige su propia presencia para que el material siga siendo utilizable.
- No tiene servidor, ni cuenta, ni telemetría.
- No mantiene información normativa fuera de los manifiestos.

Si se desinstala Attacca, el repositorio sigue siendo un árbol de carpetas
corrientes con archivos de texto legibles y audio que abre cualquier programa.

---

## Arquitectura

```
crates/attacca-core     Núcleo normativo. Sin interfaz, sin dependencias de la
                        plataforma gráfica. 264 pruebas unitarias
crates/attacca-ordenes  Estado de la sesión y las 38 órdenes que una interfaz
                        puede pedir. Sin plataforma gráfica
crates/attacca-ffi      Puente de C sobre las órdenes, para interfaces nativas
crates/attacca-cli      Línea de órdenes. Ciclo completo sin interfaz gráfica
app/src-tauri           Aplicación de escritorio: declara las órdenes ante Tauri
app/ui                  Interfaz. Todo el texto visible en textos.js
app/macos               Interfaz nativa de macOS, con Liquid Glass en 26 y
                        posteriores. Véase docs/MACOS-SWIFT.md
docs/                   Conformidad, extensiones y comprobación de anexos
```

### Una sola capa de órdenes

Las tres interfaces —la de Tauri, la nativa de macOS y la línea de órdenes—
llaman a las mismas funciones de `attacca-ordenes`, con los mismos nombres y los
mismos argumentos. Ninguna toma decisiones normativas. Una orden implementada
dos veces acabaría comportándose de dos maneras; implementada una sola vez, no
puede.

### La interfaz habla llano

El vocabulario de la norma es el que hay que emplear para citarla, auditarla y
entenderse con otra implementación. No es el que hay que emplear en un botón.
Quien está grabando no tiene por qué saber qué es un manifiesto, ni una
custodia, ni un presupuesto de ruta, para que la aplicación le sirva.

El texto visible tiene por eso dos registros, y no se mezclan:

| | Dónde aparece | Ejemplo |
|---|---|---|
| Llano | Todo el recorrido de trabajo | «Dejar el proyecto a otro estudio» |
| Normativo | Sección de detalle y ficha técnica | «Ceder custodia», apartado 41 |

Los dos están en un solo archivo por interfaz —`app/ui/textos.js` y
`app/macos/Sources/AttaccaKit/Textos.swift`— y `scripts/auditar-redaccion.py`
los comprueba en cada cambio: prohíbe la jerga en el registro llano, exige el
vocabulario del apartado 3 literal en el normativo, y falla si el núcleo ofrece
una acción que ningún registro sabe nombrar.

### El árbol es la fuente de verdad

El apartado 42 de la norma prevalece sobre cualquier otra disposición. Lo que
Attacca guarda en su índice es una caché: se reconstruye recorriendo el árbol y
su supresión no pierde nada.

```
rm ~/.stave/.attacca-index
attacca listar          # el índice se reconstruye por completo
```

### La vista de contexto único

Attacca no muestra un árbol de carpetas. Muestra, en cada momento, la carpeta
correspondiente a la etapa activa del proyecto y las acciones admisibles en
ella, conforme al Anexo I. En la pantalla eso se traduce en una frase con lo que
toca hacer, un botón para hacerlo y los archivos de esa carpeta.

La etapa se deduce del manifiesto y del contenido real, no de una preferencia
guardada en la interfaz. El resto de la estructura sigue siendo alcanzable, y la
acción de abrir la carpeta en el explorador del sistema está disponible en todo
momento. Cambiar de vista no cierra ningún archivo que otro programa tenga
abierto: Attacca observa descriptores, nunca los toca.

---

## Descarga

Cada versión publica un paquete por plataforma, con su suma de verificación:

| Plataforma | Archivo |
|---|---|
| Windows | `Attacca-windows-x86_64.exe` |
| macOS, Apple Silicon e Intel | `Attacca-macos-universal.dmg` |
| Linux | `Attacca-linux-x86_64.AppImage` |

La línea de órdenes `attacca` acompaña a cada plataforma.

```
sha256sum -c SHA256SUMS.txt      # Linux
shasum -a 256 -c SHA256SUMS.txt  # macOS
```

Los mismos tres paquetes se producen en cada cambio y quedan como artefactos en
la pestaña Actions. El detalle del flujo está en
[`docs/PUBLICACION.md`](docs/PUBLICACION.md).

---

## Compilación

Requisitos: Rust 1.82 o posterior.

En Linux, además:

```
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                 libsoup-3.0-dev libayatana-appindicator3-dev pkg-config
```

```
cargo build --release              # todas las piezas
cargo test                         # 293 pruebas
cargo run -p attacca-cli -- --help
```

Paquete de instalación por plataforma:

```
cargo install tauri-cli --version "^2"
cargo tauri build
```

En macOS, además, la interfaz nativa:

```
./app/macos/construir.sh --pruebas   # Attacca.app universal, con sus pruebas
```

Requiere Xcode 26 o posterior para que Liquid Glass se compile. Con un Xcode
anterior el paquete se produce igual, con los materiales antiguos. El detalle
está en [`docs/MACOS-SWIFT.md`](docs/MACOS-SWIFT.md).

---

## Uso desde la línea de órdenes

```
attacca init                                    # crear la raíz local
attacca crear "Canción de Ejemplo" --artista "Artista"
attacca listar
attacca validar
attacca emitir <uid> --destinatario "Sello B" --finalidad "Publicación" \
                     --control-aprobado
attacca verificar paquete.stave --emisor "Estudio A" --acuse acuse.yaml
attacca ingerir  paquete.stave --emisor "Estudio A"
attacca registro --verificar                    # encadenamiento del registro
attacca conformidad                             # declaración de conformidad
```

`attacca --help` enumera el resto.

---

## Verificar un paquete sin Attacca

Un contenedor `.stave` es un ZIP conforme a ISO/IEC 21320-1. Quien lo recibe no
necesita Attacca ni ninguna otra implementación de la norma:

```
mv paquete.stave paquete.zip
unzip paquete.zip
cd STAVE-XCHG_*
sha256sum -c manifest-sha256.txt      # Linux
shasum -a 256 -c manifest-sha256.txt  # macOS
```

`LEEME.txt`, dentro del paquete, explica el procedimiento completo en texto
plano, incluida la verificación de la firma y la dirección a la que dirigir el
acuse de recibo.

---

## Estado

Lo que está comprobado de extremo a extremo:

- creación de proyecto y validación del manifiesto contra el Anexo B.1;
- cambio de título con identificador interno estable y referencias intactas;
- derivación de proyecto y sellado del origen;
- ciclo completo de intercambio: emisión, recepción, acuse, ingesta;
- rechazo por integridad, por firma y por ruta no admitida en el contenedor;
- ciclo de custodia: cesión, retorno, vencimiento y recuperación forzosa;
- conmutación de réplica y detección de divergencia;
- contenedor extraído con `unzip` y verificado con `sha256sum`, sin Attacca;
- reconstrucción íntegra del índice tras borrar la base de datos.

El flujo de integración y publicación comprueba, además, en cada cambio y en
las tres plataformas: formato y clippy sin avisos, el presupuesto de arranque en
frío, las reglas de redacción de las dos interfaces, que el `.dmg` contenga las
dos arquitecturas, que el `Attacca.app` nativo contenga las dos y tenga el
puente enlazado, y que el AppImage arranque y siga vivo bajo una pantalla
virtual.

Lo que falta para una publicación:

- **Credenciales de firma.** Los pasos están en el flujo y documentados en
  `docs/PUBLICACION.md`. El certificado de Apple, el de Windows y la clave de
  actualización son de quien publica; sin ellos los paquetes salen sin firmar y
  el flujo continúa. La actualización verificable está implementada tras la
  característica `actualizador` y solo se compila cuando hay clave: un
  actualizador que no verifica la firma es peor que ninguno.
- **Prueba de interoperabilidad de la clase `W`.** El apartado 45 exige que otra
  implementación independiente acepte el material producido. No consta que
  exista una segunda implementación de STAVE 2.0.

---

## Licencia

Apache-2.0. Véase [`LICENSE`](LICENSE).
