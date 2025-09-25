use anyhow::Result;
use rusqlite::Connection;

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
