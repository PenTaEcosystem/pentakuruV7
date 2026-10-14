var
  VcRuntimeRestart: Boolean;
#ifdef PentaKuruRuntimeSimulation
  VcRuntimeTestReady: Boolean;
#endif

function VcRuntimeVersionFromRegistry(RootKey: Integer; var Version: Int64): Boolean;
var
  Installed: Cardinal;
  Text: String;
begin
  Result := False;
  if RegQueryDWordValue(RootKey, 'Software\Microsoft\VisualStudio\14.0\VC\Runtimes\x64', 'Installed', Installed) and (Installed = 1) and
     RegQueryStringValue(RootKey, 'Software\Microsoft\VisualStudio\14.0\VC\Runtimes\x64', 'Version', Text) then begin
    if (Length(Text) > 0) and ((Text[1] = 'v') or (Text[1] = 'V')) then Delete(Text, 1, 1);
    Result := StrToVersion(Text, Version);
  end;
end;

function VcRuntimeInstalledVersion: Int64;
var
  Version: Int64;
begin
  Result := 0;
  if VcRuntimeVersionFromRegistry(HKLM64, Version) then Result := Version;
  if VcRuntimeVersionFromRegistry(HKLM32, Version) and (ComparePackedVersion(Version, Result) > 0) then Result := Version;
end;

function VcRuntimeRequiredVersion: Int64;
begin
  if not StrToVersion('{#VcRuntimeMinimumVersion}', Result) then RaiseException('Invalid runtime version metadata.');
end;

function VcRuntimeNeedsInstall: Boolean;
begin
#ifdef PentaKuruRuntimeSimulation
  { Only test builds accept simulation; production always examines this machine. }
  Result := not VcRuntimeTestReady and (ExpandConstant('{param:RUNTIMETEST|present}') <> 'present') and
            (ExpandConstant('{param:RUNTIMETEST|present}') <> 'newer');
#else
  Result := (ComparePackedVersion(VcRuntimeInstalledVersion, VcRuntimeRequiredVersion) < 0) or
    not FileExists(ExpandConstant('{sys}\vcruntime140.dll')) or
    not FileExists(ExpandConstant('{sys}\vcruntime140_1.dll')) or
    not FileExists(ExpandConstant('{sys}\msvcp140.dll'));
#endif
end;

function VcRuntimeCanLaunch: Boolean;
begin
  Result := not VcRuntimeRestart and not VcRuntimeNeedsInstall;
#ifdef PentaKuruRuntimeSimulation
  if ExpandConstant('{param:RUNTIMETEST|present}') = 'reboot' then Result := False;
#endif
  Log('VC_RUNTIME_CAN_LAUNCH=' + IntToStr(Ord(Result)));
end;

function PrepareVcRuntime(var NeedsRestart: Boolean): String;
var
  RuntimePath, Parameters, LogDirectory, LogPath: String;
  PayloadVersion: Int64;
  ExitCode: Integer;
  Started, Repair: Boolean;
#ifdef PentaKuruRuntimeSimulation
  TestCase: String;
#endif
begin
  Result := '';
  if not VcRuntimeNeedsInstall then begin
    Log('VC_RUNTIME_SKIP: compatible x64 runtime already present');
    exit;
  end;
#ifdef PentaKuruRuntimeSimulation
  TestCase := ExpandConstant('{param:RUNTIMETEST|present}');
  if TestCase = 'declined' then begin
    Result := 'Đã hủy cài Visual C++ Runtime. PentaKuru chưa được cài.';
    Log('VC_RUNTIME_DECLINED');
    exit;
  end;
#endif
  if SuppressibleMsgBox('PentaKuru cần Microsoft Visual C++ Runtime x64.' + #13#10 +
    'Bộ cài sẽ cài hoặc cập nhật thành phần này trước. Windows có thể yêu cầu quyền quản trị.' + #13#10 +
    'Anh/chị có muốn tiếp tục?', mbConfirmation, MB_YESNO, IDYES) <> IDYES then begin
    Result := 'Cần cài Microsoft Visual C++ Runtime trước khi cài PentaKuru.';
    exit;
  end;
  ExtractTemporaryFile('vc_redist.x64.exe');
  RuntimePath := ExpandConstant('{tmp}\vc_redist.x64.exe');
  if CompareText(GetSHA256OfFile(RuntimePath), '{#VcRuntimeSHA256}') <> 0 then begin
    Result := 'Gói Visual C++ Runtime không đúng checksum. Vui lòng tải lại bộ cài PentaKuru.';
    exit;
  end;
  if not StrToVersion('{#VcRuntimePayloadVersion}', PayloadVersion) then RaiseException('Invalid payload version.');
  Repair := ComparePackedVersion(VcRuntimeInstalledVersion, PayloadVersion) >= 0;
