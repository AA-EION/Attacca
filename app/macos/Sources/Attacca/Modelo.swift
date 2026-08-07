import AttaccaKit
import Foundation
import Observation

/// Estado de la interfaz. Nada de esto es normativo.
///
/// Todo procede del núcleo y se recalcula al recargar. El apartado 42 lo exige:
/// ninguna información normativa debe existir únicamente en la interfaz. Lo que
/// se guarda aquí es qué se está mirando, no qué es cierto.
@MainActor
@Observable
final class Modelo {
    enum Pantalla: Hashable {
        case proyectos
        case proyecto(String)
        case recibido
        case ajustes
    }

    var pantalla: Pantalla = .proyectos
    var catalogo: Catalogo?
    var proyecto: VistaProyecto?
    var carpeta = ""
    var archivos: [Entrada] = []
    var reloj: EstadoReloj?
    /// Resultado de la última comprobación de integridad, mientras se enseña.
    var resultadoIntegridad: ResultadoIntegridad?
    var raiz: String?
    var fallo: ErrorDeNucleo?
    var cargando = false

    private let sesion = Sesion()

    /// Órdenes sin las cuales la interfaz no puede presentar nada.
    ///
    /// Se comprueban al arrancar contra la biblioteca enlazada. Descubrir que
    /// falta una al pulsar un botón es peor que no abrir.
    private static let ordenesRequeridas = [
        "raiz_sugerida", "abrir_repositorio", "crear_repositorio", "catalogo",
        "ver_proyecto", "listar_carpeta", "abrir_en_explorador", "reloj_conocido",
        "comprobar_reloj", "verificar_integridad", "identidad", "crear_proyecto",
    ]

    // MARK: Arranque

    func arrancar() async {
        do {
            try await sesion.comprobarOrdenes(Self.ordenesRequeridas)

            let sugerida: String? = try await sesion.invocar("raiz_sugerida")
            guard let ruta = sugerida else { return }

            let apertura: AperturaRepositorio = try await sesion.invocar(
                "abrir_repositorio", ["raiz": .texto(ruta)]
            )
            raiz = apertura.raiz
            reloj = try await sesion.invocar("reloj_conocido")
            await recargarCatalogo()
        } catch let e as ErrorDeNucleo {
            fallo = e
        } catch {
            fallo = ErrorDeNucleo(error.localizedDescription)
        }
    }

    // MARK: Navegación

    func abrir(_ uid: String) async {
        pantalla = .proyecto(uid)
        await recargarProyecto(uid)
    }

    func volver() {
        proyecto = nil
        archivos = []
        pantalla = .proyectos
    }

    func mostrar(carpeta nueva: String) async {
        carpeta = nueva
        await recargarArchivos()
    }

    // MARK: Carga

    func recargarCatalogo() async {
        await conFallo { self.catalogo = try await self.sesion.invocar("catalogo") }
    }

    func recargarProyecto(_ uid: String) async {
        await conFallo {
            let v: VistaProyecto = try await self.sesion.invocar(
                "ver_proyecto", ["uid": .texto(uid)]
            )
            self.proyecto = v
            self.carpeta = v.carpetas_etapa.first ?? ""
        }
        await recargarArchivos()
    }

    func recargarArchivos() async {
        guard let p = proyecto else { return }
        await conFallo {
            self.archivos = try await self.sesion.invocar(
                "listar_carpeta",
                ["uid": .texto(p.resumen.uid), "relativa": .texto(self.carpeta)]
            )
        }
    }

    // MARK: Acciones

    /// El apartado 44.2 exige que esta acción esté disponible en todo momento.
    func abrirEnFinder() async {
        guard let p = proyecto else { return }
        await conFallo {
            try await self.sesion.ejecutar(
                "abrir_en_explorador",
                ["uid": .texto(p.resumen.uid), "relativa": .texto(self.carpeta)]
            )
        }
    }

    func comprobarIntegridad() async {
        guard let p = proyecto else { return }
        await conFallo {
            self.resultadoIntegridad = try await self.sesion.invocar(
                "verificar_integridad", ["uid": .texto(p.resumen.uid)]
            )
        }
    }

    func comprobarHora() async {
        await conFallo { self.reloj = try await self.sesion.invocar("comprobar_reloj") }
    }

    /// Ejecuta una acción de la fase.
    ///
    /// Las que consisten en dejar archivos en su sitio abren la carpeta que
    /// corresponde y no piden nada más, igual que en la interfaz web. Las que
    /// necesitan datos todavía no están en la interfaz nativa y remiten a la
    /// carpeta, que es donde el trabajo ocurre de todos modos.
    func ejecutar(_ clave: String) async {
        guard let p = proyecto else { return }
        switch clave {
        case "abrir_en_explorador":
            await abrirEnFinder()
        case "verificar_integridad":
            await comprobarIntegridad()
        default:
            let destino = p.carpetas_etapa.first ?? ""
            await mostrar(carpeta: destino)
            await abrirEnFinder()
        }
    }

    // MARK: Auxiliares

    /// El cierre no lleva `@Sendable` a propósito: uno que lo llevara no
    /// heredaría el aislamiento del actor principal, y este modelo escribe su
    /// propio estado desde dentro.
    private func conFallo(_ cuerpo: () async throws -> Void) async {
        cargando = true
        defer { cargando = false }
        do {
            try await cuerpo()
        } catch let e as ErrorDeNucleo {
            fallo = e
        } catch {
            fallo = ErrorDeNucleo(error.localizedDescription)
        }
    }
}
