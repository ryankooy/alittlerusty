use anyhow::Result;
use rusqlite::Connection;

pub fn create_conn() -> Result<Connection> {
    let mut conn = Connection::open("../../hours.db")?;
    configure_conn(&mut conn)?;
    crate::db::create_schema(&mut conn)?;
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

    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    Ok(())
}
