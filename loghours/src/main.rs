//! Hours Logger

use std::io::{self, Write};
use anyhow::Result;
use clap::{self, Parser, Subcommand};
use tokio::{sync::mpsc, task, time::Duration};

mod error;
mod state;
mod util;

use state::{LogCommand as Command, LogState};

#[derive(Parser)]
#[command(name = "Hours Logger")]
#[command(about = "Log hours worked or summarize timesheet", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Log hours worked to file
    Log {
        /// Path of file to which to write hours
        #[arg(short, long, value_name = "FILE")]
        outfile: String,
    },

    /// Read hours from file and print summary
    Read {
        /// Path of file from which to read hours
        #[arg(short, long, value_name = "FILE")]
        file: String,

        /// Start date in format 'YYYY-mm-dd'
        #[arg(short, long, value_name = "DATE")]
        start_date: Option<String>,

        /// End date in format 'YYYY-mm-dd'
        #[arg(short, long, value_name = "DATE")]
        end_date: Option<String>,

        /// Hourly pay rate
        #[arg(short, long)]
        rate: Option<u32>,
    }
}

#[tokio::main]
async fn main() -> Result<(), error::CustomError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Log { outfile } => {
            log_hours(&outfile).await?;
        }
        Commands::Read {
            file,
            start_date,
            end_date,
            rate,
        } => {
            read_hours(file, start_date, end_date, rate)?;
        }
    }

    Ok(())
}

/// Log hours to file and stdout
async fn log_hours(filename: &String) -> Result<(), error::CustomError> {
    use termion::{event::Key, input::TermRead, raw::IntoRawMode};

    // Capture stdout and get cursor's starting line number
    let mut stdout = io::stdout().into_raw_mode()?;
    let start_line: u16 = util::get_cursor_start_line(&mut stdout)?;

    writeln!(stdout, "[S] Start, [P] Pause, [R] Resume, [Space] Toggle Pause, [Q] Quit\n")?;
    util::hide_cursor(&mut stdout)?;

    let (tx, mut rx) = mpsc::channel::<Command>(100);

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

    input_handle.await.unwrap();
    util::clear_line(&mut stdout, start_line)?;
    util::show_cursor()?;

    let hours: f64 = state.get_total_hours();

    // If hours were accrued, log them to given file and stdout
    if hours >= 0.01 {
        util::write_file(&filename, hours)?;
        writeln!(stdout, "Hours logged: {:.2}", hours)?;
    } else {
        writeln!(stdout, "No hours logged")?;
    }

    util::clear_line(&mut stdout, start_line)?;

    Ok(())
}

/**
 * Read dates and hours from given file and sum hours both by date
 * and by month; each line of input file should contain two values
 * separated by a space: a date and a floating point number of hours
 */
fn read_hours(
    filename: String,
    start_date: Option<String>,
    end_date: Option<String>,
    rate: Option<u32>,
) -> Result<()> {
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use anyhow::Context;
    use chrono::NaiveDate;

    let fmt_str: &str = "%Y-%m-%d";
    let (sdate, edate) = util::parse_dates(start_date, end_date, fmt_str)?;
    let mut hours_by_day: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    let mut total_hours: f64 = 0.0;

    // Open file
    let file = File::open(&filename)
        .with_context(|| format!("Failed to open file {}", filename))?;

    // Read file and sum hours
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Some((date_str, hours_str)) = line.split_once(' ') {
            let hours: f64 = hours_str.parse::<f64>()?;
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

            if util::within_date_range(date, sdate, edate) {
                *hours_by_day.entry(date).or_insert(0.0f64) += hours;
                total_hours += hours;
            }
        }
    }

    // Print summary
    util::print_timeframe(sdate, edate);
    if !hours_by_day.is_empty() {
        println!("Daily hours worked:\n{:#?}", hours_by_day);
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
