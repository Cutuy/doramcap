//! Synchronous SQLite access for the TypeScript store; SQL and schema live in TypeScript.
use crate::failure;
use napi::{
    Result,
    bindgen_prelude::{Buffer, Either3},
};
use napi_derive::napi;
use rusqlite::{
    Connection, OpenFlags, ToSql, params_from_iter,
    types::{ToSqlOutput, Value, ValueRef},
};
use std::collections::HashMap;

type Cell = Either3<f64, String, Buffer>;
struct Param(Option<Cell>);
impl ToSql for Param {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match &self.0 {
            None => ToSqlOutput::Owned(Value::Null),
            Some(Either3::A(number)) if number.fract() == 0.0 => {
                ToSqlOutput::Owned(Value::Integer(*number as i64))
            }
            Some(Either3::A(number)) => ToSqlOutput::Owned(Value::Real(*number)),
            Some(Either3::B(text)) => ToSqlOutput::Borrowed(ValueRef::Text(text.as_bytes())),
            Some(Either3::C(bytes)) => ToSqlOutput::Borrowed(ValueRef::Blob(bytes)),
        })
    }
}

/// One SQLite connection, used only from the JavaScript thread. Closed connections reject calls.
#[napi]
pub struct Database {
    connection: Option<Connection>,
}

#[napi]
impl Database {
    fn connection(&self) -> Result<&Connection> {
        self.connection
            .as_ref()
            .ok_or_else(|| failure("database is closed"))
    }

    /// Run one or more statements without parameters or results.
    #[napi]
    pub fn exec(&self, sql: String) -> Result<()> {
        self.connection()?.execute_batch(&sql).map_err(failure)
    }

    /// Run one statement and return its rows as objects keyed by column name.
    /// Integers beyond 2^53 lose precision; this store keeps those inside protobuf blobs.
    #[napi]
    pub fn query(
        &self,
        sql: String,
        params: Vec<Option<Cell>>,
    ) -> Result<Vec<HashMap<String, Option<Cell>>>> {
        let mut statement = self.connection()?.prepare(&sql).map_err(failure)?;
        let columns: Vec<String> = statement.column_names().map(str::to_owned).collect();
        let mut rows = statement
            .query(params_from_iter(params.into_iter().map(Param)))
            .map_err(failure)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(failure)? {
            let mut object = HashMap::with_capacity(columns.len());
            for (index, column) in columns.iter().enumerate() {
                let cell = match row.get_ref(index).map_err(failure)? {
                    ValueRef::Null => None,
                    ValueRef::Integer(number) => Some(Either3::A(number as f64)),
                    ValueRef::Real(number) => Some(Either3::A(number)),
                    ValueRef::Text(text) => {
                        Some(Either3::B(String::from_utf8_lossy(text).into_owned()))
                    }
                    ValueRef::Blob(bytes) => Some(Either3::C(bytes.to_vec().into())),
                };
                object.insert(column.clone(), cell);
            }
            result.push(object);
        }
        Ok(result)
    }

    /// Close the connection; later calls fail.
    #[napi]
    pub fn close(&mut self) {
        self.connection = None;
    }
}

/// Open or create a database file, refusing a symlink at `path`.
#[napi]
pub fn open_database(path: String) -> Result<Database> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NOFOLLOW
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Ok(Database {
        connection: Some(Connection::open_with_flags(path, flags).map_err(failure)?),
    })
}
