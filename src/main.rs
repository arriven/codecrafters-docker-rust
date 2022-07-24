
use std::io::{self, Write};
// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
fn main() {
    // Uncomment this block to pass the first stage!
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];
    let output = std::process::Command::new(command)
        .args(command_args)
        .output()
        .unwrap();
    
    if output.status.success() {
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
    } else {
        std::process::exit(1);
    }
}
