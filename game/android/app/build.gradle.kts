// Propósito del archivo:
// Script de compilación Gradle del módulo `:app` para generar el APK distribuible
// en tiendas de terceros (como Uptodown) o compilaciones Debug por arquitectura.
// Integra `cargo ndk` obligatoriamente para compilar Rust + Lua 5.4 + wgpu sin Kotlin,
// permitiendo filtrar una ABI concreta mediante `-PrustAbi=arm64-v8a` o `armeabi-v7a`
// y elegir perfil con `-PrustProfile=debug|release`.

plugins {
    id("com.android.application") version "8.7.3"
}

val cargoProjectDir = file("../..")
val jniLibsDir = file("src/main/jniLibs")

val requestedAbi = (project.findProperty("rustAbi") as String?)?.trim()?.takeIf { it.isNotEmpty() }
val requestedProfile = ((project.findProperty("rustProfile") as String?)?.trim()?.lowercase())
    ?: if (gradle.startParameter.taskNames.any { it.contains("Debug", ignoreCase = true) }) "debug" else "release"

val targetAbis = if (requestedAbi != null) {
    listOf(requestedAbi)
} else {
    listOf("armeabi-v7a", "arm64-v8a", "x86_64")
}

val buildRustAndroid by tasks.registering(Exec::class) {
    group = "rust"
    description = "Compila la librería nativa Rust + Lua 5.4 vía cargo-ndk para: ${targetAbis.joinToString(", ")} ($requestedProfile)"
    workingDir = cargoProjectDir

    doFirst {
        if (requestedAbi != null && jniLibsDir.exists()) {
            jniLibsDir.listFiles()?.forEach { abiDir ->
                if (abiDir.isDirectory && abiDir.name != requestedAbi) {
                    abiDir.deleteRecursively()
                }
            }
        }
    }

    val cmd = mutableListOf("cargo", "ndk")
    for (abi in targetAbis) {
        cmd.add("-t")
        cmd.add(abi)
    }
    cmd.addAll(listOf("-p", "24", "-o", jniLibsDir.absolutePath, "build"))
    if (requestedProfile == "release") {
        cmd.add("--release")
    }
    commandLine(cmd)
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
            abiFilters.clear()
            abiFilters += targetAbis
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
