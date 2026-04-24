use std::process::Command;

fn main() {
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
