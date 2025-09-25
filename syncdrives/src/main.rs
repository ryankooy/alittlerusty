//! Drive Syncer

use anyhow::Result;
use clap::{self, Parser};

mod config;
mod util;

use util::DriveInfo;

#[derive(Parser)]
#[command(name = "Drive Syncer")]
#[command(about = "Sync between drives", long_about = None)]
struct Cli {
    /// System username
    #[arg(short, long, value_name = "USER")]
    user: String,

    /// External drive's identifying letter
    #[arg(short = 'l', long, value_name = "LETTER")]
    external_drive_letter: Option<String>,

    /// External drive's nickname
    #[arg(short = 'n', long, value_name = "NICKNAME")]
    external_drive_nickname: Option<String>,

    /// Perform dry-run sync only
    #[arg(short, long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let base_src_dir = format!("/home/{}", cli.user);
    let mut sync_dest_dirs: bool = false;

    let cfg = config::get_config()?;
    let subdirs: Vec<String> = cfg.subdirs;
    let hidden_files: Vec<String> = cfg.hidden_files.unwrap_or(Vec::new());
    let mut dest_dirs = Vec::new();

    for drv_info in cfg.drives.iter() {
        dest_dirs.push(DriveInfo {
            mountpoint: drv_info.mountpoint.trim_end_matches('/'),
            drive: drv_info.drive.as_str(),
            dir: drv_info.dir.trim_end_matches('/'),
            desc: drv_info.desc.as_str(),
            err: None,
        });
    }

    // Get external drive info from args
    let (mountpoint, drive, dir, desc) = if let Some(letter) = cli.external_drive_letter {
        sync_dest_dirs = true;
        let mountpoint: String = format!("/mnt/{}", &letter);
        let drive: String = format!("{}:", &letter.to_uppercase());
        let dir: String = mountpoint.clone();

        let desc: String = if let Some(nickname) = cli.external_drive_nickname {
            nickname
        } else {
            String::from("External Drive")
        };

        (mountpoint, drive, dir, desc)
    } else {
        (String::new(), String::new(), String::new(), String::new())
    };

    if sync_dest_dirs {
        // External drive
        dest_dirs.push(DriveInfo {
            mountpoint: mountpoint.as_str(),
            drive: drive.as_str(),
            dir: dir.as_str(),
            desc: desc.as_str(),
            err: None,
        });
    }

    if cli.dry_run {
        println!("Dry-run sync");
    }

    // Iterate destinations and try to mount their drives and sync
    // their directories with local ones
    for dest in dest_dirs.iter_mut() {
        if let Err(e) = util::mount_drive(&dest) {
            eprintln!("{}", e);
            dest.err = Some("mount-err");
            continue;
        }

        if let Err(e) = util::sync_dirs_with_local(
            &dest,
            &subdirs,
            base_src_dir.as_str(),
            &hidden_files,
            cli.user.as_str(),
            cli.dry_run,
        ) {
            eprintln!("{}", e);
            dest.err = Some("sync-err");
            break;
        }
    }

    if sync_dest_dirs {
        // Iterate destinations again and try to sync their
        // synced/ directories with each other
        for src in dest_dirs.iter() {
            if src.err.is_none() {
                let src_sync_dir = format!("{}/synced/", src.dir);

                for dest in dest_dirs.iter() {
                    if dest.err.is_none() {
                        let dest_sync_dir = format!("{}/synced/", dest.dir);

                        if dest_sync_dir != src_sync_dir {
                            if let Err(e) = util::sync_dir(
                                src_sync_dir.as_str(),
                                dest_sync_dir.as_str(),
                                src.desc,
                                dest.desc,
                                cli.dry_run,
                            ) {
                                eprintln!("{}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
