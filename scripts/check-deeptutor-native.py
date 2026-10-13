"""Check the official pinned runtime without API credentials or model calls."""
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
HOME = ROOT / ".build" / "deeptutor-native-check"
SETTINGS = HOME / "data" / "user" / "settings"
SETTINGS.mkdir(parents=True, exist_ok=True)
(SETTINGS / "system.json").write_text(json.dumps({"backend_port":4896,"frontend_port":4897}),encoding="utf-8")
PYTHON = ROOT / ".build" / "deeptutor-check" / "Scripts" / "python.exe"
log = (HOME / "startup.log").open("w",encoding="utf-8")
process = subprocess.Popen([str(PYTHON),str(ROOT / "assets/deeptutor-runner.py"),str(HOME)],
                           cwd=HOME,stdout=log,stderr=subprocess.STDOUT,
                           creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
report = {"passed":False,"credentials_used":False}
try:
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Runtime stopped; inspect startup.log")
        try:
            with urllib.request.urlopen("http://127.0.0.1:4897/",timeout=3) as response:
                html=response.read().decode("utf-8")
                if response.status==200 and len(html)>100:
                    report.update(passed=True,frontend_status=200,frontend_html_bytes=len(html))
                    break
        except Exception:
            time.sleep(.5)
    if not report["passed"]:
        raise RuntimeError("Frontend did not start within 120 seconds")
    with urllib.request.urlopen("http://127.0.0.1:4896/api/settings/catalog",timeout=4) as response:
        catalog=json.load(response)
        report["catalog_api"]=response.status
        report["catalog_has_services"]="services" in catalog.get("catalog",{})
finally:
    if process.poll() is None:
        subprocess.run(["taskkill.exe","/PID",str(process.pid),"/T","/F"],capture_output=True)
        process.wait(timeout=10)
    log.close()
    (HOME / "result.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
print(json.dumps(report))
