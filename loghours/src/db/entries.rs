use chrono::NaiveDate;
use rusqlite::{
    named_params, Connection, Result, Row, ToSql,
    hooks::Action,
    types::{
        FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef
    },
};

use crate::db::conn::create_conn;

/// This SELECT statement includes the row number as column
/// `idx` to ensure that entries remain sorted by date.
const SELECT_STATEMENT: &'static str = r#"
SELECT ROW_NUMBER() OVER(ORDER BY date) AS idx,
    id, job, work_type, date, hours
FROM entry
"#;

const SDATE_CLAUSE: &'static str = "date >= @sdate";
const EDATE_CLAUSE: &'static str = "date < @edate";
const JOB_CLAUSE: &'static str = "job LIKE @job";
const ORDER_BY: &'static str = "ORDER BY date";

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
pub struct DbEntry {
    pub idx: i32,
    pub id: i32,
    pub job: String,
    pub work_type: Option<String>,
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
) -> Result<Vec<DbEntry>> {
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
) -> Result<Vec<DbEntry>> {
    if let Some(job) = job_name {
        let query = format!(
            "{} WHERE {} AND {} AND {} {}",
            SELECT_STATEMENT,
            SDATE_CLAUSE,
            EDATE_CLAUSE,
            JOB_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! {
                    "@sdate": DbDate(sdate),
                    "@edate": DbDate(edate),
                    "@job": job,
                },
                |row| make_entry(row),
            )?
            .collect::<Result<Vec<DbEntry>>>()
    } else {
        let query = format!(
            "{} WHERE {} AND {} {}",
            SELECT_STATEMENT,
            SDATE_CLAUSE,
            EDATE_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! {
                    "@sdate": DbDate(sdate),
                    "@edate": DbDate(edate),
                },
                |row| make_entry(row),
            )?
            .collect::<Result<Vec<DbEntry>>>()
    }
}

fn get_entries_by_sdate(
    conn: &mut Connection,
    sdate: NaiveDate,
    job_name: Option<String>,
) -> Result<Vec<DbEntry>> {
    if let Some(job) = job_name {
        let query = format!(
            "{} WHERE {} AND {} {}",
            SELECT_STATEMENT,
            SDATE_CLAUSE,
            JOB_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! { "@sdate": DbDate(sdate), "@job": job },
                |row| make_entry(row)
            )?
            .collect::<Result<Vec<DbEntry>>>()
    } else {
        let query = format!(
            "{} WHERE {} {}",
            SELECT_STATEMENT,
            SDATE_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! { "@sdate": DbDate(sdate) },
                |row| make_entry(row)
            )?
            .collect::<Result<Vec<DbEntry>>>()
    }
}

fn get_entries_by_edate(
    conn: &mut Connection,
    edate: NaiveDate,
    job_name: Option<String>,
) -> Result<Vec<DbEntry>> {
    if let Some(job) = job_name {
        let query = format!(
            "{} WHERE {} AND {} {}",
            SELECT_STATEMENT,
            EDATE_CLAUSE,
            JOB_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! { "@edate": DbDate(edate), "@job": job },
                |row| make_entry(row)
            )?
            .collect::<Result<Vec<DbEntry>>>()
    } else {
        let query = format!(
            "{} WHERE {} {}",
            SELECT_STATEMENT,
            EDATE_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(
                named_params! { "@edate": DbDate(edate) },
                |row| make_entry(row)
            )?
            .collect::<Result<Vec<DbEntry>>>()
    }
}

fn get_all_entries(
    conn: &mut Connection,
    job_name: Option<String>,
) -> Result<Vec<DbEntry>> {
    if let Some(job) = job_name {
        let query = format!(
            "{} WHERE {} {}",
            SELECT_STATEMENT,
            JOB_CLAUSE,
            ORDER_BY,
        );

        conn.prepare(&query)?
            .query_map(named_params! { "@job": job }, |row| make_entry(row))?
            .collect::<Result<Vec<DbEntry>>>()
    } else {
        let query = format!("{} {}", SELECT_STATEMENT, ORDER_BY);

        conn.prepare(&query)?
            .query_map([], |row| make_entry(row))?
            .collect::<Result<Vec<DbEntry>>>()
    }
}

fn make_entry(row: &Row) -> Result<DbEntry> {
    Ok(DbEntry {
        idx: row.get(0)?,
        id: row.get(1)?,
        job: row.get(2)?,
        work_type: row.get(3)?,
        date: row.get(4)?,
        hours: row.get(5)?,
    })
}

/// Add log entry to database
pub fn add_entry(
    connection: Option<&mut Connection>,
    date: NaiveDate,
    hours: f64,
    job: String,
    work_type: Option<String>,
) -> anyhow::Result<()> {
    let conn = match connection {
        None => &mut create_conn()?,
        Some(c) => c,
    };

    conn.execute(
        "INSERT OR IGNORE INTO entry (job, work_type, date, hours)
            VALUES (@job, @work_type, @date, @hours)",
        named_params! {
            "@job": job,
            "@work_type": work_type,
            "@date": DbDate(date),
            "@hours": hours,
        },
    )?;

    let rowid = conn.last_insert_rowid();
    if rowid != 0 {
        println!("Added entry #{}", rowid);
    } else {
        println!("Duplicate entry");
    }

    Ok(())
}

/// Remove log entries from database
pub fn remove_entry_by_id(id: i64) -> anyhow::Result<()> {
    let conn = create_conn()?;

    // Register the update hook to confirm deletions
    conn.update_hook(Some(|action: Action, _: &str, _: &str, rowid: i64| {
        if action == Action::SQLITE_DELETE {
            println!("Deleted entry #{}", rowid);
        }
    }));

    // Delete entries of specified job names + dates
    conn.execute(
        "DELETE FROM entry WHERE id = @id",
        named_params! { "@id": id },
    )?;

    Ok(())
}
