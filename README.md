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
crates/attacca-core   Núcleo normativo. Sin interfaz, sin dependencias de la
                      plataforma gráfica. 262 pruebas unitarias
crates/attacca-cli    Línea de órdenes. Ciclo completo sin interfaz gráfica
app/src-tauri         Aplicación de escritorio: superficie de órdenes
app/ui                Interfaz. Todo el texto visible en textos.js
docs/                 Conformidad, extensiones y comprobación de anexos
```

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
ella, conforme al Anexo I.

La etapa se deduce del manifiesto y del contenido real, no de una preferencia
guardada en la interfaz. El resto de la estructura sigue siendo alcanzable, y la
acción de abrir la carpeta en el explorador del sistema está disponible en todo
momento. Cambiar de vista no cierra ningún archivo que otro programa tenga
abierto: Attacca observa descriptores, nunca los toca.

---

## Compilación

Requisitos: Rust 1.82 o posterior.

En Linux, además:

```
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
                 libsoup-3.0-dev libayatana-appindicator3-dev pkg-config
```

```
cargo build --release              # las tres piezas
cargo test                         # 278 pruebas
cargo run -p attacca-cli -- --help
```

Paquete de instalación por plataforma:

```
cargo install tauri-cli --version "^2"
cargo tauri build
```

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

Lo que falta para una publicación:

- **Binarios firmados.** El flujo de compilación para las tres plataformas está
  en `.github/workflows/release.yml`, con los pasos de firma preparados. Las
  credenciales de firma —certificado de Apple, certificado de Windows y clave de
  actualización— no se han generado: son de quien publica.
- **Iconos.** Los de `app/src-tauri/icons` son provisionales.
- **Prueba de interoperabilidad de la clase `W`.** El apartado 45 exige que otra
  implementación independiente acepte el material producido. No consta que
  exista una segunda implementación de STAVE 2.0.

---

## Licencia

Apache-2.0. Véase [`LICENSE`](LICENSE).
