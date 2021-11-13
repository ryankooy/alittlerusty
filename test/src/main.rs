use std::fs::create_dir;
use std::io::{stdin, stdout, Write};

fn main() -> std::io::Result<()> {
    let mut directory = String::new();
    print!("Please enter the name of your new project: ");
    let _ = stdout().flush();
    stdin().read_line(&mut directory)
           .expect("Did not enter a correct string");
    let proper_dir = directory.replace(&['"', '\n', '\r'][..], "");
    let path = "C:/Users/rwkoo/Desktop/codec/Rust/";
    let path_to_dir = format!("{:?}",
                      format_args!("{}{}", path, proper_dir));
    create_dir(path_to_dir)?;
    Ok(())
}
