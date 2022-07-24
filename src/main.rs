use std::io::{self, Write};
// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() -> std::io::Result<()> {
    // Uncomment this block to pass the first stage!
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];

    std::fs::remove_dir_all("/sandbox");
    std::fs::create_dir("/sandbox").unwrap();
    std::fs::create_dir("/sandbox/dev").unwrap();
    std::fs::copy(command, "/sandbox/app").unwrap();
    std::fs::File::create("/sandbox/dev/null").unwrap();
    let code = unsafe {
        libc::chroot("/sandbox\0".as_ptr() as *const i8)
    };
    if code != 0 {
        return Err(std::io::Error::last_os_error());
    }
    std::env::set_current_dir("/")?;
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
