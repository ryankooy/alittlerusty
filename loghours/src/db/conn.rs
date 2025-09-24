use anyhow::Result;
use rusqlite::Connection;

use crate::db::schema::create_schema;

pub fn create_conn() -> Result<Connection> {
    // TODO: use config toml for path
    let mut conn = Connection::open("/home/ranky/sqlite/hours.db")?;
    configure_conn(&mut conn)?;
    create_schema(&mut conn)?;
    Ok(conn)
}

fn configure_conn(conn: &mut Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = TRUE;
        ",
    )?;

    Ok(())
}