#ifdef PentaKuruRuntimeSimulation
  Repair := TestCase = 'repair';
#endif
  if Repair then Parameters := '/repair' else Parameters := '/install';
  LogDirectory := ExpandConstant('{localappdata}\PentaKuru\SetupLogs');
#ifdef PentaKuruRuntimeSimulation
  LogDirectory := ExpandConstant('{tmp}');
#endif
  if not ForceDirectories(LogDirectory) then begin
    Result := 'Không tạo được thư mục log cài runtime.';
    exit;
  end;
  LogPath := AddBackslash(LogDirectory) + 'vc-runtime-' + GetDateTimeString('yyyymmdd-hhnnss', '-', ':') + '.log';
  Parameters := Parameters + ' /passive /norestart /log "' + LogPath + '"';
  Log('VC_RUNTIME_START: ' + Parameters);
#ifdef PentaKuruRuntimeSimulation
  { Simulate installer outcomes without modifying the shared Windows runtime. }
  Started := TestCase <> 'uac-cancel';
  ExitCode := 0;
  if TestCase = 'uac-cancel' then ExitCode := 1223;
  if TestCase = 'failure' then ExitCode := 1603;
  if TestCase = 'reboot' then ExitCode := 3010;
  if TestCase = 'newer-race' then ExitCode := 1638;
  VcRuntimeTestReady := Started and ((ExitCode = 0) or (ExitCode = 1638));
#else
  Started := ShellExec('runas', RuntimePath, Parameters, '', SW_SHOWNORMAL, ewWaitUntilTerminated, ExitCode);
#endif
  Log('VC_RUNTIME_EXIT=' + IntToStr(ExitCode));
  if not Started then begin
    Result := 'Chưa cài Visual C++ Runtime: ' + SysErrorMessage(ExitCode) + '. Vui lòng thử lại và đồng ý quyền quản trị của Windows.';
    exit;
  end;
  if (ExitCode = 3010) or (ExitCode = 1641) then begin
    VcRuntimeRestart := True;
    Log('VC_RUNTIME_RESTART_REQUIRED');
    exit;
  end;
  if (ExitCode = 1638) and not VcRuntimeNeedsInstall then begin
    Log('VC_RUNTIME_READY: newer runtime installed concurrently');
    exit;
  end;
  if ExitCode <> 0 then begin
    Result := 'Visual C++ Runtime cài chưa thành công (mã ' + IntToStr(ExitCode) + '). Vui lòng cài/sửa gói Microsoft Visual C++ x64 rồi chạy lại Setup. Log: ' + LogPath;
    exit;
  end;
#ifndef PentaKuruRuntimeSimulation
  if VcRuntimeNeedsInstall then begin
    Result := 'Visual C++ Runtime vẫn chưa sẵn sàng. Vui lòng khởi động lại máy, sửa gói Microsoft Visual C++ x64 rồi chạy lại Setup.';
    exit;
  end;
#endif
  Log('VC_RUNTIME_READY');
end;

function NeedRestart: Boolean;
begin
  Result := VcRuntimeRestart;
end;

function InitializeSetup: Boolean;
var
  ReportPath, Report: String;
begin
  ReportPath := ExpandConstant('{param:VCRUNTIMEREPORT|}');
  Result := True;
  if ReportPath <> '' then begin
    Report := '{"required":"{#VcRuntimeMinimumVersion}","installed":"' + VersionToStr(VcRuntimeInstalledVersion) + '","needs_install":' + IntToStr(Ord(VcRuntimeNeedsInstall)) + '}';
    if not SaveStringToFile(ReportPath, Report, False) then RaiseException('Cannot write runtime diagnostic report.');
    Result := False;
  end;
end;
