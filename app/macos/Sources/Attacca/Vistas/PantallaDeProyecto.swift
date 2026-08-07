import AttaccaKit
import SwiftUI

/// Un proyecto: qué toca hacer, con qué archivos y qué falta.
struct PantallaDeProyecto: View {
    @Bindable var modelo: Modelo
    let proyecto: VistaProyecto

    @State private var resultadoIntegridad: ResultadoIntegridad?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                encabezado
                Recorrido(faseActual: proyecto.etapa)

                if let v = proyecto.vencimiento, v.situacion != "current" {
                    AvisoDePlazo(vencimiento: v)
                }

                // Las dos tarjetas de la fila superior son de la capa de
                // navegación y están cerca: comparten región de muestreo para
                // que el cristal se funda en lugar de cortarse entre ellas.
                GrupoDeCristal(separacion: 20) {
                    HStack(alignment: .top, spacing: 20) {
                        TarjetaAhora(modelo: modelo, proyecto: proyecto)
                        VStack(spacing: 20) {
                            if !proyecto.conformidad.hallazgos.isEmpty {
                                TarjetaFalta(conformidad: proyecto.conformidad)
                            }
                            TarjetaOtrasCarpetas(modelo: modelo, proyecto: proyecto)
                        }
                        .frame(width: 300)
                    }
                }

                // Los archivos son contenido, no navegación: sin cristal.
                ListaDeArchivos(modelo: modelo, proyecto: proyecto)

                DetalleTecnico(proyecto: proyecto)
            }
            .padding(28)
            .frame(maxWidth: 1100, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .alert(
            Textos.nombre,
            isPresented: Binding(
                get: { resultadoIntegridad != nil },
                set: { if !$0 { resultadoIntegridad = nil } }
            ),
            presenting: resultadoIntegridad
        ) { _ in
            Button(Textos.accion["cerrar"] ?? "Cerrar", role: .cancel) {}
        } message: { r in
            Text(
                r.fallidos == 0
                    ? String(format: Textos.Aviso.integridadBien, r.comprobados)
                    : String(
                        format: Textos.Aviso.integridadMal,
                        r.fallidos, r.comprobados, r.rutas.joined(separator: ", ")
                    )
            )
        }
    }

    private var encabezado: some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: 2) {
                Text(proyecto.resumen.titulo.isEmpty ? proyecto.resumen.id : proyecto.resumen.titulo)
                    .font(.system(size: 32, weight: .bold))
                Text(proyecto.resumen.artista)
                    .font(.title3).foregroundStyle(.secondary)
            }
            Spacer()
            HStack(spacing: 10) {
                Button(Textos.accion["volver"] ?? "Volver") { modelo.volver() }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                    .keyboardShortcut(.escape, modifiers: [])

                Button(Textos.accion["abrir_en_explorador"] ?? "") {
                    Task { await modelo.abrirEnFinder() }
                }
                .botonSecundario()
                .keyboardShortcut("e", modifiers: .command)
            }
        }
    }
}

/// La fase, como recorrido: dice también qué ha quedado atrás y qué viene.
struct Recorrido: View {
    let faseActual: String

    var body: some View {
        let indice = Textos.recorrido.firstIndex(of: faseActual)

        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                if let actual = indice {
                    ForEach(Array(Textos.recorrido.enumerated()), id: \.offset) { i, clave in
                        if i > 0 {
                            Rectangle()
                                .fill(.quaternary)
                                .frame(width: 12, height: 1)
                        }
                        hito(
                            Textos.fase[clave] ?? clave,
                            hecho: i < actual,
                            actual: i == actual
                        )
                    }
                } else {
                    // Fase fuera del recorrido habitual: se presenta sola.
                    hito(Textos.fase[faseActual] ?? faseActual, hecho: false, actual: true)
                }
            }
        }
    }

    private func hito(_ nombre: String, hecho: Bool, actual: Bool) -> some View {
        HStack(spacing: 6) {
            Circle()
                .fill(actual ? Color.accentColor : (hecho ? .green : .quaternary))
                .frame(width: 7, height: 7)
            Text(nombre)
                .font(.callout)
                .fontWeight(actual ? .semibold : .regular)
                .foregroundStyle(actual ? Color.accentColor : .secondary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(
            actual ? Color.accentColor.opacity(0.14) : .clear,
            in: Capsule()
        )
    }
}

/// Lo que la fase pide, con la acción principal debajo.
struct TarjetaAhora: View {
    @Bindable var modelo: Modelo
    let proyecto: VistaProyecto

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Rotulo(Textos.Seccion.ahora)

            Text(Textos.ahora[proyecto.etapa] ?? "")
                .font(.title2).fontWeight(.semibold)

            if let clave = proyecto.acciones.first,
               let etiqueta = Textos.accion[clave] {
                Button(etiqueta) { Task { await modelo.ejecutar(clave) } }
                    .botonPrincipal()
                    .frame(maxWidth: .infinity)
            } else {
                Text(Textos.Vacio.sinAcciones).foregroundStyle(.secondary)
            }

            let resto = proyecto.acciones.dropFirst().compactMap { clave in
                Textos.accion[clave].map { (clave, $0) }
            }
            if !resto.isEmpty {
                FlujoDeBotones(acciones: Array(resto)) { clave in
                    Task { await modelo.ejecutar(clave) }
                }
            }

            Text(
                Textos.Rotulo.pasoSiguiente + ": "
                    + (Textos.paso[proyecto.condicion_siguiente] ?? proyecto.condicion_siguiente)
            )
            .font(.callout).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(22)
        .cristal(destacado: true)
    }
}

