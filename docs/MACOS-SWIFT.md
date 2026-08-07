# La interfaz nativa de macOS

Cómo el código portable de Attacca sostiene una interfaz escrita en Swift, con
Liquid Glass en macOS 26 y un solo binario para las dos arquitecturas.

---

## El problema que resuelve

Attacca se distribuye con Tauri. Un WebView con HTML y CSS es una manera
razonable de tener la misma interfaz en tres sistemas, y sigue siéndolo. Pero en
macOS 26 el sistema tiene un lenguaje visual propio —Liquid Glass— que un
WebView no puede reproducir: el material refracta lo que hay detrás de la
ventana, se funde entre piezas cercanas y responde al puntero. No es un degradado
que se pueda imitar con CSS. O se usan las clases del sistema, o no se tiene.

La pregunta era si eso obliga a reescribir Attacca. No obliga, y la razón es que
lo que hay que reescribir es solo la capa que dibuja.

## Qué se movió y por qué

Antes de este cambio, el reparto era este:

```
crates/attacca-core     nucleo normativo, sin interfaz
app/src-tauri           estado, ordenes Y ventana de Tauri, todo junto
```

`app/src-tauri/src/ordenes.rs` tenía las 38 operaciones que la interfaz puede
pedir, y cada una llevaba encima un `#[tauri::command]`. Esa anotación era lo
único que las ataba a Tauri: dentro no había nada de la plataforma gráfica. Pero
bastaba para que una interfaz nativa tuviera que reimplementarlas, y dos
implementaciones de la misma operación acaban comportándose de dos maneras.

El reparto ahora:

```
crates/attacca-core     nucleo normativo
crates/attacca-ordenes  estado de la sesion y las 38 ordenes. Sin plataforma
crates/attacca-ffi      puente de C sobre las ordenes
app/src-tauri           38 envoltorios de tres lineas cada uno
app/macos               interfaz nativa en Swift
```

`app/src-tauri/src/ordenes.rs` pasó de 998 líneas a un macro que declara las
órdenes ante Tauri y delega. La lógica no se tocó al moverla: es la misma, en
otro sitio.

## La frontera

Entre Rust y Swift hay una sola función:

```c
char *attacca_invocar(AttaccaSesion *sesion,
                      const char *orden,
                      const char *argumentos_json);
```

Nombre de la orden, argumentos en JSON, resultado en JSON. Es exactamente la
forma de `invoke` en la interfaz web, y no por casualidad: las dos interfaces
llaman a la misma tabla de despacho de `attacca-ordenes`, con los mismos nombres
y los mismos argumentos.

```
        interfaz web                     interfaz nativa
   invoke("ver_proyecto", …)        Sesion.invocar("ver_proyecto", …)
             │                                  │
      #[tauri::command]                 attacca_invocar (C)
             │                                  │
             └──────────► attacca_ordenes ◄─────┘
                          despacho::invocar
                                 │
                           attacca-core
```

Por qué JSON y no una interfaz de C con un tipo por operación: porque una
frontera de C con treinta y ocho firmas hay que mantenerla treinta y ocho veces,
y cada campo nuevo de un manifiesto la rompe. Con una sola firma, añadir una
orden es añadir una línea en `despacho.rs` y las dos interfaces la reciben. El
coste es serializar y deserializar en cada llamada; para una aplicación que
responde a clics, es irrelevante.

Lo que la frontera garantiza:

- **Memoria.** Toda cadena que devuelve Rust se libera con
  `attacca_cadena_liberar`. `Sesion.swift` lo hace en un `defer` inmediatamente
  después de copiarla, de modo que no hay camino de salida que se la salte.
- **Errores.** Un fallo llega como `{"error": "…"}` y se convierte en
  `ErrorDeNucleo` antes de intentar descodificar nada. El mensaje ya viene
  redactado con sus tres elementos desde el núcleo, y la interfaz no lo
  reescribe.
- **Pánicos.** `catch_unwind` en el puente. En producción el perfil aborta y no
  llega a actuar; en depuración convierte el pánico en un error con mensaje.
- **Versiones.** `attacca_ordenes()` devuelve la lista de órdenes que la
  biblioteca despacha. La aplicación la pide al arrancar y compara con lo que
  necesita. Enlazar contra una biblioteca de otra versión deja de ser un fallo
  que aparece al pulsar un botón.

## Liquid Glass

Las clases están en macOS 26. La aplicación funciona desde macOS 14, y eso
obliga a decidir qué pasa en 14 y en 15.

Todo lo que nombra Liquid Glass está en un archivo, `Cristal.swift`. El resto de
las vistas pide `.cristal(...)` y no sabe en qué sistema se ejecuta:

```swift
@ViewBuilder
func cristal(forma: some Shape = …, destacado: Bool = false) -> some View {
    if #available(macOS 26.0, *) {
        self.glassEffect(destacado ? .regular.tint(.accentColor) : .regular, in: forma)
    } else {
        self.background(destacado ? .thickMaterial : .regularMaterial, in: forma)
    }
}
```

Lo mismo con `.buttonStyle(.glassProminent)` frente a `.borderedProminent`, y
con `GlassEffectContainer` frente a una pila normal.

