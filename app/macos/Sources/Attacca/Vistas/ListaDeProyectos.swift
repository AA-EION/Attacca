import AttaccaKit
import SwiftUI

/// Los proyectos, como tarjetas.
///
/// La versión web presentaba una tabla de seis columnas: título, artista,
/// etapa, custodia, estado y nivel. Cinco de las seis interesan una vez al mes.
/// Aquí cada proyecto dice su nombre, de quién es, y una sola marca con lo que
/// hay que saber de él ahora mismo.
struct ListaDeProyectos: View {
    @Bindable var modelo: Modelo

    private let columnas = [GridItem(.adaptive(minimum: 260, maximum: 420), spacing: 16)]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 28) {
                encabezado

                if let c = modelo.catalogo {
                    if c.proyectos.isEmpty {
                        vacio
                    } else {
                        LazyVGrid(columns: columnas, spacing: 16) {
                            ForEach(c.proyectos) { p in
                                TarjetaDeProyecto(proyecto: p) {
                                    Task { await modelo.abrir(p.uid) }
                                }
                            }
                        }
                    }

                    if !c.releases.isEmpty {
                        Text(Textos.Seccion.lanzamientos)
                            .font(.caption).fontWeight(.semibold)
                            .textCase(.uppercase)
                            .foregroundStyle(.secondary)
                        LazyVGrid(columns: columnas, spacing: 16) {
                            ForEach(c.releases) { r in
                                TarjetaDeLanzamiento(release: r)
                            }
                        }
                    }
                } else if modelo.cargando {
                    ProgressView().frame(maxWidth: .infinity)
                }
            }
            .padding(28)
            .frame(maxWidth: 1000, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
    }

    private var encabezado: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(Textos.Seccion.proyectos)
                .font(.system(size: 32, weight: .bold))
            Spacer()
            if let n = modelo.catalogo?.paquetes_en_cuarentena, n > 0 {
                Label("\(n)", systemImage: "tray.and.arrow.down")
                    .labelStyle(.titleAndIcon)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .cristal(forma: Capsule())
            }
        }
    }

    private var vacio: some View {
        VStack(spacing: 8) {
            Text(Textos.Vacio.sinProyectos).font(.title3).fontWeight(.medium)
            Text(Textos.Vacio.primerProyecto).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 80)
    }
}

struct TarjetaDeProyecto: View {
    let proyecto: ProyectoResumen
    let alPulsar: () -> Void

    var body: some View {
        Button(action: alPulsar) {
            VStack(alignment: .leading, spacing: 6) {
                Text(proyecto.titulo.isEmpty ? proyecto.id : proyecto.titulo)
                    .font(.title3).fontWeight(.semibold)
                    .lineLimit(2)
                    .multilineTextAlignment(.leading)
                Text(proyecto.artista)
                    .font(.callout).foregroundStyle(.secondary)
                HStack(spacing: 6) {
                    ForEach(marcas, id: \.texto) { m in
                        Insignia(texto: m.texto, tono: m.tono)
                    }
                }
                .padding(.top, 4)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(18)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .cristal()
    }

    /// Una marca dice dónde está el trabajo. La segunda solo aparece cuando hay
    /// algo que atender.
    private var marcas: [(texto: String, tono: Insignia.Tono)] {
        var out: [(String, Insignia.Tono)] = []
        if proyecto.custodia != "propia" {
            out.append((Textos.donde[proyecto.custodia] ?? proyecto.custodia, .pendiente))
        } else if ["sealed", "archived", "delivered"].contains(proyecto.estado) {
            out.append((Textos.situacion[proyecto.estado] ?? proyecto.estado, .neutro))
        } else {
            out.append((Textos.fase[proyecto.etapa] ?? proyecto.etapa, .fase))
        }
        if proyecto.estado == "onhold" {
            out.append((Textos.situacion["onhold"] ?? "En pausa", .neutro))
        }
        return out
    }
}

struct TarjetaDeLanzamiento: View {
    let release: ReleaseResumen

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(release.titulo).font(.title3).fontWeight(.semibold)
            Text(release.artista).font(.callout).foregroundStyle(.secondary)
            HStack(spacing: 8) {
                Insignia(texto: release.clase, tono: .neutro)
                Text(
                    release.temas == 1
                        ? Textos.Cuenta.unTema
                        : String(format: Textos.Cuenta.variosTemas, release.temas)
                )
                    .font(.caption).foregroundStyle(.secondary)
            }
            .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(18)
        .cristal()
    }
}

/// Marca de estado. Una o dos palabras, con su color.
struct Insignia: View {
    enum Tono { case fase, bien, pendiente, problema, salvedad, neutro }

    let texto: String
    var tono: Tono = .neutro

    var body: some View {
        Text(texto)
            .font(.caption).fontWeight(.semibold)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.14), in: Capsule())
            .foregroundStyle(color)
    }

    private var color: Color {
        switch tono {
        case .fase: .accentColor
        case .bien: .green
        // Un dato que aún no toca no es un fallo, y no se pinta en rojo.
        case .pendiente: .orange
        case .problema: .red
        case .salvedad: .purple
        case .neutro: .secondary
        }
    }
}
