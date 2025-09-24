//! Drive Syncer

use clap::{self, Parser};

mod util;
use util::Drive;

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

fn main() {
    let cli = Cli::parse();
    let base_src_dir = format!("/home/{}", cli.user);
    let subdirs = vec!["bin", "docs", "scripts"];
    let hidden_files = vec![
        ".bash_aliases", ".bashrc", ".config/nvim/init.vim", ".gitconfig",
        ".profile", ".tmux.conf", ".tmuxp.yaml"
    ];
    let mut sync_dest_dirs: bool = false;

    // Google Drive
    let dest_gdrv = Drive {
        mountpoint: "/mnt/g",
        drive: "G:",
        dir: "/mnt/g/My Drive",
        desc: "Google Drive",
        err: None,
    };

    let mut dest_dirs = vec![dest_gdrv];

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
        let dest_extl = Drive {
            mountpoint: mountpoint.as_str(),
            drive: drive.as_str(),
            dir: dir.as_str(),
            desc: desc.as_str(),
            err: None,
        };

        dest_dirs.push(dest_extl);
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
}
