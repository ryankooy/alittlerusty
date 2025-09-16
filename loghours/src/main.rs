use std::io::{self, Write};
use anyhow::Result;
use clap::{self, Parser};
use termion::{event::Key, input::TermRead, raw::IntoRawMode};
use tokio::{sync, task};
use tokio::time::Duration;

mod error;
mod loopstate;
mod util;

use loopstate::{Command, State as LoopState};

#[derive(Parser)]
#[command(name = "Hour Logger")]
#[command(about = "Log hours worked", long_about = None)]
struct Cli {
    /// Path of file to which to log hours
    #[arg(short, long, value_name = "FILEPATH")]
    filepath: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), error::CustomError> {
    let cli = Cli::parse();

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
            // Match input keypress to command
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

    let mut state = LoopState::new();
    let mut interval = tokio::time::interval(Duration::from_millis(50));
    let mut counter: u64 = 0;

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                // We got a command, so call a LoopState method accordingly
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
                // if LoopState is active
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
                            "{} min {}\r",
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

    // Clear last couple lines and show cursor
    (0..=1).for_each(|i| util::clear_line(&mut stdout, start_line - i).unwrap());
    util::show_cursor()?;

    let hours: f64 = state.get_total_hours();

    // If hours were accrued, log them to given file and stdout
    if hours >= 0.01 {
        util::write_file(&cli.filepath, hours)?;
        writeln!(stdout, "Hours logged: {:.2}", hours)?;
    } else {
        writeln!(stdout, "No hours logged")?;
    }

    util::clear_line(&mut stdout, start_line)?;
    Ok(())
}
