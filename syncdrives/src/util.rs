//! Utility functions for Drive Syncer

use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, Output};
use anyhow::{bail, Result};

use crate::config::Drive;

#[derive(Debug)]
pub struct DriveInfo {
    pub letter: String,
    pub nickname: String,
    pub base_dir: String,
    pub mountpoint: String,
    pub err: Option<DestError>,
}

impl DriveInfo {
    pub fn new(letter: String, nickname: Option<String>) -> Self {
        let drive = Drive::new(letter, nickname, None);
        make_drive_info(&drive)
    }

    pub fn from_drive(drive: &Drive) -> Self {
        make_drive_info(drive)
    }
}

fn make_drive_info(drive: &Drive) -> DriveInfo {
    let letter = drive.get_letter();
    let nickname = drive.get_nickname();
    let base_dir = drive.get_base_dir();
    let mountpoint = drive.get_mountpoint();

    DriveInfo {
        letter,
        nickname,
        base_dir,
        mountpoint,
        err: None,
    }
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

pub fn mount_drive(dest: &DriveInfo) -> Result<()> {
    // Try to create mountpoint
    if let Err(e) = fs::create_dir(&dest.mountpoint) {
        match e.kind() {
            ErrorKind::AlreadyExists => (),
            _ => bail!("Failed to create {}: {}", dest.mountpoint, e),
        }
    }

    if is_mountpoint_empty(&dest.mountpoint) {
        // Mount the drive contents at mountpoint
        let mount = Command::new("mount")
            .args(["-t", "drvfs", dest.letter.as_str(), dest.mountpoint.as_str()])
            .output();

        if !is_success(&mount) {
            bail!("Failed to mount {} at {}", dest.letter, dest.mountpoint);
        }
    }

    Ok(())
}

fn is_mountpoint_empty(mountpoint: &String) -> bool {
    // Check if mountpoint is empty
    match PathBuf::from(mountpoint)
        .read_dir()
        .map(|mut i| i.next().is_none())
    {
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
    dest: &DriveInfo,
    base_src_dir: &str,
    subdirs: &Vec<String>,
    hidden_files: &Vec<String>,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let mut rsync_opts = vec![
        "-a", "--no-links", "--itemize-changes", "--update", "--delete",
    ];

    if dry_run {
        rsync_opts.push("--dry-run");
    }

    if !hidden_files.is_empty() {
        // Sync hidden files
        if let Err(e) = copy_hidden_files(
            base_src_dir,
            dest.base_dir.as_str(),
            dest.nickname.as_str(),
            &hidden_files,
            user,
            dry_run
        ) {
            bail!("{}", e);
        }
    }

    // Sync with local subdirectories
    for subdir in subdirs.iter() {
        let src_dir = format!("{}/{}/", base_src_dir, subdir);
        let dest_dir = format!("{}/wsl/{}/{}/", dest.base_dir, user, subdir);

        let rsync = run_rsync(
            src_dir.as_str(),
            dest_dir.as_str(),
            "Local",
            dest.nickname.as_str(),
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
            bail!("Failed to sync `{}` with `{}`", dest_dir, src_dir);
        }
    }

    Ok(())
}

pub fn sync_dir(
    src_dir: &str,
    dest_dir: &str,
    src_nickname: &str,
    dest_nickname: &str,
    dry_run: bool,
) -> Result<()> {
    let mut rsync_opts = vec![
        "--itemize-changes", "--recursive", "--ignore-existing",
    ];

    if dry_run {
        rsync_opts.push("--dry-run");
    }

    let rsync = run_rsync(
        src_dir, dest_dir, src_nickname, dest_nickname, "synced", &rsync_opts,
    );

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
    src_nickname: &str,
    dest_nickname: &str,
    subdir: &str,
    rsync_opts: &Vec<&str>,
) -> Result<Output, Error> {
    println!(
        "\n{src} {sdir}/ -> {dest} {sdir}/",
        src=src_nickname, dest=dest_nickname, sdir=subdir,
    );

    // Try to create subdirectory path
    let _ = fs::create_dir_all(dest_dir)?;

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
    dest_nickname: &str,
    files: &Vec<String>,
    user: &str,
    dry_run: bool,
) -> Result<()> {
    let dest_dir = format!("{}/wsl/{}/", base_dest_dir, user);

    if dry_run {
        println!("Would copy hidden files from `{}/` to `{}`", src_dir, dest_dir);
    } else {
        let cp = run_cp_hidden_files(
            src_dir, dest_dir.as_str(), dest_nickname, files,
        );

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
    dest_nickname: &str,
    files: &Vec<String>,
) -> Result<Output, Error> {
    println!("\nLocal hidden files -> {}", dest_nickname);
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
            eprintln!("{}", e);
        }
    }

    success
}
