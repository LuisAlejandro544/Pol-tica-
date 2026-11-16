//! Propósito del archivo:
//! Binario ejecutable de escritorio para pruebas rápidas fuera de Android.

#[cfg(not(target_os = "android"))]
fn main() -> eframe::Result<()> {
    horizonte_latam::run_desktop()
}

#[cfg(target_os = "android")]
fn main() {}
