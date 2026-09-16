# Native XGBoost release packaging

FrameWork trains XGBoost models through `xgb = 3.0.6`, whose default native
binding is `xgboost_lib-sys = 3.0.5`. Native training is built for the
supported macOS arm64, Windows x64 and Linux x64 targets. Any other target
keeps the model import and pure Rust prediction path, but reports native
training as unsupported.

The workspace applies the sys crate through `[patch.crates-io]` from
`vendor/xgboost_lib-sys`: the published crate with `bindgen` moved from 0.71
to 0.72, because 0.71 generates fieldless C structs against libclang 20 or
newer and its own layout assertion then fails to compile. Its build script,
download pins and link directives are the published ones; the directory's
README says what to remove once upstream carries the bump.

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

## Linux x64

The preparation step downloads the checksum-pinned `linux_amd64`
`libxgboost.so` from the same sys-crate tag. Cargo links against the staged
copy, and `src-tauri/build.rs` gives the executable an rpath of
`$ORIGIN/../lib/framework`. The Linux release config installs the shared
object at `/usr/lib/framework/libxgboost.so` in the `.deb` and `.rpm`, which
is where that rpath resolves from `/usr/bin`, and declares the GNU OpenMP
runtime (`libgomp1` on Debian, `libgomp` on RPM systems) as a package
dependency, because the prebuilt library takes OpenMP and the C++ runtime
from the host rather than carrying its own. The AppImage bundler resolves the
executable's shared libraries with `ldd`, so the release step and the
`tauri-native.mjs` wrapper export `LD_LIBRARY_PATH` pointing at the staged
directory; linuxdeploy then copies `libxgboost.so` and `libgomp` into the
image and rewrites the rpath to its own layout.

The prebuilt library requires glibc 2.34 and `GLIBCXX_3.4.30`, the GCC 12
C++ runtime. Ubuntu 22.04, the release builder and oldest supported
distribution, ships both. Building the sys crate needs libclang for bindgen,
so the Linux CI and release jobs install `libclang-dev`. The XGBoost license
is included as an application resource.

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

- A debug `.deb` built locally on Ubuntu 24.04 (2026-09-15) carried
  `libgomp1` in `Depends`, `libxgboost.so` at `/usr/lib/framework`, and an
  executable whose `RUNPATH` of `$ORIGIN/../lib/framework` resolved it under
  `ldd` with no `LD_LIBRARY_PATH`. Still to do: build the `.rpm` and AppImage
  on the Ubuntu 22.04 release runner, confirm the AppImage resolves the
  library from its own `usr/lib`, and run fit/predict on clean Ubuntu 22.04
  and 24.04 installs without Rust, a compiler, or a developer environment.

## Building on macOS 27

A clean build on the local macOS 27 host exposed
[Rust issue 157750](https://github.com/rust-lang/rust/issues/157750): implicit
stripping can produce proc-macro dylibs rejected by dyld as a misaligned LINKEDIT
string pool. Cargo profiles explicitly disable stripping for dependency/build-tool
artifacts while retaining `debug = 0` for dependencies. A focused bytemuck derive
build confirmed that removing the stripping step fixes the observed load failure.
This is a compiler-host workaround, not a change to the macOS 26 application floor.
