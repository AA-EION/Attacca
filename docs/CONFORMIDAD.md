# Declaración de conformidad

Emitida conforme al apartado 44 de STAVE 2.0. El procedimiento de verificación
es el del apartado 45.

| Campo | Valor |
|---|---|
| Implementación | Attacca 0.1.0 |
| Versión de la norma aplicada | STAVE 2.0 |
| Clase declarada | `M` — Gestora |
| Fecha de la declaración | 2026-08-07 |
| Responsable de la declaración | Quien publica esta compilación |

---

## 1 Clase declarada

Attacca declara la clase `M` (Gestora) de la Tabla 36, que comprende las
capacidades de las clases `R` y `W`.

| Clase | Capacidad exigida | Estado |
|---|---|---|
| `R` — Lectora | Interpretar manifiestos, verificar integridad, extraer y validar contenedores `.stave` recibidos y emitir informe de conformidad | Implementada |
| `W` — Escritora | Todo lo de `R`, más generación de estructura, manifiestos, exportaciones conformes, contenedores `.stave` y paquetes de intercambio conformes a la Parte 4 | Implementada |
| `M` — Gestora | Todo lo de `W`, más gestión de volúmenes, réplicas, registro de eventos, accesos, no conformidades, archivado, el ciclo completo de emisión, recepción, acuse y revocación de la Parte 4, y el ciclo de custodia del apartado 41 | Implementada |

La condición de conformidad de la clase `W` del apartado 45 exige que el
material y los paquetes producidos sean aceptados por al menos otra
implementación independiente. Esa comprobación no puede acreditarse desde aquí:
no consta la existencia de una segunda implementación de STAVE 2.0. Lo que sí se
acredita, y consta en la suite de aceptación, es que los contenedores producidos
se extraen y se verifican con las utilidades incluidas en el sistema operativo
—`unzip`, `sha256sum`, `shasum`— sin intervención de Attacca. Véase el
apartado 6 de este documento.

---

## 2 Partes implementadas

| Parte | Contenido | Estado |
|---|---|---|
| Parte 1 | Estructura, nomenclatura, formatos, metadatos, derechos y ciclo de vida | Implementada |
| Parte 2 | Registro de eventos encadenado, no conformidades, trazabilidad y referencia temporal | Implementada |
| Parte 3 | Control de acceso por medios técnicos, protección en reposo y atribución de copias | Implementada en lo que corresponde a una aplicación de escritorio; véase el apartado 4 |
| Parte 4 | Intercambio completo: paquete, manifiesto, control de datos, emisión, recepción, acuse, rechazo, revocación y custodia | Implementada |
| Parte 5 | Autodescripción, requisitos de formato y conformidad de implementación | Implementada |

---

## 3 Cumplimiento de las obligaciones comunes (apartado 44.1)

| Obligación | Cómo se cumple |
|---|---|
| Preservar los campos de manifiesto no comprendidos | Los manifiestos se manipulan como árbol de nodos con orden estable, no como estructuras fijas. Todo campo desconocido sobrevive a cualquier reescritura, en cualquier nivel de anidamiento. Acreditado en `preserva_los_campos_de_manifiesto_que_no_comprende` |
| Registrar nombre y versión al modificar un proyecto | `stave.written_by` se escribe en cada guardado con `Attacca <versión>` |
| No exigir la propia presencia para que el material siga siendo utilizable | El repositorio es un árbol de carpetas corrientes. La línea de órdenes `attacca` opera el ciclo completo sin interfaz gráfica, y las utilidades del sistema operativo bastan para leer y verificar el material |
| Exportación completa a la estructura de la Parte 1 | El repositorio **es** la estructura de la Parte 1. No hay formato intermedio del que exportar |
| Documentar toda extensión con prefijo propio | Prefijo `x_attacca_`. Documentación en `docs/EXTENSIONES.md` y en el apartado 5 de este documento |
| No almacenar el audio en un sistema de archivos reservado ni interponer capas | Attacca no interpone ninguna capa. El audio son archivos corrientes en carpetas corrientes |
| No incluir en un paquete elementos que solo Attacca interprete, ni omitir campos exigibles | El paquete contiene únicamente los artefactos del apartado 32.1. El manifiesto de intercambio se valida contra los campos exigibles antes de emitir |
| Respetar el estado de custodia declarado | Toda operación de modificación comprueba el estado antes de actuar. El marcador no se suprime mientras el estado lo exija |
| Registrar los asientos de la cronología sin modificar ni suprimir los existentes | Ningún asiento se suprime ni se altera en su contenido. Al fusionar dos series el número de secuencia se recalcula, y solo él. Véase la interpretación del apartado 7.2 |
| Producir contenedores extraíbles con utilidades de uso general y leer los de otra implementación | Acreditado en `contenedor_extraido_con_utilidad_del_sistema` y comprobado a mano con `unzip` y `sha256sum` |

