use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::db::{
    config::get_config,
    schema::create_schema,
};

pub fn create_conn() -> Result<Connection> {
    let config = get_config()?;

    if let Some(db) = config.get_path() {
        let mut conn = Connection::open(db)?;
        configure_conn(&mut conn)?;
        create_schema(&mut conn)?;

        Ok(conn)
    } else {
        bail!("Bad config");
    }
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
