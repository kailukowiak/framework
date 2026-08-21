use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DataArtifact {
    pub id: String,
    pub path: String,
    pub row_count: usize,
    pub format: ArtifactFormat,
    pub source_name: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ArtifactFormat {
    Parquet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum ConnectorRecipe {
    File {
        source_path: String,
    },
    /// A machine-local command profile that writes data to stdout. The
    /// executable and fixed arguments deliberately do not travel with the
    /// document: opening a shared workbook must never execute a command its
    /// author smuggled into the file. A collaborator maps this id to their own
    /// locally approved profile and credentials.
    Cli {
        profile_id: String,
        source_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        query: Option<String>,
    },
    /// A cached SQL result whose credentials and URI stay in the local
    /// desktop connection store. The document carries the useful, portable
    /// part of the recipe: which local connection to use and which query
    /// rebuilds this table.
    Database {
        connection_id: String,
        source_name: String,
        query: String,
    },
}

impl ConnectorRecipe {
    /// A useful, non-secret source address for labels and diagnostics.
    pub fn source_label(&self) -> String {
        match self {
            Self::File { source_path } => source_path.clone(),
            Self::Cli { source_label, .. } => source_label.clone(),
            Self::Database { source_name, .. } => source_name.clone(),
        }
    }

    /// The source's own filename. It becomes the artifact name after a pull
    /// and is also the compact label shown on a frame.
    pub fn source_name(&self) -> String {
        match self {
            Self::File { source_path } => file_name_of(source_path),
            Self::Cli { source_label, .. } => source_label.clone(),
            Self::Database { source_name, .. } => source_name.clone(),
        }
    }
}

fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cli_recipe_contains_no_machine_command_or_credentials() {
        let recipe = ConnectorRecipe::Cli {
            profile_id: "finance-readonly".into(),
            source_label: "warehouse/ledger".into(),
            query: Some("select * from ledger".into()),
        };
        let json = serde_json::to_value(&recipe).unwrap();
        assert_eq!(json["profileId"], "finance-readonly");
        assert!(json.get("program").is_none());
        assert!(json.get("arguments").is_none());
        assert!(json.get("credentials").is_none());
        assert_eq!(recipe.source_label(), "warehouse/ledger");
    }

    #[test]
    fn a_database_recipe_contains_no_uri_or_credentials() {
        let recipe = ConnectorRecipe::Database {
            connection_id: "finance-readonly".into(),
            source_name: "Ledger".into(),
            query: "select * from finance.ledger".into(),
        };
        let json = serde_json::to_value(&recipe).unwrap();
        assert_eq!(json["connectionId"], "finance-readonly");
        assert!(json.get("uri").is_none());
        assert!(json.get("credentials").is_none());
    }
}
