//! Utility functions for Hours Logger

use std::fs::{File, OpenOptions};
use std::io::{self, Stdout, Write};
use std::path::Path;
use anyhow::{Context, Result};
use chrono::Local;
use termion::clear;
use termion::cursor::{self, DetectCursorPos};
use termion::raw::{IntoRawMode, RawTerminal};

pub fn get_cursor_start_line(stdout: &mut RawTerminal<Stdout>) -> Result<u16> {
    let y_pos: u16 = stdout.cursor_pos()?.1;
    Ok(y_pos - 1)
}

pub fn clear_line(stdout: &mut RawTerminal<Stdout>, line: u16) -> Result<()> {
    write!(stdout, "{}{}", cursor::Goto(1, line), clear::CurrentLine)?;
    stdout.flush()?;
    Ok(())
}

pub fn hide_cursor(stdout: &mut RawTerminal<Stdout>) -> Result<()> {
    write!(stdout, "{}", cursor::Hide)?;
    stdout.flush()?;
    Ok(())
}

pub fn show_cursor() -> Result<()> {
    let mut stdout = io::stdout().into_raw_mode()?;
    let start_line: u16 = get_cursor_start_line(&mut stdout)?;

    write!(stdout, "{}", cursor::Show)?;
    clear_line(&mut stdout, start_line)?;

    Ok(())
}

pub fn write_file(filename: &Option<String>, hours: f64) -> Result<()> {
    if let Some(f) = filename {
        if !Path::new(&f).exists() {
            let _ = File::create(f)
                .with_context(|| format!("Failed to create file {}", f))?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(f)
            .with_context(|| format!("Failed to open file {}", f))?;

        let date = Local::now().format("%Y-%m-%d").to_string();
        writeln!(file, "{} {:.2}", date, hours)
            .with_context(|| format!("Failed to write to file {}", f))?;
    }

    Ok(())
}
