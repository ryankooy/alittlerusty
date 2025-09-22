//! Utility functions for Hours Logger

use std::fs::{File, OpenOptions};
use std::io::{self, Stdout, Write};
use std::path::Path;
use anyhow::{Context, Result};
use chrono::{Datelike, Local, NaiveDate};
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

pub fn write_file(filename: &String, hours: f64, fmt_str: &str) -> Result<()> {
    if !Path::new(filename).exists() {
        let _ = File::create(filename)
            .with_context(|| format!("Failed to create file {}", filename))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(filename)
        .with_context(|| format!("Failed to open file {}", filename))?;

    let date = Local::now().format(fmt_str).to_string();
    writeln!(file, "{} {:.2}", date, hours)
        .with_context(|| format!("Failed to write to file {}", filename))?;

    Ok(())
}

pub fn parse_dates(
    start_date: Option<String>,
    end_date: Option<String>,
    fmt_str: &str,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>)> {
    let sdate: Option<NaiveDate> = if let Some(d) = start_date {
        Some(NaiveDate::parse_from_str(d.as_str(), fmt_str)
             .context("Failed to parse start date")?)
    } else { None };

    let edate: Option<NaiveDate> = if let Some(d) = end_date {
        Some(NaiveDate::parse_from_str(d.as_str(), fmt_str)
             .context("Failed to parse end date")?)
    } else { None };

    Ok((sdate, edate))
}

pub fn within_date_range(
    date: NaiveDate,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> bool {
    let sdate_before: bool = start_date.is_none() || start_date.unwrap() <= date;
    let edate_after: bool = end_date.is_none() || end_date.unwrap() > date;
    sdate_before && edate_after
}

/// Print timeframe in a format like "September 15 - October 15, 2025".
pub fn print_timeframe(start_date: Option<NaiveDate>, end_date: Option<NaiveDate>) {
    let full_date_fmt: &str = "%B %-d, %C%y";
    let month_day_fmt: &str = "%B %-d";
    let day_year_fmt: &str = "%-d, %C%y";

    match (start_date, end_date) {
        (Some(sdate), Some(edate)) => {
            let (sdate_str, edate_str) = if sdate.year() == edate.year() {
                if sdate.month() == edate.month() {
                    (sdate.format(month_day_fmt), edate.format(day_year_fmt))
                } else {
                    (sdate.format(month_day_fmt), edate.format(full_date_fmt))
                }
            } else {
                (sdate.format(full_date_fmt), edate.format(full_date_fmt))
            };

            println!("{} - {}", sdate_str, edate_str);
        }
        (Some(sdate), None) => println!("Since {}", sdate.format(full_date_fmt)),
        (None, Some(edate)) => println!("Before {}", edate.format(full_date_fmt)),
        (None, None) => println!("All time"),
    }
}
