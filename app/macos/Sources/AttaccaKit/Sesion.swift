import CAttacca
import Foundation

/// Error que devuelve el núcleo.
///
/// El mensaje ya viene redactado con sus tres elementos —qué ocurrió, qué
/// consecuencia tiene y qué se puede hacer—, así que se presenta tal cual. La
/// interfaz no lo reescribe: si un mensaje se lee mal, se arregla en el núcleo,
/// donde lo comparten las tres interfaces.
public struct ErrorDeNucleo: LocalizedError, Sendable, Equatable {
    public let mensaje: String
    public var errorDescription: String? { mensaje }

    public init(_ mensaje: String) {
        self.mensaje = mensaje
    }
}

/// Sesión abierta contra una carpeta de trabajo.
///
/// Es un actor porque el núcleo guarda el estado tras un `Mutex` y no espera
/// llamadas simultáneas desde la interfaz. Serializar aquí cuesta menos que
/// razonar sobre concurrencia al otro lado de la frontera de C.
public actor Sesion {
    /// `AttaccaSesion` es un tipo incompleto en el encabezado: se declara y no
    /// se define, para que su contenido pueda cambiar sin mover la frontera.
    /// Swift importa los punteros a un tipo así como `OpaquePointer`, de modo
    /// que aquí no hay nada que convertir.
    ///
    /// `nonisolated(unsafe)` porque `OpaquePointer` no es `Sendable` y `deinit`
    /// no está aislado en el actor. La seguridad no la da el compilador aquí,
    /// la dan dos hechos: el puntero se asigna una vez en `init` y no vuelve a
    /// escribirse, y el estado al otro lado vive tras un `Mutex` en Rust.
    private nonisolated(unsafe) let puntero: OpaquePointer

    /// Prepara el proceso y abre la sesión.
    ///
    /// `attacca_inicializar` captura el desplazamiento de zona horaria y debe
    /// ejecutarse antes de que existan otros hilos. Ocurre una sola vez aunque
    /// se abran varias sesiones.
    public init() {
        Self.preparado()
        guard let p = attacca_sesion_nueva() else {
            fatalError("no se pudo reservar la sesión del núcleo")
        }
        puntero = p
    }

    deinit {
        attacca_sesion_cerrar(puntero)
    }

    private static let inicializacion: Void = {
        attacca_inicializar()
    }()

    private static func preparado() {
        _ = inicializacion
    }

    // MARK: - Invocación

    /// Ejecuta una orden y descodifica su resultado.
    ///
    /// Los nombres de orden y de argumento son los mismos que emplea la
    /// interfaz web. Coincidir no es casualidad: las dos hablan con
    /// `attacca-ordenes` a través de la misma tabla de despacho.
    public func invocar<R: Decodable & Sendable>(
        _ orden: String,
        _ argumentos: [String: JSON] = [:],
        como _: R.Type = R.self
    ) throws -> R {
        let bruto = try invocarBruto(orden, argumentos)
        do {
            return try JSONDecoder().decode(Respuesta<R>.self, from: bruto).ok
        } catch let e as ErrorDeNucleo {
            throw e
        } catch {
            throw ErrorDeNucleo(
                """
                La respuesta de «\(orden)» no tiene la forma esperada. \
                La pantalla no se ha podido actualizar. \
                Comprobar que la aplicación y la biblioteca son de la misma versión.
                """
            )
        }
    }

    /// Ejecuta una orden cuyo resultado no interesa.
    public func ejecutar(_ orden: String, _ argumentos: [String: JSON] = [:]) throws {
        _ = try invocarBruto(orden, argumentos)
    }

    private func invocarBruto(_ orden: String, _ argumentos: [String: JSON]) throws -> Data {
        let cuerpo = try JSONEncoder().encode(argumentos)
        let json = String(decoding: cuerpo, as: UTF8.self)

        // Copias propias en lugar de dos `withCString` anidados. El anidamiento
        // hace que la cadena de fuera cruce hacia el cierre de dentro, que está
        // aislado en el actor, y el compilador lo rechaza con razón. Dos
        // reservas por llamada no se notan en una aplicación que responde a
        // clics.
        guard let o = strdup(orden), let a = strdup(json) else {
            throw ErrorDeNucleo(
                """
                No hay memoria para preparar la orden «\(orden)». \
                La operación no se ha iniciado. \
                Cerrar otras aplicaciones y volver a intentarlo.
                """
            )
        }
        defer {
            free(o)
            free(a)
        }

        guard let salida = attacca_invocar(puntero, o, a) else {
            throw ErrorDeNucleo(
                """
                El núcleo no devolvió nada al ejecutar «\(orden)». \
                La operación no se ha completado. \
                Cerrar la aplicación y volver a abrirla.
                """
            )
        }
        defer { attacca_cadena_liberar(salida) }

        let datos = Data(String(cString: salida).utf8)

        // Un fallo del núcleo llega como `{"error": "..."}`. Se detecta antes
        // de intentar descodificar el resultado, para que el mensaje que
        // aparezca sea el suyo y no un error de descodificación.
        if let fallo = try? JSONDecoder().decode(SoloError.self, from: datos) {
            throw ErrorDeNucleo(fallo.error)
        }
        return datos
    }

    // MARK: - Comprobación al arrancar

    /// Comprueba que la biblioteca enlazada despacha las órdenes que la
    /// interfaz va a pedirle.
    ///
    /// Enlazar contra una biblioteca de otra versión no da error hasta que
    /// alguien pulsa el botón que usa la orden que falta. Preguntarlo al
    /// arrancar convierte ese fallo tardío en uno inmediato y con nombre.
    public func comprobarOrdenes(_ requeridas: [String]) throws {
        guard let salida = attacca_ordenes() else {
            throw ErrorDeNucleo(
                """
                La biblioteca del núcleo no responde. \
                La aplicación no puede operar. \
                Volver a instalar Attacca.
                """
            )
        }
        defer { attacca_cadena_liberar(salida) }
        let datos = Data(String(cString: salida).utf8)
        let disponibles = Set(try JSONDecoder().decode(Respuesta<[String]>.self, from: datos).ok)

        let faltan = requeridas.filter { !disponibles.contains($0) }.sorted()
        guard faltan.isEmpty else {
            throw ErrorDeNucleo(
                """
                La biblioteca del núcleo no ofrece \(faltan.joined(separator: ", ")). \
                Parte de la aplicación no funcionaría. \
                Volver a instalar Attacca para que las dos piezas coincidan.
                """
            )
        }
    }

    /// Versión de Attacca y de la norma que aplica.
    public func version() throws -> Version {
        guard let salida = attacca_version() else {
            throw ErrorDeNucleo(
                """
                La biblioteca del núcleo no informa de su versión. \
                No se puede comprobar la correspondencia. \
                Volver a instalar Attacca.
                """
            )
        }
        defer { attacca_cadena_liberar(salida) }
        let datos = Data(String(cString: salida).utf8)
        return try JSONDecoder().decode(Respuesta<Version>.self, from: datos).ok
    }
}

