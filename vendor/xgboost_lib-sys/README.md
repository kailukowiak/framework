# xgboost_lib-sys 3.0.5, patched

This is `xgboost_lib-sys` 3.0.5 as published on crates.io (MIT,
<https://crates.io/crates/xgboost_lib-sys/3.0.5>), applied through
`[patch.crates-io]` in the workspace manifest. Two things differ from the
published crate:

- `bindgen` is `0.72` rather than `0.71`. With libclang 20 or newer, bindgen
  0.71 generates every C struct without its fields while still emitting the
  layout assertion for the real size, so `size_of::<_IO_FILE>() - 216` fails
  to compile on such a host. bindgen 0.72 visits the fields correctly. The
  XGBoost C API itself only passes handles, scalars and strings, so the
  generated bindings FrameWork calls are the same either way.
- Only `xgboost/include/xgboost/c_api.h` is kept from the header tree, because
  it is the one file `build.rs` hands to bindgen. The `dmlc-core` include path
  the script still passes does not exist here and clang ignores it.

`build.rs` and `src/lib.rs` are byte-for-byte the published files, so the
prebuilt-library download, its checksum pins, the `XGBOOST_LIB_DIR` override
and the link directives are unchanged on every platform. Remove this
directory and the patch entry once a published release moves to bindgen 0.72
or later.
