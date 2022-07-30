use std::io::{self, Write};
// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() -> std::io::Result<()> {
    // Uncomment this block to pass the first stage!
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];

    init_sandbox("/sandbox").unwrap();
    std::fs::copy(command, "/sandbox/app").unwrap();
    chroot("/sandbox").unwrap();
    
    let output = std::process::Command::new("/app")
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

fn chroot(path: &str) -> std::io::Result<()> {
    let code = unsafe {
        libc::chroot(path.as_ptr().cast::<i8>())
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
