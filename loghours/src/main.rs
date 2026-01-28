//! Hours Logger

use std::collections::{BTreeMap, HashMap};
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

    /// Nickname for job/company for which hours worked
    ///
    /// Required for `log` and `add` commands
    #[arg(short = 'n', long, value_name = "NICKNAME")]
    job_name: Option<String>,

    /// Nickname for type of work
    ///
    /// Ignored if writing hours to file
    #[arg(short = 't', long, value_name = "WORKTYPE")]
    work_type: Option<String>,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    /// Log hours worked to file
    Log {
        /// Optional
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

        /// Whether to round a day's hours to nearest quarter-hour
        #[arg(short = 'q', long)]
        round_quarter: bool,

        /// Show raw database log entries (including `id` column)
        #[arg(short = 'i', long)]
        show_raw_entries: bool,
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

    /// Delete log entry from database
    Remove {
        /// Id of entry to delete
        #[arg(short = 'i', long)]
        entry_id: i64,
    },

    /// Import log entries from file to database
    Import {
        /// File from which to import hours
        #[arg(short, long)]
        file: String,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Log { outfile } => {
            if !(outfile.is_none() && cli.job_name.is_none()) {
                log_hours(outfile, cli.job_name, cli.work_type).await?;
            } else {
                bail!("Job name required when logging hours to database");
            }
        }
        Commands::Read {
            file,
            start_date,
            end_date,
            rate,
            round_quarter,
            show_raw_entries,
        } => {
            read_hours(
                file, start_date, end_date, cli.job_name,
                rate, round_quarter, show_raw_entries,
            )?;
        }
        Commands::Add { date, hours } => {
            if let Some(job_name) = cli.job_name {
                let d = NaiveDate::parse_from_str(date.as_str(), DATE_FMT_STR)?;
                db::add_entry(None, d, hours, job_name, cli.work_type)?;
            } else {
                bail!("Job name required for `add` operation");
            }
        }
        Commands::Remove { entry_id } => db::remove_entry_by_id(entry_id)?,
        Commands::Import { file } => import_hours(file, cli.job_name)?,
    }

    Ok(())
}

/// Log hours to file and stdout.
async fn log_hours(
    filename: Option<String>,
    job_name: Option<String>,
    work_type: Option<String>,
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
            db::add_entry(None, today, hours, job, work_type)?;
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
    round_quarter: bool,
    show_raw_entries: bool,
) -> Result<()> {
    // Hours map format: (idx, job, work_type, date) => hours
    let mut hours_map: BTreeMap<(i32, String, String, NaiveDate), f64> = BTreeMap::new();

    let by_job: bool = job_name.is_some();
    let job: String = job_name.clone().unwrap_or(String::from("-"));
    let mut raw_entries = Vec::new();

    let (sdate, edate) = util::parse_dates(start_date, end_date, DATE_FMT_STR)?;
    util::print_timeframe(sdate, edate);

    if let Some(f) = filename {
        // Read file and sum hours
        for line in util::read_lines(&f)?.map_while(Result::ok) {
            if let Some(entry) = util::entry_from_line(line, DATE_FMT_STR)? {
                if !(by_job && entry.job != job) &&
                    util::within_date_range(entry.date, sdate, edate)
                {
                    // For each entry key's index and work type, use the dummy
                    // values 1 and "-", respectively, since entries in files
                    // don't have either column
                    *hours_map.entry((1, entry.job, String::from("-"), entry.date))
                        .or_insert(0.0f64) += entry.hours;
                }
            }
        }
    } else {
        // Index map format: (job, work_type, date) => idx
        let mut index_map: HashMap<(String, String, NaiveDate), i32> = HashMap::new();

        // Read hours from database
        for entry in db::get_entries_by_date_range(sdate, edate, job_name)?
            .iter()
        {
            if show_raw_entries {
                // Add raw entry
                raw_entries.push((
                    entry.id,
                    entry.job.clone(),
                    entry.work_type.clone().unwrap_or(String::from("-")),
                    entry.date.date_naive(),
                    entry.hours,
                ));
            } else {
                let job_key: String = entry.job.clone();
                let work_type_key: String = entry.work_type
                    .clone()
                    .unwrap_or(String::from("-"));
                let date_key: NaiveDate = entry.date.date_naive();

                // Get index for entry sort order
                let idx: i32 = *index_map.entry((
                    job_key.to_owned(),
                    work_type_key.to_owned(),
                    date_key,
                ))
                .or_insert(entry.idx);

                // Update hours map
                *hours_map.entry((idx, job_key, work_type_key, date_key))
                    .or_insert(0.0f64) += entry.hours;
            }
        }
    }

    if show_raw_entries {
        println!("ID\tJOB\tTYPE\tDATE\t\tHOURS");

        // Print raw entries
        for (i, j, t, d, h) in raw_entries.iter() {
            println!(
                "{id}\t{job}\t{typ}\t{date}\t{hours}",
                id=i, job=j, typ=t, date=d, hours=h,
            );
        }
    } else {
        let mut total_hours: f64 = 0.0;

        if !hours_map.is_empty() {
            println!("JOB\tTYPE\tDATE\t\tHOURS");

            // Print entries
            for ((_, j, t, d), h) in hours_map.iter() {
                let daily_hours: f64 = round_hours(h, round_quarter);

                if daily_hours > 0.0 {
                    total_hours += daily_hours;
                    println!(
                        "{job}\t{typ}\t{date}\t{hours:.2}",
                        job=j, typ=t, date=d, hours=daily_hours,
                    );
                }
            }
            println!();

            // Print total hours worked
            if total_hours > 0.0 {
                println!("Total hours worked: {:.2}", total_hours);

                if let Some(hourly_rate) = rate {
                    let pay: f64 = (hourly_rate as f64) * total_hours;
                    println!("Gross wage: ${:.2}", pay);
                }
            }
        }

        if total_hours == 0.0 {
            println!("No hours worked");
        }
    }

    Ok(())
}

/// Import entries from file into database.
fn import_hours(filename: String, job_name: Option<String>) -> Result<()> {
    let mut conn = db::create_conn()?;
    let by_job: bool = job_name.is_some();
    let job: String = job_name.clone().unwrap_or(String::from("-"));

    for line in util::read_lines(&filename)?.map_while(Result::ok) {
        if let Some(entry) = util::entry_from_line(line, DATE_FMT_STR)? {
            if !(by_job && entry.job != job) {
                println!(
                    "Importing entry ({} {} {})",
                    entry.job, entry.date, entry.hours,
                );

                db::add_entry(
                    Some(&mut conn),
                    entry.date,
                    entry.hours,
                    entry.job,
                    None,
                )?;
            }
        }
    }

    Ok(())
}

/// Round hours by nearest hundredth or quarter depending
/// on truthiness of `round_quarter`.
fn round_hours(hours: &f64, by_quarter: bool) -> f64 {
    let val: f64 = if by_quarter { 4.0 } else { 100.0 };
    (hours * val).round() / val
}
