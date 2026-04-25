use std::process::Command;
use std::path::Path;

fn main() {
    // Verificar si existe client_secret.json
    if !Path::new("client_secret.json").exists() {
        panic!("\n\nERROR: No se encontró 'client_secret.json' en la raíz del proyecto.\nEste archivo es necesario para compilar la aplicación.\nConsulta el README para saber cómo obtenerlo.\n\n");
    }

    let output = Command::new("git")
        .args(&["log", "-1", "--format=%cd", "--date=format:%Y%m%d%H%M%S"])
        .output();
    
    let version = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8(o.stdout).unwrap_or_else(|_| "unknown".to_string())
        },
        _ => "unknown".to_string(),
    };
    
    println!("cargo:rustc-env=APP_VERSION={}", version.trim());
}
