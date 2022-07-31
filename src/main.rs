use std::io::{self, Write};
// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() -> std::io::Result<()> {
    // Uncomment this block to pass the first stage!
    let args: Vec<_> = std::env::args().collect();
    let image = &args[2];
    let command = &args[3];
    let command_args = &args[4..];

    init_sandbox("sandbox").unwrap();
    pull(image, "sandbox").unwrap();
    chroot("sandbox").unwrap();

    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .unwrap();

    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
    if let Some(code) = output.status.code() {
        std::process::exit(code);
    }
    Ok(())
}

fn pull(image: &str, path: &str) -> std::io::Result<()> {
    let parts = image.split(":").collect::<Vec<&str>>();
    let repo = if parts[0].contains("/") {
        parts[0].to_string()
    } else {
        format!("library/{}", parts[0])
    };
    let tag = if parts.len() < 2 { "latest" } else { parts[1] };

    let registry_uri = format!(
        "https://registry.hub.docker.com/v2/{}/manifests/{}",
        repo, tag
    );

    let response = reqwest::blocking::get(&registry_uri).unwrap();
    assert!(
        response.status() == reqwest::StatusCode::UNAUTHORIZED,
        "we don't want to handle all cases since it's just a challenge and not a prod app"
    );
    let auth_header = response
        .headers()
        .get("Www-Authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    let oauth_values = auth_header
        .trim_start_matches("Bearer ")
        .split(",")
        .map(parse_oauth_value)
        .collect::<std::collections::HashMap<&str, &str>>();

    let body: serde_json::Value = reqwest::blocking::get(&format!(
        "{}?service={}&scope={}",
        oauth_values.get("realm").unwrap(),
        oauth_values.get("service").unwrap().to_string(),
        oauth_values.get("scope").unwrap()
    ))
    .unwrap()
    .json()
    .unwrap();

    if let serde_json::Value::String(ref token) = body["token"] {
        let client = reqwest::blocking::Client::new();
        let manifest: serde_json::Value = client
            .get(&registry_uri)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .unwrap()
            .json()
            .unwrap();
        if let serde_json::Value::Array(ref fs_layers) = manifest["fsLayers"] {
            for fs_layer in fs_layers.iter() {
                if let serde_json::Value::String(ref digest) = fs_layer["blobSum"] {
                    let response = client
                        .get(&format!(
                            "https://registry.hub.docker.com/v2/{}/blobs/{}",
                            repo, digest
                        ))
                        .header("Authorization", format!("Bearer {}", token))
                        .send()
                        .unwrap();
                    let layer = response.bytes().unwrap();
                    let filename = digest.replace(":", "_");
                    let mut file = std::fs::File::create(format!("{}.tar", filename))?;
                    file.write_all(&layer)?;
                    let output = std::process::Command::new("tar")
                        .args(["xf", &format!("{}.tar", filename), "-C", path])
                        .output()
                        .unwrap();
                    assert!(output.status.success());
                }
            }
        }
    }
    Ok(())
}

fn parse_oauth_value(value: &str) -> (&str, &str) {
    let parts = value.split("=").collect::<Vec<&str>>();
    assert!(parts.len() == 2);
    let key = &parts[0];
    let value = &parts[1].trim_matches('\"');
    (key, value)
}

fn chroot(path: &str) -> std::io::Result<()> {
    let code = unsafe { libc::chroot(format!("{}\0", path).as_ptr().cast::<i8>()) };
    if code != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let code = unsafe { libc::unshare(libc::CLONE_NEWPID) };
    if code != 0 {
        return Err(std::io::Error::last_os_error());
    }
    std::env::set_current_dir("/")
}

fn init_sandbox(path: &str) -> std::io::Result<()> {
    if std::path::Path::new(path).exists() {
        std::fs::remove_dir_all(path).unwrap();
    }
    std::fs::create_dir(path).unwrap();
    std::fs::create_dir(path.to_string() + "/dev").unwrap();
    std::fs::File::create(path.to_string() + "/dev/null").unwrap();
    Ok(())
}
