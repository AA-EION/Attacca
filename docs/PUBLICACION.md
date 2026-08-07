# Publicación

Qué produce el flujo, cómo se ejecuta y qué credenciales hacen falta.

---

## 1 Artefactos

Cada versión publica un paquete por plataforma, con la línea de órdenes al lado
y una suma de verificación para cada archivo.

| Plataforma | Paquete | Empaquetador |
|---|---|---|
| Windows | `Attacca-windows-x86_64.exe` | NSIS |
| macOS, Apple Silicon e Intel | `Attacca-macos-universal.dmg` | un solo binario universal |
| Linux | `Attacca-linux-x86_64.AppImage` | AppImage |

Junto a cada uno:

| Archivo | Contenido |
|---|---|
| `attacca-windows-x86_64.exe` | Línea de órdenes |
| `attacca-macos-universal` | Línea de órdenes, universal |
| `attacca-linux-x86_64` | Línea de órdenes |
| `SHA256SUMS.txt` | Suma de verificación de todos los archivos |

El `.dmg` de macOS es universal: contiene `arm64` y `x86_64` en el mismo
binario. El flujo lo comprueba con `lipo` y falla si falta cualquiera de las dos
arquitecturas; no lo da por supuesto.

---

## 2 Flujos

```
construir.yml   reutilizable. Produce los tres paquetes
ci.yml          pruebas en las tres plataformas, y llama a construir
release.yml     pruebas, llama a construir con firma, y publica
```

`construir.yml` es el mismo en ambos casos, de modo que **lo que se publica es
exactamente lo que se comprobó**. Los paquetes se producen en cada cambio, no
solo el día de la versión: un paquete que solo se construye al publicar es un
paquete sin probar.

### Lo que el flujo comprueba antes de subir nada

- las 278 pruebas, en las tres plataformas;
- formato y clippy sin avisos;
- el contenedor `.stave` se extrae y se verifica con `unzip` y `sha256sum`, sin
  Attacca;
- las reglas de redacción de la interfaz;
- el arranque en frío por debajo del presupuesto con 500 proyectos;
- el `.dmg` contiene las dos arquitecturas;
- el AppImage **arranca** bajo una pantalla virtual y sigue vivo a los 25
  segundos;
- la línea de órdenes empaquetada responde a `attacca conformidad`.

### Artefactos de cada cambio

Cada ejecución de la comprobación deja los tres paquetes como artefactos del
flujo, con 30 días de retención. Se descargan desde la pestaña Actions, en la
ejecución correspondiente.

---

## 3 Publicar una versión

```
git tag v0.1.0
git push origin v0.1.0
```

El flujo crea la versión en estado de borrador. Se revisa y se publica a mano:
una versión no se hace pública sin que alguien mire lo que contiene.

También puede lanzarse desde la pestaña Actions con `workflow_dispatch`,
indicando la etiqueta.

---

## 4 Credenciales de firma

**Sin credenciales el flujo funciona igual y produce paquetes sin firmar.** Eso
sirve para comprobar; no para distribuir. Un `.dmg` sin firmar exige a quien lo
instala saltarse Gatekeeper, y un `.exe` sin firmar provoca la advertencia de
SmartScreen.

Los secretos se crean en Settings → Secrets and variables → Actions.

### Actualizador

La actualización verificable **está implementada pero no compilada por
defecto**. Un actualizador que no verifica la firma de lo que descarga es peor
que ninguno, y la firma exige una clave que solo quien publica puede generar.

```
cargo tauri signer generate -w ~/.tauri/attacca.key
```

| Secreto | Valor |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Contenido de `~/.tauri/attacca.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | La contraseña que se eligió |

La **clave pública** que imprime la orden se copia en
`app/src-tauri/tauri.actualizador.conf.json`, en `plugins.updater.pubkey`.

Con el secreto `TAURI_SIGNING_PRIVATE_KEY` presente, el flujo compila con
`--features actualizador`, aplica esa configuración y publica el manifiesto
`latest.json` junto a los paquetes. Sin el secreto, los paquetes salen sin
actualizador y el flujo continúa.

Para comprobarlo en local:

```
cargo build --release -p attacca-app --features actualizador
```

### macOS

Requiere una cuenta de desarrollador de Apple y un certificado Developer ID
Application.

```
security find-identity -v -p codesigning     # ver la identidad
base64 -i certificado.p12 | pbcopy           # para APPLE_CERTIFICATE
```

| Secreto | Valor |
|---|---|
| `APPLE_CERTIFICATE` | El `.p12` en base64 |
| `APPLE_CERTIFICATE_PASSWORD` | Contraseña del `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Nombre (EQUIPO)` |
| `APPLE_ID` | Correo de la cuenta de desarrollador |
| `APPLE_PASSWORD` | Contraseña específica de aplicación |
| `APPLE_TEAM_ID` | Identificador del equipo |

El certificado se importa en un llavero temporal que se descarta con la máquina.
No queda en disco compartido.

### Windows

Requiere un certificado de firma de código.

```
[Convert]::ToBase64String([IO.File]::ReadAllBytes("certificado.pfx")) | Set-Clipboard
```

| Secreto | Valor |
|---|---|
| `WINDOWS_CERTIFICATE` | El `.pfx` en base64 |
| `WINDOWS_CERTIFICATE_PASSWORD` | Contraseña del `.pfx` |

---

## 5 Comprobar una descarga

La misma operación que la verificación de un paquete de intercambio, y con la
misma herramienta:

```
sha256sum -c SHA256SUMS.txt      # Linux
shasum -a 256 -c SHA256SUMS.txt  # macOS
```

En Windows, archivo por archivo:

```
certutil -hashfile Attacca-windows-x86_64.exe SHA256
```

---

## 6 Compilar en local

```
cargo tauri build --bundles appimage    # Linux
cargo tauri build --bundles nsis        # Windows
cargo tauri build --target universal-apple-darwin --bundles dmg   # macOS
```

El objetivo universal de macOS exige las dos cadenas instaladas:

```
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```
