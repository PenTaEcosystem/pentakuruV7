"""Check the official pinned runtime without API credentials or model calls."""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument("--home", type=Path, default=ROOT / ".build" / "deeptutor-native-check")
parser.add_argument("--python", type=Path, default=ROOT / ".build" / "deeptutor-check" / "Scripts" / "python.exe")
args = parser.parse_args()
HOME = args.home.resolve()
with socket.socket() as backend, socket.socket() as frontend:
    backend.bind(("127.0.0.1", 0))
    frontend.bind(("127.0.0.1", 0))
    backend_port, frontend_port = backend.getsockname()[1], frontend.getsockname()[1]
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
SETTINGS = HOME / "data" / "user" / "settings"
SETTINGS.mkdir(parents=True, exist_ok=True)
(SETTINGS / "system.json").write_text(json.dumps({"backend_port":backend_port,"frontend_port":frontend_port}),encoding="utf-8")
PYTHON = args.python.resolve()
environment = dict(os.environ, PYTHONUTF8="1", PYTHONIOENCODING="utf-8", NO_PROXY="127.0.0.1,localhost", no_proxy="127.0.0.1,localhost")
paths = HOME / "runtime-paths.json"
if not paths.exists():
    paths = PYTHON.parent.parent.parent / "runtime-paths.json"
if paths.exists():
    node_dir = json.loads(paths.read_text(encoding="utf-8-sig"))["node_dir"]
    environment["PATH"] = node_dir + os.pathsep + environment.get("PATH", "")
log = (HOME / "startup.log").open("w",encoding="utf-8")
process = subprocess.Popen([str(PYTHON),"-I","-X","utf8",str(ROOT / "assets/deeptutor-runner.py"),str(HOME)],
                           cwd=HOME,stdout=log,stderr=subprocess.STDOUT,env=environment,
                           creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
report = {"passed":False,"credentials_used":False}
try:
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError("Runtime stopped; inspect startup.log")
        try:
            with opener.open(f"http://127.0.0.1:{frontend_port}/",timeout=3) as response:
                html=response.read().decode("utf-8")
                if response.status==200 and len(html)>100:
                    report.update(frontend_status=200,frontend_html_bytes=len(html))
                    break
        except Exception:
            time.sleep(.5)
    if report.get("frontend_status") != 200:
        raise RuntimeError("Frontend did not start within 120 seconds")
    with opener.open(f"http://127.0.0.1:{backend_port}/api/settings/catalog",timeout=4) as response:
        catalog=json.load(response)
        report["catalog_api"]=response.status
        report["catalog_has_services"]="services" in catalog.get("catalog",{})
    with opener.open(f"http://127.0.0.1:{backend_port}/health/ready",timeout=4) as response:
        report["backend_ready"] = response.status
    report["passed"] = True
finally:
    if process.poll() is None:
        subprocess.run(["taskkill.exe","/PID",str(process.pid),"/T","/F"],capture_output=True)
        process.wait(timeout=10)
    log.close()
    (HOME / "result.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
print(json.dumps(report))
