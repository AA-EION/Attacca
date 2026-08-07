import AttaccaKit
import SwiftUI

@main
struct AttaccaApp: App {
    @State private var modelo = Modelo()

    var body: some Scene {
        Window(Textos.nombre, id: "principal") {
            Marco(modelo: modelo)
                .frame(minWidth: 900, minHeight: 600)
                .task { await modelo.arrancar() }
        }
        // El fondo unificado deja que la barra de título comparta superficie
        // con el contenido, que es lo que Liquid Glass espera de una ventana.
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 1180, height: 780)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}

/// Armazón: contenido y, debajo, la barra permanente.
struct Marco: View {
    @Bindable var modelo: Modelo

    var body: some View {
        VStack(spacing: 0) {
            contenido
                .frame(maxWidth: .infinity, maxHeight: .infinity)

            // La fase, dónde está el proyecto y en qué disco. Visibles sin
            // pedir nada, que es lo que exige el apartado 44.2.
            BarraDeSituacion(modelo: modelo)
        }
        .background(FondoDeVentana())
        .alert(
            Textos.nombre,
            isPresented: Binding(
                get: { modelo.fallo != nil },
                set: { if !$0 { modelo.fallo = nil } }
            ),
            presenting: modelo.fallo
        ) { _ in
            Button(Textos.accion["cerrar"] ?? "Cerrar", role: .cancel) {}
        } message: { e in
            Text(e.mensaje)
        }
    }

    @ViewBuilder
    private var contenido: some View {
        switch modelo.pantalla {
        case .proyectos:
            ListaDeProyectos(modelo: modelo)
        case .proyecto:
            if let p = modelo.proyecto {
                PantallaDeProyecto(modelo: modelo, proyecto: p)
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        case .recibido, .ajustes:
            ListaDeProyectos(modelo: modelo)
        }
    }
}

/// Barra permanente. Tres datos y nada que compita con ellos.
struct BarraDeSituacion: View {
    let modelo: Modelo

    var body: some View {
        HStack(spacing: 28) {
            if let p = modelo.proyecto {
                pareja(Textos.Rotulo.fase, Textos.fase[p.etapa] ?? p.etapa)
                pareja(
                    Textos.Rotulo.donde,
                    Textos.donde[p.resumen.custodia] ?? p.resumen.custodia
                )
                pareja(Textos.Rotulo.disco, discoActivo(p))
            }
            Spacer()
            // La hora solo se nombra cuando impide algo.
            if let r = modelo.reloj, !r.admite_emision {
                Label(
                    String(format: Textos.Aviso.horaDesviada, r.desviacion_ms / 1000),
                    systemImage: "clock.badge.exclamationmark"
                )
                .foregroundStyle(.orange)
            }
        }
        .font(.callout)
        .padding(.horizontal, 24)
        .padding(.vertical, 10)
        .background(.bar)
    }

    private func discoActivo(_ p: VistaProyecto) -> String {
        let activa = p.replicas.first { $0.volumen == p.replica_activa }
        guard let estado = activa?.estado else { return Textos.disco["activa"] ?? "" }
        return Textos.disco[estado] ?? estado
    }

    private func pareja(_ clave: String, _ valor: String) -> some View {
        HStack(spacing: 6) {
            Text(clave).foregroundStyle(.secondary)
            Text(valor).fontWeight(.medium)
        }
    }
}
