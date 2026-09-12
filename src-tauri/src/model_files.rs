//! File selection only: the returned data enters the same core import operation
//! as pasted model JSON. Keeping parsing and validation out of this module means
//! MCP, paste, and native-file authoring cannot disagree about accepted models.

use serde::Serialize;
use std::io::Read;
use std::path::Path;

const MAX_MODEL_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelFileSource {
    name: String,
    contents: String,
    bytes: Option<Vec<u8>>,
}

#[tauri::command]
pub(crate) fn pick_model_file() -> Result<Option<ModelFileSource>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Trained model (XGBoost JSON or ONNX)", &["json", "onnx"])
        .pick_file()
    else {
        return Ok(None);
    };
    read_model_file(&path).map(Some)
}

fn read_model_file(path: &Path) -> Result<ModelFileSource, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut contents = Vec::new();
    // A metadata check alone races a growing file. Read at most one byte past
    // the import boundary, so selecting an unexpectedly large file cannot
    // allocate its entire contents before the core gets a chance to refuse it.
    file.take(MAX_MODEL_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|error| error.to_string())?;
    if contents.len() as u64 > MAX_MODEL_BYTES {
        return Err("Model files must be at most 16 MiB".into());
    }
    let (contents, bytes) = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("onnx"))
    {
        (String::new(), Some(contents))
    } else {
        (
            String::from_utf8(contents)
                .map_err(|_| "Choose a UTF-8 XGBoost JSON or ONNX model file".to_string())?,
            None,
        )
    };
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Imported model")
        .to_string();
    Ok(ModelFileSource {
        name,
        contents,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_json_is_data_for_the_normal_import_path() {
        let path = std::env::temp_dir().join(format!("model-{}.json", uuid::Uuid::new_v4()));
        std::fs::write(&path, r#"{"learner":{}}"#).unwrap();
        let result = read_model_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result.contents, r#"{"learner":{}}"#);
        assert!(result.bytes.is_none());
        assert!(result.name.starts_with("model-"));
    }

    #[test]
    fn selected_onnx_bytes_reach_the_normal_import_path_unchanged() {
        let path = std::env::temp_dir().join(format!("model-{}.onnx", uuid::Uuid::new_v4()));
        let bytes = [8, 9, 255, 0];
        std::fs::write(&path, bytes).unwrap();
        let result = read_model_file(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result.bytes, Some(bytes.to_vec()));
        assert!(result.contents.is_empty());
    }

    #[test]
    fn reading_stops_at_the_model_size_limit() {
        let path = std::env::temp_dir().join(format!("model-{}.json", uuid::Uuid::new_v4()));
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_MODEL_BYTES + 1).unwrap();
        let error = read_model_file(&path).err().unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(error.contains("16 MiB"));
    }
}
