# Comprobación frente a los anexos D, F y H

Cada punto de las tres listas, con lo que Attacca hace y dónde se comprueba.
Los anexos son informativos; su comprobación no es condición de conformidad,
pero sí es el mecanismo que la norma ofrece para verificarla.

---

## Anexo D — Lista de comprobación de cierre de proyecto

Aplicable al cierre de todo proyecto de nivel B o superior.

| Punto | Qué hace Attacca | Comprobado en |
|---|---|---|
| Manifiesto completo, con nivel y estado correctos | `validate::project` distingue campo pendiente, incumplimiento y excepción declarada, y presenta las tres por separado | `distingue_pendiente_de_incumplimiento_al_avanzar_la_etapa` |
| Estado de custodia `propia` o `reclamada`, sin `CUSTODY.lock` en la raíz | La validación detecta el marcador ausente cuando debería estar y presente cuando no | `detecta_el_marcador_de_custodia_ausente_o_sobrante` |
| Cronología completa, consecutiva y sin asientos pendientes de retorno | `custody::check_chronology` detecta duplicados y huecos; la validación los eleva a incumplimiento | `detecta_una_cronologia_con_asiento_suprimido` |
| Sesión consolidada, con todo el audio dentro de la carpeta | Lo ejecuta la estación de trabajo. Attacca abre la carpeta y registra el paso; véase el apartado 4 de la declaración de conformidad | — |
| Stems planos exportados, con el mismo origen e idéntica duración | La comprobación de origen y duración exige análisis de audio, delegado. Attacca impone la nomenclatura, el prefijo de orden y la ubicación | — |
| Hoja de recall con las versiones de los plugins | La produce la estación de trabajo; Attacca la sitúa en `00_ADMIN/Notes` | — |
| Créditos y letra completos y archivados | Grupo Personas del manifiesto y `00_ADMIN/Credits` | `creacion_de_proyecto_y_validacion_del_manifiesto` |
| Acuerdo de reparto firmado y autorizaciones resueltas | `validate::ready_for_delivery` bloquea con autorizaciones pendientes | `la_entrega_se_bloquea_con_autorizaciones_pendientes` |
| Identificadores asignados y registrados | Grupo Derechos, exigible al concluir el control de calidad | `validate::FIELD_GROUPS` |
| Mediciones del máster registradas en el manifiesto | Grupo Máster, exigible al concluir el mastering | `distingue_pendiente_de_incumplimiento_al_avanzar_la_etapa` |
| Informe de control de calidad firmado | La emisión se bloquea sin él | `sin_informe_de_control_de_calidad_no_se_emite` |
| Paquetes congelados y fechados, con sus acuses archivados | La copia congelada va a `08_DELIVERY` o `09_TRANSFER` según el perfil y queda en solo lectura | `ciclo_completo_de_intercambio` |
| Copias de seguridad automáticas y regenerables eliminados | Anexo C aplicado de forma incondicional en el recorrido | `los_regenerables_del_anexo_c_no_entran_en_el_manifiesto` |
| Manifiesto de integridad generado y comprobado | `integrity::generate` y `integrity::verify_tree` | `verifica_un_arbol_intacto` |
| Sin rutas superiores a 200 caracteres, ni caracteres fuera del conjunto | Se comprueba contra la réplica de ruta más larga, y se avisa antes de crear el nombre | `el_presupuesto_se_mide_contra_la_replica_mas_larga`, `rechaza_un_titulo_que_agota_el_presupuesto_de_ruta` |
| Pico real medido con sobremuestreo no inferior a cuatro | Medidor externo. Attacca registra el valor y lo contrasta con el techo del release | `verifica_los_parametros_comunes_en_cada_proyecto` |
| Relojes sincronizados al generar las marcas | Se consulta al arrancar y antes de emitir; la emisión se bloquea sin sincronización válida | `sin_reloj_sincronizado_no_se_emite` |

---

## Anexo F — Lista de comprobación de recepción de un paquete

