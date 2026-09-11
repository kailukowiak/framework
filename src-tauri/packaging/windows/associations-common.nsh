; Shared by the release and dev-bundle installer hooks, which differ only in
; the ProgIds they own. See associations.nsh for why either exists.

!macro FRAMEWORK_QUOTE_COMMAND PROGID
  ; Rewrite the open command and icon with the executable path quoted.
  ;
  ; The bundler emits `$INSTDIR\app.exe "%1"` with $INSTDIR bare. Windows still
  ; *launches* that when the path contains a space, because CreateProcess
  ; retries each space-delimited prefix -- but the shell's handler enumerator
  ; does not retry. It reads the executable as everything up to the first
  ; space, finds no file there, and drops the handler, so the app never appears
  ; under Open With at all. "FrameWork Dev" has a space and "FrameWork" does
  ; not, which is the whole reason a release install looks fine and a dev
  ; install looks broken.
  WriteRegStr SHCTX "Software\Classes\${PROGID}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  WriteRegStr SHCTX "Software\Classes\${PROGID}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
!macroend

!macro FRAMEWORK_MAKE_ALTERNATE EXT PROGID
  Push $0
  ReadRegStr $0 SHCTX "Software\Classes\.${EXT}" "${PROGID}_backup"
  StrCmp $0 "" 0 framework_restore_${EXT}
    ; Nothing held the default before. Remove the value rather than blanking
    ; it: an empty default and no default are different to the shell, and the
    ; second is what was there.
    DeleteRegValue SHCTX "Software\Classes\.${EXT}" ""
    Goto framework_offer_${EXT}
  framework_restore_${EXT}:
    WriteRegStr SHCTX "Software\Classes\.${EXT}" "" "$0"
  framework_offer_${EXT}:
  ; What puts FrameWork in the Open With menu once it is not the default. The
  ; shell reads the value's name; the value itself carries nothing.
  WriteRegStr SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}" ""
  Pop $0
!macroend

!macro FRAMEWORK_FORGET_ALTERNATE EXT PROGID
  Push $0
  DeleteRegValue SHCTX "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.${EXT}\OpenWithProgids"
  ; APP_UNASSOCIATE writes the backup back unconditionally, which leaves an
  ; empty default behind where there had been none. Take that out too, so an
  ; uninstall leaves the extension exactly as the install found it.
  ReadRegStr $0 SHCTX "Software\Classes\.${EXT}" ""
  StrCmp $0 "" 0 framework_keep_${EXT}
    DeleteRegValue SHCTX "Software\Classes\.${EXT}" ""
  framework_keep_${EXT}:
  Pop $0
!macroend
