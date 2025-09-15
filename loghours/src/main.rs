use std::fs::{File, OpenOptions};
use std::io::{self, Stdout, Write};
use std::path::Path;
use chrono::{Local};
use clap::{self, Parser};
use termion::clear;
use termion::cursor::{self, DetectCursorPos};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use tokio::{sync, task};
use tokio::time::Duration;

mod loopstate;
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
async fn main() -> io::Result<()> {
    println!("[S] Start, [P] Pause, [R] Resume, [Space] Toggle Pause, [Q] Quit\n");

    let cli = Cli::parse();
    let mut stdout = io::stdout().into_raw_mode()?;
    let start_line: u16 = get_cursor_line(&mut stdout) - 1;

    write!(stdout, "{}", cursor::Hide)?;
    stdout.flush()?;

    let (tx, mut rx) = sync::mpsc::channel::<Command>(100);

    // Key handler
    let input_handle = task::spawn_blocking(move || {
        let stdin = io::stdin();
        let mut keys = stdin.keys();

        while let Some(Ok(key)) = keys.next() {
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
                if state.is_running() {
                    clear_line(&mut stdout, start_line);
                    if state.is_paused() {
                        println!("Paused at {:.2} hours", state.get_total_hours());
                    } else {
                        if counter == u64::MAX {
                            counter = 0;
                        }
                        counter += 1;
                        println!(
                            "{} min {}\r",
                            state.get_total_minutes(),
                            "★".repeat((counter % 20) as usize + 1)
                        );
                        clear_line(&mut stdout, start_line + 1);
                    }
                }
            }
        }
    }

    input_handle.abort();
    (0..=1).for_each(|i| clear_line(&mut stdout, start_line - i));

    let total_hours: f64 = state.get_total_hours();
    write_file(&cli.filepath, total_hours)?;

    println!("Total hours: {:.2}", total_hours);
    write!(stdout, "{}", cursor::Show)?;
    clear_line(&mut stdout, start_line);

    Ok(())
}

fn get_cursor_line(stdout: &mut RawTerminal<Stdout>) -> u16 {
    let line = stdout.cursor_pos().unwrap().1;
    line
}

fn clear_line(stdout: &mut RawTerminal<Stdout>, line: u16) {
    write!(stdout, "{}{}", cursor::Goto(1, line), clear::CurrentLine).unwrap();
    stdout.flush().unwrap();
}

fn write_file(filepath: &Option<String>, time_elapsed: f64) -> io::Result<()> {
    if let Some(f) = filepath {
        if !Path::new(&f).exists() {
            let _ = File::create(f)?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(f)?;

        let date = Local::now().format("%Y-%m-%d").to_string();
        writeln!(file, "{} {:.2}", date, time_elapsed)?;
    }

    Ok(())
}
