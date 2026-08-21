use crate::model::document::Document;
use crate::operation::event::VersionVector;
use serde::{Deserialize, Serialize};

pub const FRAMEWORK_FILE_EXTENSION: &str = "fw";

pub const FRAMEWORK_FILE_FORMAT: &str = "framework-document";

pub const FRAMEWORK_FILE_VERSION: u32 = 1;

/// The exact tutorial contract understood by this build.
///
/// A tutorial is executable product documentation: its saved formulas,
/// transformations, and expected gestures all describe one version of the
/// application. Unlike an ordinary workbook, silently carrying it forward can
/// teach an interaction that no longer exists. Bump this when a tutorial must
/// be regenerated; old working copies will then ask to be reset instead of
/// opening under different behavior.
pub const FRAMEWORK_TUTORIAL_VERSION: u32 = 1;

pub const MAX_IMPORT_ROWS: usize = 5_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrameworkDocumentFile {
    pub(crate) format: String,
    pub(crate) format_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tutorial_version: Option<u32>,
    pub(crate) document: Document,
    #[serde(default)]
    pub(crate) version_vector: VersionVector,
}

pub(crate) fn json_contains_string(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(value) => value == needle,
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_contains_string(value, needle)),
        serde_json::Value::Object(values) => values
            .values()
            .any(|value| json_contains_string(value, needle)),
        _ => false,
    }
}

pub(crate) fn update_plot_field_titles(
    value: &mut serde_json::Value,
    field_id: &str,
    old_name: &str,
    new_name: &str,
) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                update_plot_field_titles(value, field_id, old_name, new_name);
            }
        }
        serde_json::Value::Object(values) => {
            let matches_field = values
                .get("field")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|field| field == field_id);
            let uses_default_title = values
                .get("title")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|title| title == old_name);
            if matches_field && uses_default_title {
                values.insert("title".into(), serde_json::Value::String(new_name.into()));
            }
            for value in values.values_mut() {
                update_plot_field_titles(value, field_id, old_name, new_name);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::test_support::*;
    #[allow(unused_imports)]
    use crate::*;
    #[allow(unused_imports)]
    use std::{fs, path::PathBuf};
    #[allow(unused_imports)]
    use uuid::Uuid;

    #[test]
    pub(crate) fn unsupported_fw_versions_are_rejected() {
        let directory = temporary_test_directory("future-file");
        let path = directory.join("future.fw");
        let file = FrameworkDocumentFile {
            format: FRAMEWORK_FILE_FORMAT.into(),
            format_version: FRAMEWORK_FILE_VERSION + 1,
            tutorial_version: None,
            document: Document::demo(),
            version_vector: VersionVector::new(),
        };
        fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();

        assert!(matches!(
            Store::load(&path),
            Err(CoreError::Load(message)) if message.contains("Unsupported FrameWork document version")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    pub(crate) fn tutorial_versions_are_exact_and_survive_saves() {
        let directory = temporary_test_directory("tutorial-version");
        let path = directory.join("tutorial.fw");
        let store = Store::new_tutorial(Document::blank("Lesson"));
        store.save(&path).unwrap();

        let mut serialized: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(serialized["tutorialVersion"], FRAMEWORK_TUTORIAL_VERSION);
        let reopened = Store::load(&path).unwrap();
        reopened.save(&path).unwrap();
        serialized = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(serialized["tutorialVersion"], FRAMEWORK_TUTORIAL_VERSION);

        for (version, age) in [
            (FRAMEWORK_TUTORIAL_VERSION - 1, "older"),
            (FRAMEWORK_TUTORIAL_VERSION + 1, "newer"),
        ] {
            serialized["tutorialVersion"] = version.into();
            fs::write(&path, serde_json::to_string_pretty(&serialized).unwrap()).unwrap();
            assert!(matches!(
                Store::load(&path),
                Err(CoreError::Load(message))
                    if message.contains(age) && message.contains("Reset tutorials")
            ));
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
