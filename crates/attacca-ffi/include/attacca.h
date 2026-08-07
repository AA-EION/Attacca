/* Puente de C sobre la capa de órdenes de Attacca.
 *
 * Una sola frontera entre el núcleo, escrito en Rust, y una interfaz nativa.
 * Se invoca una orden por su nombre con los argumentos en JSON y se recibe el
 * resultado en JSON. Es la misma superficie que emplea la interfaz web, con
 * los mismos nombres de orden y los mismos argumentos.
 *
 * Toda cadena devuelta por este encabezado se libera con
 * attacca_cadena_liberar. Liberarla con free corrompe el montículo.
 *
 * El resultado de attacca_invocar, attacca_ordenes y attacca_version es
 * siempre un objeto con exactamente una de estas dos claves:
 *
 *     { "ok":    <valor de la orden> }
 *     { "error": "<qué ocurrió, qué consecuencia tiene y qué se puede hacer>" }
 */

#ifndef ATTACCA_H
#define ATTACCA_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Sesión abierta contra un repositorio. Opaca: su contenido cambia entre
 * versiones sin que la frontera se mueva. */
typedef struct AttaccaSesion AttaccaSesion;

/* Prepara el proceso. Se llama una vez, antes de crear ningún hilo: el
 * desplazamiento de zona horaria del que dependen todas las marcas de tiempo
 * no se obtiene de forma fiable en un proceso con varios hilos. Llamarla más
 * de una vez es inocuo. */
void attacca_inicializar(void);

/* Abre una sesión. Se cierra con attacca_sesion_cerrar. */
AttaccaSesion *attacca_sesion_nueva(void);

/* Cierra una sesión. Un puntero nulo se ignora. */
void attacca_sesion_cerrar(AttaccaSesion *sesion);

/* Ejecuta una orden sobre la sesión.
 *
 * argumentos_json es un objeto JSON; nulo o vacío equivale a {}.
 * El resultado se libera con attacca_cadena_liberar. */
char *attacca_invocar(AttaccaSesion *sesion,
                      const char *orden,
                      const char *argumentos_json);

/* Nombres de las órdenes que esta biblioteca despacha, como array JSON.
 * Permite comprobar al arrancar que la biblioteca enlazada ofrece lo que la
 * interfaz espera, en lugar de descubrirlo al pulsar un botón. */
char *attacca_ordenes(void);

/* Versión de Attacca y versión de la norma que aplica, como objeto JSON. */
char *attacca_version(void);

/* Libera una cadena devuelta por cualquier función de este encabezado.
 * Un puntero nulo se ignora. */
void attacca_cadena_liberar(char *cadena);

#ifdef __cplusplus
}
#endif

#endif /* ATTACCA_H */
