#ifndef AppVersion
  #define AppVersion "1.3.0"
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
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"; InfoBeforeFile: "{#Root}\installer\before-install.ru.txt"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "portuguese"; MessagesFile: "compiler:Languages\Portuguese.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "polish"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "ukrainian"; MessagesFile: "compiler:Languages\Ukrainian.isl"

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

german.CableTask=VB-CABLE automatisch installieren, falls es fehlt
german.CableStatus=VB-CABLE wird installiert…
german.CableAdmin=VB-CABLE benötigt Administratorrechte. Setup ohne /CURRENTUSER starten oder die VB-CABLE-Option abwählen.
german.CableError=VB-CABLE konnte nicht installiert werden. Code: %1. VESPER wurde nicht installiert. Windows neu starten und erneut versuchen oder den Treiber von vb-cable.com installieren.
french.CableTask=Installer automatiquement VB-CABLE si nécessaire
french.CableStatus=Installation de VB-CABLE…
french.CableAdmin=VB-CABLE nécessite les droits administrateur. Lancez Setup sans /CURRENTUSER ou désactivez l'option VB-CABLE.
french.CableError=Échec de l'installation de VB-CABLE. Code : %1. VESPER n'a pas été installé. Redémarrez Windows puis réessayez, ou installez le pilote depuis vb-cable.com.
spanish.CableTask=Instalar VB-CABLE automáticamente si falta
spanish.CableStatus=Instalando VB-CABLE…
spanish.CableAdmin=VB-CABLE necesita permisos de administrador. Ejecuta Setup sin /CURRENTUSER o desmarca la opción VB-CABLE.
spanish.CableError=No se pudo instalar VB-CABLE. Código: %1. VESPER no se ha instalado. Reinicia Windows y vuelve a intentarlo o instala el controlador desde vb-cable.com.
portuguese.CableTask=Instalar VB-CABLE automaticamente se necessário
portuguese.CableStatus=A instalar VB-CABLE…
portuguese.CableAdmin=VB-CABLE requer permissões de administrador. Execute Setup sem /CURRENTUSER ou desmarque a opção VB-CABLE.
portuguese.CableError=Não foi possível instalar VB-CABLE. Código: %1. VESPER não foi instalado. Reinicie o Windows e tente novamente ou instale o controlador de vb-cable.com.
italian.CableTask=Installa automaticamente VB-CABLE se manca
italian.CableStatus=Installazione di VB-CABLE…
italian.CableAdmin=VB-CABLE richiede permessi di amministratore. Avvia Setup senza /CURRENTUSER o deseleziona VB-CABLE.
italian.CableError=Impossibile installare VB-CABLE. Codice: %1. VESPER non è stato installato. Riavvia Windows e riprova oppure installa il driver da vb-cable.com.
polish.CableTask=Automatycznie zainstaluj VB-CABLE, jeśli go brakuje
polish.CableStatus=Instalowanie VB-CABLE…
polish.CableAdmin=VB-CABLE wymaga uprawnień administratora. Uruchom Setup bez /CURRENTUSER lub odznacz opcję VB-CABLE.
polish.CableError=Nie udało się zainstalować VB-CABLE. Kod: %1. VESPER nie został zainstalowany. Uruchom ponownie Windows i spróbuj ponownie lub zainstaluj sterownik z vb-cable.com.
turkish.CableTask=VB-CABLE yoksa otomatik yükle
turkish.CableStatus=VB-CABLE yükleniyor…
turkish.CableAdmin=VB-CABLE yönetici izni gerektirir. Setup'ı /CURRENTUSER olmadan çalıştırın veya VB-CABLE seçeneğini kaldırın.
turkish.CableError=VB-CABLE yüklenemedi. Kod: %1. VESPER yüklenmedi. Windows'u yeniden başlatıp tekrar deneyin veya sürücüyü vb-cable.com'dan yükleyin.
ukrainian.CableTask=Автоматично встановити VB-CABLE, якщо його немає
ukrainian.CableStatus=Встановлення VB-CABLE…
ukrainian.CableAdmin=Для VB-CABLE потрібні права адміністратора. Запусти Setup без /CURRENTUSER або вимкни опцію VB-CABLE.
ukrainian.CableError=Не вдалося встановити VB-CABLE. Код: %1. VESPER не встановлено. Перезавантаж Windows і повтори спробу або встанови драйвер із vb-cable.com.

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
