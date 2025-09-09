use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::string::String;

// Run `cargo add clap --features derive`
use clap::{self, Parser};

#[derive(Parser)]
#[command(name = "Drive Syncer")]
#[command(about = "Sync between drives", long_about = None)]
struct Cli {
    /// System username
    #[arg(short, long, value_name = "USER")]
    user: String,

    /// External drive's identifying letter
    #[arg(short = 'l', long, value_name = "LETTER")]
    drive_letter: Option<String>,

    /// Perform dry-run sync only
    #[arg(short, long)]
    dry_run: bool,
}

fn main() {
    let cli = Cli::parse();
    let base_src_dir = format!("/home/{}", cli.user);
    let subdirs = vec!["bin", "docs", "scripts", "synced"];
    let mut sync_all: bool = false;

    // Google Drive
    let dest_gdrv = HashMap::from([
        ("mp", "/mnt/g"),
        ("drv", "G:"),
        ("dir", "/mnt/g/My Drive"),
        ("desc", "Google Drive")
    ]);

    let mut dest_dirs = vec![dest_gdrv];

    // Get external SSD "Redkid" info
    let (mp_extl, drv_extl, dir_extl) = if let Some(letter) = cli.drive_letter.as_deref() {
        sync_all = true;
        let mp: String = format!("/mnt/{}", &letter);
        let drv: String = format!("{}:", &letter.to_uppercase());
        let dir: String = mp.clone();
        (mp, drv, dir)
    } else {
        (String::new(), String::new(), String::new())
    };

    if sync_all {
        // Redkid
        let dest_extl = HashMap::from([
            ("mp", mp_extl.as_str()),
            ("drv", drv_extl.as_str()),
            ("dir", dir_extl.as_str()),
            ("desc", "Redkid")
        ]);

        dest_dirs.push(dest_extl);
    }

    if cli.dry_run {
        println!("Dry-run sync");
    }

    // Iterate destinations and try to mount their drives and sync
    // their directories with local ones
    for dest in dest_dirs.iter_mut() {
        if let Err(e) = mount_drive(&dest) {
            eprintln!("{}", e);
            dest.insert("err", "mount-err");
            continue;
        }

        if let Err(e) = sync_dirs_with_local(&dest, &subdirs, base_src_dir.as_str(), &cli) {
            eprintln!("{}", e);
            dest.insert("err", "sync-err");
            break;
        }
    }

    if sync_all {
        // Iterate destinations again and try to sync their
        // synced/ directories with each other
        for src in dest_dirs.iter() {
            if src.get("err").is_none() {
                let src_sync_dir = format!("{}/wsl/{}/synced/", src.get("dir").unwrap(), cli.user);
                let src_desc: &str = src.get("desc").unwrap();

                for dest in dest_dirs.iter() {
                    if dest.get("err").is_none() {
                        let dest_sync_dir = format!("{}/wsl/{}/synced/", dest.get("dir").unwrap(), cli.user);

                        if dest_sync_dir != src_sync_dir {
                            let dest_desc: &str = dest.get("desc").unwrap();

                            if let Err(e) = sync_between(
                                src_sync_dir.as_str(),
                                dest_sync_dir.as_str(),
                                src_desc,
                                dest_desc,
                                cli.dry_run
                            ) {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mount_drive(dest: &HashMap<&str, &str>) -> Result<(), Error> {
    let mp: &str = dest.get("mp").unwrap();

    // Try to create mountpoint
    let _ = fs::create_dir(mp);

    if is_mountpoint_empty(mp) {
        // Mount the drive contents at mountpoint
        let drv: &str = dest.get("drv").unwrap();
        let mount = Command::new("mount")
            .args(["-t", "drvfs", drv, mp])
            .output();

        if !is_success(&mount) {
            let err_msg = format!("Could not mount {} at {}", drv, mp);
            return Err(Error::new(ErrorKind::NotFound, err_msg));
        }
    }

    Ok(())
}

fn is_mountpoint_empty(mountpoint: &str) -> bool {
    // Check if mountpoint is empty
    match PathBuf::from(mountpoint).read_dir().map(|mut i| i.next().is_none()) {
        Ok(is_empty) => is_empty,
        Err(e) => match e.kind() {
            ErrorKind::InvalidInput => true,
            _ => {
                eprintln!("{}", e);
                false
            }
        }
    }
}

fn sync_dirs_with_local(
    dest: &HashMap<&str, &str>,
    subdirs: &Vec<&str>,
    base_src_dir: &str,
    cli: &Cli,
) -> Result<(), Error> {
    let mut rsync_opts = vec!["-a", "--no-links", "--itemize-changes", "--update"];

    if cli.dry_run {
        rsync_opts.push("--dry-run");
    }

    let base_dest_dir: &str = dest.get("dir").unwrap();
    let dest_desc: &str = dest.get("desc").unwrap();

    // Sync with local subdirectories
    for subdir in subdirs.iter() {
        let src_dir = format!("{}/{}/", base_src_dir, subdir);
        let dest_dir = format!("{}/wsl/{}/{}/", base_dest_dir, cli.user, subdir);
        let rsync = sync_dir(src_dir.as_str(), dest_dir.as_str(), "Local", dest_desc, subdir, &rsync_opts);

        if is_success(&rsync) {
            for line in get_stdout(&rsync).lines() {
                if line.starts_with(">") {
                    println!("{}", line);
                }
            }

            if cli.dry_run {
                println!("Would sync {} with {}", dest_dir, src_dir);
            } else {
                println!("Synced {} with {}", dest_dir, src_dir);
            }
        } else {
            let err_msg = format!("Could not sync {} with {}", dest_dir, src_dir);
            return Err(Error::new(ErrorKind::Other, err_msg));
        }
    }

    Ok(())
}

fn sync_between(
    src_dir: &str,
    dest_dir: &str,
    src_desc: &str,
    dest_desc: &str,
    dry_run: bool,
) -> Result<(), Error> {
    let mut rsync_opts = vec!["--itemize-changes", "--recursive", "--ignore-existing"];

    if dry_run {
        rsync_opts.push("--dry-run");
    }

    let rsync = sync_dir(src_dir, dest_dir, src_desc, dest_desc, "synced", &rsync_opts);

    if is_success(&rsync) {
        let output = get_stdout(&rsync);

        if !output.is_empty() {
            println!("{}", output);
        }

        if dry_run {
            println!("Would sync {} with {}", dest_dir, src_dir);
        } else {
            println!("Synced {} with {}", dest_dir, src_dir);
        }
    } else {
        let err_msg = format!("Could not sync {} with {}", dest_dir, src_dir);
        return Err(Error::new(ErrorKind::Other, err_msg));
    }

    Ok(())
}

fn sync_dir(
    src_dir: &str,
    dest_dir: &str,
    src_desc: &str,
    dest_desc: &str,
    subdir: &str,
    rsync_opts: &Vec<&str>,
) -> Result<Output, Error> {
        println!("\n{src} {sdir}/ -> {dest} {sdir}/", src=src_desc, dest=dest_desc, sdir=subdir);

        // Try to create subdirectory path
        let _ = fs::create_dir_all(dest_dir);

        // Run rsync command
        Command::new("rsync")
            .args(rsync_opts)
            .args([src_dir, dest_dir])
            .output()
}

fn get_stdout(output: &Result<Output, Error>) -> String {
    let out = output.as_ref().unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
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
                eprintln!("{}", err_str);
            }
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
        }
    }

    success
}
