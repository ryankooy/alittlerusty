use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{
    named_params, Connection, ToSql,
    types::{
        FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef
    },
};

#[derive(Clone, Debug)]
pub struct DbDate(pub NaiveDate);

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

pub fn get_entries_by_date_range(
    conn: &mut Connection,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> rusqlite::Result<Vec<Entry>> {
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

#[allow(dead_code)]
pub fn add_entry(conn: &mut Connection, date: NaiveDate, hours: f64) -> Result<u64> {
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

#[allow(dead_code)]
pub fn create_schema(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;

        CREATE TABLE IF NOT EXISTS entry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            hours REAL NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_entry_date
            ON entry (date);

        COMMIT;",
    )?;

    Ok(())
}
