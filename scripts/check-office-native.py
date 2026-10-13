"""Verify the real visible UNO bridge against disposable DOCX/PPTX fixtures."""
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / ".build" / "office-native-check"
BASE.mkdir(parents=True, exist_ok=True)
STAMP = str(time.time_ns())
PROFILE = BASE / ("profile-" + STAMP)
PIPE = "pentakuruofficenativecheck" + STAMP
PYTHON = Path("C:/Program Files/LibreOffice/program/python.exe")
BRIDGE = ROOT / "src" / "office" / "bridge.py"
WORKER = subprocess.Popen([str(PYTHON), str(BRIDGE)], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,encoding="utf-8")


def call(path, command, **fields):
    print("  " + command, flush=True)
    request = dict(path=str(path), command=command, profile=str(PROFILE),
                   pipe=PIPE, **fields)
    WORKER.stdin.write(json.dumps(request,ensure_ascii=False) + "\n")
    WORKER.stdin.flush()
    reply = json.loads(WORKER.stdout.readline())
    if not reply["ok"]:
        raise RuntimeError(reply["error"])
    return reply["data"]


def verify():
    report = {}
    stamp = STAMP
    for name, factory, filter_name in [
        ("sample.docx", "private:factory/swriter", "Office Open XML Text"),
        ("sample.pptx", "private:factory/simpress", "Impress MS PowerPoint 2007 XML")]:
        path = BASE / (stamp + "-Giáo-án-" + name)
        print("Checking " + name, flush=True)
        if path.exists():
            try:
                call(path, "close_fixture")
            except RuntimeError:
                pass
        call(path, "create_fixture", factory=factory, filter=filter_name)
        call(path, "open")
        before = call(path, "read")
        if name.endswith("docx"):
            call(path,"select_fixture",unit="body/0")
            selected=call(path,"read",selection=True)
            call(path,"select_fixture",unit="body/1")
            original=selected["units"][0]
            selected_change=dict(id=original["id"],old=original["text"],new=original["text"].replace("hiễu","hiểu"))
            call(path,"apply",selection=True,snapshot_id=selected["snapshot_id"],revision=selected["revision"],changes=[selected_change])
            selected_result=call(path,"read")
            assert "hiểu" in selected_result["units"][0]["text"]
            assert "hiễu" in selected_result["units"][1]["text"]
            call(path,"undo")
        changes = [dict(id=item["id"], old=item["text"], new=item["text"].replace("hiễu", "hiểu"))
                   for item in before["units"] if "hiễu" in item["text"]]
        assert changes, name
        result = call(path, "apply", snapshot_id=before["snapshot_id"],revision=before["revision"], changes=changes)
        assert result["changed"] == len(changes)
        after = call(path, "read")
        expected = {change["id"]:change["new"] for change in changes}
        assert all(item["text"] == expected.get(item["id"],item["text"]) for item in after["units"]),json.dumps({"actual":after["units"],"expected":expected})
        assert not any("hiễu" in item["text"] for item in after["units"])
        call(path, "undo")
        undone = call(path, "read")
        assert undone["units"] == before["units"], (name, undone)
        call(path, "style", font="Liberation Serif", font_size=15)
        call(path, "undo")
        image_result = call(path, "images")
        assert image_result["changed"] == 1
        call(path, "undo")
        call(path, "apply", revision=undone["revision"], changes=changes)
        call(path, "save")
        call(path, "close_fixture")
        call(path, "open")
        reopened = call(path, "read")
        assert any("hiểu" in item["text"] for item in reopened["units"])
        assert not any("hiễu" in item["text"] for item in reopened["units"])
        try:
            call(path, "apply", revision=before["revision"], changes=changes)
            raise AssertionError("Stale revision accepted")
        except RuntimeError as exc:
            assert "thay đổi" in str(exc)
        call(path, "close_fixture")
        report[name] = {"visible_open": True, "edit": True, "undo": True,
                        "style": True, "image_alignment_undo":True, "save_reopen": True, "stale_revision_rejected": True}
    (BASE / "result.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    call(BASE / "cleanup.docx", "shutdown_fixture")
    WORKER.stdin.close()
    WORKER.wait(timeout=10)
    print(json.dumps(report))


if __name__ == "__main__":
    verify()
