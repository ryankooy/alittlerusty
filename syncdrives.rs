use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};

fn main() {
    let source_dir = "/home/ranky/synced/";

    let dest_eins = HashMap::from([
        ("mp", "/mnt/g"),
        ("drv", "G:"),
        ("dir", "/mnt/g/My Drive/synced/"),
        ("desc", "Google Drive")
    ]);

    let dest_zwei = HashMap::from([
        ("mp", "/mnt/i"),
        ("drv", "I:"),
        ("dir", "/mnt/i/synced/"),
        ("desc", "Redkid")
    ]);

    let mut dest_dirs = vec![dest_eins, dest_zwei];

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
            .args([source_dir, dir])
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

        let rsync_from_local = Command::new("rsync")
            .args(["-a", "--update", source_dir, dir])
            .output();

        if is_success(&rsync_from_local) {
            let desc: &str = dest.get("desc").unwrap();
            println!(">>> Synced {} with local directory", desc);
        } else {
            eprintln!("Could not sync {} with local directory", dir);
            dest.insert("err", "sync-err");
        }
    }

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
                    println!("{}", rsync_output);

                    let other_desc: &str = other_dest.get("desc").unwrap();
                    println!(">>> Synced {} with {}", other_desc, desc);
                } else {
                    eprintln!("Could not sync {} with {}", other_dir, dir);
                }
            }
        }
    }

    println!("End"); //TODO: REMOVE
}

fn is_success(output: &Result<Output, Error>) -> bool {
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