/// Botones secundarios que se reparten en las líneas que hagan falta.
struct FlujoDeBotones: View {
    let acciones: [(String, String)]
    let alPulsar: (String) -> Void

    var body: some View {
        ViewThatFits(in: .horizontal) {
            fila
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 170), spacing: 8)], spacing: 8) {
                botones
            }
        }
    }

    private var fila: some View {
        HStack(spacing: 8) { botones }
    }

    @ViewBuilder
    private var botones: some View {
        ForEach(acciones, id: \.0) { clave, etiqueta in
            Button(etiqueta) { alPulsar(clave) }
                .buttonStyle(.plain)
                .font(.callout)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .contentShape(Rectangle())
        }
    }
}

/// Lo que falta, en llano. Los apartados de la norma quedan en el detalle.
struct TarjetaFalta: View {
    let conformidad: Conformidad

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Rotulo(Textos.Seccion.pendiente)
            ForEach(conformidad.hallazgos) { h in
                HStack(alignment: .top, spacing: 8) {
                    Insignia(texto: etiqueta(h.severidad), tono: tono(h.severidad))
                    Text(h.detalle).font(.callout)
                }
            }
            if conformidad.pendientes > 0 {
                Text(Textos.Falta.notaPendiente)
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .cristal()
    }

    private func etiqueta(_ s: String) -> String {
        switch s {
        case "pendiente": Textos.Falta.pendiente
        case "excepcion": Textos.Falta.salvedad
        default: Textos.Falta.problema
        }
    }

    private func tono(_ s: String) -> Insignia.Tono {
        switch s {
        case "pendiente": .pendiente
        case "excepcion": .salvedad
        default: .problema
        }
    }
}

/// El resto de la estructura sigue alcanzable, fuera del recorrido principal,
/// como exige el apartado 44.2.
struct TarjetaOtrasCarpetas: View {
    @Bindable var modelo: Modelo
    let proyecto: VistaProyecto

    var body: some View {
        let otras = proyecto.carpetas_existentes.filter {
            !proyecto.carpetas_etapa.contains($0)
        }
        VStack(alignment: .leading, spacing: 8) {
            Rotulo(Textos.Seccion.otrasCarpetas)
            if otras.isEmpty {
                Text(Textos.Vacio.sinCarpetas).font(.callout).foregroundStyle(.secondary)
            } else {
                ForEach(otras, id: \.self) { c in
                    Button(Textos.nombreDeCarpeta(c)) {
                        Task { await modelo.mostrar(carpeta: c) }
                    }
                    .botonSecundario()
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            Text(Textos.Aviso.notaAbrir).font(.caption).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .cristal()
    }
}

struct AvisoDePlazo: View {
    let vencimiento: Vencimiento

    var body: some View {
        Label(texto, systemImage: "calendar.badge.exclamationmark")
            .font(.callout)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 10))
    }

    private var texto: String {
        let plantilla = vencimiento.situacion == "reclaim_available"
            ? Textos.Plazo.margenVencido
            : Textos.Plazo.vencido
        return String(format: plantilla, vencimiento.fecha)
    }
}

struct Rotulo: View {
    let texto: String
    init(_ texto: String) { self.texto = texto }

    var body: some View {
        Text(texto)
            .font(.caption).fontWeight(.semibold)
            .textCase(.uppercase)
            .kerning(0.5)
            .foregroundStyle(.secondary)
    }
}