### 3.1 Presentación guiada (apartado 44.2)

Attacca adopta la presentación guiada. Los requisitos que ello activa:

| Requisito | Cómo se cumple |
|---|---|
| Determinar la etapa activa a partir del manifiesto y del contenido, no de una preferencia de interfaz | `stage::active_stage` recibe el manifiesto y la evidencia observada en el árbol. Ningún valor almacenado en la interfaz interviene. Acreditado en `la_etapa_no_procede_de_una_preferencia_guardada` |
| No impedir el acceso a las demás carpetas | El panel «Otras carpetas del proyecto» las lista todas y las presenta en una acción |
| Ofrecer en todo momento una acción que abra la carpeta en el explorador del sistema | Presente en la vista de carpeta y con atajo de teclado `Ctrl E` |
| No cerrar, bloquear ni desmontar archivos abiertos por otro proceso al cambiar la carpeta presentada | Cambiar de carpeta es una operación de lectura de la interfaz. Attacca no abre descriptores sobre el material que presenta, no llama a ninguna función de cierre y no monta ni desmonta volúmenes. La detección de descriptores abiertos solo observa |
| Indicar etapa, custodia y réplica sin exigir ninguna acción | Presentes de forma permanente en la barra inferior, junto con el nivel, la conformidad, el presupuesto de ruta y el estado del reloj |

---

## 4 Requisitos delegados en herramientas externas

El apartado 21.3 establece que un requisito inaplicable en la práctica se
documenta y se comunica, no se elude. Los siguientes no los ejecuta Attacca por
sí misma, y se declara en qué se apoya cada uno.

| Apartado | Requisito | En qué se apoya |
|---|---|---|
| 35.1, 35.3 | Firma criptográfica del manifiesto de integridad y de los acuses | Herramienta OpenPGP externa. Attacca declara la firma en el manifiesto de intercambio, sitúa el archivo de firma separada en el paquete e indica en `LEEME.txt` la orden exacta de verificación. El resultado de la comprobación criptográfica se aporta a la verificación de autenticidad, que comprueba además que la identidad esté declarada en el acuerdo de intercambio y no revocada |
| 15 | Las once verificaciones del control de calidad | Escucha y juicio humanos. Attacca registra el resultado, exige el informe archivado en `00_ADMIN/Notes` y bloquea la emisión sin él |
| 10.2 | Medición de sonoridad y de pico real conforme a ITU-R BS.1770 con sobremuestreo no inferior a cuatro | Medidor externo. Attacca registra los valores en el manifiesto y comprueba el techo declarado frente a los parámetros comunes del release |
| 29.2 | Marca inaudible por destinatario | Herramienta de marcado externa. Attacca declara su existencia y su ámbito en el manifiesto de intercambio conforme al apartado 35.4 |
| 17.1, paso 5 | Archivo de intercambio conforme a AES31-3 | La estación de trabajo. El propio apartado lo exige como mejor esfuerzo; Attacca registra en las notas técnicas qué elementos no sobrevivieron |
| 17.1, pasos 1 a 4 | Consolidación de sesión, exportación de stems y de secuenciación, hoja de recall | La estación de trabajo. Attacca abre la carpeta de destino, comprueba el resultado y registra el paso |
| 6.4 | Sistema de archivos con suma de verificación por bloque y capacidad de reparación | Infraestructura de almacenamiento. Attacca sonda las capacidades del volumen y las declara en el descriptor |
| 28 | Cifrado de volúmenes extraíbles | Mecanismos del sistema operativo. Attacca declara el estado en el descriptor de volumen |

---

## 5 Extensiones propias

Todas llevan el prefijo `x_attacca_`. Ninguna contiene información normativa: su
supresión no altera la conformidad de un proyecto ni impide interpretarlo. Otra
implementación puede descartarlas sin consecuencia, y Attacca preserva las
extensiones ajenas por el mismo motivo.

