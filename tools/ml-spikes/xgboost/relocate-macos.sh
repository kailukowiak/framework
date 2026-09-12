#!/bin/sh
# A CLI bundle probe, not a signed/notarized Tauri distribution. Copies only
# spike artifacts; never rewrites installed Homebrew libraries or app binaries.
set -eu
spike_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
release_dir="$spike_dir/target/release"
bundle_dir="$spike_dir/output/relocated"
omp_source="${FRAMEWORK_SPIKE_LIBOMP:-/opt/homebrew/opt/libomp/lib/libomp.dylib}"
mkdir -p "$bundle_dir/bin" "$bundle_dir/lib"
cp "$release_dir/framework-xgboost-spike" "$bundle_dir/bin/"
cp "$release_dir/deps/libxgboost.dylib" "$bundle_dir/lib/"
cp "$omp_source" "$bundle_dir/lib/libomp.dylib"
chmod u+w "$bundle_dir/lib/libomp.dylib"
install_name_tool -change "$release_dir/deps/libxgboost.dylib" \
  '@executable_path/../lib/libxgboost.dylib' "$bundle_dir/bin/framework-xgboost-spike"
install_name_tool -change "$omp_source" \
  '@executable_path/../lib/libomp.dylib' "$bundle_dir/bin/framework-xgboost-spike"
install_name_tool -id '@loader_path/libxgboost.dylib' \
  -change '@rpath/libomp.dylib' '@loader_path/libomp.dylib' \
  "$bundle_dir/lib/libxgboost.dylib"
install_name_tool -id '@loader_path/libomp.dylib' "$bundle_dir/lib/libomp.dylib"
for library in "$bundle_dir/lib/"*.dylib; do
  codesign --force --sign - "$library"
done
codesign --force --sign - "$bundle_dir/bin/framework-xgboost-spike"
otool -L "$bundle_dir/bin/framework-xgboost-spike" "$bundle_dir/lib/"*.dylib
DYLD_PRINT_LIBRARIES=1 "$bundle_dir/bin/framework-xgboost-spike" "$bundle_dir/results"
