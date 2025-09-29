//! Hours Logger

use std::io::{self, Write};
use anyhow::{bail, Result};
use chrono::{Local, NaiveDate};
use clap::{self, Parser, Subcommand};
use tokio::{sync::mpsc, task, time::Duration};

mod db;
mod state;
mod util;

use state::{LogCommand as Command, LogState};
use util::TerminalRestorer;

const DATE_FMT_STR: &str = "%Y-%m-%d";

#[derive(Parser)]
#[command(name = "Hours Logger")]
#[command(about = "Log hours worked or summarize timesheet", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Nickname of job/company for which hours worked
    ///
    /// Note that this field is required for `add`
    /// and `remove` commands
    #[arg(short = 'n', long, value_name = "NICKNAME")]
    job_name: Option<String>,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Log hours worked to file
    Log {
        /// Optional path of file to which to write hours
        #[arg(short, long, value_name = "FILE")]
        outfile: Option<String>,
    },

    /// Read hours from file and print summary
    Read {
        /// Optional path of file from which to read hours
        #[arg(short, long, value_name = "FILE")]
        file: Option<String>,

        /// Start date in format 'YYYY-mm-dd'
        #[arg(short, long, value_name = "DATE")]
        start_date: Option<String>,

        /// End date in format 'YYYY-mm-dd'
        #[arg(short, long, value_name = "DATE")]
        end_date: Option<String>,

        /// Hourly pay rate
        #[arg(short, long)]
        rate: Option<u32>,
    },

    /// Add log entry to database
    Add {
        /// Date hours logged ('YYYY-mm-dd')
        #[arg(short, long)]
        date: String,

        /// Hours logged for given date (e.g., '3.25')
        #[arg(short = 't', long)]
        hours: f64,
    },

    /// Delete log entries from database
    Remove {
        /// Date of entries to delete ('YYYY-mm-dd')
        #[arg(short, long)]
        date: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Log { outfile } => {
            if !(outfile.is_none() && cli.job_name.is_none()) {
                log_hours(outfile, cli.job_name).await?;
            } else {
                bail!("Job name required when logging hours to database");
            }
        }
        Commands::Read {
            file,
            start_date,
            end_date,
            rate,
        } => {
            read_hours(file, start_date, end_date, cli.job_name, rate)?;
        }
        Commands::Add { date, hours } => {
            if let Some(job_name) = cli.job_name {
                let d = NaiveDate::parse_from_str(date.as_str(), DATE_FMT_STR)?;
                db::add_entry(d, hours, job_name)?;
            } else {
                bail!("Job name required for `add` operation");
            }
        }
        Commands::Remove { date } => {
            if let Some(job_name) = cli.job_name {
                let d = NaiveDate::parse_from_str(date.as_str(), DATE_FMT_STR)?;
                db::remove_entries_by_date(d, job_name)?;
            } else {
                bail!("Job name required for `remove` operation");
            }
        }
    }

    Ok(())
}

/// Log hours to file and stdout.
async fn log_hours(
    filename: Option<String>,
    job_name: Option<String>,
) -> Result<()> {
    use termion::{event::Key, input::TermRead, raw::IntoRawMode};

    // Capture stdout and get cursor's starting line number
    let stdout = io::stdout().into_raw_mode()?;
    let mut stdout = TerminalRestorer(stdout);
    let start_line: u16 = util::get_cursor_start_line(&mut stdout.0)?;

    writeln!(stdout.0, "[S] Start, [P] Pause, [R] Resume, [Space] Toggle Pause, [Q] Quit\n")?;
    util::hide_cursor(&mut stdout.0)?;

    let (tx, mut rx) = mpsc::channel::<Command>(100);

    // Key handler
    task::spawn_blocking(move || {
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
                    util::clear_line(&mut stdout.0, start_line)?;

                    if state.is_paused() {
                        writeln!(
                            stdout.0,
                            "Paused at {:.2} hours",
                            state.get_total_hours()
                        )?;
                    } else {
                        if counter == u64::MAX {
                            counter = 0;
                        }
                        counter += 1;

                        writeln!(
                            stdout.0,
                            "{} min {}",
                            state.get_total_minutes(),
                            "★".repeat((counter % 20) as usize + 1)
                        )?;

                        util::clear_line(&mut stdout.0, start_line + 1)?;
                    }
                }
            }
        }
    }

    util::clear_line(&mut stdout.0, start_line - 1)?;
    util::show_cursor(&mut stdout.0)?;

    let hours: f64 = state.get_total_hours();

    // If hours were accrued, log them to given file and stdout
    if hours >= 0.01 {
        writeln!(stdout.0, "Hours logged: {:.2}", hours)?;

        if let Some(f) = filename {
            // Log hours to file
            util::write_file(&f, hours, job_name, DATE_FMT_STR)?;
        } else if let Some(job) = job_name {
            // Log hours to database
            let today = Local::now().date_naive();
            db::add_entry(today, hours, job)?;
        }
    } else {
        writeln!(stdout.0, "No hours logged")?;
    }

    util::clear_line(&mut stdout.0, start_line)?;

    Ok(())
}

/**
 * Read dates and hours from given file and sum hours both by date
 * and by month. Each line of input file should contain two values
 * separated by a space: a date and a floating point number of hours.
 */
fn read_hours(
    filename: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    job_name: Option<String>,
    rate: Option<u32>,
) -> Result<()> {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use anyhow::Context;

    let mut hours_map: BTreeMap<(String, NaiveDate), f64> = BTreeMap::new();
    let mut total_hours: f64 = 0.0;
    let by_job: bool = job_name.is_some();
    let job: String = job_name.clone().unwrap_or(String::new());

    let (sdate, edate) = util::parse_dates(start_date, end_date, DATE_FMT_STR)?;
    util::print_timeframe(sdate, edate);

    if let Some(f) = filename {
        // Open file
        let file = File::open(&f)
            .with_context(|| format!("Failed to open file {}", f))?;

        // Read file and sum hours
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let job_str = parts.next().unwrap();

            if !(by_job && job_str != job) {
                let date = NaiveDate::parse_from_str(
                    parts.next().unwrap(), DATE_FMT_STR
                )?;
                let hours: f64 = parts.next().unwrap().parse::<f64>()?;

                if util::within_date_range(date, sdate, edate) {
                    *hours_map.entry((job_str.to_string(), date))
                        .or_insert(0.0f64) += hours;
                    total_hours += hours;
                }
            }
        }
    } else {
        // Read hours from database
        let entries = db::get_entries_by_date_range(sdate, edate, job_name)?;

        for entry in entries.iter() {
            *hours_map.entry((entry.job.clone(), entry.date.date_naive()))
                .or_insert(0.0f64) += entry.hours;
            total_hours += entry.hours;
        }
    }

    // Print summary
    if !hours_map.is_empty() {
        println!("JOB\t\tDATE\t\tHOURS");
        for ((j, d), h) in hours_map.iter() {
            println!("{}\t\t{}\t{:.2}", j, d, h);
        }
        println!();

        println!("Total hours worked: {:.2}", total_hours);

        if let Some(hourly_rate) = rate {
            let pay: f64 = (hourly_rate as f64) * total_hours;
            println!("Gross wage: ${:.2}", pay);
        }
    } else {
        println!("No hours worked");
    }

    Ok(())
}
