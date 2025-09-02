use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};

use clap::{self, Arg};

fn main() -> Result<(), Box<dyn Error>> {
    let cfg = parse_config()?;
    let source_dir: String = format!("/home/{}/synced/", cfg.user);

    // Google Drive
    let dest_gd = HashMap::from([
        ("mp", "/mnt/g"),
        ("drv", "G:"),
        ("dir", "/mnt/g/My Drive/synced/"),
        ("desc", "Google Drive")
    ]);

    // Get external SSD "Redkid" info
    let letter: String = cfg.drive_letter.to_lowercase();
    let mp_ot: String = format!("/mnt/{}", &letter);
    let drv_ot: String = format!("{}:", &letter.to_uppercase());
    let dir_ot: String = format!("/mnt/{}/synced/", &letter);

    // Redkid
    let dest_ot = HashMap::from([
        ("mp", mp_ot.as_str()),
        ("drv", drv_ot.as_str()),
        ("dir", dir_ot.as_str()),
        ("desc", "Redkid")
    ]);

    let mut dest_dirs = vec![dest_gd, dest_ot];

    // Iterate destination drives and try to mount and sync
    // them with local source directory
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

        let dir: &str = dest.get("dir").unwrap();

        // Print dry-run output of sync with local source directory
        let dry_run_rsync = Command::new("rsync")
            .args(["-a", "--itemize-changes", "--update", "--dry-run"])
            .args([source_dir.as_str(), dir])
            .output();

        if is_success(&dry_run_rsync) {
            let output = &dry_run_rsync.unwrap();
            let dry_run_rsync_output = String::from_utf8_lossy(&output.stdout);

            for line in dry_run_rsync_output.lines() {
                if line.starts_with(">") {
                    println!("{}", line);
                }
            }
        } else {
            eprintln!("Could not sync {} with local directory", dir);
            dest.insert("err", "sync-err");
            continue;
        }

        // Sync drive with local source directory
        let rsync_from_local = Command::new("rsync")
            .args(["-a", "--update", source_dir.as_str(), dir])
            .output();

        if is_success(&rsync_from_local) {
            let desc: &str = dest.get("desc").unwrap();
            println!(">>> Synced {} with local directory", desc);
        } else {
            eprintln!("Could not sync {} with local directory", dir);
            dest.insert("err", "sync-err");
        }
    }

    // Iterate destination drives again and try to sync
    // them with each other
    for dest in dest_dirs.iter() {
        if dest.get("err").is_some() {
            continue;
        }

        let dir: &str = dest.get("dir").unwrap();
        let desc: &str = dest.get("desc").unwrap();

        for other_dest in dest_dirs.iter() {
            let other_dir: &str = other_dest.get("dir").unwrap();

            if other_dir != dir && other_dest.get("err").is_none() {
                let rsync = Command::new("rsync")
                    .args(["-irlD", "--ignore-existing"])
                    .args([dir, other_dir])
                    .output();

                if is_success(&rsync) {
                    let output = &rsync.unwrap();
                    let rsync_output = String::from_utf8_lossy(&output.stdout);

                    if !rsync_output.is_empty() {
                        println!("{}", rsync_output);
                    }

                    let other_desc: &str = other_dest.get("desc").unwrap();
                    println!(">>> Synced {} with {}", other_desc, desc);
                } else {
                    eprintln!("Could not sync {} with {}", other_dir, dir);
                }
            }
        }
    }

    println!("That's all, folks!"); //TODO: REMOVE
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
                .help("External drive identifying letter"))
        .get_matches();

    let user: Option<&String> = matches.get_one("user");
    if user.is_none() {
        return Err("Argument required for --user");
    }

    let letter: Option<&String> = matches.get_one("drive-letter");
    if letter.is_none() {
        return Err("Argument required for --drive-letter");
    }

    Ok(Config {
        user: user.unwrap().to_string(),
        drive_letter: letter.unwrap().to_string(),
    })
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
