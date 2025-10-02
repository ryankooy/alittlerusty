//! Utility functions for Drive Syncer

use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};
use anyhow::{bail, Result};

#[derive(Debug)]
pub struct DriveInfo<'a> {
    pub mountpoint: &'a str,
    pub drive: &'a str,
    pub dir: &'a str,
    pub desc: &'a str,
    pub err: Option<DestError>,
}

#[derive(Clone, Copy, Debug)]
pub enum DestError {
    MountError,
    SyncError,
}

impl DestError {
    pub fn kind(&self) -> String {
        match self {
            DestError::MountError => "mount".to_string(),
            DestError::SyncError => "sync".to_string(),
        }
    }
}

// MOUNTING

pub fn mount_drive(dest: &mut DriveInfo) -> Result<()> {
    // Try to create mountpoint
    let _ = fs::create_dir(dest.mountpoint);

    if is_mountpoint_empty(dest.mountpoint) {
        // Mount the drive contents at mountpoint
        let mount = Command::new("mount")
            .args(["-t", "drvfs", dest.drive, dest.mountpoint])
            .output();

        if !is_success(&mount) {
            dest.err = Some(DestError::MountError);
            bail!("Failed to mount {} at {}", dest.drive, dest.mountpoint);
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

// SYNCING

pub fn sync_dirs_with_local(
    dest: &mut DriveInfo,
    base_src_dir: &str,
    subdirs: &Vec<String>,
    hidden_files: &Vec<String>,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let mut rsync_opts = vec!["-a", "--no-links", "--itemize-changes", "--update"];

    if dry_run {
        rsync_opts.push("--dry-run");
    }

    if !hidden_files.is_empty() {
        // Sync hidden files
        if let Err(e) = copy_hidden_files(
            base_src_dir,
            dest.dir,
            dest.desc,
            &hidden_files,
            user,
            dry_run
        ) {
            dest.err = Some(DestError::SyncError);
            bail!("{}", e);
        }
    }

    // Sync with local subdirectories
    for subdir in subdirs.iter() {
        let src_dir = format!("{}/{}/", base_src_dir, subdir);
        let dest_dir = format!("{}/wsl/{}/{}/", dest.dir, user, subdir);

        let rsync = run_rsync(
            src_dir.as_str(),
            dest_dir.as_str(),
            "Local",
            dest.desc,
            subdir,
            &rsync_opts
        );

        if is_success(&rsync) {
            print_rsync_output_lines(&rsync);

            if dry_run {
                println!("Would sync `{}` with `{}`", dest_dir, src_dir);
            } else {
                println!("Synced `{}` with `{}`", dest_dir, src_dir);
            }
        } else {
            dest.err = Some(DestError::SyncError);
            bail!("Failed to sync `{}` with `{}`", dest_dir, src_dir);
        }
    }

    Ok(())
}

pub fn sync_dir(
    src_dir: &str,
    dest_dir: &str,
    src_desc: &str,
    dest_desc: &str,
    dry_run: bool,
) -> Result<()> {
    let mut rsync_opts = vec!["--itemize-changes", "--recursive", "--ignore-existing"];

    if dry_run {
        rsync_opts.push("--dry-run");
    }

    let rsync = run_rsync(src_dir, dest_dir, src_desc, dest_desc, "synced", &rsync_opts);

    if is_success(&rsync) {
        let output = get_stdout(&rsync);
        if !output.is_empty() {
            println!("{}", output);
        }

        if dry_run {
            println!("Would sync `{}` with `{}`", dest_dir, src_dir);
        } else {
            println!("Synced `{}` with `{}`", dest_dir, src_dir);
        }
    } else {
        bail!("Failed to sync `{}` with `{}`", dest_dir, src_dir);
    }

    Ok(())
}

fn run_rsync(
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

// COPYING

fn copy_hidden_files(
    src_dir: &str,
    base_dest_dir: &str,
    dest_desc: &str,
    files: &Vec<String>,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let dest_dir = format!("{}/wsl/{}/", base_dest_dir, user);

    if dry_run {
        println!("Would copy hidden files from `{}/` to `{}`", src_dir, dest_dir);
    } else {
        let cp = run_cp_hidden_files(src_dir, dest_dir.as_str(), dest_desc, files);

        if is_success(&cp) {
            println!("Copied hidden files from `{}/` to `{}`", src_dir, dest_dir);
        } else {
            bail!("Could not copy hidden files from `{}/` to `{}`", src_dir, dest_dir);
        }
    }

    Ok(())
}

fn run_cp_hidden_files(
    src_dir: &str,
    dest_dir: &str,
    dest_desc: &str,
    files: &Vec<String>,
) -> Result<Output, Error> {
    println!("\nLocal hidden files -> {}", dest_desc);
    println!("`{}`", files.join("`, `"));

    let mut hidden_files: Vec<String> = Vec::new();
    for filename in files.iter() {
        let path = format!("{}/{}", src_dir, filename);
        hidden_files.push(path);
    }

    // Run cp command
    Command::new("cp").args(hidden_files).arg(dest_dir).output()
}

// COMMAND OUTPUT

fn print_rsync_output_lines(output: &Result<Output, Error>) {
    for line in get_stdout(output).lines() {
        if line.starts_with(">") {
            println!("{}", line);
        }
    }
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
