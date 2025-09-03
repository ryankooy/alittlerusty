use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::string::String;

use clap::{self, Arg, builder::PathBufValueParser};

fn main() -> Result<(), Box<dyn Error>> {
    let cfg = parse_config()?;
    let user: &str = cfg.user.as_str();

    let base_src_dir_string: String = format!("/home/{}", user);
    let base_src_dir: &str = base_src_dir_string.as_str();

    // Google Drive
    let dest_gdrv = HashMap::from([
        ("mp", "/mnt/g"),
        ("drv", "G:"),
        ("dir", "/mnt/g/My Drive"),
        ("desc", "Google Drive")
    ]);

    let mut dest_dirs = vec![dest_gdrv];
    let sync_external: bool = !cfg.drive_letter.is_empty();

    // Get external SSD "Redkid" info
    let letter: String = cfg.drive_letter.to_lowercase();
    let mp_extl: String = format!("/mnt/{}", &letter);
    let drv_extl: String = format!("{}:", &letter.to_uppercase());
    let dir_extl: String = mp_extl.clone();

    if sync_external {
        // Redkid
        let dest_extl = HashMap::from([
            ("mp", mp_extl.as_str()),
            ("drv", drv_extl.as_str()),
            ("dir", dir_extl.as_str()),
            ("desc", "Redkid")
        ]);

        dest_dirs.push(dest_extl);
    }

    let subdirs = vec!["bin", "docs", "scripts", "synced"];

    // Iterate destination drives and try to mount them and sync their
    // directories with local ones
    for dest in dest_dirs.iter_mut() {
        let mp: &str = dest.get("mp").unwrap();

        // Try to create mountpoint
        let _ = fs::create_dir(mp);

        // Check if mountpoint is empty
        let mp_empty: bool = match PathBuf::from(mp)
            .read_dir()
            .map(|mut i| i.next().is_none()) {
                Ok(is_empty) => is_empty,
                Err(e) => match e.kind() {
                    ErrorKind::InvalidInput => true,
                    _ => {
                        eprintln!("{}", e);
                        false
                    }
                }
        };

        if mp_empty {
            // Mount the drive contents at mountpoint
            let drv: &str = dest.get("drv").unwrap();
            let mount = Command::new("mount")
                .args(["-t", "drvfs", drv, mp])
                .output();

            if !is_success(&mount) {
                eprintln!("Could not mount {} at {}", drv, mp);
                dest.insert("err", "mount-err");
                continue;
            }
        }

        let base_dest_dir: &str = dest.get("dir").unwrap();

        // Sync with local subdirectories
        for subdir in subdirs.iter() {
            let src_dir_string: String = format!("{}/{}/", base_src_dir, subdir);
            let src_dir: &str = src_dir_string.as_str();

            let dest_dir_string: String = format!("{}/wsl/{}/{}/", base_dest_dir, user, subdir);
            let dest_dir: &str = dest_dir_string.as_str();

            // Try to create subdirectory path
            let _ = fs::create_dir_all(dest_dir);

            let desc: &str = dest.get("desc").unwrap();
            println!("\nLocal {}/ -> {} {}/", subdir, desc, subdir);

            // Sync with local subdirectory
            let rsync = Command::new("rsync")
                .args(["-a", "--no-links", "--itemize-changes", "--update"])
                .args([src_dir, dest_dir])
                .output();

            if is_success(&rsync) {
                let output = &rsync.unwrap();
                let rsync_output = String::from_utf8_lossy(&output.stdout);

                for line in rsync_output.lines() {
                    if line.starts_with(">") {
                        println!("{}", line);
                    }
                }

                println!("Synced {} with {}", dest_dir, src_dir);
            } else {
                eprintln!("Could not sync {} with {}", dest_dir, src_dir);
                dest.insert("err", "sync-err");
                break;
            }
        }
    }

    if sync_external {
        // Iterate destination drives again and try to sync their
        // synced/ directories with each other
        for dest in dest_dirs.iter() {
            if dest.get("err").is_some() {
                continue;
            }

            let src_sync_dir_string: String = format!("{}/wsl/{}/synced/", dest.get("dir").unwrap(), user);
            let src_sync_dir: &str = src_sync_dir_string.as_str();
            let src_desc: &str = dest.get("desc").unwrap();

            for other_dest in dest_dirs.iter() {
                let dest_sync_dir_string: String = format!("{}/wsl/{}/synced/", other_dest.get("dir").unwrap(), user);
                let dest_sync_dir: &str = dest_sync_dir_string.as_str();

                if dest_sync_dir != src_sync_dir && other_dest.get("err").is_none() {
                    let dest_desc: &str = other_dest.get("desc").unwrap();
                    println!("\n{} synced/ -> {} synced/", src_desc, dest_desc);

                    let rsync = Command::new("rsync")
                        .args(["--itemize-changes", "--recursive", "--ignore-existing"])
                        .args([src_sync_dir, dest_sync_dir])
                        .output();

                    if is_success(&rsync) {
                        let output = &rsync.unwrap();
                        let rsync_output = String::from_utf8_lossy(&output.stdout);

                        if !rsync_output.is_empty() {
                            println!("{}", rsync_output);
                        }

                        println!("Synced {} with {}", dest_sync_dir, src_sync_dir);
                    } else {
                        eprintln!("Could not sync {} with {}", dest_sync_dir, src_sync_dir);
                    }
                }
            }
        }
    }

    Ok(())
}

struct Config {
    user: String,
    drive_letter: String,
}

fn parse_config() -> Result<Config, &'static str> {
    let matches = clap::Command::new("Drive Syncer")
        .about("Sync between drives")
        .arg(Arg::new("user")
                .short('u')
                .long("user")
                .help("System username"))
        .arg(Arg::new("drive-letter")
                .short('l')
                .long("drive-letter")
                .help("External drive identifying letter")
                .value_parser(PathBufValueParser::default()))
        .get_matches();

    let user_arg: Option<&String> = matches.get_one("user");
    if user_arg.is_none() {
        return Err("Argument required for --user");
    }

    let user: String = user_arg.unwrap().to_string();
    let drive_letter: String = matches.get_one("drive-letter")
        .unwrap_or(&PathBuf::new())
        .display()
        .to_string();

    Ok(Config { user, drive_letter })
}

fn is_success(output: &Result<Output, io::Error>) -> bool {
    let mut success: bool = false;

    match output {
        Ok(output) => {
            if output.status.success() {
                success = true;
            } else {
                let mut err_str = String::from_utf8_lossy(&output.stderr);
                if err_str.is_empty() {
                    err_str = output.status.to_string().into();
                }
                eprintln!("ERROR: {}", err_str);
            }
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
        }
    }

    success
}
