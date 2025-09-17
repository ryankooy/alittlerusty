/*!
 * Hours Logger
 * Optionally allows specifying file for writing date/hours logged
 */

use std::io::{self, Write};
use anyhow::Result;
use clap::{self, Parser, Subcommand};
use tokio::{sync, task, time::Duration};

mod error;
mod state;
mod util;

use state::{LogCommand as Command, LogState};

#[derive(Parser)]
#[command(name = "Hours Logger")]
#[command(about = "Log hours worked", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Log Hours
    Log {
        /// Path of file to which to log hours
        #[arg(short, long, value_name = "FILE")]
        filename: Option<String>,
    },

    /// Read Hours
    Read {
        /// Path of file to which to read hours
        #[arg(short, long, value_name = "FILE")]
        filename: String,

        /// Path of file to which to write hours
        #[arg(short, long, value_name = "FILE")]
        out_filename: Option<String>,
    }
}

#[tokio::main]
async fn main() -> Result<(), error::CustomError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Log { filename } => {
            log_hours(&filename).await?;
        }
        Commands::Read { filename, out_filename } => {
            read_hours(filename, out_filename)?;
        }
    }

    Ok(())
}

async fn log_hours(filename: &Option<String>) -> Result<(), error::CustomError> {
    use termion::{event::Key, input::TermRead, raw::IntoRawMode};

    // Capture stdout and get cursor's starting line number
    let mut stdout = io::stdout().into_raw_mode()?;
    let start_line: u16 = util::get_cursor_start_line(&mut stdout)?;

    writeln!(stdout, "[S] Start, [P] Pause, [R] Resume, [Space] Toggle Pause, [Q] Quit\n")?;
    util::hide_cursor(&mut stdout)?;

    let (tx, mut rx) = sync::mpsc::channel::<Command>(100);

    // Key handler
    let input_handle = task::spawn_blocking(move || {
        let mut keys = io::stdin().keys();

        while let Some(Ok(key)) = keys.next() {
            // Match keypress to command
            let command = match key {
                Key::Char('s') => Some(Command::Start),
                Key::Char('p') => Some(Command::Pause),
                Key::Char('r') => Some(Command::Resume),
                Key::Char(' ') => Some(Command::TogglePause),
                Key::Char('q') | Key::Ctrl('c') => Some(Command::Quit),
                _ => None,
            };

            if let Some(cmd) = command {
                if tx.blocking_send(cmd).is_err() || matches!(cmd, Command::Quit) {
                    break;
                }
            }
        }
    });

    let mut state = LogState::new();
    let mut interval = tokio::time::interval(Duration::from_millis(50));
    let mut counter: u64 = 0;

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                // We got a command, so call a LogState method accordingly
                match cmd {
                    Some(Command::Start) => state.start(),
                    Some(Command::Pause) => state.pause(),
                    Some(Command::Resume) => state.resume(),
                    Some(Command::TogglePause) => state.toggle_pause(),
                    Some(Command::Quit) => {
                        state.quit();
                        break;
                    }
                    None => break,
                }
            }
            _ = interval.tick() => {
                // No input received, so write some stuff to stdout
                // if LogState is active
                if state.is_running() {
                    util::clear_line(&mut stdout, start_line)?;

                    if state.is_paused() {
                        writeln!(stdout, "Paused at {:.2} hours", state.get_total_hours())?;
                    } else {
                        if counter == u64::MAX {
                            counter = 0;
                        }
                        counter += 1;

                        writeln!(
                            stdout,
                            "{} min {}",
                            state.get_total_minutes(),
                            "★".repeat((counter % 20) as usize + 1)
                        )?;

                        util::clear_line(&mut stdout, start_line + 1)?;
                    }
                }
            }
        }
    }

    // Stop the key handler
    input_handle.abort();

    util::clear_line(&mut stdout, start_line)?;
    util::show_cursor()?;

    let hours: f64 = state.get_total_hours();

    // If hours were accrued, log them to given file and stdout
    if hours >= 0.01 {
        util::write_file(filename, hours)?;
        writeln!(stdout, "Hours logged: {:.2}", hours)?;
    } else {
        writeln!(stdout, "No hours logged")?;
    }

    util::clear_line(&mut stdout, start_line)?;

    Ok(())
}

fn read_hours(filename: String, out_filename: Option<String>) -> Result<()> {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use anyhow::Context;
    //use std::path::Path;

    // Open file
    let file = File::open(&filename)
        .with_context(|| format!("Failed to open file {}", filename))?;

    let mut hours_by_date: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_hours: f64 = 0.0;

    // Sum hours by date
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Some((date, hours_str)) = line.split_once(' ') {
            let hours: f64 = hours_str.parse::<f64>()?;
            let hours_for_day: &mut f64 = hours_by_date.entry(date.to_string()).or_insert(0.0f64);
            *hours_for_day += hours;
            total_hours += hours;
        }
    }

    println!("{:#?}", hours_by_date);
    println!("Total hours: {:.2}", total_hours);

    if let Some(out) = out_filename {
        println!("out file: {}", out);
    }

    Ok(())
}

