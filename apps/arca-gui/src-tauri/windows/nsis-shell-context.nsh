!macro ARCA_REGISTER_FILE_ASSOCIATION EXT DESCRIPTION
  WriteRegStr HKCU "Software\Classes\.${EXT}" "" "Arca.${EXT}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\OpenWithProgids" "Arca.${EXT}" ""
  WriteRegStr HKCU "Software\Classes\Arca.${EXT}" "" "Arca ${DESCRIPTION}"
  WriteRegStr HKCU "Software\Classes\Arca.${EXT}\DefaultIcon" "" "$INSTDIR\arca-gui.exe,0"
  WriteRegStr HKCU "Software\Classes\Arca.${EXT}\shell\open\command" "" "$\"$INSTDIR\arca-gui.exe$\" --arca-shell-open $\"%1$\""
!macroend

!macro ARCA_REGISTER_ARCHIVE_SHELL EXT
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaOpen" "" "Open in Arca"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaOpen" "Icon" "$INSTDIR\arca-gui.exe,0"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaOpen\command" "" "$\"$INSTDIR\arca-gui.exe$\" --arca-shell-open $\"%1$\""

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaExtract" "" "Extract with Arca"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaExtract" "Icon" "$INSTDIR\arca-gui.exe,0"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaExtract\command" "" "$\"$INSTDIR\arca-gui.exe$\" --arca-shell-extract $\"%1$\""

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaTest" "" "Test with Arca"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaTest" "Icon" "$INSTDIR\arca-gui.exe,0"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaTest\command" "" "$\"$INSTDIR\arca-gui.exe$\" --arca-shell-test $\"%1$\""
!macroend

!macro ARCA_UNREGISTER_FILE_ASSOCIATION EXT
  ReadRegStr $0 HKCU "Software\Classes\.${EXT}" ""
  StrCmp $0 "Arca.${EXT}" 0 +2
  DeleteRegValue HKCU "Software\Classes\.${EXT}" ""
  DeleteRegValue HKCU "Software\Classes\.${EXT}\OpenWithProgids" "Arca.${EXT}"
  DeleteRegKey HKCU "Software\Classes\Arca.${EXT}"
!macroend

!macro ARCA_UNREGISTER_ARCHIVE_SHELL EXT
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaOpen"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaExtract"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.${EXT}\shell\ArcaTest"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "zip" "ZIP Archive"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "tar" "Tar Archive"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "tgz" "Tar Gzip Archive"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "tbz2" "Tar Bzip2 Archive"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "txz" "Tar XZ Archive"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "gz" "Gzip Stream"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "bz2" "Bzip2 Stream"
  !insertmacro ARCA_REGISTER_FILE_ASSOCIATION "xz" "XZ Stream"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "zip"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "tar"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "tgz"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "tbz2"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "txz"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "gz"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "bz2"
  !insertmacro ARCA_REGISTER_ARCHIVE_SHELL "xz"
  System::Call 'shell32::SHChangeNotify(l, l, p, p) v (0x08000000, 0, 0, 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "zip"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "tar"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "tgz"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "tbz2"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "txz"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "gz"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "bz2"
  !insertmacro ARCA_UNREGISTER_FILE_ASSOCIATION "xz"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "zip"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "tar"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "tgz"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "tbz2"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "txz"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "gz"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "bz2"
  !insertmacro ARCA_UNREGISTER_ARCHIVE_SHELL "xz"
  System::Call 'shell32::SHChangeNotify(l, l, p, p) v (0x08000000, 0, 0, 0)'
!macroend