Concentrarlo tiene una razón práctica: una comprobación de disponibilidad
repartida por veinte vistas es, en la versión siguiente, veinte sitios que
revisar. En uno solo, el día que el mínimo suba a macOS 26 se borra la mitad del
archivo y no hay nada más que tocar.

### Las tres reglas del material, y dónde se aplican

El material impone condiciones que no son de estilo sino de funcionamiento:

1. **El cristal es de la capa de navegación, nunca del contenido.** En Attacca
   lo llevan las tarjetas de acción, las de estado y la insignia de lo que ha
   llegado. No lo lleva la lista de archivos, que es contenido: sobre cristal se
   lee peor y compite con la acción principal.
2. **No se apila cristal sobre cristal.** Una superficie de cristal no puede
   muestrear otra. El resultado de intentarlo es una mancha gris, no un error de
   compilación, así que la regla hay que respetarla a mano.
3. **Las piezas cercanas van dentro de un `GlassEffectContainer`.** Comparten
   región de muestreo y se funden al acercarse en lugar de cortarse. Las dos
   tarjetas de la fila superior de la pantalla de un proyecto están así.

Y una de accesibilidad: con `Reducir transparencia` el sistema atenúa el
material por su cuenta, pero la tarjeta de la acción principal necesita además
fondo opaco propio para no perder contraste. `FondoDeVentana` se encarga.

### Lo que no se declara

`UIDesignRequiresCompatibility` es la renuncia temporal a Liquid Glass. Solo
existe en iOS, caduca con la versión siguiente del sistema, y adoptarla ahora
sería aplazar el mismo trabajo un año. No está en el `Info.plist`.

## El binario universal

Universal significa las dos arquitecturas en un solo archivo, y hay que
conseguirlo en las dos mitades por separado.

**El núcleo** se compila dos veces y se funde:

```
cargo build --release -p attacca-ffi --target aarch64-apple-darwin
cargo build --release -p attacca-ffi --target x86_64-apple-darwin
lipo -create -output libattacca_ffi.a  …/aarch64/…  …/x86_64/…
```

**La aplicación** se compila una vez con las dos arquitecturas, que es algo que
SwiftPM ya sabe hacer:

```
swift build -c release --arch arm64 --arch x86_64 \
  -Xlinker -L -Xlinker <ruta> -Xlinker -lattacca_ffi
```

La biblioteca no se declara en `Package.swift`. Hacerlo exigiría `unsafeFlags`
con una ruta absoluta, y un paquete con `unsafeFlags` no se puede usar como
dependencia. La ruta la pasa `construir.sh`, que es quien sabe dónde dejó el
archivo.

`construir.sh` termina comprobando con `lipo -info` que el ejecutable del
paquete tiene de verdad las dos, y con `nm -u` que el puente quedó enlazado. Un
`.app` que solo arranca en la mitad de las máquinas, o al que le falta un
símbolo, se construye sin quejarse y falla al abrirlo.

## Cómo se compila

```
./app/macos/construir.sh              # produce app/macos/salida/Attacca.app
./app/macos/construir.sh --pruebas    # además ejecuta las pruebas del puente
./app/macos/construir.sh --firmar     # firma con APPLE_SIGNING_IDENTITY
```

Requiere macOS y Xcode 26 o posterior. Con un Xcode anterior el paquete se
produce igual: `#available(macOS 26.0, *)` es siempre falso en un SDK que no
conoce macOS 26, de modo que la aplicación se compila con los materiales
antiguos y sin Liquid Glass. Compilar contra el SDK de 26 es lo que activa el
material, no ejecutar en 26.

La comprobación de cada cambio lo ejecuta en `macos-15` y sube el `.app` como
artefacto.

## Qué cubre y qué no

La interfaz nativa cubre el recorrido principal: la lista de proyectos, la
pantalla de un proyecto con la fase activa y su acción, los archivos de la
carpeta que la fase presenta, el detalle técnico y la apertura en el Finder. Los
tres requisitos de presentación del apartado 44.2 que se comprueban mirando
—etapa, custodia y réplica visibles sin pedir nada; acceso a las demás carpetas;
abrir en el explorador del sistema en todo momento— están cubiertos.

Lo que todavía no tiene son los diálogos que piden datos: crear proyecto,
enviar, dejar el proyecto a otro estudio, revisar lo recibido. Esas operaciones
existen en el puente y las ejecutan la interfaz web y la línea de órdenes; en la
nativa, una acción que necesita datos abre la carpeta correspondiente, que es
donde el trabajo ocurre de todos modos.

**La interfaz nativa no es la que se publica.** El paquete de macOS sigue siendo
el `.dmg` de Tauri. La nativa se construye y se prueba en cada cambio para que
el camino no se pudra, y para poder cambiar de una a otra cuando cubra el ciclo
entero, sin que eso obligue a tocar nada por debajo de `app/`.

## Los textos

`AttaccaKit/Textos.swift` tiene todo lo visible de la interfaz nativa, igual que
`app/ui/textos.js` lo tiene de la web, y con los mismos dos registros: el llano
y el normativo. `scripts/auditar-redaccion.py` audita los dos archivos con las
mismas reglas. Un registro que se relaje en una de las dos interfaces deja de
ser una regla y pasa a ser una costumbre de un archivo.
