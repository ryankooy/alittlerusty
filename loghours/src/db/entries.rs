use chrono::NaiveDate;
use rusqlite::{
    named_params, Connection, Result, Row, ToSql,
    hooks::Action,
    types::{
        FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef
    },
};

use crate::db::conn::create_conn;

#[derive(Clone, Debug)]
pub struct DbDate(pub NaiveDate);

impl DbDate {
    pub fn date_naive(&self) -> NaiveDate {
        let DbDate(naive_date) = *self;
        naive_date
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Entry {
    pub id: i32,
    pub job: String,
    pub date: DbDate,
    pub hours: f64,
}

impl FromSql for DbDate {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).and_then(|as_string| {
            NaiveDate::parse_from_str(as_string.as_str(), "%Y-%m-%d")
                .map(|d| DbDate(d))
                .map_err(FromSqlError::other)
        })
    }
}

impl ToSql for DbDate {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let date: String = self.0.format("%Y-%m-%d").to_string();
        Ok(ToSqlOutput::from(date))
    }
}

/// Query log entries by start and end dates
pub fn get_entries_by_date_range(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    job_name: Option<String>,
) -> Result<Vec<Entry>> {
    let mut conn = create_conn().unwrap();

    let rows = match (start_date, end_date) {
        (Some(sdate), Some(edate)) => {
            get_entries_by_sdate_and_edate(&mut conn, sdate, edate, job_name)?
        }
        (Some(sdate), None) => get_entries_by_sdate(&mut conn, sdate, job_name)?,
        (None, Some(edate)) => get_entries_by_edate(&mut conn, edate, job_name)?,
        (None, None) => get_all_entries(&mut conn, job_name)?,
    };

    Ok(rows)
}

fn get_entries_by_sdate_and_edate(
    conn: &mut Connection,
    sdate: NaiveDate,
    edate: NaiveDate,
    job_name: Option<String>,
) -> Result<Vec<Entry>> {
    if let Some(job) = job_name {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE date >= @sdate AND date < @edate
                AND job LIKE @job
            ORDER BY date",
        )?
        .query_map(
            named_params! {
                "@sdate": DbDate(sdate),
                "@edate": DbDate(edate),
                "@job": job,
            },
            |row| make_entry(row),
        )?
        .collect::<Result<Vec<Entry>>>()
    } else {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE date >= @sdate AND date < @edate
            ORDER BY date",
        )?
        .query_map(
            named_params! {
                "@sdate": DbDate(sdate),
                "@edate": DbDate(edate),
            },
            |row| make_entry(row),
        )?
        .collect::<Result<Vec<Entry>>>()
    }
}

fn get_entries_by_sdate(
    conn: &mut Connection,
    sdate: NaiveDate,
    job_name: Option<String>,
) -> Result<Vec<Entry>> {
    if let Some(job) = job_name {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
             WHERE date >= @sdate AND job LIKE @job
             ORDER BY date",
         )?
        .query_map(
            named_params! { "@sdate": DbDate(sdate), "@job": job },
            |row| make_entry(row)
        )?
        .collect::<Result<Vec<Entry>>>()
    } else {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE date >= @sdate ORDER BY date",
        )?
        .query_map(
            named_params! { "@sdate": DbDate(sdate) },
            |row| make_entry(row)
        )?
        .collect::<Result<Vec<Entry>>>()
    }
}

fn get_entries_by_edate(
    conn: &mut Connection,
    edate: NaiveDate,
    job_name: Option<String>,
) -> Result<Vec<Entry>> {
    if let Some(job) = job_name {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE date < @edate AND job LIKE @job
            ORDER BY date",
        )?
        .query_map(
            named_params! { "@edate": DbDate(edate), "@job": job },
            |row| make_entry(row)
        )?
        .collect::<Result<Vec<Entry>>>()
    } else {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE date < @edate ORDER BY date",
        )?
        .query_map(
            named_params! { "@edate": DbDate(edate) },
            |row| make_entry(row)
        )?
        .collect::<Result<Vec<Entry>>>()
    }
}

fn get_all_entries(
    conn: &mut Connection,
    job_name: Option<String>,
) -> Result<Vec<Entry>> {
    if let Some(job) = job_name {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry
            WHERE job LIKE @job ORDER BY date",
        )?
        .query_map(named_params! { "@job": job }, |row| make_entry(row))?
        .collect::<Result<Vec<Entry>>>()
    } else {
        conn.prepare(
            "SELECT id, job, date, hours FROM entry ORDER BY date",
        )?
        .query_map([], |row| make_entry(row))?
        .collect::<Result<Vec<Entry>>>()
    }
}

fn make_entry(row: &Row) -> Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        job: row.get(1)?,
        date: row.get(2)?,
        hours: row.get(3)?,
    })
}

/// Add log entry to database
pub fn add_entry(
    date: NaiveDate,
    hours: f64,
    job: String,
) -> anyhow::Result<()> {
    let conn = create_conn()?;

    conn.execute(
        "INSERT INTO entry (job, date, hours)
            VALUES (@job, @date, @hours)",
        named_params! {
            "@job": job,
            "@date": DbDate(date),
            "@hours": hours,
        },
    )?;

    println!("Added entry #{}", conn.last_insert_rowid());

    Ok(())
}

/// Remove log entries from database
pub fn remove_entries_by_date(
    date: NaiveDate,
    job: String,
) -> anyhow::Result<()> {
    let conn = create_conn()?;

    // Register the update hook to confirm deletions
    conn.update_hook(Some(|action: Action, _: &str, _: &str, rowid: i64| {
        if action == Action::SQLITE_DELETE {
            println!("Deleted entry #{}", rowid);
        }
    }));

    // Delete entries of specified job names + dates
    conn.execute(
        "DELETE FROM entry WHERE date = @date AND job = @job",
        named_params! { "@date": DbDate(date), "@job": job },
    )?;

    Ok(())
}
