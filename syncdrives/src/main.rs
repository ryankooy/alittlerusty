//! Drive Syncer

use anyhow::{bail, Result};
use clap::{self, Parser, Subcommand};

mod config;
mod gdrive;
mod util;

use util::DriveInfo;

#[derive(Parser)]
#[command(name = "Drive Syncer")]
#[command(about = "Sync drives or upload file to Google Drive", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Sync drives
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

    /// Upload file to Google Drive
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
            let base_src_dir = format!("/home/{}", user);

            if dry_run {
                println!("::: Dry-run sync :::");
            }

            let subdirs: Vec<String> = cfg.subdirs;
            let hidden_files: Vec<String> = cfg.hidden_files.unwrap_or(Vec::new());

            let mut dests = Vec::new();

            for drv_info in cfg.drives.iter() {
                dests.push(DriveInfo {
                    mountpoint: drv_info.mountpoint.trim_end_matches('/'),
                    drive: drv_info.drive.as_str(),
                    dir: drv_info.dir.trim_end_matches('/'),
                    desc: drv_info.desc.as_str(),
                    err: None,
                });
            }

            let mut drive_added: bool = false;

            // Get external drive info from args
            let (mountpoint, drive, dir, desc) = if let Some(letter) = drive_letter {
                drive_added = true;
                let mountpoint: String = format!("/mnt/{}", &letter);
                let drive: String = format!("{}:", &letter.to_uppercase());
                let dir: String = mountpoint.clone();

                let desc: String = if let Some(nickname) = drive_nickname {
                    nickname
                } else {
                    String::from("External Drive")
                };

                (mountpoint, drive, dir, desc)
            } else {
                (String::new(), String::new(), String::new(), String::new())
            };

            if drive_added {
                // Add cli-specified drive to destinations
                dests.push(DriveInfo {
                    mountpoint: mountpoint.as_str(),
                    drive: drive.as_str(),
                    dir: dir.as_str(),
                    desc: desc.as_str(),
                    err: None,
                });
            }

            // Iterate destinations and try to mount their drives and sync
            // their directories with local ones
            println!("::: Syncing drives with local :::");
            for mut dest in dests.iter_mut() {
                if let Err(e) = util::mount_drive(&mut dest) {
                    eprintln!("{} mount error: {}", dest.desc, e);
                    continue;
                }

                if let Err(e) = util::sync_dirs_with_local(
                    &mut dest,
                    &subdirs,
                    base_src_dir.as_str(),
                    &hidden_files,
                    user.as_str(),
                    dry_run,
                ) {
                    eprintln!("{} sync error: {}", dest.desc, e);
                    eprintln!("Aborting syncs with local...");
                    break;
                }
            }

            // If multiple destinations specified, iterate them again and
            // try to sync their synced/ directories with each other
            if dests.len() > 1 {
                println!("\n::: Syncing between `synced` directories :::\n");
                for src in dests.iter() {
                    if src.err.is_none() {
                        let src_sync_dir = format!("{}/synced/", src.dir);

                        for dest in dests.iter() {
                            let dest_sync_dir = format!("{}/synced/", dest.dir);

                            if dest_sync_dir != src_sync_dir {
                                if let Some(e) = dest.err {
                                    println!(
                                        "Skipping {s} -> {d} sync due to {d} {err} error",
                                        s=src.desc, d=dest.desc, err=e.kind(),
                                    );
                                    continue;
                                }

                                if let Err(e) = util::sync_dir(
                                    src_sync_dir.as_str(),
                                    dest_sync_dir.as_str(),
                                    src.desc,
                                    dest.desc,
                                    dry_run,
                                ) {
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                    }
                }
            }
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
