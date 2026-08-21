use connectorx::prelude::{CXQuery, SourceConn, SourceType, get_arrow};
use parquet::arrow::ArrowWriter;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};
use ts_rs::TS;
use uuid::Uuid;

pub const CONNECTION_STORE_NAME: &str = "database-connections.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DatabaseConnection {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub uri: String,
}

pub fn load(path: &Path) -> Result<Vec<DatabaseConnection>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut connections: Vec<DatabaseConnection> =
        serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    connections.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(connections)
}

pub fn save(path: &Path, mut connection: DatabaseConnection) -> Result<DatabaseConnection, String> {
    connection.name = connection.name.trim().to_string();
    connection.uri = connection.uri.trim().to_string();
    if connection.name.is_empty() || connection.uri.is_empty() {
        return Err("A database connection needs a name and URI".into());
    }
    let source = SourceConn::try_from(connection.uri.as_str())
        .map_err(|error| format!("That database URI is not supported: {error}"))?;
    if !matches!(
        source.ty,
        SourceType::Postgres | SourceType::MySQL | SourceType::SQLite | SourceType::MsSQL
    ) {
        return Err("Use a PostgreSQL, MySQL/MariaDB, SQLite, or SQL Server URI".into());
    }
    if connection.id.trim().is_empty() {
        connection.id = id_for_name(&connection.name);
    }
    if connection.id.is_empty() {
        return Err("A connection name needs at least one letter or number".into());
    }

    let mut connections = load(path)?;
    if let Some(existing) = connections
        .iter_mut()
        .find(|existing| existing.id == connection.id)
    {
        *existing = connection.clone();
    } else {
        connections.push(connection.clone());
    }
    connections.sort_by(|left, right| left.name.cmp(&right.name));
    let parent = path
        .parent()
        .ok_or_else(|| "Database connection file has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".database-connections-{}.tmp", Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&connections).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    replace_store(&temporary, path)?;
    Ok(connection)
}

pub fn by_id(path: &Path, id: &str) -> Result<DatabaseConnection, String> {
    load(path)?
        .into_iter()
        .find(|connection| connection.id == id)
        .ok_or_else(|| {
            format!(
                "This machine has no database connection for {id}. Add or remap it in the Data library"
            )
        })
}

#[derive(Debug)]
pub struct QueryOutput {
    pub path: PathBuf,
    directory: PathBuf,
}

impl Drop for QueryOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub fn run_query(connection: &DatabaseConnection, query: &str) -> Result<QueryOutput, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("A database table needs a SQL query".into());
    }
    let source = SourceConn::try_from(connection.uri.as_str())
        .map_err(|error| format!("Could not read the database URI: {error}"))?;
    let destination = get_arrow(&source, None, &[CXQuery::from(query)], None)
        .map_err(|error| format!("Database query failed: {error}"))?;
    let schema = destination.arrow_schema();
    let batches = destination
        .arrow()
        .map_err(|error| format!("Could not read the database result: {error}"))?;

    let directory = std::env::temp_dir().join(format!("framework-database-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join("result.parquet");
    let file = fs::File::create(&path).map_err(|error| error.to_string())?;
    let mut writer = ArrowWriter::try_new(file, schema, None)
        .map_err(|error| format!("Could not cache the database result: {error}"))?;
    for batch in batches {
        writer
            .write(&batch)
            .map_err(|error| format!("Could not cache the database result: {error}"))?;
    }
    writer
        .close()
        .map_err(|error| format!("Could not finish the database cache: {error}"))?;
    Ok(QueryOutput { path, directory })
}

fn id_for_name(name: &str) -> String {
    let mut id = String::new();
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            id.push(character.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    id.trim_end_matches('-').to_string()
}

fn replace_store(temporary: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_connections_round_trip() {
        let directory = std::env::temp_dir().join(format!("framework-db-test-{}", Uuid::new_v4()));
        let path = directory.join(CONNECTION_STORE_NAME);
        let saved = save(
            &path,
            DatabaseConnection {
                id: String::new(),
                name: "Finance warehouse".into(),
                uri: "sqlite:///:memory:".into(),
            },
        )
        .unwrap();
        assert_eq!(saved.id, "finance-warehouse");
        assert_eq!(load(&path).unwrap(), vec![saved]);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn sqlite_query_becomes_parquet() {
        let directory =
            std::env::temp_dir().join(format!("framework-sqlite-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let uri = format!("sqlite://{}", directory.join("source.sqlite").display());
        let output = run_query(
            &DatabaseConnection {
                id: "test".into(),
                name: "Test".into(),
                uri,
            },
            "select 1 as amount union all select 2",
        )
        .unwrap();
        assert!(output.path.is_file());
        assert!(fs::metadata(&output.path).unwrap().len() > 0);
        let _ = fs::remove_dir_all(directory);
    }
}
