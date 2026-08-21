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

use tauri::menu::{Menu, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Emitted at the webview with the chosen item's id as its payload.
pub const MENU_COMMAND_EVENT: &str = "framework-menu-command";

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
    scratchpad: MenuItem<R>,
    add_block: MenuItem<R>,
    add_text: MenuItem<R>,
    add_frame: MenuItem<R>,
    add_container: MenuItem<R>,
    data_library: MenuItem<R>,
    toggle_sources: MenuItem<R>,
    inspector_selection: MenuItem<R>,
    inspector_format: MenuItem<R>,
    inspector_wrangle: MenuItem<R>,
    tidy_layout: MenuItem<R>,
    fit_view: MenuItem<R>,
    collapse_view: MenuItem<R>,
    keyboard_shortcuts: MenuItem<R>,
    zoom_in: MenuItem<R>,
    zoom_out: MenuItem<R>,
    zoom_reset: MenuItem<R>,
}

impl<R: Runtime> CanvasMenuItems<R> {
    fn insert_menu(&self, app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
        SubmenuBuilder::new(app, "Insert")
            .item(&self.scratchpad)
            .separator()
            .item(&self.add_block)
            .item(&self.add_text)
            .item(&self.add_frame)
            .item(&self.add_container)
            .build()
    }

    fn view_menu(&self, app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
        let view = SubmenuBuilder::new(app, "View")
            .item(&self.toggle_sources)
            .item(&self.data_library)
            .separator()
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
        SubmenuBuilder::new(app, "Help")
            .item(&self.keyboard_shortcuts)
            .build()
    }
}

fn canvas_menu_items<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<CanvasMenuItems<R>> {
    let item = |id, label, accelerator| {
        MenuItemBuilder::with_id(id, label)
            .accelerator(accelerator)
            .build(app)
    };
    Ok(CanvasMenuItems {
        scratchpad: item("scratchpad", "Scratchpad", "CmdOrCtrl+J")?,
        add_block: item("add-block", "Formula Block", "CmdOrCtrl+Alt+B")?,
        add_text: item("add-text", "Text", "CmdOrCtrl+Alt+T")?,
        add_frame: item("add-frame", "Frame", "CmdOrCtrl+Alt+F")?,
        add_container: item("add-container", "Container", "CmdOrCtrl+Alt+G")?,
        data_library: item("data-library", "Data Library…", "CmdOrCtrl+Shift+L")?,
        toggle_sources: item("toggle-sources", "Data Panel", "CmdOrCtrl+Shift+D")?,
        inspector_selection: item(
            "inspector-selection",
            "Selection Inspector",
            "CmdOrCtrl+Digit1",
        )?,
        inspector_format: item("inspector-format", "Format Inspector", "CmdOrCtrl+Digit2")?,
        inspector_wrangle: item("inspector-wrangle", "Wrangle Inspector", "CmdOrCtrl+Digit3")?,
        tidy_layout: item("tidy-layout", "Arrange Left to Right", "CmdOrCtrl+Shift+A")?,
        fit_view: item(
            "fit-view",
            "Fit Selected Card to Window",
            "CmdOrCtrl+Shift+F",
        )?,
        collapse_view: item(
            "collapse-view",
            "Collapse or Expand Selected Card",
            "CmdOrCtrl+Shift+M",
        )?,
        keyboard_shortcuts: item(
            "keyboard-shortcuts",
            "Keyboard Shortcuts…",
            "CmdOrCtrl+Slash",
        )?,
        zoom_in: item("zoom-in", "Zoom In", "CmdOrCtrl+Equal")?,
        zoom_out: item("zoom-out", "Zoom Out", "CmdOrCtrl+Minus")?,
        zoom_reset: item("zoom-reset", "Actual Size", "CmdOrCtrl+Digit0")?,
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
    let new_window = MenuItemBuilder::with_id("new-window", "New Window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    let new_document = MenuItemBuilder::with_id("new-document", "New Document…")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_document = MenuItemBuilder::with_id("open-document", "Open…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    // No Save. Every operation is written to disk on its way through the
    // store, so an item here could only ever re-save a saved document — and a
    // menu command that is always available and never does anything teaches
    // people to press it for reassurance it cannot give. Save As stays,
    // because giving a scratch canvas a file is a real thing to want.
    let save_document_as = MenuItemBuilder::with_id("save-document-as", "Save As…")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;
    let package_document =
        MenuItemBuilder::with_id("package-document", "Package This Document").build(app)?;
    let compact_data =
        MenuItemBuilder::with_id("compact-data", "Reclaim Unused Data Files").build(app)?;
    let preferences = MenuItemBuilder::with_id("preferences", "Settings…")
        .accelerator("CmdOrCtrl+Comma")
        .build(app)?;

    // Not the predefined undo/redo: those are the text field's, routed to
    // whatever holds focus, and this application's undo is the document's.
    // They start disabled because a freshly opened document has no history.
    let undo = MenuItemBuilder::with_id("undo", "Undo")
        .accelerator("CmdOrCtrl+Z")
        .enabled(false)
        .build(app)?;
    let redo = MenuItemBuilder::with_id("redo", "Redo")
        .accelerator("CmdOrCtrl+Shift+Z")
        .enabled(false)
        .build(app)?;

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
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let insert = canvas.insert_menu(app)?;
    let view = canvas.view_menu(app)?;

    let window = SubmenuBuilder::new(app, "Window")
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
#[cfg_attr(feature = "e2e", allow(dead_code))]
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
    {
        let _ = app.emit_to(window.label(), MENU_COMMAND_EVENT, id);
    }
}