| Punto | Verificación de la Tabla 31 | Comprobado en |
|---|---|---|
| El contenedor se extrae con una utilidad de uso general y no contiene rutas absolutas, referencias al directorio superior ni enlaces simbólicos | 3, `container` | `rechazo_por_ruta_no_admitida_en_el_contenedor` |
| El envío procede de una parte con acuerdo vigente | 1, `provenance` | `rechaza_por_procedencia_sin_acuerdo_vigente` |
| La firma es válida y su identidad no está revocada | 2, `authenticity` | `rechazo_por_firma` |
| El inventario del empaquetado y sus sumas son correctos | 4, `package_integrity` | `detecta_la_alteracion_del_material_y_la_del_manifiesto` |
| Todos los archivos coinciden, sin ausencias ni no declarados | 5, `content_integrity` | `rechazo_por_integridad`, `detecta_un_archivo_no_declarado_en_la_carga` |
| `EXCHANGE.yaml` valida y no omite ningún campo exigible | 6, `manifest_schema` | `la_omision_de_un_campo_exigible_obliga_al_rechazo` |
| El perfil y la versión son admitidos | 7, `profile_supported` | `ciclo_completo_de_intercambio` |
| El alcance declarado coincide con el contenido | 10, `scope_match` | `el_alcance_declarado_coincide_con_el_contenido` |
| Condiciones de uso, ámbito y retención compatibles | 11, `usage_accepted` | `una_verificacion_no_bloqueante_produce_aceptacion_con_reservas` |
| Las categorías de datos personales se corresponden | 12, `personal_data_match` | `ingest::verify` |
| `content/` respeta la estructura de la Parte 1 | 13, `structure` | `ingest::check_structure` |
| El perfil `P` incluye stems, secuenciación, recall y parámetros de continuación | 6 y 7 sobre el bloque `continuation` | `el_perfil_p_exige_los_parametros_de_continuacion` |
| La clasificación declarada se aplica al material en destino | Se registra en el grupo Fuentes del proyecto de destino | `ingest::ingest` |
| El manifiesto de destino registra envío, condiciones y retención | Grupo Fuentes | `ciclo_de_custodia_cesion_y_retorno` |
| El acuse se emite dentro del plazo declarado | El plazo consta en el manifiesto y en `LEEME.txt` | `el_leeme_explica_como_extraer_sin_attacca` |
| Cuando el envío cede la custodia, el cedente es su titular y declara fecha de retorno y plazo de gracia | 8, `custody` | `la_cesion_exige_fecha_de_retorno_y_plazo_de_gracia` |
| Los asientos recibidos son consecutivos y no duplican | 9, `chronology` | `un_asiento_suprimido_bloquea_la_aceptacion` |
| El acuse declara de forma expresa la aceptación o el rechazo de la custodia | Bloque `custody` del Anexo B.3 | `un_acuse_que_no_acepta_la_custodia_produce_reservas` |
| El proyecto de destino queda con custodia `propia` y sin `CUSTODY.lock` | Se establece al asumir la custodia | `ciclo_de_custodia_cesion_y_retorno` |
| La copia de `40_INBOX` se elimina tras la ingesta | Se elimina el paquete y la extracción | `ciclo_completo_de_intercambio` |

---

## Anexo H — Lista de comprobación de interoperabilidad

Las tres primeras preguntas son las decisivas: una respuesta negativa a
cualquiera de ellas indica que el material está cautivo de una herramienta.

| Pregunta | Respuesta | Fundamento |
|---|---|---|
| ¿Puede localizarse un proyecto y entenderse su contenido usando únicamente el explorador de archivos del sistema? | **Sí** | El repositorio es la estructura del apartado 6 y del apartado 7, con prefijos numéricos que ordenan igual en cualquier sistema. Attacca no crea ningún dominio ni carpeta ajenos a la norma |
| ¿Puede leerse el manifiesto con un editor de texto corriente? | **Sí** | YAML 1.2 en UTF-8, con orden de campos estable. Los campos pendientes constan con nulo explícito, de modo que el manifiesto describe también lo que falta |
| Si se elimina la base de datos de la aplicación, ¿puede reconstruirse íntegramente a partir del árbol? | **Sí** | El índice es una caché. Su supresión no pierde nada y la reconstrucción es completa | 
| ¿Puede abrirse el audio de trabajo con cualquier programa? | **Sí** | Archivos corrientes. Attacca no interpone ninguna capa |
| ¿Puede verificarse la integridad de un proyecto archivado con las utilidades del sistema operativo? | **Sí** | `MANIFEST.sha256` en el formato exacto de `sha256sum` y `shasum -a 256` |
| ¿Puede descifrarse un volumen protegido con una herramienta independiente del fabricante? | **Sí** | El cifrado lo aplica el sistema operativo o una herramienta libre. Attacca no cifra |
| ¿Sobreviven los campos desconocidos del manifiesto cuando otra herramienta lo reescribe? | **Sí** | Requisito comprobado en cada reescritura |
| ¿Existe exportación completa a la estructura de la Parte 1, sin elementos exclusivos del fabricante? | **Sí** | El repositorio ya es esa estructura; no hay nada de lo que exportar |
| ¿Puede otra implementación verificar e ingerir un paquete generado por esta, sin disponer de ella? | **Sí en lo comprobable** | El paquete es BagIt sobre ZIP conforme a ISO/IEC 21320-1. La verificación con utilidades del sistema está acreditada. La comprobación con una segunda implementación de STAVE no puede realizarse: no consta que exista |
| ¿Puede recuperarse el contenido de un contenedor `.stave` con la utilidad de descompresión del sistema? | **Sí** | Comprobado con `unzip` tras renombrar a `.zip`. `LEEME.txt` lo explica a quien no tiene Attacca |
| ¿Respeta la aplicación el estado de custodia declarado, impidiendo modificar un proyecto cedido? | **Sí** | Toda operación de modificación lo comprueba, y el bloqueo se aplica además por medios técnicos |
| ¿Está documentada públicamente cada extensión propia, con prefijo identificable? | **Sí** | Prefijo `x_attacca_`, documentadas en `docs/EXTENSIONES.md` |

### Comprobación de las tres primeras preguntas, paso a paso

```
# 1. Localizar un proyecto con el explorador del sistema
ls ~/.stave/20_PROJECTS/1_ACTIVE/

# 2. Leer su manifiesto con un editor corriente
cat ~/.stave/20_PROJECTS/1_ACTIVE/*/PROJECT.yaml

# 3. Borrar la base de datos y comprobar que no se pierde nada
rm ~/.stave/.attacca-index
attacca listar
```
