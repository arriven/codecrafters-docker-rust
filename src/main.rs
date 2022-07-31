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
    if parts.len() != 2 {
        println!("image {}", image);
    }
    let repo = parts[0];
    let tag = parts[1];
    //https://registry.hub.docker.com/v2/library/ubuntu/manifests/latest
    let response = reqwest::blocking::get(&format!("https://registry.hub.docker.com/v2/library/{}/manifests/{}", repo, tag)).unwrap();
    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        let auth_header = response.headers().get("Www-Authenticate").unwrap().to_str().unwrap();
        let oauth_endpoint = parse_oauth_header(auth_header);

        let response = reqwest::blocking::get(&format!("{}?service={}&scope={}", oauth_endpoint.realm, oauth_endpoint.service, oauth_endpoint.scope)).unwrap();
        
        let body: serde_json::Value = response.json().unwrap();
        if let serde_json::Value::String(ref token) = body["token"] {
            let client = reqwest::blocking::Client::new();
            let manifest: serde_json::Value = client.get(&format!("https://registry.hub.docker.com/v2/library/{}/manifests/{}", repo, tag))
                .header("Authorization", format!("Bearer {}", token)).send().unwrap().json().unwrap();
            if let serde_json::Value::Array(ref fs_layers) = manifest["fsLayers"] {
                for fs_layer in fs_layers.iter() {
                    if let serde_json::Value::String(ref digest) = fs_layer["blobSum"] {
                        let response = client.get(&format!("https://registry.hub.docker.com/v2/library/{}/blobs/{}", repo, digest))
                            .header("Authorization", format!("Bearer {}", token)).send().unwrap();
                        let layer = response.bytes().unwrap();
                        let filename = digest.replace(":", "_");
                        let mut file = std::fs::File::create(format!("{}.tar", filename))?;
                        file.write_all(&layer)?;
                        let output = std::process::Command::new("tar")
                            .args(["xf", &format!("{}.tar", filename), "-C", path])
                            .output()
                            .unwrap();
                        io::stdout().write_all(&output.stdout).unwrap();
                        io::stderr().write_all(&output.stderr).unwrap();
                        assert!(output.status.success());
                    }
                }
            } else {
                println!("wrong type");
            }
        }
    }
    //https://auth.docker.io/token\?service\=registry.docker.io\&scope\=repository:library/ubuntu:pull
    Ok(())
}

#[derive(Debug)]
struct OauthEndpoint {
    realm: String,
    service: String,
    scope: String,
}

fn parse_oauth_header(header: &str) -> OauthEndpoint {
    let auth_values = header.trim_start_matches("Bearer ").split(",").map(parse_oauth_value).collect::<std::collections::HashMap<&str,&str>>();
    OauthEndpoint{
        realm: auth_values.get("realm").unwrap().to_string(),
        service: auth_values.get("service").unwrap().to_string(),
        scope: auth_values.get("scope").unwrap().to_string(),
    }
}

fn parse_oauth_value(value: &str) -> (&str, &str) {
    let parts = value.split("=").collect::<Vec<&str>>();
    assert!(parts.len() == 2);
    let key = &parts[0];
    let value = &parts[1].trim_matches('\"');
    (key, value)
}

fn chroot(path: &str) -> std::io::Result<()> {
    let code = unsafe {
        libc::chroot(format!("{}\0", path).as_ptr().cast::<i8>())
    };
    if code != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let code = unsafe {
        libc::unshare(libc::CLONE_NEWPID)
    };
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
