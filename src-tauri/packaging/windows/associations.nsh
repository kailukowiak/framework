; Makes FrameWork an alternate handler for the data formats it can open,
; rather than the handler a double-click reaches.
;
; `rank: "Alternate"` in the file associations says installing FrameWork should
; add a way to open a CSV, not change where CSVs already go. macOS honours that
; rank. The NSIS bundler has no notion of it: APP_ASSOCIATE writes the
; extension's default ProgId and nothing else, so without this hook installing
; FrameWork silently takes .csv from whatever the person already uses -- and
; takes it again on every update, undoing their correction each time.
;
; APP_ASSOCIATE saves the previous default as `<ProgId>_backup` for its own
; uninstaller. That is exactly the value to put back, so this never has to
; guess what the extension pointed at before installing.
;
; .fw is deliberately absent. Its rank is "Owner" and it is our own format: a
; FrameWork document should open in FrameWork.

!include "${__FILEDIR__}\associations-common.nsh"

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.CSV"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.TSV"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Parquet"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.NDJSON"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Document"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "csv" "FrameWork.CSV"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "tsv" "FrameWork.TSV"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "parquet" "FrameWork.Parquet"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "ndjson" "FrameWork.NDJSON"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "jsonl" "FrameWork.NDJSON"
  ; Tell the shell the associations changed. FileAssociation.nsh defines
  ; this macro for exactly that and the tauri template never calls it, so
  ; without it Explorer keeps serving its cached Open With list and the
  ; entry does not appear until something else invalidates the cache.
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "csv" "FrameWork.CSV"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "tsv" "FrameWork.TSV"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "parquet" "FrameWork.Parquet"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "ndjson" "FrameWork.NDJSON"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "jsonl" "FrameWork.NDJSON"
  ; Tell the shell the associations changed. FileAssociation.nsh defines
  ; this macro for exactly that and the tauri template never calls it, so
  ; without it Explorer keeps serving its cached Open With list and the
  ; entry does not appear until something else invalidates the cache.
  !insertmacro UPDATEFILEASSOC
!macroend
