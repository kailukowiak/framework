# Flatpak: what has to be true first

FrameWork does not ship a Flatpak. A manifest was written and then removed
rather than kept as dead infrastructure, because a manifest that has never
built reads as "Flatpak is handled" when it is not. This records what the
attempt established, so that picking the work up again starts from the
findings rather than from a file that fails on first contact.

Nothing here blocks the `.deb`, `.rpm`, or `.AppImage` bundles. Those are
unaffected by any of it.

## The blocker: documents are not self-contained

This is the one that matters, and it is a storage question rather than a
packaging question.

Saving `~/work/Orders.fw` also creates and writes
`~/work/.framework/<document-uuid>/` beside it — see
`CollaborationPaths::for_document` in `crates/framework-core`. A file chosen
through the XDG document portal grants a handle to that one file and to
nothing around it, so a portal-only sandbox can open a document and then fail
to save it.

The manifest worked around this with `--filesystem=host`, which is the whole
sandbox. Flathub challenges that permission, and the answer they accept — the
one an editor gives, that the app's whole job is user files — is weaker here
than it looks, because the app does not merely read user files, it writes
sibling state the user never named.

Two ways out, in rough order of preference:

1. Make a `.fw` document self-contained, so a portal handle to the file is
   sufficient. This is worth doing on its own merits, independent of Flatpak.
2. Keep collaboration state in `XDG_DATA_HOME` keyed by document UUID, and
   accept that the state no longer travels when a user copies the document.

Until one of those lands, a Flatpak is either broken or unsandboxed.

## Smaller items, each straightforward once the above is settled

**The updater has to be turned off.** A Flatpak cannot replace its own binary,
so `tauri-plugin-updater` downloads and then fails in a way `src/lib/updates.ts`
reports as "failed" rather than "unsupported". Gate the check on the
`FLATPAK_ID` environment variable and report "unsupported" — Flatpak updates
come from the repo, not from the app.

**The MCP command it prints is unreachable.** `installed_mcp_executable()` in
`src-tauri/src/lib.rs` finds a sibling binary and shows its path, which inside
the sandbox is `/app/bin/framework-mcp` — a path no external MCP client can
run. Under Flatpak the command to print is
`flatpak run --command=framework-mcp com.framework.canvas`.

**Flathub needs a hosted screenshot.** `com.framework.canvas.metainfo.xml`
validates as AppStream today but is not submittable without at least one
screenshot at a reachable `https` URL. The metainfo file carries a commented
example of the markup.

**Flathub builds are offline.** The manifest fetched during build via
`build-args: --share=network`, which is fine locally and in CI but not for a
submission. That needs vendored cargo sources and a generated node sources
manifest instead.

## Two things that were established and are worth keeping

**The app ID is already consistent.** `com.framework.canvas` is the
`identifier` in `tauri.conf.json`, the `.desktop` stem, the AppStream component
id, and the GLib program name set in `run()`. Flatpak derives the desktop stem,
icon name, and component id from the app id by construction, so the class of
bug that put a generic icon on the COSMIC dock cannot recur under Flatpak. Note
that Flatpak does *not* set the Wayland `app_id` — that still comes from the
program name the app sets itself. The two agree because both are
`com.framework.canvas`, not because Flatpak enforces it.

**The runtime version is unverified.** The attempt targeted
`org.gnome.Platform//47`. Tauri's Linux backend is GTK3 and webkit2gtk-4.1, not
the GTK4 and webkitgtk-6.0 that newer GNOME runtimes lead with, so if a build
cannot find webkit2gtk-4.1, try `46` or `45` before assuming anything else is
wrong.

## Also, if a manifest is written again

Build the frontend explicitly. Invoking `cargo build` directly bypasses
`tauri.conf.json`'s `beforeBuildCommand`, so `npm ci` alone leaves `dist/`
missing and `tauri-codegen` panics with `The `frontendDist` configuration is
set to "../dist" but this path doesn't exist`. The removed manifest had exactly
this bug. A `npm run build` step between the install and the cargo build is
what it needed.
