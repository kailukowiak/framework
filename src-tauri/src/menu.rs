//! The application menu.
//!
//! Everything here used to be a control in the window's own header: a File
//! button that opened a drop-down, a pair of undo arrows, a tidy-layout
//! button. A desktop application already has a menu bar — on macOS the system
//! draws one whether we fill it or not — so those controls were a second copy
//! of it, costing header space and disagreeing with the first.
//!
//! The menu does not act. It names an intent and emits it; the webview owns
//! every behaviour, because that is where the document state lives. The one
//! exception is the enabled flag on Undo and Redo, which has to be pushed the
//! other way (see [`sync_history`]) — a menu item that is always available is
//! the affordance the toolbar buttons used to provide, and losing it is what
//! would make deleting them a downgrade.

use serde::Serialize;
use tauri::menu::{Menu, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use ts_rs::TS;

/// Emitted at the webview with the chosen item's id as its payload.
pub const MENU_COMMAND_EVENT: &str = "framework-menu-command";

/// Which menu draws the command.
///
/// Not decoration. Three hand-maintained lists used to name these same
/// actions — the menu, the command palette, and the set the Scratchwork
/// pop-out forwards to its workbook — and nothing compared them, so a new
/// item could be enabled in the pop-out and inert, or missing from ⇧⌘P,
/// with everything still compiling. The group travels with the id so the
/// frontend's checks can report a drift in the terms a person would use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MenuGroup {
    File,
    Edit,
    Insert,
    View,
    Window,
    Help,
}

/// One application command exactly as the menu declares it.
#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MenuCommand {
    /// The payload the webview receives on [`MENU_COMMAND_EVENT`].
    pub id: String,
    pub label: String,
    /// The platform accelerator string Tauri binds, e.g. `CmdOrCtrl+Shift+P`.
    /// `None` for an item deliberately left without a key equivalent.
    pub accelerator: Option<String>,
    pub group: MenuGroup,
}

struct MenuCommandSpec {
    id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
    group: MenuGroup,
}