// MARK: - Forma de la respuesta

private struct Respuesta<T: Decodable>: Decodable {
    let ok: T
}

private struct SoloError: Decodable {
    let error: String
}

public struct Version: Decodable, Sendable, Equatable {
    public let attacca: String
    public let stave: String
}

// MARK: - Argumentos

/// Valor de argumento para una orden.
///
/// Las órdenes reciben objetos con cadenas, números, booleanos y objetos
/// anidados. Un tipo cerrado sobre esos casos evita `Any` y deja que el
/// compilador compruebe lo que se envía.
public enum JSON: Encodable, Sendable, Equatable {
    case texto(String)
    case entero(Int)
    case booleano(Bool)
    case nulo
    case objeto([String: JSON])
    case lista([JSON])

    public func encode(to encoder: any Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .texto(let v): try c.encode(v)
        case .entero(let v): try c.encode(v)
        case .booleano(let v): try c.encode(v)
        case .nulo: try c.encodeNil()
        case .objeto(let v): try c.encode(v)
        case .lista(let v): try c.encode(v)
        }
    }
}

extension JSON: ExpressibleByStringLiteral {
    public init(stringLiteral value: String) { self = .texto(value) }
}

extension JSON: ExpressibleByIntegerLiteral {
    public init(integerLiteral value: Int) { self = .entero(value) }
}

extension JSON: ExpressibleByBooleanLiteral {
    public init(booleanLiteral value: Bool) { self = .booleano(value) }
}

extension JSON: ExpressibleByNilLiteral {
    public init(nilLiteral _: ()) { self = .nulo }
}
