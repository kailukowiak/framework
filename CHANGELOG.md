# Changelog

What changed, written for the person using FrameWork rather than the person
who wrote it. The release workflow lifts the section matching the tag it is
building into the GitHub release and into the in-app update offer, so an
entry here is the only place release notes come from — see
`.github/workflows/release.yml` and `src/lib/updates.ts`.

Every version has a section, newest first, headed `## <version>` with no `v`.
Work lands under `## Unreleased`; cutting a release renames that heading to
the version being tagged and opens a fresh `## Unreleased` above it.

## Unreleased

## 0.1.4

- Opening FrameWork on Windows no longer leaves a terminal window sitting
  behind it.
- Release notes in the update dialog render as markdown — headings, lists,
  tables, and links, wrapped to the dialog — instead of a block of raw text.
- The update offer no longer repeats the download and first-launch
  instructions. You are already running FrameWork and one click from an
  automatic install; it shows what changed and nothing else.

## 0.1.3

- Linux desktops recognise FrameWork and its `.fw` documents: the app appears
  in the applications menu with its icon, and double-clicking a document opens
  it.

## 0.1.2

- FrameWork offers its own updates. When a newer release exists it says so
  once, and Install and restart downloads, verifies the signature, and
  replaces the running copy in place. Skip stops it asking about that version;
  Check for Updates in the menu asks whenever you want an answer.
- Copies installed through `apt`, `dnf`, or a software centre say so instead
  of failing: those update the way everything else on the system does.

## 0.1.1

- macOS no longer reports the app as damaged and refusing to open. The bundle
  is signed, so the first launch is the ordinary unidentified-developer
  warning that **Open Anyway** clears, not a dead end.
- The download link on the site points at the current release rather than a
  fixed version.

## 0.1.0

First public build: the canvas, frames, formulas, and documents, packaged as
a `.dmg`, `.exe`/`.msi`, `.deb`, `.rpm`, and `.AppImage`.
