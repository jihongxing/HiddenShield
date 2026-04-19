; HiddenShield NSIS Uninstall Hooks
; Cleans up application data on uninstall

!macro NSIS_HOOK_UNINSTCONFIRM
  ; Ask user if they want to remove vault data
  MessageBox MB_YESNO|MB_ICONQUESTION \
    "是否同时删除版权库数据？（删除后不可恢复）$\n$\n选择「否」将仅清理 FFmpeg 缓存和日志文件。" \
    IDYES removeAll IDNO removeCache

  removeAll:
    ; Remove entire app data directory
    RMDir /r "$APPDATA\com.hiddenshield.desktop"
    Goto done

  removeCache:
    ; Remove only FFmpeg binaries and logs, preserve vault.db
    Delete "$APPDATA\com.hiddenshield.desktop\ffmpeg.exe"
    Delete "$APPDATA\com.hiddenshield.desktop\ffprobe.exe"
    Delete "$APPDATA\com.hiddenshield.desktop\telemetry_config.json"
    RMDir /r "$APPDATA\com.hiddenshield.desktop\logs"

  done:
!macroend
