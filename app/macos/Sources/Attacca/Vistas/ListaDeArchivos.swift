import AttaccaKit
import SwiftUI

/// Los archivos de la carpeta que la fase presenta.
///
/// Sin cristal a propósito: es contenido, y el material se reserva para la capa
/// de navegación. Una lista de archivos sobre cristal se lee peor y compite con
/// la tarjeta de la acción principal.
struct ListaDeArchivos: View {
    @Bindable var modelo: Modelo
    let proyecto: VistaProyecto

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Rotulo(Textos.Seccion.archivos)

            if proyecto.carpetas_etapa.count > 1 {
                HStack(spacing: 6) {
                    ForEach(proyecto.carpetas_etapa, id: \.self) { c in
                        Button(Textos.nombreDeCarpeta(c)) {
                            Task { await modelo.mostrar(carpeta: c) }
                        }
                        .buttonStyle(.plain)
                        .font(.callout)
                        .fontWeight(modelo.carpeta == c ? .semibold : .regular)
                        .foregroundStyle(modelo.carpeta == c ? Color.accentColor : .secondary)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 5)
                        .background(
                            modelo.carpeta == c ? Color.accentColor.opacity(0.14) : .clear,
                            in: Capsule()
                        )
                    }
                }
            }

            if modelo.archivos.isEmpty {
                Text(Textos.Vacio.sinArchivos)
                    .font(.callout).foregroundStyle(.secondary)
                    .padding(.vertical, 12)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(modelo.archivos.enumerated()), id: \.element.id) { i, e in
                        if i > 0 { Divider() }
                        fila(e)
                    }
                }
                .background(.quinary, in: RoundedRectangle(cornerRadius: 12))

                Text(
                    modelo.archivos.count == 1
                        ? Textos.Cuenta.unArchivo
                        : String(format: Textos.Cuenta.variosArchivos, modelo.archivos.count)
                )
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private func fila(_ e: Entrada) -> some View {
        let contenido = HStack(spacing: 10) {
            Image(systemName: e.es_carpeta ? "folder" : "waveform")
                .foregroundStyle(.secondary)
                .frame(width: 16)
            Text(e.nombre).lineLimit(1).truncationMode(.middle)
            Spacer()
            if !e.es_carpeta {
                Text(tamanoLegible(e.tamano))
                    .font(.callout).monospacedDigit()
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 9)
        .contentShape(Rectangle())

        if e.es_carpeta {
            Button { Task { await modelo.mostrar(carpeta: e.ruta) } } label: { contenido }
                .buttonStyle(.plain)
        } else {
            contenido
        }
    }
}

func tamanoLegible(_ n: UInt64) -> String {
    if n < 1024 { return "\(n) B" }
    let unidades = ["kB", "MB", "GB", "TB"]
    var v = Double(n) / 1024
    var i = 0
    while v >= 1024, i < unidades.count - 1 {
        v /= 1024
        i += 1
    }
    return String(format: v < 10 ? "%.1f %@" : "%.0f %@", v, unidades[i])
}

/// Detalle técnico. Cerrado por omisión.
///
/// Nada de lo que la versión anterior enseñaba se ha perdido: está aquí, con el
/// vocabulario del apartado 3 de la norma, para poder citarlo y compararlo con
/// otra implementación. Lo que ha cambiado es que ya no compite por la atención
/// de quien solo quiere grabar.
struct DetalleTecnico: View {
    let proyecto: VistaProyecto
    @State private var abierto = false

    var body: some View {
        DisclosureGroup(isExpanded: $abierto) {
            VStack(alignment: .leading, spacing: 10) {
                Text(Textos.Tecnico.nota)
                    .font(.caption).foregroundStyle(.secondary)
                    .padding(.bottom, 4)

                ForEach(filas, id: \.0) { clave, valor in
                    HStack(alignment: .top) {
                        Text(clave).foregroundStyle(.secondary)
                        Spacer()
                        Text(valor).multilineTextAlignment(.trailing)
                    }
                    .font(.callout)
                    Divider()
                }

                if !proyecto.conformidad.hallazgos.isEmpty {
                    Rotulo(Textos.Tecnico.conformidad).padding(.top, 8)
                    ForEach(proyecto.conformidad.hallazgos) { h in
                        HStack(alignment: .top, spacing: 8) {
                            Text(h.clausula)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(.secondary)
                                .frame(width: 44, alignment: .leading)
                            Text(
                                (Textos.Tecnico.severidad[h.severidad] ?? h.severidad)
                                    + "  " + h.detalle
                            )
                            .font(.callout)
                        }
                    }
                }
            }
            .padding(.top, 12)
        } label: {
            Text(Textos.Seccion.detalle)
                .font(.callout).fontWeight(.semibold)
                .foregroundStyle(.secondary)
        }
        .padding(18)
        .background(.quinary, in: RoundedRectangle(cornerRadius: 14))
    }

    private var filas: [(String, String)] {
        var f: [(String, String)] = [
            (Textos.Tecnico.identificadorLegible, proyecto.resumen.id),
            (Textos.Tecnico.identificadorInterno, proyecto.resumen.uid),
            (Textos.Tecnico.etapaActiva, Textos.Tecnico.etapa[proyecto.etapa] ?? proyecto.etapa),
            (
                Textos.Tecnico.custodiaRotulo,
                Textos.Tecnico.custodia[proyecto.resumen.custodia] ?? proyecto.resumen.custodia
            ),
            (Textos.Tecnico.nivel, proyecto.resumen.nivel),
            (
                Textos.Tecnico.presupuestoRuta,
                "\(proyecto.presupuesto_ruta.actual) de \(proyecto.presupuesto_ruta.limite) caracteres"
            ),
            (Textos.Tecnico.frecuencia, proyecto.audio.frecuencia.map { "\($0) Hz" } ?? "—"),
            (Textos.Tecnico.bits, proyecto.audio.bits.map { "\($0) bits" } ?? "—"),
            (Textos.Rotulo.tempo, proyecto.audio.tempo.map(String.init) ?? "—"),
            (Textos.Rotulo.tonalidad, proyecto.audio.tonalidad ?? "—"),
            (Textos.Tecnico.afinacion, proyecto.audio.afinacion.map { "\($0) Hz" } ?? "—"),
            (Textos.Tecnico.ruta, proyecto.ruta_absoluta),
        ]
        for r in proyecto.replicas {
            f.append((
                Textos.Tecnico.replicaRotulo,
                (Textos.Tecnico.replica[r.estado] ?? r.estado) + "  " + r.ruta
            ))
        }
        return f
    }
}
