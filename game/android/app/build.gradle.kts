// Propósito del archivo:
// Script de compilación Gradle del módulo `:app` para generar el APK distribuible
// en tiendas de terceros (como Uptodown). Integra `cargo ndk` obligatoriamente para
// compilar el motor Rust + Lua 5.4 en arquitecturas de 32 bits (armeabi-v7a) y
// 64 bits (arm64-v8a, x86_64) sin usar código Kotlin.

plugins {
    id("com.android.application") version "8.7.3"
}

val cargoProjectDir = file("../..")
val jniLibsDir = file("src/main/jniLibs")

val buildRustAndroid by tasks.registering(Exec::class) {
    group = "rust"
    description = "Compila la librería nativa Rust + Lua 5.4 para 32-bit y 64-bit vía cargo-ndk"
    workingDir = cargoProjectDir
    commandLine(
        "cargo", "ndk",
        "-t", "armeabi-v7a",
        "-t", "arm64-v8a",
        "-t", "x86_64",
        "-p", "24",
        "-o", jniLibsDir.absolutePath,
        "build", "--release"
    )
}

android {
    namespace = "com.horizontelatam.estrategia"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "com.horizontelatam.estrategia"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            isShrinkResources = false
        }
        debug {
            isDebuggable = true
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs(jniLibsDir)
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }
}

tasks.configureEach {
    if (name.contains("merge") && name.contains("JniLibFolders")) {
        dependsOn(buildRustAndroid)
    }
}
