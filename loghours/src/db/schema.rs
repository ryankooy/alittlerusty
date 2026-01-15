use anyhow::Result;
use rusqlite::Connection;

pub fn create_schema(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;

        CREATE TABLE IF NOT EXISTS entry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            job TEXT NOT NULL,
            work_type TEXT,
            date TEXT NOT NULL,
            hours REAL NOT NULL,
            CONSTRAINT unique_job_date_hours UNIQUE (job, date, hours)
        );

        CREATE INDEX IF NOT EXISTS idx_entry_job
            ON entry (job);

        COMMIT;",
    )?;

    Ok(())
}