/// **The** definition of the application's menu commands.
///
/// Everything else is downstream of this table: the items built below read
/// their label and accelerator from it, `cargo test -p framework-desktop
/// export_menu_commands` writes it to `src/lib/menuCommands.json`, and the
/// frontend's tests check the palette and the pop-out's forwarding list
/// against that file. Adding an item here and nowhere else is now a failing
/// test rather than a command that quietly does nothing.
const COMMANDS: &[MenuCommandSpec] = &[
    MenuCommandSpec {
        id: "new-window",
        label: "New Window",
        accelerator: Some("CmdOrCtrl+Shift+N"),
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "new-document",
        label: "New Document…",
        accelerator: Some("CmdOrCtrl+N"),
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "open-document",
        label: "Open…",
        accelerator: Some("CmdOrCtrl+O"),
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "save-document-as",
        label: "Save As…",
        accelerator: Some("CmdOrCtrl+Shift+S"),
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "package-document",
        label: "Package This Document",
        accelerator: None,
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "compact-data",
        label: "Reclaim Unused Data Files",
        accelerator: None,
        group: MenuGroup::File,
    },
    // macOS draws Settings in the application menu; the group names the
    // menu it belongs to everywhere else, and the id is the same either way.
    MenuCommandSpec {
        id: "preferences",
        label: "Settings…",
        accelerator: Some("CmdOrCtrl+Comma"),
        group: MenuGroup::File,
    },
    MenuCommandSpec {
        id: "undo",
        label: "Undo",
        accelerator: Some("CmdOrCtrl+Z"),
        group: MenuGroup::Edit,
    },
    MenuCommandSpec {
        id: "redo",
        label: "Redo",
        accelerator: Some("CmdOrCtrl+Shift+Z"),
        group: MenuGroup::Edit,
    },
    MenuCommandSpec {
        id: "find",
        label: "Find…",
        accelerator: Some("CmdOrCtrl+F"),
        group: MenuGroup::Edit,
    },
    MenuCommandSpec {
        id: "scratchpad",
        label: "Scratchwork",
        accelerator: Some("CmdOrCtrl+J"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-variable",
        label: "Variable",
        accelerator: Some("CmdOrCtrl+Alt+V"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-block",
        label: "Formula Block",
        accelerator: Some("CmdOrCtrl+Alt+B"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-text",
        label: "Text",
        accelerator: Some("CmdOrCtrl+Alt+T"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-matrix",
        label: "Calculation Matrix",
        accelerator: Some("CmdOrCtrl+Alt+M"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-frame",
        label: "Frame",
        accelerator: Some("CmdOrCtrl+Alt+F"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "add-container",
        label: "Container",
        accelerator: Some("CmdOrCtrl+Alt+G"),
        group: MenuGroup::Insert,
    },
    MenuCommandSpec {
        id: "quick-commands",
        label: "Quick Commands…",
        accelerator: Some("CmdOrCtrl+Shift+P"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "canvas-only",
        label: "Canvas Only",
        accelerator: Some("CmdOrCtrl+Shift+C"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "toggle-sources",
        label: "Data Panel",
        accelerator: Some("CmdOrCtrl+Shift+D"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "data-library",
        label: "Data Library…",
        accelerator: Some("CmdOrCtrl+Shift+L"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "inspector-toggle",
        label: "Inspector",
        accelerator: Some("CmdOrCtrl+Shift+I"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "inspector-selection",
        label: "Selection Inspector",
        accelerator: Some("CmdOrCtrl+Digit1"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "inspector-format",
        label: "Format Inspector",
        accelerator: Some("CmdOrCtrl+Digit2"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "inspector-wrangle",
        label: "Wrangle Inspector",
        accelerator: Some("CmdOrCtrl+Digit3"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "tidy-layout",
        label: "Arrange Left to Right",
        accelerator: Some("CmdOrCtrl+Shift+A"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "fit-view",
        label: "Fit Selected Card to Window",
        accelerator: Some("CmdOrCtrl+Shift+F"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "collapse-view",
        label: "Collapse or Expand Selected Card",
        accelerator: Some("CmdOrCtrl+Shift+M"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "zoom-in",
        label: "Zoom In",
        accelerator: Some("CmdOrCtrl+Equal"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "zoom-out",
        label: "Zoom Out",
        accelerator: Some("CmdOrCtrl+Minus"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "zoom-reset",
        label: "Actual Size",
        accelerator: Some("CmdOrCtrl+Digit0"),
        group: MenuGroup::View,
    },
    MenuCommandSpec {
        id: "open-scratchwork-window",
        label: "Scratchwork Window",
        accelerator: None,
        group: MenuGroup::Window,
    },
    MenuCommandSpec {
        id: "reference",
        label: "Reference…",
        accelerator: Some("CmdOrCtrl+K"),
        group: MenuGroup::Help,
    },
    MenuCommandSpec {
        id: "keyboard-shortcuts",
        label: "Keyboard Shortcuts…",
        accelerator: Some("CmdOrCtrl+Slash"),
        group: MenuGroup::Help,
    },
    // No accelerator: this is a rare, deliberate action, and every chord
    // spent here is one unavailable to the canvas. Its macOS home is the
    // application menu — see `help_menu`.
    MenuCommandSpec {
        id: "check-for-updates",
        label: "Check for Updates…",
        accelerator: None,
        group: MenuGroup::Help,
    },
];

fn spec(id: &str) -> &'static MenuCommandSpec {
    COMMANDS
        .iter()
        .find(|command| command.id == id)
        .unwrap_or_else(|| panic!("no menu command is defined for `{id}`"))
}

fn command_builder(id: &str) -> MenuItemBuilder {
    let spec = spec(id);
    let builder = MenuItemBuilder::with_id(spec.id, spec.label);
    match spec.accelerator {
        Some(accelerator) => builder.accelerator(accelerator),
        None => builder,
    }
}

fn item<R: Runtime>(app: &AppHandle<R>, id: &str) -> tauri::Result<MenuItem<R>> {
    command_builder(id).build(app)
}

/// The menu's own definition, in the shape the frontend checks itself against.
///
/// Only the exporting test below reads it in-process; it is `pub` because it
/// is the module's statement of what the menu is, not a test helper.
#[cfg_attr(not(test), allow(dead_code))]
pub fn menu_commands() -> Vec<MenuCommand> {
    COMMANDS
        .iter()
        .map(|command| MenuCommand {
            id: command.id.to_string(),
            label: command.label.to_string(),
            accelerator: command.accelerator.map(str::to_string),
            group: command.group,
        })
        .collect()
}

/// The two items whose enabled state follows the document rather than the
/// menu's own structure, kept so the webview can grey them out.
pub struct HistoryMenuItems<R: Runtime> {
    undo: MenuItem<R>,
    redo: MenuItem<R>,
}

impl<R: Runtime> HistoryMenuItems<R> {
    pub fn set(&self, can_undo: bool, can_redo: bool) {
        let _ = self.undo.set_enabled(can_undo);
        let _ = self.redo.set_enabled(can_redo);
    }
}

struct CanvasMenuItems<R: Runtime> {
    quick_commands: MenuItem<R>,
    scratchpad: MenuItem<R>,
    add_variable: MenuItem<R>,
    add_block: MenuItem<R>,
    add_text: MenuItem<R>,
    add_matrix: MenuItem<R>,
    add_frame: MenuItem<R>,
    add_container: MenuItem<R>,
    data_library: MenuItem<R>,
    canvas_only: MenuItem<R>,
    toggle_sources: MenuItem<R>,
    inspector_toggle: MenuItem<R>,
    inspector_selection: MenuItem<R>,
    inspector_format: MenuItem<R>,
    inspector_wrangle: MenuItem<R>,
    tidy_layout: MenuItem<R>,
    fit_view: MenuItem<R>,
    collapse_view: MenuItem<R>,
    reference: MenuItem<R>,
    keyboard_shortcuts: MenuItem<R>,
    zoom_in: MenuItem<R>,
    zoom_out: MenuItem<R>,
    zoom_reset: MenuItem<R>,
    check_for_updates: MenuItem<R>,
}

impl<R: Runtime> CanvasMenuItems<R> {
    fn insert_menu(&self, app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
        SubmenuBuilder::new(app, "Insert")
            .item(&self.scratchpad)
            .separator()
            .item(&self.add_variable)
            .item(&self.add_block)
            .item(&self.add_text)
            .item(&self.add_matrix)
            .item(&self.add_frame)
            .item(&self.add_container)
            .build()
    }

    fn view_menu(&self, app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
        let view = SubmenuBuilder::new(app, "View")
            .item(&self.quick_commands)
            .separator()
            .item(&self.canvas_only)
            .item(&self.toggle_sources)
            .item(&self.data_library)
            .separator()
            .item(&self.inspector_toggle)
            .item(&self.inspector_selection)
            .item(&self.inspector_format)
            .item(&self.inspector_wrangle)
            .separator()
            .item(&self.tidy_layout)
            .item(&self.fit_view)
            .item(&self.collapse_view)
            .separator()
            .item(&self.zoom_in)
            .item(&self.zoom_out)
            .item(&self.zoom_reset);
        #[cfg(target_os = "macos")]
        let view = view.separator().fullscreen();
        view.build()
    }

    fn help_menu(&self, app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
        let help = SubmenuBuilder::new(app, "Help")
            .item(&self.reference)
            .separator()
            .item(&self.keyboard_shortcuts);
        // macOS keeps Check for Updates in the application menu next to About,
        // which is the first place a Mac user looks; everywhere else Help is
        // where it has always lived. Same command id from either position.
        #[cfg(not(target_os = "macos"))]
        let help = help.separator().item(&self.check_for_updates);
        help.build()
    }
}

fn canvas_menu_items<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<CanvasMenuItems<R>> {
    Ok(CanvasMenuItems {
        quick_commands: item(app, "quick-commands")?,
        scratchpad: item(app, "scratchpad")?,
        add_variable: item(app, "add-variable")?,
        add_block: item(app, "add-block")?,
        add_text: item(app, "add-text")?,
        add_matrix: item(app, "add-matrix")?,
        add_frame: item(app, "add-frame")?,
        add_container: item(app, "add-container")?,
        data_library: item(app, "data-library")?,
        canvas_only: item(app, "canvas-only")?,
        toggle_sources: item(app, "toggle-sources")?,
        inspector_toggle: item(app, "inspector-toggle")?,
        inspector_selection: item(app, "inspector-selection")?,
        inspector_format: item(app, "inspector-format")?,
        inspector_wrangle: item(app, "inspector-wrangle")?,
        tidy_layout: item(app, "tidy-layout")?,
        fit_view: item(app, "fit-view")?,
        collapse_view: item(app, "collapse-view")?,
        reference: item(app, "reference")?,
        keyboard_shortcuts: item(app, "keyboard-shortcuts")?,
        zoom_in: item(app, "zoom-in")?,
        zoom_out: item(app, "zoom-out")?,
        zoom_reset: item(app, "zoom-reset")?,
        check_for_updates: item(app, "check-for-updates")?,
    })
}

/// Builds the menu, and puts the history items into managed state on the way
/// past so [`crate::set_history_menu_state`] can reach them.
///
/// This is the builder's `menu` hook rather than a `set_menu` call during
/// setup, so the default menu Tauri would otherwise assemble is never built:
/// its File menu holds a lone Close Window, and its Undo is the text field's.
///
/// Accelerators live here rather than on a `keydown` listener in the webview:
/// the platform swallows a key equivalent it has a menu item for, so any
/// handler that also claimed one would fire twice on the platforms that draw
/// the menu inside the window. The webview keeps its own bindings only for
/// the browser dev server, where there is no menu at all — and for e2e
/// builds, which run menu-less for the same reason the dev server does:
/// WebDriver's synthesized keys never reach a native menu.
#[cfg_attr(feature = "e2e", allow(dead_code))]
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let new_window = item(app, "new-window")?;
    let new_document = item(app, "new-document")?;
    let open_document = item(app, "open-document")?;
    // No Save. Every operation is written to disk on its way through the
    // store, so an item here could only ever re-save a saved document — and a
    // menu command that is always available and never does anything teaches
    // people to press it for reassurance it cannot give. Save As stays,
    // because giving a scratch canvas a file is a real thing to want.
    let save_document_as = item(app, "save-document-as")?;
    let package_document = item(app, "package-document")?;
    let compact_data = item(app, "compact-data")?;
    let preferences = item(app, "preferences")?;

    // Not the predefined undo/redo: those are the text field's, routed to
    // whatever holds focus, and this application's undo is the document's.
    // They start disabled because a freshly opened document has no history.
    // Find lives in Edit next to the system's own text commands, which is
    // where every application puts it and therefore the only place looking
    // for it costs nothing.
    let find = item(app, "find")?;

    let undo = command_builder("undo").enabled(false).build(app)?;
    let redo = command_builder("redo").enabled(false).build(app)?;

    let canvas = canvas_menu_items(app)?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&new_window)
        .item(&new_document)
        .item(&open_document)
        .separator()
        .item(&save_document_as)
        .separator()
        .item(&package_document)
        .item(&compact_data)
        .separator();
    // macOS puts Settings and Quit in the application menu; everywhere else
    // the File menu is where they have always been.
    #[cfg(not(target_os = "macos"))]
    let file = file.item(&preferences).separator();
    let file = file.close_window();
    #[cfg(not(target_os = "macos"))]
    let file = file.quit();
    let file = file.build()?;

    // Cut/Copy/Paste stay predefined: the webview's text fields and grid both
    // listen for the real system clipboard events they raise.
    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&undo)
        .item(&redo)
        .separator()
        .item(&find)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let insert = canvas.insert_menu(app)?;
    let view = canvas.view_menu(app)?;

    let scratchwork_window = item(app, "open-scratchwork-window")?;
    let window = SubmenuBuilder::new(app, "Window")
        .item(&scratchwork_window)
        .separator()
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    let help = canvas.help_menu(app)?;

    let menu = Menu::new(app)?;
    #[cfg(target_os = "macos")]
    {
        let application = SubmenuBuilder::new(app, "FrameWork")
            .about(None)
            .item(&canvas.check_for_updates)
            .separator()
            .item(&preferences)
            .separator()
            .services()
            .separator()
            .hide()
            .hide_others()
            .show_all()
            .separator()
            .quit()
            .build()?;
        menu.append(&application)?;
    }
    menu.append(&file)?;
    menu.append(&edit)?;
    menu.append(&insert)?;
    menu.append(&view)?;
    menu.append(&window)?;
    menu.append(&help)?;

    app.manage(HistoryMenuItems { undo, redo });
    Ok(menu)
}

/// Forwards a chosen item to the webview, which owns what it means.
///
/// The focused window is asked first so a multi-window document routes the
/// command to the window the person is looking at. But focus is not a
/// precondition: during the instant after a menu closes — or under any
/// input that activates the menu without making a window key — no window
/// reports focus, and a command dropped here dies silently behind an
/// enabled menu item. With one window there is nothing to disambiguate, so
/// any window is the right recipient; with several, the first is still
/// better than a swallowed Undo.
#[cfg_attr(feature = "e2e", allow(dead_code))]
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let windows = app.webview_windows();
    let target = windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| windows.values().next());
    if let Some(window) = target {
        deliver(app, window.label(), id);
    }
}

/// The one place a menu command becomes an event. Both the real click and
/// the e2e replay below go through it, so a spec that drives the replay is
/// exercising the delivery a person's click uses, not a parallel one.
fn deliver<R: Runtime>(app: &AppHandle<R>, label: &str, id: &str) {
    let _ = app.emit_to(label, MENU_COMMAND_EVENT, id);
}

/// Replays a menu command the way choosing the item does.
///
/// The e2e build runs menu-less — WebDriver's synthesized keys never reach
/// an NSMenu, so a native menu would make every menu-owned accelerator
/// undrivable — which left menu construction and command routing with no
/// automated coverage at all: an id that no longer matched a handler was
/// only ever found by hand. This puts the *routing* under test without
/// pretending to test the native menu itself: it refuses an id the menu
/// does not declare, and then emits through [`deliver`], exactly as a click
/// does. Compiled only under the `e2e` feature, which a release build
/// cannot have (see the `compile_error!` in `lib.rs`).
#[cfg(feature = "e2e")]
#[tauri::command]
pub fn replay_menu_command(
    app: AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    window_label: Option<String>,
) -> Result<(), String> {
    if !COMMANDS.iter().any(|command| command.id == id) {
        return Err(format!("`{id}` is not a menu command"));
    }
    let label = window_label.unwrap_or_else(|| window.label().to_string());
    if app.get_webview_window(&label).is_none() {
        return Err(format!("no window is labelled `{label}`"));
    }
    deliver(&app, &label, &id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, menu_commands};

    /// Writes the menu's definition where the frontend's tests read it.
    ///
    /// Same mechanism as the ts-rs bindings next to it — a `cargo test` that
    /// regenerates a checked-in file — because the alternative, a command the
    /// webview asks at runtime, cannot be consulted by a vitest run and so
    /// would leave the drift undetected until the app was launched.
    #[test]
    fn export_menu_commands() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/menuCommands.json");
        let mut generated = serde_json::to_string_pretty(&menu_commands()).expect("serializable");
        generated.push('\n');
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != generated {
            std::fs::write(&path, &generated).expect("writable");
            panic!(
                "{} was out of date and has been rewritten — commit it",
                path.display()
            );
        }
    }

    #[test]
    fn command_ids_are_unique() {
        let mut ids: Vec<&str> = COMMANDS.iter().map(|command| command.id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "two menu commands share an id");
    }
}
