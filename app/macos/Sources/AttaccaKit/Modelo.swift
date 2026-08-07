import Foundation

// Tipos que devuelve la capa de órdenes.
//
// Los nombres de campo son los del crate `attacca-ordenes` y no se traducen: si
// allí cambia uno, aquí debe fallar la descodificación en lugar de quedarse
// callada con un valor por omisión.

public struct ProyectoResumen: Decodable, Sendable, Identifiable, Equatable {
    public let uid: String
    public let id: String
    public let titulo: String
    public let artista: String
    public let tipo: String
    public let nivel: String
    public let estado: String
    public let custodia: String
    public let etapa: String
    public let release_uid: String?
    public let ruta: String
}

public struct ReleaseResumen: Decodable, Sendable, Identifiable, Equatable {
    public let uid: String
    public let id: String
    public let titulo: String
    public let artista: String
    public let clase: String
    public let estado: String
    public let temas: Int
}

public struct Catalogo: Decodable, Sendable, Equatable {
    public let proyectos: [ProyectoResumen]
    public let releases: [ReleaseResumen]
    /// Manifiestos que no se pudieron interpretar. No se ocultan: son lo único
    /// que puede dejar material fuera del alcance de la aplicación.
    public let ilegibles: [[String]]
    public let paquetes_en_cuarentena: Int
    public let atraso_cuarentena: Int
}

public struct Hallazgo: Decodable, Sendable, Equatable, Identifiable {
    public let severidad: String
    public let clausula: String
    public let campo: String
    public let detalle: String

    public var id: String { clausula + campo + detalle }
}

public struct Conformidad: Decodable, Sendable, Equatable {
    public let pendientes: Int
    public let incumplimientos: Int
    public let excepciones: Int
    public let conforme: Bool
    public let hallazgos: [Hallazgo]
}

public struct ReplicaVista: Decodable, Sendable, Equatable, Identifiable {
    public let volumen: String
    public let ruta: String
    public let estado: String
    public let ultima_sync: String?

    public var id: String { volumen }
}

public struct Vencimiento: Decodable, Sendable, Equatable {
    /// `current`, `claim_due` o `reclaim_available`.
    public let situacion: String
    public let fecha: String
}

public struct PresupuestoVista: Decodable, Sendable, Equatable {
    public let actual: Int
    public let limite: Int
    public let excedido: Bool
    public let ruta: String
    public let replica: String
}

public struct AudioVista: Decodable, Sendable, Equatable {
    public let frecuencia: Int?
    public let bits: Int?
    public let afinacion: Int?
    public let tempo: Int?
    public let tonalidad: String?
    public let origen: String?
}

public struct VistaProyecto: Decodable, Sendable, Equatable {
    public let resumen: ProyectoResumen
    /// Fase activa, deducida del manifiesto y del contenido real.
    public let etapa: String
    /// Carpetas que la Tabla I.1 indica presentar en primer plano.
    public let carpetas_etapa: [String]
    /// Claves de las acciones admisibles en la fase.
    public let acciones: [String]
    public let condicion_siguiente: String
    public let carpetas_existentes: [String]
    public let conformidad: Conformidad
    public let replicas: [ReplicaVista]
    public let replica_activa: String?
    public let vencimiento: Vencimiento?
    public let presupuesto_ruta: PresupuestoVista
    public let audio: AudioVista
    public let audio_modificable: Bool
    public let ruta_absoluta: String
}

public struct Entrada: Decodable, Sendable, Equatable, Identifiable {
    public let nombre: String
    public let es_carpeta: Bool
    public let tamano: UInt64
    public let ruta: String

    public var id: String { ruta }
}

public struct AperturaRepositorio: Decodable, Sendable, Equatable {
    public let raiz: String
    public let aviso_sincronizacion: String?
    public let operaciones_a_medias: Int
    public let temporales_abandonados: Int
}

public struct EstadoReloj: Decodable, Sendable, Equatable {
    /// `synced`, `drifted` o `unavailable`.
    public let estado: String
    public let desviacion_ms: Int
    /// `ntp` o `http_date`.
    public let precision: String
    public let admite_emision: Bool
    public let marca: String
}

public struct ResultadoIntegridad: Decodable, Sendable, Equatable {
    public let comprobados: Int
    public let fallidos: Int
    public let rutas: [String]
}

public struct Identidad: Codable, Sendable, Equatable {
    public let organizacion: String
    public let persona: String

    public init(organizacion: String, persona: String) {
        self.organizacion = organizacion
        self.persona = persona
    }
}
