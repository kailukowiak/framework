use framework_core::ConnectorRecipe;
use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use ts_rs::TS;
use uuid::Uuid;

/// Runs a connector's program without the console window Windows would
/// otherwise open in front of the document.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const PROFILE_STORE_NAME: &str = "cli-connectors.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CliConnectorProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: CliConnectionKind,
    pub name: String,
    pub program: String,
    /// One process argument per entry. `{source}` and `{query}` are replaced
    /// inside the argument without ever passing through a shell.
    #[serde(default)]
    pub arguments: Vec<String>,
    pub output: CliOutputFormat,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CliConnectionKind {
    Database,
    Api,
    #[default]
    Script,
}

impl CliConnectionKind {
    fn id_prefix(self) -> &'static str {
        match self {
            Self::Database => "database",
            Self::Api => "api",
            Self::Script => "",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CliOutputFormat {
    Csv,
    Tsv,
    Parquet,
}

impl CliOutputFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Parquet => "parquet",
        }
    }
}

pub fn load_profiles(path: &Path) -> Result<Vec<CliConnectorProfile>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut profiles: Vec<CliConnectorProfile> =
        serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(profiles)
}

pub fn save_profile(
    path: &Path,
    mut profile: CliConnectorProfile,
) -> Result<CliConnectorProfile, String> {
    profile.name = profile.name.trim().to_string();
    profile.program = profile.program.trim().to_string();
    profile.arguments = profile
        .arguments
        .into_iter()
        .map(|argument| argument.trim_end().to_string())
        .filter(|argument| !argument.is_empty())
        .collect();
    if profile.name.is_empty() || profile.program.is_empty() {
        return Err("A command connection needs a name and executable".into());
    }
    if profile.program.contains('\0') || profile.arguments.iter().any(|value| value.contains('\0'))
    {
        return Err("Command connections cannot contain null characters".into());
    }
    let mut profiles = load_profiles(path)?;
    if profile.id.trim().is_empty() {
        profile.id = profile_id_for_name(profile.kind, &profile.name);
        if profile.id.is_empty() {
            return Err("A command connection name needs at least one letter or number".into());
        }
    }
    if let Some(existing) = profiles
        .iter_mut()
        .find(|existing| existing.id == profile.id)
    {
        *existing = profile.clone();
    } else {
        profiles.push(profile.clone());
    }
    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    let parent = path
        .parent()
        .ok_or_else(|| "Connector profile file has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!(".cli-connectors-{}.tmp", Uuid::new_v4()));
    let json = serde_json::to_vec_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&temporary, json).map_err(|error| error.to_string())?;
    replace_profile_store(&temporary, path)?;
    Ok(profile)
}

fn replace_profile_store(temporary: &Path, destination: &Path) -> Result<(), String> {
    #[cfg(windows)]
    if destination.exists() {
        // std::fs::rename cannot replace an existing file on Windows. The
        // profile contains commands but no credentials, so the small crash
        // window is preferable to making the whole connector surface
        // platform-specific. The still-complete temporary file is retained
        // if either operation fails.
        fs::remove_file(destination).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}

fn profile_id_for_name(kind: CliConnectionKind, name: &str) -> String {
    let mut id = String::new();
    let prefix = kind.id_prefix();
    if !prefix.is_empty() {
        id.push_str(prefix);
        id.push('-');
    }
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            id.push(character.to_ascii_lowercase());
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    id.trim_end_matches('-').to_string()
}

pub fn profile_by_id(path: &Path, id: &str) -> Result<CliConnectorProfile, String> {
    load_profiles(path)?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| {
            format!(
                "This machine has no command connection for {id}. Add or remap it in the Data library"
            )
        })
}

#[derive(Debug)]
pub struct CommandOutput {
    pub path: PathBuf,
    directory: PathBuf,
}

