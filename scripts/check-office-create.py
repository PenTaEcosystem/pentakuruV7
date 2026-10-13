"""Native regression: blank Word/Excel/PowerPoint creation and overwrite rejection."""
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / '.build' / 'office-create-check' / str(time.time_ns())
BASE.mkdir(parents=True)
worker = subprocess.Popen(['C:/Program Files/LibreOffice/program/python.exe', str(ROOT / 'src/office/bridge.py')], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding='utf-8')

def call(path, command, **fields):
    request = dict(path=str(path), command=command, profile=str(BASE / 'profile'), pipe='pentakuruofficenativecheck' + BASE.name, **fields)
    worker.stdin.write(json.dumps(request) + '\n')
    worker.stdin.flush()
    return json.loads(worker.stdout.readline())

try:
    for kind, extension in [('Word', 'docx'), ('Excel', 'xlsx'), ('PowerPoint', 'pptx')]:
        path = BASE / ('blank.' + extension)
        assert call(path, 'create', kind=kind)['ok']
        assert path.is_file() and path.stat().st_size > 0
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        assert not call(path, 'create', kind=kind)['ok']
        assert hashlib.sha256(path.read_bytes()).hexdigest() == digest
        assert call(path, 'close_fixture')['ok']
        assert call(path, 'open')['ok']
        assert call(path, 'close_fixture')['ok']
        print(kind + ': create, reopen, reject overwrite passed', flush=True)
finally:
    call(BASE / 'cleanup.docx', 'shutdown_fixture')
    worker.stdin.close()
    worker.wait(timeout=15)
