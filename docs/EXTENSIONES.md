# Extensiones propias

Documentación pública exigida por el apartado 44.1, quinto guion: toda extensión
que una implementación introduzca debe documentarse empleando un prefijo propio
en los nombres de campo.

## Prefijo

Attacca emplea `x_attacca_`. Ningún campo fuera del Anexo B se escribe sin él.

## Principio

Ninguna extensión contiene información normativa. La supresión de cualquiera de
ellas no altera la conformidad de un proyecto, no impide interpretarlo y no
pierde nada que la norma exija conservar. Otra implementación puede descartarlas
sin consecuencia.

Attacca aplica la misma regla en sentido inverso: preserva los campos que no
comprende, incluidas las extensiones de otras implementaciones y los campos de
versiones posteriores de la norma, en cualquier nivel de anidamiento.

## Catálogo

### `x_attacca_daw_hints`

- **Artefacto**: manifiesto de proyecto (Anexo B.1), nivel superior.
- **Tipo**: aplicación de clave a cadena.
- **Finalidad**: recuerda la estación de trabajo elegida al crear cada
  subcarpeta de `02_SESSIONS`, para volver a ofrecerla por defecto.
- **Por qué no es normativa**: el apartado 13.1 ya recoge el programa principal
  y su versión en el grupo Herramientas, y el apartado 7.4 fija la subcarpeta
  por programa. Este campo solo evita repetir una elección; su ausencia hace que
  la aplicación pregunte de nuevo.

```yaml
x_attacca_daw_hints:
  ultima: "Reaper"
```

### `x_attacca_last_opened`

- **Artefacto**: manifiesto de proyecto (Anexo B.1), nivel superior.
- **Tipo**: cadena conforme a RFC 3339 con desplazamiento de zona explícito.
- **Finalidad**: ordenar la lista de proyectos por uso reciente.
- **Por qué no es normativa**: no participa en ninguna comprobación de
  conformidad, no interviene en la trazabilidad y no sustituye a ninguna marca
  del registro de eventos ni de la cronología de custodia. La apertura de un
  proyecto no es un evento del apartado 22.1.

```yaml
x_attacca_last_opened: "2026-08-06T17:20:00-05:00"
```

## Archivos propios que no son extensiones de manifiesto

Estos archivos no son artefactos de la norma y no llevan prefijo porque no son
campos. Empiezan por punto, figuran entre las exclusiones que aplica el
recorrido y no entran en respaldos, manifiestos de integridad ni paquetes.

| Archivo | Ubicación | Contenido |
|---|---|---|
| `.attacca-index` | Raíz del repositorio | Caché del índice. Derivada, reconstruible, prescindible |
| `.attacca-journal` | Raíz del repositorio | Diario de operaciones en curso, para la recuperación tras un cierre inesperado. Su pérdida no invalida el repositorio |
| `.*.attacca-tmp` | Junto a su destino | Temporal de una escritura atómica en curso. Un temporal abandonado indica una operación interrumpida y puede suprimirse: el destino conserva su contenido anterior |
