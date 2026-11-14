#ifndef AppVersion
  #define AppVersion "1.2.0"
#endif
#define Root ExtractFileDir(RemoveBackslash(SourcePath))
#ifndef StageDir
  #define StageDir Root + "\dist\stage\" + AppVersion
#endif
#ifndef VendorDir
  #define VendorDir Root + "\.tools\vendor\VBCABLE_Driver_Pack46"
#endif

[Setup]
AppId={{9D5E047B-1ECA-4AE0-A50F-D7F19D11D159}
AppName=VESPER
AppVersion={#AppVersion}
AppVerName=VESPER {#AppVersion}
AppPublisher=VESPER contributors
DefaultDirName={autopf}\VESPER
DefaultGroupName=VESPER
DisableProgramGroupPage=yes
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#Root}\dist\release\{#AppVersion}
OutputBaseFilename=Vesper-{#AppVersion}-Setup-x64
SetupIconFile={#Root}\assets\vesper.ico
UninstallDisplayIcon={app}\Vesper.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern dark
WizardSizePercent=110
LicenseFile={#Root}\LICENSE
InfoBeforeFile={#Root}\installer\before-install.txt
CloseApplications=yes
RestartApplications=no
DisableWelcomePage=no
SetupLogging=yes
UninstallDisplayName=VESPER
VersionInfoVersion={#AppVersion}.0
VersionInfoDescription=VESPER Voice Studio Setup
VersionInfoCopyright=2026 VESPER contributors

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "vbcable"; Description: "{cm:CableTask}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#VendorDir}\*"; DestDir: "{tmp}\VesperCable"; Flags: dontcopy recursesubdirs createallsubdirs
Source: "{#StageDir}\Vesper.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Root}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#VendorDir}\*"; DestDir: "{app}\VirtualCable"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\VESPER"; Filename: "{app}\Vesper.exe"
Name: "{autodesktop}\VESPER"; Filename: "{app}\Vesper.exe"; Tasks: desktopicon
Name: "{group}\VB-CABLE — support VB-Audio"; Filename: "https://vb-audio.com/Services/licensing.htm"

[Run]
Filename: "{app}\Vesper.exe"; Description: "{cm:LaunchProgram,VESPER}"; Flags: nowait postinstall skipifsilent runasoriginaluser; Check: CanLaunch

[CustomMessages]
russian.CableTask=VB-CABLE: автоматически установить виртуальный микрофон, если его ещё нет
english.CableTask=VB-CABLE: automatically install the virtual microphone if missing
russian.CableStatus=Устанавливаем VB-CABLE — виртуальный микрофон от VB-Audio…
english.CableStatus=Installing VB-CABLE, the virtual microphone by VB-Audio…
russian.CableAdmin=Для установки VB-CABLE нужны права администратора. Запусти Setup обычным способом, без /CURRENTUSER, или отключи опцию VB-CABLE.
english.CableAdmin=VB-CABLE needs administrator permission. Run Setup without /CURRENTUSER, or deselect the VB-CABLE task.
russian.CableError=VB-CABLE не удалось установить автоматически. Код: %1. Установка VESPER не начата. Перезагрузи Windows и повтори Setup либо установи драйвер с vb-cable.com. Подробности в журнале установки.
english.CableError=VB-CABLE could not be installed automatically. Code: %1. VESPER installation has not started. Restart Windows and retry, or install the driver from vb-cable.com. See the setup log.

[Code]
var
  DriverAdded: Boolean;
  DriverReady: Boolean;

function CableInstalled(): Boolean;
var
  Locator, Service, Devices, Device: Variant;
begin
  Result := False;
  try
    Locator := CreateOleObject('WbemScripting.SWbemLocator');
    Service := Locator.ConnectServer('', 'root\CIMV2');
    Devices := Service.ExecQuery('SELECT Name, ConfigManagerErrorCode FROM Win32_PnPEntity WHERE Name = ''VB-Audio Virtual Cable''');
    if Devices.Count > 0 then begin
      Device := Devices.ItemIndex(0);
      Result := Device.ConfigManagerErrorCode = 0;
    end;
  except
    Log('VB-CABLE detection failed: ' + GetExceptionMessage);
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
  Started: Boolean;
begin
  Result := '';
  if DriverReady or not WizardIsTaskSelected('vbcable') then Exit;
  if CableInstalled() then begin
    Log('Existing VB-CABLE detected; driver install skipped.');
    DriverReady := True;
    Exit;
  end;
  if not IsAdminInstallMode then begin
    Result := CustomMessage('CableAdmin');
    Exit;
  end;
  ExtractTemporaryFiles('{tmp}\VesperCable\*');
  WizardForm.PreparingLabel.Caption := CustomMessage('CableStatus');
  Log('Installing original VB-Audio driver package with -i -h.');
  Started := Exec(ExpandConstant('{tmp}\VesperCable\VBCABLE_Setup_x64.exe'),
    '-i -h', ExpandConstant('{tmp}\VesperCable'), SW_HIDE, ewWaitUntilTerminated, Code);
  Log(Format('VB-CABLE installer started=%d exit=%d', [Ord(Started), Code]));
  if not Started then begin
    Result := FmtMessage(CustomMessage('CableError'), [IntToStr(Code)]);
    Exit;
  end;
  { A working existing driver is accepted even if the vendor reports it as an
    already-installed condition. A new successful installation needs a reboot. }
  if (Code <> 0) and not CableInstalled() then begin
    Result := FmtMessage(CustomMessage('CableError'), [IntToStr(Code)]);
    Exit;
  end;
  DriverAdded := True;
  DriverReady := True;
end;

function NeedRestart(): Boolean;
begin
  Result := DriverAdded;
end;

function CanLaunch(): Boolean;
begin
  Result := not DriverAdded and not CheckForMutexes('Local\VesperVoiceStudio-1.1');
end;
