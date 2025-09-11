use std::fs::{File, OpenOptions};
use std::io::{self, Stdout, Write};
use std::path::Path;
use chrono::{Local};
use termion::clear;
use termion::cursor::{self, DetectCursorPos};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use tokio::{sync, task};
use tokio::time::{self, Duration, Instant};

#[derive(Debug, Clone, Copy)]
enum Command {
    Pause,
    TogglePause,
    Resume,
    Quit,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("[S] Start, [P] Pause, [R] Resume, [Space] Toggle Pause, [Enter] Quit");

    let mut stdout = io::stdout().into_raw_mode()?;
    let y_pos: u16 = get_cursor_y_pos(&mut stdout);

    write!(stdout, "{}", cursor::Hide)?;
    stdout.flush()?;

    // Detect keydown events
    for k in io::stdin().keys() {
        write!(stdout, "{}", clear::CurrentLine)?;
        stdout.flush()?;

        match k.unwrap() {
            Key::Char('s') | Key::Char('S') => {
                break;
            },
            _ => (),
        }
    }

    let now = Instant::now();
    let waiting: Vec<&str> = ["★    ", "  ★  ", "    ★"].to_vec();

    let (tx, mut rx) = sync::mpsc::channel::<Command>(100);

    // Key handler
    let input_handle = task::spawn_blocking(move || {
        let stdin = io::stdin();
        let mut keys = stdin.keys();

        while let Some(Ok(key)) = keys.next() {
            let command = match key {
                Key::Char(' ') => Some(Command::TogglePause),
                Key::Char('p') | Key::Char('P') => Some(Command::Pause),
                Key::Char('r') | Key::Char('R') => Some(Command::Resume),
                Key::Char('\n') | Key::Ctrl('c') => Some(Command::Quit),
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

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    Some(Command::TogglePause) => state.toggle_pause(),
                    Some(Command::Pause) => state.pause(),
                    Some(Command::Resume) => state.resume(),
                    Some(Command::Quit) | None => break,
                }
            }
            _ = interval.tick() => {
                if state.is_fresh {
                    clear_line(&mut stdout, y_pos - 1);
                }

                if !state.is_paused {
                    for _ in 1..=20 {
                        for i in waiting.iter() {
                            println!("{}", i);
                            let _ = time::sleep(Duration::from_millis(50));
                            clear_line(&mut stdout, y_pos - 1);
                        }
                    }

                    //FIXME: GET MINUTES TO DISPLAY
                    //let min: u64 = now.elapsed().as_secs() / 60;
                    //println!("{} minutes", min);
                    //let _ = time::sleep(Duration::from_millis(1000));
                    //clear_line(&mut stdout, y_pos - 1);
                } else if state.is_fresh {
                    let hours: f64 = now.elapsed().as_secs_f64() / 3600.0;
                    println!("Paused at {:.2} hours", hours);
                    write_file(hours)?;
                }
            }
        }
    }

    // Clean up
    input_handle.abort();

    let _ = io::stdin().lock().read_line();

    (1..=2).for_each(|i| clear_line(&mut stdout, y_pos - i));

    let total_hours: f64 = now.elapsed().as_secs_f64() / 3600.0;
    write_file(total_hours)?;
    println!("Total hours: {:.2}", total_hours);

    write!(stdout, "{}", cursor::Show)?;
    clear_line(&mut stdout, y_pos - 1);

    Ok(())
}

struct LoopState {
    is_paused: bool,
    is_fresh: bool,
    hours: f64,
}

impl LoopState {
    fn new() -> Self {
        Self {
            is_paused: false,
            is_fresh: false,
            hours: 0.0f64,
        }
    }

    fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        self.is_fresh = true;
    }

    fn pause(&mut self) {
        self.is_fresh = !self.is_paused;
        self.is_paused = true;
    }

    fn resume(&mut self) {
        self.is_fresh = self.is_paused;
        self.is_paused = false;
    }
}

fn get_cursor_y_pos(stdout: &mut RawTerminal<Stdout>) -> u16 {
    let y_pos = stdout.cursor_pos().unwrap().1;
    y_pos
}

fn clear_line(stdout: &mut RawTerminal<Stdout>, y_pos: u16) {
    write!(stdout, "{}{}", cursor::Goto(1, y_pos), clear::CurrentLine).unwrap();
    stdout.flush().unwrap();
}

fn write_file(time_elapsed: f64) -> io::Result<()> {
    //let filename: &str = "/home/ranky/docs/contract/hours.txt";
    let filename: &str = "/home/ranky/tmp.txt";

    if !Path::new(&filename).exists() {
        let _ = File::create(filename)?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(filename)
        .unwrap();

    let date = Local::now().format("%Y-%m-%d").to_string();
    writeln!(file, "{} {:.2}", date, time_elapsed)?;

    Ok(())
}
