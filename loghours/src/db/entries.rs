use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{
    named_params, ToSql,
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
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let date: String = self.0.format("%Y-%m-%d").to_string();
        Ok(ToSqlOutput::from(date))
    }
}

/// Query log entries by start and end dates
pub fn get_entries_by_date_range(
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> rusqlite::Result<Vec<Entry>> {
    let conn = create_conn().unwrap();

    let mut stmt = match (start_date, end_date) {
        (Some(sdate), Some(edate)) => {
            let mut s = conn.prepare(
                "SELECT id, date, hours FROM entry
                WHERE @sdate <= date AND @edate > date",
            )?;
            s.execute(named_params! {
                "@sdate": DbDate(sdate),
                "@edate": DbDate(edate),
            })?;
            s
        }
        (Some(sdate), None) => {
            let mut s = conn.prepare(
                "SELECT id, date, hours FROM entry
                WHERE @sdate <= date",
            )?;
            s.execute(named_params! { "@sdate": DbDate(sdate) })?;
            s
        }
        (None, Some(edate)) => {
            let mut s = conn.prepare(
                "SELECT id, date, hours FROM entry
                WHERE @edate > date",
            )?;
            s.execute(named_params! { "@edate": DbDate(edate) })?;
            s
        }
        (None, None) => {
            conn.prepare("SELECT id, date, hours FROM entry")?
        }
    };

    let rows = stmt.query_map([], |row| {
        Ok(Entry {
            id: row.get(0)?,
            date: row.get(1)?,
            hours: row.get(2)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<Entry>>>()
}

/// Add log entry to database
pub fn add_entry(
    date: NaiveDate,
    hours: f64,
) -> Result<u64> {
    let conn = create_conn()?;
    conn.execute(
        "INSERT INTO entry (date, hours)
            VALUES (@date, @hours)",
        named_params! {
            "@date": DbDate(date),
            "@hours": hours,
        },
    )?;

    Ok(conn.last_insert_rowid() as u64)
}

/// Remove log entries from database
pub fn remove_entries_by_date(
    date: NaiveDate,
) -> Result<()> {
    let conn = create_conn()?;

    // Register the update hook to confirm deletions
    conn.update_hook(Some(|action: Action, _: &str, _: &str, rowid: i64| {
        if action == Action::SQLITE_DELETE {
            println!("Deleted entry #{}", rowid);
        }
    }));

    // Delete entries of specified dates
    conn.execute(
        "DELETE FROM entry WHERE date = @date",
        named_params! { "@date": DbDate(date) },
    )?;

    Ok(())
}
