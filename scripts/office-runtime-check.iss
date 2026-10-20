// Run prerequisites before reporting successful installation or copying the application.
function PrepareOffice: String;
var
  Code: Integer;
  ScriptPath, Parameters, OfficeDirectory, OfficeLog: String;
  Progress: TOutputProgressWizardPage;
begin
  Result := '';
  if not (WizardIsComponentSelected('office\writer') or
          WizardIsComponentSelected('office\impress') or
          WizardIsComponentSelected('office\calc')) then begin
    Log('OFFICE_RUNTIME_SKIP: not selected');
    exit;
  end;
#ifdef PentaKuruRuntimeSimulation
  if ExpandConstant('{param:OFFICETEST|existing}') = 'failure' then begin
    Result := 'Không cài được LibreOffice (kiểm tra mô phỏng).';
    Log('OFFICE_RUNTIME_FAILURE');
  end else Log('OFFICE_RUNTIME_READY: simulated existing runtime');
#else
  ExtractTemporaryFile('install-office.ps1');
  ExtractTemporaryFile('LibreOffice-26.8.1.msi');
  if CompareText(GetSHA256OfFile(ExpandConstant('{tmp}\LibreOffice-26.8.1.msi')), '{#OfficeRuntimeSha256}') <> 0 then begin
    Result := 'Bộ cài LibreOffice đi kèm bị hỏng. Vui lòng tải lại PentaKuru Setup.';
    exit;
  end;
  ScriptPath := ExpandConstant('{tmp}\install-office.ps1');
  OfficeDirectory := ExpandConstant('{app}\Penteye Notes\Office');
  OfficeLog := OfficeDirectory + '\install.log';
  Parameters := '-NoProfile -ExecutionPolicy Bypass -File "' + ScriptPath +
    '" -Destination "' + OfficeDirectory + '" -InstallerPath "' +
    ExpandConstant('{tmp}\LibreOffice-26.8.1.msi') + '"';
  Progress := CreateOutputProgressPage('Chuẩn bị LibreOffice',
    'Đang kiểm tra và cài LibreOffice đi kèm. Windows có thể yêu cầu quyền quản trị.');
  Progress.Show;
  try
    Progress.SetProgress(0, 1);
    if not Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
      Parameters, '', SW_HIDE, ewWaitUntilTerminated, Code) then
      Result := 'Không khởi chạy được bộ cài LibreOffice: ' + SysErrorMessage(Code)
    else if Code <> 0 then begin
      Result := 'Chưa cài được LibreOffice. Kiểm tra dung lượng đĩa hoặc quyền cài đặt rồi bấm thử lại.' + #13#10 +
        'Chi tiết: ' + OfficeLog;
      Log('OFFICE_RUNTIME_FAILURE: ' + IntToStr(Code));
    end else begin
      Log('OFFICE_RUNTIME_READY');
      Progress.SetProgress(1, 1);
    end;
  finally
    Progress.Hide;
    Progress.Free;
  end;
#endif
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := PrepareVcRuntime(NeedsRestart);
  if Result = '' then Result := PrepareWebView;
  if Result = '' then Result := PrepareOffice;
end;