| Campo | Artefacto | Finalidad |
|---|---|---|
| `x_attacca_daw_hints` | Manifiesto de proyecto | Estación de trabajo elegida al crear cada subcarpeta de `02_SESSIONS`, para volver a ofrecerla por defecto. El apartado 13.1 ya recoge el programa principal en el grupo Herramientas; este campo solo evita repetir la elección |
| `x_attacca_last_opened` | Manifiesto de proyecto | Marca temporal de la última apertura, para ordenar la lista por uso reciente. No participa en ninguna comprobación de conformidad |

---

## 6 Verificación de conformidad (apartado 45)

El apartado 45 exige un repositorio de referencia publicado por el custodio, con
casos válidos e inválidos. No consta que exista. En su ausencia, la verificación
se acredita mediante la suite de aceptación de
`crates/attacca-core/tests/aceptacion.rs`, que construye sus propios casos
válidos e inválidos, y las 262 pruebas unitarias del núcleo.

Casos deliberadamente inválidos que la suite construye y clasifica:

- contenedor con referencia al directorio superior en una entrada;
- contenedor con ruta absoluta, en forma Unix y con unidad de disco;
- contenedor con enlace simbólico;
- contenedor cuya primera entrada no es `mimetype`, o cuyo `mimetype` está comprimido o tiene contenido distinto del exacto;
- paquete con material alterado en tránsito;
- paquete cuyo alcance declarado no coincide con el contenido;
- paquete firmado con identidad no declarada, y con identidad revocada;
- paquete procedente de una parte sin acuerdo de intercambio vigente;
- cronología con asiento suprimido y con número de secuencia duplicado;
- cronología con marca temporal decreciente, que no invalida por sí sola;
- manifiesto de intercambio que omite campos exigibles;
- manifiesto de intercambio de perfil `E` que declara cesión de custodia.

Pruebas de la clase `M` que el apartado 45 enumera:

| Prueba | Acreditada en |
|---|---|
| Reconciliación entre volúmenes | `conmutacion_de_replica_y_deteccion_de_divergencia` |
| Integridad del registro de eventos | `la_supresion_de_una_entrada_intermedia_es_detectable`, `la_alteracion_del_contenido_es_detectable` |
| Restauración desde archivo | `reconstruccion_integra_del_indice_tras_borrar_la_base_de_datos` |
| Ciclo completo de intercambio, con acuse | `ciclo_completo_de_intercambio` |
| Revocación | `package::emit::revoke`, registrada conforme a la Tabla B.1 |
| Cesión, retorno y recuperación de la custodia | `ciclo_de_custodia_cesion_y_retorno`, `ciclo_de_custodia_vencimiento_y_recuperacion_forzosa` |

---

## 7 Interpretaciones adoptadas

Cuando el articulado admite más de una lectura, se documenta la adoptada, con su
fundamento. Esta es la vía del apartado 21.3.

### 7.1 Alcance de la comparación en la reconciliación (apartado 6.6.4)

El apartado exige comparar el contenido de la réplica reconectada con el de la
activa mediante los manifiestos de integridad. Los marcadores `REPLICA.hold` y
`CUSTODY.lock` describen el estado de cada réplica, no su material: el primero
existe por definición en la que no es activa y no en la activa. Incluirlos en la
comparación produciría divergencia en toda reconciliación, sin excepción. Se
excluyen de la comparación. El manifiesto sigue prevaleciendo sobre ambos
marcadores conforme a los apartados 6.6.5 y 14.3.3.

### 7.2 Numeración de la cronología con dos partes (apartados 14.3.4 y 41.3)

Mientras un envío está en tránsito, ambas partes generan asientos sin conocer
los de la otra: el cedente registra `custody_transferred` al recibir el acuse, y
el cesionario ha registrado ya `received`, `imported` y `custody_assumed`. Ambas
series parten del mismo número y colisionan.

El apartado 14.3.4 exige cuatro cosas a la vez: que los números sean
consecutivos, que ninguno se duplique, que las marcas temporales no retrocedan,
y que al recibir un retorno se incorporen los asientos de la otra parte
«conservando su orden y sin suprimir ninguno». Con numeración independiente, las
cuatro solo se satisfacen renumerando.

