//! Turns an [`std::io::Error`] hit while reading a document into a sentence
//! a person can act on, instead of the operating system's own wording.
//!
//! The motivating case is macOS TCC: under `tauri dev` the dev binary's
//! Documents-folder access is attributed to whatever launched it (an editor
//! or a terminal), so a stored Deny surfaces here as a bare `Operation not
//! permitted (os error 1)`. That is true but useless — it names an errno,
//! not a place to go fix it.

use std::io;
use std::path::Path;

/// Describes `error`, which occurred while reading `path`, in terms of what
/// the person can do about it. Falls back to the io error's own `Display`
/// text for anything that is not a recognized, actionable case.
pub fn describe_io_error(error: &io::Error, path: &Path) -> String {
    match error.kind() {
        io::ErrorKind::PermissionDenied => permission_denied_message(path),
        io::ErrorKind::NotFound => format!("{} no longer exists.", path.display()),
        _ => error.to_string(),
    }
}

fn permission_denied_message(path: &Path) -> String {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| path.display().to_string());
    if cfg!(target_os = "macos") {
        format!(
            "macOS is not letting FrameWork read {parent}. In System Settings → Privacy & Security → Files and Folders, allow access for FrameWork; when running a development build, allow it for the app that launched FrameWork (the terminal or editor) instead."
        )
    } else {
        format!(
            "FrameWork is not allowed to read {parent}. Check that folder's permissions and allow access for FrameWork."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_denied_names_the_parent_folder_and_the_dev_launcher() {
        let path = Path::new("/Users/someone/Documents/FrameWork Tutorials/Workbook.fw");
        let error = io::Error::from(io::ErrorKind::PermissionDenied);
        let message = describe_io_error(&error, path);
        assert!(message.contains("/Users/someone/Documents/FrameWork Tutorials"));
        assert!(message.contains("System Settings") == cfg!(target_os = "macos"));
        assert!(message.contains("terminal or editor") == cfg!(target_os = "macos"));
    }

    #[test]
    fn not_found_names_the_path_itself() {
        let path = Path::new("/tmp/does-not-exist/Workbook.fw");
        let error = io::Error::from(io::ErrorKind::NotFound);
        let message = describe_io_error(&error, path);
        assert_eq!(message, "/tmp/does-not-exist/Workbook.fw no longer exists.");
    }

    #[test]
    fn anything_else_falls_back_to_the_io_error_text() {
        let path = Path::new("/tmp/Workbook.fw");
        let error = io::Error::other("disk went away");
        let message = describe_io_error(&error, path);
        assert_eq!(message, "disk went away");
    }
}
