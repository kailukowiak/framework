# Actual XGBoost packaging spike

Status: **local fit and relocation proved; release integration implemented,
with clean-machine verification remaining**.
Measured 2026-09-11 on macOS 27.0 / arm64, Rust 1.97.1. This is evidence for
the bounded spike in `MLConsensus.md`, not a production dependency decision.
The spike itself changed no application or workspace manifests. Production
integration is now described in [ml-xgboost-packaging.md](ml-xgboost-packaging.md).

## What ran

The isolated [harness](../tools/ml-spikes/xgboost/) pins `xgb = 3.0.6` and
commits its own lockfile, resolving `xgboost_lib-sys = 3.0.5`. Its prebuilt
native library actually writes XGBoost **3.0.0** model JSON. Crate, artifact
tag, and engine versions must be recorded separately in model provenance.

The release executable loaded the real native library, fitted a binary
classifier (128 rows, two features, 20 histogram-tree rounds, one thread,
seed 42), asserted finite probabilities and separation of the deliberately
easy training data, wrote `binary.json`, reloaded it, and asserted exactly
equal predictions. It also predicted a row with a missing numerical feature.
This is a packaging smoke test, not held-out model evaluation or evidence
for general training performance.

| Local measurement | Result |
| --- | --- |
| First release build, dependency downloads included | Cargo reported 17.20 seconds |
| Matrix construction + fit in one measured run | 1.255 ms |
| First four probabilities | 0.014070604, 0.9859294, 0.014070604, 0.9859294 |
| JSON prediction round trip | Exact equality |
| Missing numerical input | Finite probability |
| JSON model | 10,496 bytes |
| Stripped spike executable | 429,120 bytes |
| Linked `libxgboost.dylib` | 6,589,184 bytes |
| Local `libomp.dylib` | 721,120 bytes |
| XGBoost dylib minimum macOS, from `LC_BUILD_VERSION` | 15.0 |

These are individual observations, not benchmark distributions or projected
application size increases. The native dylibs together are about 6.97 MiB
uncompressed; the application already has its own dependencies.

An initial harness using an uncached raw Booster failed with the native
"0 feature is supplied" error. Creating it with the training DMatrix cache
fixed the harness. The checked-in harness contains that fix.

## Packaging finding and relocation result

The default build is **not distributable by copying the executable**. `otool`
showed these direct executable dependencies:

```text
.../tools/ml-spikes/xgboost/target/release/deps/libxgboost.dylib
/opt/homebrew/opt/libomp/lib/libomp.dylib
```

The published sys crate verifies the downloaded XGBoost bytes, then rewrites
the dylib install name to the absolute Cargo `deps` path and signs it ad hoc.
That solves developer loadability, not deployment. The XGBoost library itself
also references `@rpath/libomp.dylib`.

The [relocation probe](../tools/ml-spikes/xgboost/relocate-macos.sh) copies the
executable and both dylibs into its ignored output directory, rewrites their
load commands to executable/loader-relative paths, and signs the **copies** ad
hoc. The relocated executable passed the same assertions. `DYLD_PRINT_LIBRARIES`
confirmed both native libraries loaded from that relocated directory, rather
than Homebrew or Cargo. Original installed libraries were not modified.

This proves a relocatable CLI layout on this machine. It does not prove a
Tauri `.app`/DMG, hardened runtime, distribution signing/notarization, or a
clean machine without developer tooling. Homebrew's OpenMP copy is an
experimental input, not yet a pinned redistributable production artifact.

## Supported targets: evidence versus work remaining

FrameWork's current release workflow targets Apple Silicon macOS, Windows
x64, and Ubuntu 22.04 x64. Intel Mac is currently excluded explicitly, so the
binding's absence of a prebuilt Intel Mac binary is not a present release
blocker.

