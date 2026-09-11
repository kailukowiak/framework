; The dev bundle's hook. Same job as associations.nsh, over the ProgIds in
; tauri.dev-bundle.windows.conf.json rather than the release's.
;
; .fw is demoted here where the release keeps it. A debug build is not what
; should open a document on a double-click; the installed release is, and
; restoring the default is what leaves it that way.

!include "${__FILEDIR__}\associations-common.nsh"

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Dev.CSV"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Dev.TSV"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Dev.Parquet"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Dev.NDJSON"
  !insertmacro FRAMEWORK_QUOTE_COMMAND "FrameWork.Dev.Document"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "csv" "FrameWork.Dev.CSV"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "tsv" "FrameWork.Dev.TSV"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "parquet" "FrameWork.Dev.Parquet"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "ndjson" "FrameWork.Dev.NDJSON"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "jsonl" "FrameWork.Dev.NDJSON"
  !insertmacro FRAMEWORK_MAKE_ALTERNATE "fw" "FrameWork.Dev.Document"
  ; Tell the shell the associations changed. FileAssociation.nsh defines
  ; this macro for exactly that and the tauri template never calls it, so
  ; without it Explorer keeps serving its cached Open With list and the
  ; entry does not appear until something else invalidates the cache.
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "csv" "FrameWork.Dev.CSV"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "tsv" "FrameWork.Dev.TSV"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "parquet" "FrameWork.Dev.Parquet"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "ndjson" "FrameWork.Dev.NDJSON"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "jsonl" "FrameWork.Dev.NDJSON"
  !insertmacro FRAMEWORK_FORGET_ALTERNATE "fw" "FrameWork.Dev.Document"
  ; Tell the shell the associations changed. FileAssociation.nsh defines
  ; this macro for exactly that and the tauri template never calls it, so
  ; without it Explorer keeps serving its cached Open With list and the
  ; entry does not appear until something else invalidates the cache.
  !insertmacro UPDATEFILEASSOC
!macroend
