// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ZVaultSDK",
    platforms: [.macOS(.v13), .iOS(.v16), .tvOS(.v16), .watchOS(.v9)],
    products: [
        .library(name: "ZVault", targets: ["ZVault"]),
    ],
    targets: [
        .target(name: "ZVault", path: "Sources/ZVault"),
        .testTarget(name: "ZVaultTests", dependencies: ["ZVault"]),
    ]
)
