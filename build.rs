use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Credenciales OAuth que se embeben en el binario como último recurso (ver src/api.rs).
    // Si existe client_secret.json en la raíz, se embebe; si no, se usa un placeholder vacío
    // para que el proyecto compile igualmente (CI / clones limpios). En runtime, la app prioriza
    // ~/.config/pomotask/client_secret.json y luego ./client_secret.json.
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR no definido");
    let dest = Path::new(&out_dir).join("embedded_secret.json");

    let placeholder = r#"{"installed":{"client_id":"","project_id":"pomotask","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","client_secret":"","redirect_uris":["http://localhost"]}}"#;

    println!("cargo:rerun-if-changed=client_secret.json");
    if Path::new("client_secret.json").exists() {
        let content = fs::read_to_string("client_secret.json").unwrap_or_else(|_| placeholder.to_string());
        fs::write(&dest, content).expect("no se pudo escribir embedded_secret.json");
    } else {
        fs::write(&dest, placeholder).expect("no se pudo escribir embedded_secret.json");
        println!("cargo:warning=client_secret.json no encontrado: se compila SIN credenciales embebidas. Coloca tus credenciales en ~/.config/pomotask/client_secret.json para sincronizar con Google Tasks.");
    }

    // Versión = timestamp del último commit de git.
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=format:%Y%m%d%H%M%S"])
        .output();

    let version = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8(o.stdout).unwrap_or_else(|_| "unknown".to_string())
        }
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=APP_VERSION={}", version.trim());
}