| Target | Verified here | Required release evidence |
| --- | --- | --- |
| macOS arm64 | Build, actual fit/predict, JSON reload, relocated dylib loading | Choose supported OS floor; obtain/rebuild native artifacts for it; pin OpenMP; actual Tauri bundle and clean-machine signed launch |
| Windows x64 | Published sys source lists DLL and import-library hashes; not executed | Build/run on Windows; DLL dependency inspection and redistributable requirements; MSI/NSIS installation and fit/predict without a developer PATH |
| Linux x64 | Published sys source lists SO hash and links `gomp`, `stdc++`, `stdc++fs`; not executed | Ubuntu 22.04 build/run; inspect ELF/glibc/GLIBCXX requirements; package library/runtime paths; clean .deb/.rpm/AppImage fit/predict |

The published sys build script also lists Linux arm64. Its platform selection
uses build-host `cfg!`, not target architecture, so cross-compilation must not
be assumed correct. Native matching runners avoid that specific mismatch.

The XGBoost dylib's measured macOS 15.0 minimum deserves an explicit decision:
either that matches the intended app floor or we need a compatible native
build. Successful execution on macOS 27 says nothing about older systems.

## Pins, sources, and reproduction

```sh
cargo run --release --locked --manifest-path tools/ml-spikes/xgboost/Cargo.toml -- tools/ml-spikes/xgboost/output
sh tools/ml-spikes/xgboost/relocate-macos.sh
cargo clippy --locked --offline --manifest-path tools/ml-spikes/xgboost/Cargo.toml --all-targets -- -D warnings
```

The first build needs network access for crates and native artifacts. This
session used sandbox escalation after the sandbox could not resolve crates.io.
Subsequent local runs worked offline. Mac builds need libomp and the normal
Rust/Clang toolchain; the sys crate generates bindings even with prebuilt
XGBoost. The relocation script is macOS-only and defaults to Homebrew's
standard arm64 OpenMP path; `FRAMEWORK_SPIKE_LIBOMP` can select another copy.
The isolated all-target Clippy command above passed with warnings denied;
`cargo fmt --check` and shell syntax validation passed too.

The native download is pinned to artifact tag `v3.0.5`. Its original arm64
XGBoost SHA-256 was checked locally:

```text
e438dacf4a1ec44e4f5f1e5005f291f7b66156af7fc49e543b60a5fdc079c024
```

Mach-O patching changes the deployed hash, as expected. Keep original artifact
hash and deployed artifact provenance distinct. The `XGBOOST_LIB_DIR` override
bypasses the sys crate's download verification path, so a production artifact
pipeline using it needs its own checksum check.

Primary references: [published xgb 3.0.6](https://docs.rs/crate/xgb/3.0.6),
[published sys build source](https://docs.rs/crate/xgboost_lib-sys/3.0.5/source/build.rs),
[XGBoost installation](https://xgboost.readthedocs.io/en/release_3.0.0/install.html),
and [model IO](https://xgboost.readthedocs.io/en/release_3.0.0/tutorials/saving_model.html).
Dependency claims above were checked against the downloaded published crate
source, not inferred from the similarly named repository tag. In this survey
the repository tag's web view and published crate build source differed;
production reproducibility should use Cargo's locked source checksum.

## Decision and next tests

There is now positive evidence that real XGBoost is practical to call and
relocate locally. There is also a concrete packaging bill, rather than an
assumed C++ build requirement on every machine. **Do not defer in-app XGBoost
based on this result alone, and do not declare it release-ready.**

1. Run this same locked harness on the two missing native release targets and
   inventory loader dependencies, artifact size, and OS/runtime minimums.
2. Produce pinned redistributable XGBoost/OpenMP artifacts with appropriate
   license notices; prove signed Tauri installation on clean supported hosts.
3. Extend model correctness coverage to regression and multiclass, class order,
   feature mismatch/errors, early stopping with preserved iteration policy,
   and JSON parity with the supported Python export versions. This toy binary
   test does not establish those contracts or sklearn pipeline support.
4. Measure realistic training memory/time, cancellation and background-job
   behavior. Choose thread limits and a concurrency model before app integration.

Perpetual remains an independent estimator candidate. This spike makes no
performance or quality comparison and does not use it as an XGBoost substitute.
