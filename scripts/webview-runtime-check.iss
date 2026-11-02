function WebViewRuntimeReady: Boolean;
var
  Version: String;
  VersionNumber: Int64;
begin
  Result := False;
  if RegQueryStringValue(HKLM32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) then
    if StrToVersion(Version, VersionNumber) then Result := VersionNumber > 0;
  if not Result and RegQueryStringValue(HKCU, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) then
    if StrToVersion(Version, VersionNumber) then Result := VersionNumber > 0;
end;
function PrepareWebView: String;
var
  Code: Integer;
  RuntimePath: String;
  Progress: TOutputProgressWizardPage;
begin
  Result := '';
#ifdef PentaKuruRuntimeSimulation
  if ExpandConstant('{param:WEBVIEWTEST|present}') = 'failure' then begin
    Result := 'WebView2 installation failed (test).';
    Log('WEBVIEW_RUNTIME_FAILURE');
  end else if ExpandConstant('{param:WEBVIEWTEST|present}') = 'missing' then
    Log('WEBVIEW_RUNTIME_READY: simulated installation')
  else Log('WEBVIEW_RUNTIME_SKIP: simulated existing runtime');
#else
  if WebViewRuntimeReady then begin
    Log('WEBVIEW_RUNTIME_SKIP: already installed');
    exit;
  end;
  ExtractTemporaryFile('MicrosoftEdgeWebView2RuntimeInstallerX64.exe');
  RuntimePath := ExpandConstant('{tmp}\MicrosoftEdgeWebView2RuntimeInstallerX64.exe');
  if CompareText(GetSHA256OfFile(RuntimePath), '{#WebViewRuntimeSha256}') <> 0 then begin
    Result := 'WebView2 payload is corrupt. Download PentaKuru Setup again.';
    exit;
  end;
  Progress := CreateOutputProgressPage('Chuẩn bị giao diện web', 'Đang cài Microsoft WebView2 cho Sally, Music và Clip.');
  Progress.Show;
  try
    if not Exec(RuntimePath, '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, Code) then
      Result := 'Cannot start Microsoft WebView2: ' + SysErrorMessage(Code)
    else if not WebViewRuntimeReady then
      Result := 'WebView2 is not ready (exit ' + IntToStr(Code) + '). Please retry Setup.'
    else Log('WEBVIEW_RUNTIME_READY');
  finally
    Progress.Hide;
    Progress.Free;
  end;
#endif
end;
