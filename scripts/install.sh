#!/bin/zsh
# Build the release app and replace the copy in /Applications, then launch it.
# The installed app is what registers the .fw file association, so Finder
# double-clicks only work from here, never from `tauri dev`.
set -euo pipefail
cd "$(dirname "$0")/.."

bunx tauri build --bundles app

# Quit any running copy first — installed or `tauri dev`, which share the
# app identifier: single-instance forwarding would otherwise hand the fresh
# launch's arguments to the old process and exit.
if pgrep -x framework-desktop >/dev/null 2>&1; then
	osascript -e 'tell application id "com.framework.canvas" to quit' >/dev/null 2>&1 || true
	for _ in {1..5}; do
		pgrep -x framework-desktop >/dev/null 2>&1 || break
		sleep 1
	done
	# A bare `cargo run` binary ignores the AppleScript quit (it has no
	# LaunchServices registration to receive it), yet still holds the
	# single-instance lock — the fresh launch below would silently forward
	# to it and exit. Documents persist on every operation, so SIGTERM is
	# safe here.
	pkill -x framework-desktop 2>/dev/null || true
	for _ in {1..5}; do
		pgrep -x framework-desktop >/dev/null 2>&1 || break
		sleep 1
	done
fi

rm -rf /Applications/FrameWork.app
mv target/release/bundle/macos/FrameWork.app /Applications/
open /Applications/FrameWork.app
echo "Installed and launched /Applications/FrameWork.app"
