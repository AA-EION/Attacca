import SwiftUI

// Liquid Glass, y qué hacer donde no lo hay.
//
// Los modificadores `glassEffect`, `GlassEffectContainer` y el estilo de botón
// `.glass` existen desde macOS 26. Este archivo es el único sitio donde se
// nombran: el resto de las vistas pide `.cristal(...)` y no sabe en qué sistema
// se está ejecutando.
//
// Concentrarlo aquí tiene una razón práctica. Una comprobación de
// disponibilidad repartida por veinte vistas se convierte, en la versión
// siguiente, en veinte sitios que revisar. En uno solo, la retirada del camino
// alternativo el día que el mínimo suba a macOS 26 es borrar este archivo a la
// mitad.
//
// TRES REGLAS QUE EL MATERIAL IMPONE, Y QUE AQUÍ SE RESPETAN:
//
//  1. El cristal es de la capa de navegación, nunca del contenido. Se aplica a
//     barras, controles flotantes y tarjetas de acción; jamás al fondo de una
//     lista de archivos, que es contenido.
//  2. No se apila cristal sobre cristal. Una superficie de cristal no puede
//     muestrear otra, y el resultado de intentarlo es una mancha gris.
//  3. Las piezas de cristal cercanas van dentro de un `GlassEffectContainer`
//     para que compartan región de muestreo y se fundan al acercarse.
//
// DOS CONDICIONES, NO UNA.
//
// `#available(macOS 26.0, *)` decide en tiempo de ejecución, y no basta: para
// que el compilador acepte `glassEffect` el SDK tiene que conocerlo, y el de
// Xcode 16 no lo conoce. De ahí `#if HAY_CRISTAL`, que `construir.sh` define
// cuando el compilador es 6.2 o posterior, que es el que trae Xcode 26.
//
// Con un Xcode anterior la aplicación se compila igual y se presenta con los
// materiales de siempre. Es lo que promete `docs/MACOS-SWIFT.md`, y hasta ahora
// no era cierto: sin la condición de compilación no compilaba en absoluto.
//
// Y una de accesibilidad: con `Reducir transparencia` activo no hay cristal.
// El sistema lo atenúa por su cuenta, pero la tarjeta de acción principal
// necesita además un fondo opaco propio para no perder contraste.

extension View {
    /// Superficie de cristal para un elemento de la capa de navegación.
    ///
    /// En macOS 26 emplea Liquid Glass. En 14 y en 15 cae en el material más
    /// parecido de esas versiones, que da la misma jerarquía sin la refracción.
    @ViewBuilder
    func cristal(
        forma: some Shape = RoundedRectangle(cornerRadius: 16, style: .continuous),
        destacado: Bool = false
    ) -> some View {
        #if HAY_CRISTAL
        if #available(macOS 26.0, *) {
            self.glassEffect(destacado ? .regular.tint(.accentColor) : .regular, in: forma)
        } else {
            self.materialDeSiempre(forma: forma, destacado: destacado)
        }
        #else
        self.materialDeSiempre(forma: forma, destacado: destacado)
        #endif
    }

    /// El material anterior a Liquid Glass. Es el camino que se recorre en
    /// macOS 14 y 15, y también el único que existe cuando se compila con un
    /// Xcode que no conoce macOS 26.
    fileprivate func materialDeSiempre(forma: some Shape, destacado: Bool) -> some View {
        self.background(
            destacado ? AnyShapeStyle(.thickMaterial) : AnyShapeStyle(.regularMaterial),
            in: forma
        )
    }

    /// Estilo del botón de la acción que la fase pide.
    @ViewBuilder
    func botonPrincipal() -> some View {
        #if HAY_CRISTAL
        if #available(macOS 26.0, *) {
            self.buttonStyle(.glassProminent).controlSize(.large)
        } else {
            self.buttonStyle(.borderedProminent).controlSize(.large)
        }
        #else
        self.buttonStyle(.borderedProminent).controlSize(.large)
        #endif
    }

    /// Estilo de los botones secundarios de una fase.
    @ViewBuilder
    func botonSecundario() -> some View {
        #if HAY_CRISTAL
        if #available(macOS 26.0, *) {
            self.buttonStyle(.glass)
        } else {
            self.buttonStyle(.bordered)
        }
        #else
        self.buttonStyle(.bordered)
        #endif
    }
}

/// Agrupa piezas de cristal próximas para que compartan región de muestreo.
///
/// Fuera de macOS 26 es una pila sin efecto: el espaciado lo pone quien la usa,
/// de modo que la disposición no cambia entre versiones.
struct GrupoDeCristal<Contenido: View>: View {
    var separacion: CGFloat = 12
    @ViewBuilder var contenido: Contenido

    var body: some View {
        #if HAY_CRISTAL
        if #available(macOS 26.0, *) {
            GlassEffectContainer(spacing: separacion) { contenido }
        } else {
            contenido
        }
        #else
        contenido
        #endif
    }
}

/// Fondo de la ventana.
///
/// Con Liquid Glass la ventana deja pasar el escritorio por los bordes de la
/// barra, y el contenido necesita un fondo propio que no compita con ella. Sin
/// Liquid Glass, el fondo es el de siempre.
struct FondoDeVentana: View {
    var body: some View {
        #if HAY_CRISTAL
        if #available(macOS 26.0, *) {
            Color.clear.background(.background)
        } else {
            Color(nsColor: .windowBackgroundColor)
        }
        #else
        Color(nsColor: .windowBackgroundColor)
        #endif
    }
}
