# Native XGBoost release packaging

FrameWork trains XGBoost models through `xgb = 3.0.6`, whose default native
binding is `xgboost_lib-sys = 3.0.5`. Native training is built only for the
supported macOS arm64 and Windows x64 targets. Linux keeps the model import and
pure Rust prediction path, but reports native training as unsupported.

The native preparation step is `node scripts/prepare-xgboost-native.mjs`. It
runs before the release build and fetches exact artifacts into the ignored
`src-tauri/native/` staging directory. Every downloaded file is accepted only
when its SHA-256 matches the value in the script. `XGBOOST_LIB_DIR` then points
the sys crate's linker at those same staged files, so Cargo cannot silently
link one XGBoost copy while Tauri bundles another.

Development and e2e use `scripts/tauri-native.mjs` through `npm run tauri`.
It prepares the same artifacts, passes the platform bundle config, and supplies
the loader paths needed by an unbundled development executable. `build:dev-app`
and `test:e2e` use this wrapper too.

## macOS arm64

The release uses the XGBoost 3.0.5 `mac_arm64` dylib and LLVM OpenMP 23.1.1
from Homebrew's checksum-addressed arm64 Tahoe bottle. The extracted OpenMP
library is checked separately after extraction. That library declares macOS
26.0 as its minimum, matching FrameWork's supported OS floor.

The preparation script changes both install IDs to `@rpath` and changes
XGBoost's OpenMP dependency to `@rpath/libomp.dylib`. The release config lists
both as Tauri macOS frameworks. Tauri adds
`@executable_path/../Frameworks` to the application rpath, copies the dylibs
into `FrameWork.app/Contents/Frameworks`, and signs the nested code before it
signs the enclosing bundle. The script applies an ad-hoc signature to each
rewritten staging copy because arm64 macOS refuses to load a Mach-O file with
an invalidated signature; the final bundle signature supersedes it.

The OpenMP and XGBoost license texts are included as application resources.

## Windows x64

The preparation step downloads the checksum-pinned XGBoost DLL and import
library from the same sys-crate tag. Cargo links against the staged import
library. The Windows release config places `xgboost.dll` in the resource root,
which is beside the installed executable where the Windows loader searches for
an ordinary imported DLL. The XGBoost license is included as an application
resource.

## Release verification still required

The preparation script and relocated macOS dylibs have been executed locally;
their checksums, load commands, macOS floor, and ad-hoc signatures were
verified. A release candidate still needs these platform checks before this is
considered proven end to end:

- Build the actual arm64 `.app` and DMG with the release configs, inspect the
  executable and both nested dylibs with `otool -L`, and run fit/predict on a
  clean macOS 26 machine after copying through quarantine. Repeat once a real
  Developer ID and notarization replace the current ad-hoc app signature.
- Build MSI and NSIS installers on Windows 11 x64, inspect the executable's DLL
  imports, install each on a clean machine without Rust, Visual Studio, XGBoost,
  or a developer `PATH`, and run fit/predict plus model reload.
- Confirm the Windows prebuilt DLL's transitive runtime dependencies on the CI
  runner. If it imports a Microsoft Visual C++ runtime that is not provided by
  Windows 11, the installer must add the matching redistributable before the
  target can be called self-contained.

Linux release packaging is deliberately unchanged because native training is
not compiled there. Supporting it later requires a separate pinned `libgomp`
and C++ runtime policy plus clean Ubuntu compatibility testing.

## Building on macOS 27

A clean build on the local macOS 27 host exposed
[Rust issue 157750](https://github.com/rust-lang/rust/issues/157750): implicit
stripping can produce proc-macro dylibs rejected by dyld as a misaligned LINKEDIT
string pool. Cargo profiles explicitly disable stripping for dependency/build-tool
artifacts while retaining `debug = 0` for dependencies. A focused bytemuck derive
build confirmed that removing the stripping step fixes the observed load failure.
This is a compiler-host workaround, not a change to the macOS 26 application floor.
