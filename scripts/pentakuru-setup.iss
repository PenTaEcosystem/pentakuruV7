; Build with Inno Setup 6.7.3+; no account credentials are embedded.
#define Root AddBackslash(SourcePath) + ".."
#include Root + "\dist\prerequisites\vc-runtime-version.iss"
#ifdef PentaKuruTest
  #ifndef PentaKuruRealRuntimeCheck
    #define PentaKuruRuntimeSimulation
  #endif
  #define InstallerId "{{E85C7981-7822-482D-AD2A-C87F86B657CA}"
  #define InstallerOutput Root + "\.build\installer-test"
  #ifdef PentaKuruRealRuntimeCheck
    #define InstallerName "PentaKuru-Setup-Test-RealRuntime"
  #else
    #define InstallerName "PentaKuru-Setup-Test"
  #endif
#else
  #define InstallerId "{{4C845C79-7719-495E-A2E2-67DD1A9A8E49}"
  #define InstallerOutput Root + "\dist"
  #define InstallerName "PentaKuru-Setup"
#endif
[Setup]
AppId={#InstallerId}
AppName=PentaKuru
AppVersion=2026.10.14
AppPublisher=Pentasys
DefaultDirName={localappdata}\Programs\PentaKuru
DefaultGroupName=PentaKuru
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern dark includetitlebar
WizardSizePercent=115
DisableWelcomePage=no
DisableDirPage=no
DisableProgramGroupPage=yes
UsePreviousAppDir=yes
UsePreviousSetupType=yes
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\PentaKuru.exe
SetupIconFile={#Root}\assets\pentakuru-icon.ico
WizardImageFile={#Root}\assets\pentakuru-wizard.bmp
WizardSmallImageFile={#Root}\assets\pentakuru-wizard-small.bmp
Compression=lzma2
SolidCompression=yes
OutputDir={#InstallerOutput}
OutputBaseFilename={#InstallerName}
LicenseFile={#Root}\LICENSE

[Languages]
Name: "vi"; MessagesFile: "compiler:Default.isl"

[Messages]
ClickNext=Bấm Tiếp tục để tiếp tục hoặc Hủy để thoát bộ cài.
ButtonWizardBrowse=Duyệt...
ButtonNewFolder=Tạo thư mục
ButtonYes=Có
ButtonNo=Không
ExitSetupTitle=Thoát bộ cài
ExitSetupMessage=Cài đặt chưa hoàn tất. Anh có muốn thoát và cài lại sau không?
SetupWindowTitle=Cài đặt PentaKuru
WelcomeLabel1=Chào mừng đến với PentaKuru
WelcomeLabel2=Bộ cài sẽ hướng dẫn anh chọn nơi cài và những tiện ích cần sử dụng.%n%nCó thể thay đổi tiện ích sau trong Settings. Office cài LibreOffice khi được chọn; DeepTutor tải runtime khi bật; Music và Clip cần kết nối Internet.%n%nBấm Tiếp tục để bắt đầu.
ButtonBack=< Quay lại
ButtonNext=Tiếp tục >
ButtonInstall=Cài đặt
ButtonCancel=Hủy
ButtonFinish=Hoàn tất
ButtonBrowse=Duyệt...
WizardLicense=Điều khoản sử dụng
LicenseLabel=Vui lòng đọc giấy phép trước khi tiếp tục.
LicenseAccepted=Tôi đồng ý với điều khoản
LicenseNotAccepted=Tôi không đồng ý
WizardSelectDir=Chọn nơi cài đặt
SelectDirDesc=PentaKuru sẽ được cài vào thư mục nào?
SelectDirLabel3=Bấm Tiếp tục để dùng thư mục này hoặc Duyệt để chọn thư mục khác.
WizardSelectComponents=Chọn tiện ích
SelectComponentsDesc=Chọn những tiện ích anh muốn sử dụng.
SelectComponentsLabel2=Bỏ chọn những tiện ích chưa cần. Office chuẩn bị LibreOffice ngay trong Setup. DeepTutor tự chuẩn bị Python/Node còn thiếu; cấu hình API/model AI trong Settings để bắt đầu học.
WizardSelectTasks=Tùy chọn bổ sung
SelectTasksDesc=Chọn biểu tượng truy cập nhanh.
SelectTasksLabel2=Chọn những tùy chọn anh muốn tạo, sau đó bấm Tiếp tục.
WizardReady=Sẵn sàng cài đặt
ReadyLabel1=Kiểm tra lựa chọn trước khi cài PentaKuru.
ReadyLabel2b=Bấm Cài đặt để tiếp tục hoặc Quay lại để đổi lựa chọn.
ReadyMemoDir=Thư mục cài đặt:
ReadyMemoType=Kiểu cài:
ReadyMemoComponents=Tiện ích:
ReadyMemoTasks=Tùy chọn:
WizardInstalling=Đang cài đặt
InstallingLabel=Đang giải nén PentaKuru và ghi lựa chọn tiện ích. Vui lòng chờ.
FinishedHeadingLabel=Cài đặt PentaKuru hoàn tất
FinishedLabel=PentaKuru đã được cài trên máy. Mở Settings để cấu hình API AI và đăng nhập Music/Clip. Những tiện ích chưa chọn có thể bật sau.

[Types]
Name: "standard"; Description: "Cơ bản — ghi chú, tìm file, chính tả, Sally"
Name: "minimal"; Description: "Tối giản — PentaKuru và AI"
Name: "custom"; Description: "Tự chọn tiện ích"; Flags: iscustom

[Components]
Name: "core"; Description: "PentaKuru và AI (bắt buộc)"; Types: standard minimal custom; Flags: fixed
Name: "notes"; Description: "Ghi chú"; Types: standard
Name: "search"; Description: "Tìm file"; Types: standard
Name: "spell"; Description: "Sửa chính tả"; Types: standard
Name: "sally"; Description: "Sally — phiên dịch"; Types: standard
Name: "office"; Description: "Office — LibreOffice"; Flags: disablenouninstallwarning
Name: "office\writer"; Description: "Writer — tài liệu Word, AI chỉnh sửa"; Flags: disablenouninstallwarning
Name: "office\impress"; Description: "Impress — PowerPoint, AI chỉnh sửa"; Flags: disablenouninstallwarning
Name: "office\calc"; Description: "Calc — mở bảng tính Excel"; Flags: disablenouninstallwarning
Name: "music"; Description: "Penta Music — nhạc, playlist và tìm kiếm"; Flags: disablenouninstallwarning
Name: "clip"; Description: "Penta Clip — video"; Flags: disablenouninstallwarning
Name: "deeptutor"; Description: "DeepTutor HKUDS — học tập với AI (tự cài runtime)"; Flags: disablenouninstallwarning
Name: "uitars"; Description: "UI-TARS — thao tác máy tính bằng AI"; Flags: disablenouninstallwarning

[Tasks]
Name: "desktopicon"; Description: "Tạo biểu tượng trên Desktop"; Flags: unchecked

[Files]
Source: "{#Root}\scripts\install-office.ps1"; Flags: dontcopy
Source: "{#Root}\dist\prerequisites\vc_redist.x64.exe"; Flags: dontcopy
Source: "{#Root}\target\release\pentakuru.exe"; DestDir: "{app}"; DestName: "PentaKuru.exe"; Flags: ignoreversion; Components: core
Source: "{#Root}\LICENSE"; DestDir: "{app}"; Components: core
Source: "{#Root}\LICENSE-PENTEYE-MIT"; DestDir: "{app}"; Components: core
Source: "{#Root}\THIRD-PARTY-NOTICES.md"; DestDir: "{app}"; Components: core
Source: "{#Root}\docs\OFFICE-MEDIA-INTEGRATION.md"; DestDir: "{app}\docs"; Components: core
Source: "{#Root}\docs\INSTALLER.md"; DestDir: "{app}\docs"; Components: core
Source: "{#Root}\docs\DEEPTUTOR-ONECLICK.md"; DestDir: "{app}\docs"; Components: core
Source: "{#Root}\dist\PentaKuru-Watch-1.2.3.zab"; DestDir: "{app}\watch"; Components: core; Flags: skipifsourcedoesntexist

[InstallDelete]
Type: files; Name: "{app}\PentaEye.exe"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Penteye"; ValueData: """{app}\PentaKuru.exe"""; Flags: uninsdeletevalue; Check: LegacyAutostartEnabled

[Icons]
Name: "{group}\PentaKuru"; Filename: "{app}\PentaKuru.exe"; WorkingDir: "{app}"
Name: "{group}\Gỡ PentaKuru"; Filename: "{uninstallexe}"
Name: "{autodesktop}\PentaKuru"; Filename: "{app}\PentaKuru.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\PentaKuru.exe"; Description: "Mở PentaKuru ngay"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent; Check: VcRuntimeCanLaunch

[UninstallDelete]
Type: files; Name: "{app}\installation-options.json"

[Code]
#include "vc-runtime-check.iss"
#include "office-runtime-check.iss"
function LegacyAutostartEnabled: Boolean;
begin
#ifdef PentaKuruTest
  Result := False;
#else
  Result := RegValueExists(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Penteye');
#endif
end;

function SelectedBool(Name: String): String;
begin
  if WizardIsComponentSelected(Name) then Result := 'true' else Result := 'false';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Plan, PlanPath: String;
begin
  if CurStep = ssPostInstall then begin
    Log('VC_RUNTIME_LAUNCH_READY=' + IntToStr(Ord(VcRuntimeCanLaunch)));
    Plan := '{"schema":1,"installation_id":"' + GetDateTimeString('yyyymmddhhnnsszzz', '-', ':') + '",';
    Plan := Plan + '"notes":' + SelectedBool('notes') + ',"search":' + SelectedBool('search');
    Plan := Plan + ',"spell":' + SelectedBool('spell') + ',"sally":' + SelectedBool('sally');
    Plan := Plan + ',"ui_tars":' + SelectedBool('uitars') + ',"writer":' + SelectedBool('office\writer');
    Plan := Plan + ',"impress":' + SelectedBool('office\impress') + ',"calc":' + SelectedBool('office\calc');
    Plan := Plan + ',"music":' + SelectedBool('music') + ',"clip":' + SelectedBool('clip');
    Plan := Plan + ',"deeptutor":' + SelectedBool('deeptutor') + '}';
    PlanPath := ExpandConstant('{app}\installation-options.json');
    if not SaveStringToFile(PlanPath, Plan, False) then
      RaiseException('Không ghi được lựa chọn tiện ích. Vui lòng cài lại vào thư mục có quyền ghi.');
  end;
end;
