// swift-tools-version: 6.0
//
// Interfaz nativa de Attacca para macOS.
//
// El núcleo es el mismo que emplean la línea de órdenes y la interfaz web: el
// crate `attacca-ffi` se compila como biblioteca estática y se enlaza aquí. No
// hay una segunda implementación de la norma, ni puede haberla, porque esta
// aplicación no sabe nada de manifiestos: sabe pedir órdenes por su nombre.
//
// La biblioteca no se enlaza desde este archivo. La produce `construir.sh`, que
// compila las dos arquitecturas, las funde con `lipo` y pasa la ruta al
// enlazador. Declararla aquí con `unsafeFlags` ataría el paquete a una ruta
// absoluta y lo haría inutilizable como dependencia.

import PackageDescription

let package = Package(
    name: "Attacca",
    // Liquid Glass exige macOS 26. La aplicación no: en 14 y en 15 se presenta
    // con los materiales de esas versiones y funciona igual. Lo que decide es
    // la comprobación de disponibilidad de `Cristal.swift`, no este número.
    platforms: [.macOS(.v14)],
    targets: [
        // Encabezado del puente de C. El módulo se limita a exponerlo; los
        // símbolos los aporta la biblioteca estática al enlazar.
        .systemLibrary(name: "CAttacca"),

        // Envoltura en Swift: tipos, sesión y traducción de errores.
        .target(name: "AttaccaKit", dependencies: ["CAttacca"]),

        // La aplicación.
        .executableTarget(
            name: "Attacca",
            dependencies: ["AttaccaKit"],
            swiftSettings: [.swiftLanguageMode(.v6)]
        ),

        .testTarget(name: "AttaccaKitTests", dependencies: ["AttaccaKit"]),
    ]
)