impl Drop for CommandOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub fn run_profile(
    profile: &CliConnectorProfile,
    connector: &ConnectorRecipe,
) -> Result<CommandOutput, String> {
    let ConnectorRecipe::Cli {
        source_label,
        query,
        ..
    } = connector
    else {
        return Err("That source is not a command connector".into());
    };
    let arguments = profile
        .arguments
        .iter()
        .map(|argument| render_argument(argument, source_label, query.as_deref()))
        .collect::<Result<Vec<_>, _>>()?;
    let directory = std::env::temp_dir().join(format!("framework-cli-{}", Uuid::new_v4()));
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("output.{}", profile.output.extension()));
    let stdout = fs::File::create(&path).map_err(|error| error.to_string())?;
    let mut command = Command::new(&profile.program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let result = command.output();
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            let _ = fs::remove_dir_all(&directory);
            return Err(format!("Could not start {}: {error}", profile.program));
        }
    };
    if !output.status.success() {
        let _ = fs::remove_dir_all(&directory);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("{} exited with {}", profile.program, output.status)
        } else {
            format!(
                "{} exited with {}: {detail}",
                profile.program, output.status
            )
        });
    }
    Ok(CommandOutput { path, directory })
}

fn render_argument(template: &str, source: &str, query: Option<&str>) -> Result<String, String> {
    if template.contains("{query}") && query.is_none() {
        return Err("This command connection needs a base query".into());
    }
    Ok(template
        .replace("{source}", source)
        .replace("{query}", query.unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connector(query: Option<&str>) -> ConnectorRecipe {
        ConnectorRecipe::Cli {
            profile_id: "profile".into(),
            source_label: "warehouse".into(),
            query: query.map(str::to_string),
        }
    }

    #[test]
    fn substitutions_remain_one_process_argument() {
        assert_eq!(
            render_argument("--command={query}", "source", Some("select 'a b'")).unwrap(),
            "--command=select 'a b'"
        );
    }

    #[test]
    fn new_profile_ids_are_portable_names_not_machine_ids() {
        assert_eq!(
            profile_id_for_name(CliConnectionKind::Script, "Finance Read-only"),
            "finance-read-only"
        );
    }

    #[test]
    fn saving_an_existing_profile_replaces_the_store_cross_platform() {
        let directory = std::env::temp_dir().join(format!("framework-profile-{}", Uuid::new_v4()));
        let path = directory.join(PROFILE_STORE_NAME);
        let profile = CliConnectorProfile {
            id: String::new(),
            kind: CliConnectionKind::Script,
            name: "Warehouse".into(),
            program: "first".into(),
            arguments: Vec::new(),
            output: CliOutputFormat::Csv,
        };
        let saved = save_profile(&path, profile).unwrap();
        save_profile(
            &path,
            CliConnectorProfile {
                program: "second".into(),
                ..saved
            },
        )
        .unwrap();
        assert_eq!(load_profiles(&path).unwrap()[0].program, "second");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_missing_query_is_named_before_the_process_starts() {
        let profile = CliConnectorProfile {
            id: "profile".into(),
            kind: CliConnectionKind::Database,
            name: "Database".into(),
            program: "unused".into(),
            arguments: vec!["--command={query}".into()],
            output: CliOutputFormat::Csv,
        };
        assert_eq!(
            run_profile(&profile, &connector(None)).unwrap_err(),
            "This command connection needs a base query"
        );
    }

    #[cfg(unix)]
    #[test]
    fn runs_without_a_shell_and_captures_stdout() {
        let profile = CliConnectorProfile {
            id: "profile".into(),
            kind: CliConnectionKind::Script,
            name: "printf".into(),
            program: "/usr/bin/printf".into(),
            arguments: vec!["name,amount\\nAlpha,10\\n".into()],
            output: CliOutputFormat::Csv,
        };
        let output = run_profile(&profile, &connector(None)).unwrap();
        assert_eq!(
            fs::read_to_string(&output.path).unwrap(),
            "name,amount\nAlpha,10\n"
        );
    }
}
