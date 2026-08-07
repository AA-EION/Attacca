import Testing
@testable import AttaccaKit

// Estas pruebas cruzan la frontera de C de verdad: enlazan contra la misma
// biblioteca estática que la aplicación. Lo que comprueban no es el núcleo,
// que tiene las suyas, sino que la envoltura de Swift lea bien lo que el
// núcleo devuelve y traduzca sus fallos en lugar de tragárselos.

@Suite("Puente con el núcleo")
struct PuenteTests {
    @Test("La biblioteca declara la versión de la norma que aplica")
    func version() async throws {
        let s = Sesion()
        let v = try await s.version()
        #expect(v.stave == "2.0")
        #expect(!v.attacca.isEmpty)
    }

    @Test("Las órdenes que la interfaz necesita están todas")
    func ordenes() async throws {
        let s = Sesion()
        try await s.comprobarOrdenes([
            "catalogo", "ver_proyecto", "listar_carpeta", "abrir_en_explorador",
        ])
    }

    @Test("Una orden inexistente se rechaza nombrándola")
    func ordenInexistente() async {
        let s = Sesion()
        await #expect(throws: ErrorDeNucleo.self) {
            let _: [String] = try await s.invocar("inventada")
        }
    }

    @Test("Sin carpeta de trabajo, el fallo del núcleo llega como tal")
    func sinRepositorio() async throws {
        let s = Sesion()
        do {
            let _: Catalogo = try await s.invocar("catalogo")
            Issue.record("se esperaba un fallo y no lo hubo")
        } catch let e as ErrorDeNucleo {
            // El núcleo redacta sus mensajes; la envoltura no los reescribe.
            #expect(e.mensaje == "sin_repositorio")
        }
    }

    @Test("Una orden sin argumentos responde sin necesitar repositorio")
    func declaracion() async throws {
        let s = Sesion()
        let texto: String = try await s.invocar("declaracion_conformidad")
        #expect(texto.contains("STAVE"))
    }
}

@Suite("Registro llano")
struct TextosTests {
    @Test("Toda fase del recorrido tiene nombre y frase")
    func recorridoCompleto() {
        for clave in Textos.recorrido {
            #expect(Textos.fase[clave] != nil, "falta el nombre de «\(clave)»")
            #expect(Textos.ahora[clave] != nil, "falta la frase de «\(clave)»")
        }
    }

    @Test("Las carpetas de la norma se leen en llano")
    func carpetas() {
        #expect(Textos.nombreDeCarpeta("05_STEMS") == "Stems")
        #expect(Textos.nombreDeCarpeta("00_ADMIN/Notes") == "Papeles › Notas")
        // Una carpeta que no esté en la tabla pierde el número y poco más.
        #expect(Textos.nombreDeCarpeta("99_OTRA_COSA") == "OTRA COSA")
    }

    @Test("El vocabulario normativo se conserva literal")
    func vocabulario() {
        #expect(Textos.Tecnico.custodia["en_transito"] == "en tránsito")
        #expect(Textos.Tecnico.replica["en_espera"] == "en espera")
        // Y el llano no lo repite: dice dónde está el proyecto, no su custodia.
        #expect(Textos.donde["en_transito"] == "De camino")
    }
}
