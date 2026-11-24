; Circord's Windows installer. scripts/package-windows.py compiles it with
; Inno Setup 7 under Wine, from the same files as the portable ZIP.
;
; It installs for the person running it, without administrator rights, and can
; start Circord in the tray when they sign in. That is the Run key value
; Settings > Advanced > "Start when you sign in" writes (src/autostart.rs), so
; the installer and the setting turn the same thing on and off.

#ifndef AppVersion
  #error Compile through scripts/package-windows.py, which defines AppVersion, VersionNumber and SourceDir
#endif
; The full preview defines Edition and FileSuffix; the Discord release neither.
; Both share the AppId, so installing one replaces the other.
#ifdef Edition
  #define DisplayName "Circord (" + Edition + ")"
#else
  #define DisplayName "Circord"
  #define FileSuffix ""
#endif

[Setup]
; Windows knows an installed Circord by this; never change it.
AppId={{19FF8D53-8D7E-4E66-91BB-E2F5BE307E9A}
AppName=Circord
AppVersion={#AppVersion}
AppVerName={#DisplayName} {#AppVersion}
AppPublisher=Circord
AppPublisherURL=https://circord.com
AppSupportURL=https://github.com/circord/circord/issues
AppUpdatesURL=https://github.com/circord/circord/releases
VersionInfoVersion={#VersionNumber}
; Per user: %LOCALAPPDATA%\Programs\Circord, with no UAC prompt.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Circord
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=Circord-{#AppVersion}{#FileSuffix}-windows-x64-setup
SetupIconFile=circord.ico
UninstallDisplayIcon={app}\circord.exe
UninstallDisplayName={#DisplayName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; Circord keeps running in the tray with its window closed, so an update or
; uninstall closes it rather than asking.
CloseApplications=force
RestartApplications=no
#ifdef LicenseFile
LicenseFile={#LicenseFile}
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.StartOnSignIn=Start Circord automatically when Windows starts
english.StartOnSignInGroup=Startup:

[Tasks]
Name: "startonsignin"; Description: "{cm:StartOnSignIn}"; GroupDescription: "{cm:StartOnSignInGroup}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; README.txt explains the ZIP; the installer needs no instructions.
Source: "{#SourceDir}\*"; Excludes: "README.txt"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Circord"; Filename: "{app}\circord.exe"
Name: "{autodesktop}\Circord"; Filename: "{app}\circord.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Circord"; ValueData: """{app}\circord.exe"" --background"; Tasks: startonsignin
; Chosen here, it starts even if Task Manager had turned it off before.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Circord"; Flags: deletevalue dontcreatekey; Tasks: startonsignin
; Circord's own links open here; claiming Discord's stays the person's choice.
Root: HKCU; Subkey: "Software\Classes\circord"; ValueType: string; ValueName: ""; ValueData: "URL:Circord Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\circord"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\circord\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\circord.exe"",0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\circord\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\circord.exe"" --open-link ""%1"""; Flags: uninsdeletekey
; Settings keeps the app discord:// links opened before claiming them here;
; uninstalling gives them back (below) and drops what was kept.
Root: HKCU; Subkey: "Software\Circord"; Flags: uninsdeletekeyifempty dontcreatekey
Root: HKCU; Subkey: "Software\Circord\DiscordProtocol"; Flags: uninsdeletekey dontcreatekey
; Each start registers Circord's name and icon for its notifications
; (src/notify.rs); uninstalling forgets them.
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\io.github.circord.Circord"; Flags: uninsdeletekey dontcreatekey
; Uninstalling stops starting at sign-in, however it was turned on.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Circord"; Flags: uninsdeletevalue dontcreatekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "Circord"; Flags: uninsdeletevalue dontcreatekey

[Run]
Filename: "{app}\circord.exe"; Description: "{cm:LaunchProgram,Circord}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Before the files go: discord:// links open in the app that had them again,
; if Settings claimed them for this Circord.
Filename: "{app}\circord.exe"; Parameters: "--release-discord-links"; Flags: runhidden waituntilterminated; RunOnceId: "ReleaseDiscordLinks"