Añadir los asientos recibidos a continuación de los propios cumple las tres
primeras, pero no la tercera salvo que todos los propios precedan en el tiempo a
todos los recibidos. No es el caso: el cedente registra `custody_transferred`
cuando le llega el acuse, esto es, después de que el cesionario haya registrado
`received`. Las dos series se entrelazan en el tiempo, y concatenarlas produce
una cronología cuyo asiento 4 es anterior al 3.

Attacca ordena la cronología fusionada **por instante** y la renumera de forma
consecutiva desde 1. La ordenación es estable, con lo que cada parte conserva el
orden relativo de sus asientos y ninguno se suprime. El precio es que los
números propios pueden cambiar; el apartado 14.3.4 no los declara inmutables, y
es la única lectura que satisface las cuatro exigencias a la vez. El acto, el
instante y la organización identifican cada asiento, de modo que uno ya presente
no se duplica aunque su número difiera.

Esta interpretación se comunica al custodio conforme al apartado 21.3, por si
procede fijar en una versión posterior un criterio de numeración entre partes.

### 7.3 Fuente de tiempo equivalente (apartado 22.3.1)

El apartado admite «RFC 5905 o un protocolo de sincronización horaria
equivalente». Attacca consulta primero SNTP. Muchas redes de estudio bloquean el
puerto 123; en ese caso recurre a la cabecera `Date` de una respuesta HTTPS,
descontando la mitad del tiempo de ida y vuelta y descartando las respuestas
cuyo trayecto supere dos segundos.

Se declara la limitación: la resolución de esa cabecera es de un segundo, igual
a la tolerancia del apartado. Una desviación inferior a un segundo no es
distinguible de una desviación nula por esta vía. La interfaz indica cuál de las
dos fuentes produjo la medida.

### 7.4 Perfil `P` sin parámetros de continuación (apartados 31.3 y 13.2)

El apartado 31.3 exige que un envío de perfil `P` declare la frecuencia de
muestreo, la profundidad de bits, la afinación, el tempo, el punto temporal de
origen y el estado del vocabulario. El apartado 13.2 hace exigibles el tempo, la
tonalidad y el origen al concluir la composición, no antes.

Un proyecto cuya composición no ha concluido no puede emitirse en perfil `P` sin
producir un manifiesto que la parte receptora debe rechazar conforme al
apartado 33. Attacca detiene la emisión antes de constituir el paquete e indica
qué parámetros faltan. No inventa valores ni emite un manifiesto incompleto.

### 7.5 Réplica única (apartado 6.6.1)

El invariante habla de «todas las réplicas de un proyecto». Un proyecto con una
sola copia y sin inventario declarado no infringe el invariante: esa copia es la
activa. La comprobación se aplica en cuanto el inventario declara alguna réplica.

### 7.6 Carga de un paquete congelado (apartados 32.1 y 32.3)

La copia congelada de un paquete emitido reside dentro del proyecto y lleva en
`data/content/` una copia del manifiesto de proyecto. Ese manifiesto describe el
material del envío, no un proyecto del repositorio. El recorrido de
descubrimiento no desciende en `08_DELIVERY`, en `09_TRANSFER` ni en ningún
directorio que contenga `bagit.txt`.

---

## 8 Requisitos no aplicables a esta implementación

| Apartado | Motivo |
|---|---|
| 18, 23, 24, 25 | Roles, competencia, gestión del riesgo y auditoría son requisitos de la organización que adopta la norma, no de la herramienta. Attacca aporta la evidencia que esos procesos consumen: registro de eventos, registro de no conformidades e informes de conformidad |
| 12.3 | El registro ante sociedades de gestión y organismos de recaudo es un acto ante terceros. Attacca conserva los comprobantes en `00_ADMIN/Rights` y los referencia desde el manifiesto |
| 31.4 | El acuerdo de intercambio se establece entre organizaciones. Attacca consume sus datos: partes con acuerdo vigente, identidades declaradas y revocadas, y plazos de acuse |

---

## 9 Cómo comprobar esta declaración

```
cargo test                 # 278 pruebas: 262 unitarias y 16 de aceptación
attacca conformidad        # declaración en texto plano
```

La comprobación de que un contenedor se extrae y se verifica sin Attacca no
requiere la aplicación:

```
mv paquete.stave paquete.zip
unzip paquete.zip
cd STAVE-XCHG_*
sha256sum -c manifest-sha256.txt
sha256sum -c tagmanifest-sha256.txt
```
