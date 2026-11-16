// Propósito del archivo:
// Configuración raíz de Gradle para empaquetar el APK de Horizonte Latam
// con compilación nativa Rust (cargo-ndk) sin dependencias de Google Play ni Kotlin.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "HorizonteLatam"
include(":app")
