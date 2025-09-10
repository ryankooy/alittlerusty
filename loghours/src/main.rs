use std::fs::{File, OpenOptions};
use std::io::{self, stdin, stdout, Write};
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{Local};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn write_file(time_elapsed: f64) -> io::Result<()> {
    let filename: &str = "/home/ranky/docs/contract/hours.txt";

    if !Path::new(&filename).exists() {
        let _ = File::create(filename)?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(filename)
        .unwrap();

    let date = Local::now().format("%Y-%m-%d").to_string();
    if let Err(e) = writeln!(file, "{:.2} hrs: {}", time_elapsed, date) {
        eprintln!("Couldn't write to file: {}", e);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let mut sout = stdout().into_raw_mode().unwrap();
    write!(
        sout,
        "{}{}Hit 's' to start tracking hours{}",
        termion::cursor::Goto(1, 1),
        termion::clear::All,
        termion::cursor::Hide
    ).unwrap();
    sout.flush().unwrap();

    // detect keydown events
    for k in stdin().keys() {
        write!(
            sout,
            "{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All
        ).unwrap();

        match k.unwrap() {
            Key::Char('s') => {
                break;
            },
            _ => (),
        }

        sout.flush().unwrap();
    }

    let now = Instant::now();
    let waiting: Vec<&str> = ["*..", ".*.", "..*"].to_vec();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut sout = stdout().into_raw_mode().unwrap();

        loop {
            let min: u64 = now.elapsed().as_secs() / 60;
            println!("{} min", min);
            thread::sleep(Duration::from_millis(300));
            write!(sout, "{}{}", termion::cursor::Goto(1, 1), termion::clear::All).unwrap();
            sout.flush().unwrap();

            for _ in 1..50 {
                for i in &waiting {
                    println!("{}", i);
                    thread::sleep(Duration::from_millis(100));
                    write!(sout, "{}{}", termion::cursor::Goto(1, 1), termion::clear::All).unwrap();
                    sout.flush().unwrap();
                }
            }

            match rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                Err(TryRecvError::Empty) => ()
            }
        }
    });

    let _ = stdin().lock().read_line();
    let _ = tx.send(());

    let total_hours: f64 = now.elapsed().as_secs_f64() / 3600.0;
    write_file(total_hours)?;

    writeln!(
        sout,
        "{}Total hours: {:.2}{}",
        termion::cursor::Goto(1, 1),
        total_hours,
        termion::cursor::Show
    ).unwrap();

    Ok(())
}
