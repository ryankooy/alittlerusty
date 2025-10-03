//! Drive Syncer

use anyhow::{bail, Result};
use clap::{self, Parser, Subcommand};

mod config;
mod gdrive;
mod util;

use config::Config;
use util::{DestError, DriveInfo};

#[derive(Parser)]
#[command(name = "Drive Syncer")]
#[command(about = "Sync drives or upload to Google Drive", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Sync external drives with local
    Sync {
        /// System username
        #[arg(short, long, value_name = "USER")]
        user: String,

        /// Additional drive's identifying letter
        #[arg(short = 'l', long, value_name = "LETTER")]
        drive_letter: Option<String>,

        /// Additional drive's nickname
        #[arg(short = 'n', long, value_name = "NICKNAME")]
        drive_nickname: Option<String>,

        /// Perform dry-run sync only
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Upload single file to Google Drive
    Upload {
        /// Local path of file to upload
        #[arg(short, long)]
        file: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Get info from config file
    let cfg = config::get_config()?;

    match cli.command {
        Commands::Sync {
            user,
            drive_letter,
            drive_nickname,
            dry_run,
        } => {
            sync_drives(&cfg, user, drive_letter, drive_nickname, dry_run)?;
        }
        Commands::Upload { file } => {
            if let Some(folder_id) = cfg.gd_folder_id {
                let hub = gdrive::get_drivehub().await?;

                gdrive::upload_file_to_drive(
                    &hub,
                    file.as_str(),
                    Some(folder_id.as_str()),
                )
                .await?;
            } else {
                bail!("No gd_folder_id specified in config");
            }
        }
    }

    Ok(())
}

/// Sync external drives with local and then sync between
/// external drives if multiple specified.
fn sync_drives(
    cfg: &Config,
    user: String,
    drive_letter: Option<String>,
    drive_nickname: Option<String>,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!("::: Dry-run sync :::");
    }

    let mut dests = Vec::new();

    for d in cfg.drives.iter() {
        dests.push(DriveInfo::from_drive(&d));
    }

    if let Some(letter) = drive_letter {
        // Add cli-specified drive to destinations
        dests.push(DriveInfo::new(letter, drive_nickname));
    }

    let base_src_dir = format!("/home/{}", user);
    let hidden_files: Vec<String> = cfg.hidden_files
        .clone()
        .unwrap_or(Vec::new());

    // Iterate destinations and try to mount their drives and sync
    // their directories with local ones
    println!("::: Syncing drives with local :::");
    for dest in dests.iter_mut() {
        if let Err(e) = util::mount_drive(&dest) {
            eprintln!("Error: {} - {}", dest.nickname, e);
            dest.err = Some(DestError::MountError);
            continue;
        }

        if let Err(e) = util::sync_dirs_with_local(
            &dest,
            base_src_dir.as_str(),
            &cfg.subdirs,
            &hidden_files,
            user.as_str(),
            dry_run,
        ) {
            dest.err = Some(DestError::SyncError);
            eprintln!("Error: {} - {}", dest.nickname, e);
            eprintln!("Aborting syncs with local...");
            break;
        }
    }

    // If multiple destinations specified, iterate them again and
    // try to sync their synced/ directories with each other
    if dests.len() > 1 {
        println!("\n::: Syncing between `synced` directories :::");
        for src in dests.iter() {
            if src.err.is_none() {
                let src_sync_dir = format!("{}/synced/", src.base_dir);

                for dest in dests.iter() {
                    let dest_sync_dir = format!("{}/synced/", dest.base_dir);

                    if dest_sync_dir != src_sync_dir {
                        if let Some(e) = dest.err {
                            println!(
                                "Skipping {s} -> {d} sync due to {d} {err} error",
                                s=src.nickname, d=dest.nickname, err=e.kind(),
                            );
                            continue;
                        }

                        if let Err(e) = util::sync_dir(
                            src_sync_dir.as_str(),
                            dest_sync_dir.as_str(),
                            src.nickname.as_str(),
                            dest.nickname.as_str(),
                            dry_run,
                        ) {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
