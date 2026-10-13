"""Local UNO bridge. JSON on stdin/stdout; credentials never enter this process."""
import difflib
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid
import uno
import unohelper
from com.sun.star.task import XInteractionHandler
HISTORY = {}
SNAPSHOTS = {}


class InteractionHandler(unohelper.Base, XInteractionHandler):
    def handle(self, request):
        payload = request.getRequest()
        kind = getattr(payload, "typeName", type(payload).__name__)
        # Never ask a hidden automation thread for a document password.
        desired = "XInteractionAbort" if "Password" in kind else "XInteractionApprove"
        for name in [desired, "XInteractionAbort"]:
            interface = uno.getTypeByName("com.sun.star.task." + name)
            for continuation in request.getContinuations():
                supported = continuation.queryInterface(interface)
                if supported:
                    supported.select()
                    return


INTERACTION = InteractionHandler()


def prop(name, value):
    p = uno.createUnoStruct("com.sun.star.beans.PropertyValue")
    p.Name, p.Value = name, value
    return p


def desktop(request):
    local = uno.getComponentContext()
    resolver = local.ServiceManager.createInstanceWithContext(
        "com.sun.star.bridge.UnoUrlResolver", local)
    name = request["pipe"]
    if not name.isalnum():
        raise ValueError("Invalid pipe")
    address = f"uno:pipe,name={name};urp;StarOffice.ComponentContext"
    if request["command"] in ("open", "create_fixture"):
        # Start the acceptor before connecting. A second Windows pipe resolve
        # after a failed connection can block waiting for a server indefinitely.
        profile = Path(request["profile"]).resolve()
        profile.mkdir(parents=True, exist_ok=True)
        program = Path(sys.executable).parent
        subprocess.Popen([
            str(program / "soffice.exe"),
            "-env:UserInstallation=" + uno.systemPathToFileUrl(str(profile)),
            f"--accept=pipe,name={name};urp;StarOffice.ServiceManager",
            "--norestore", "--nodefault", "--nofirststartwizard"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
    try:
        context = resolver.resolve(address)
    except Exception:
        if request["command"] not in ("open", "create_fixture"):
            raise ValueError("LibreOffice chưa mở. Bấm file để mở lại phiên.")
        profile = Path(request["profile"]).resolve()
        profile.mkdir(parents=True, exist_ok=True)
        program = Path(sys.executable).parent
        subprocess.Popen([
            str(program / "soffice.exe"),
            "-env:UserInstallation=" + uno.systemPathToFileUrl(str(profile)),
            f"--accept=pipe,name={name};urp;StarOffice.ServiceManager",
            "--norestore", "--nodefault", "--nofirststartwizard"],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        deadline = time.monotonic() + 25
        while True:
            try:
                context = resolver.resolve(address)
                break
            except Exception:
                if time.monotonic() > deadline:
                    raise ValueError("Không kết nối được LibreOffice trong 25 giây.")
                time.sleep(.15)
    return context.ServiceManager.createInstanceWithContext(
        "com.sun.star.frame.Desktop", context)


def document(desktop, request):
    raw = request["path"]
    if raw.startswith("\\\\?\\UNC\\"):
        raw = "\\\\" + raw[8:]
    elif raw.startswith("\\\\?\\"):
        raw = raw[4:]
    path = Path(raw).resolve()
    url = uno.systemPathToFileUrl(str(path))
    enumeration = desktop.getComponents().createEnumeration()
    while enumeration.hasMoreElements():
        component = enumeration.nextElement()
        if hasattr(component, "getURL") and component.getURL() == url:
            return component
    if request["command"] != "open":
        raise ValueError("File đã đóng trong LibreOffice. Bấm file để mở lại.")
    if not path.is_file():
        raise ValueError("File không còn tồn tại.")
    doc = desktop.loadComponentFromURL(url, "_blank", 0, (
        prop("Hidden", False), prop("ReadOnly", False),
        prop("MacroExecutionMode", 0), prop("UpdateDocMode", 0)))
    if doc is None:
        raise ValueError("LibreOffice không mở được tài liệu.")
    if doc.isReadonly():
        raise ValueError("Tài liệu đang khóa hoặc chỉ đọc. Đóng bản mở khác trước.")
    return doc


def writer_units(text, prefix="body"):
    result = []
    enum = text.createEnumeration()
    index = 0
    while enum.hasMoreElements():
        part = enum.nextElement()
        key = f"{prefix}/{index}"
        if part.supportsService("com.sun.star.text.TextTable"):
            for name in part.getCellNames():
                result += writer_units(part.getCellByName(name), key + "/" + name)
        elif hasattr(part, "getString"):
            result.append((key, part))
        index += 1
    return result


def shapes(page, prefix):
    result = []
    for i in range(page.getCount()):
        shape = page.getByIndex(i)
        key = f"{prefix}/{i}"
        if shape.supportsService("com.sun.star.drawing.GroupShape"):
            result += shapes(shape, key)
        else:
            result.append((key, shape))
    return result


def all_units(doc):
    if doc.supportsService("com.sun.star.text.TextDocument"):
        units = writer_units(doc.getText())
        for name in doc.getTextFrames().getElementNames():
            units += writer_units(doc.getTextFrames().getByName(name).getText(), "frame/" + name)
        return units
    if doc.supportsService("com.sun.star.presentation.PresentationDocument"):
        return [(key, shape) for i in range(doc.getDrawPages().getCount())
                for key, shape in shapes(doc.getDrawPages().getByIndex(i), "slide/" + str(i))
                if hasattr(shape, "getString")]
    raise ValueError("AI trực tiếp hiện hỗ trợ Writer và Impress; Calc chỉ mở tài liệu.")


def units(doc, selection):
    if not selection:
        return all_units(doc)
    selected = doc.getCurrentController().getSelection()
    if selected is None:
        raise ValueError("Chọn đoạn chữ hoặc đối tượng trong LibreOffice trước.")
    if hasattr(selected, "getCount"):
        result = [("selection/" + str(i), selected.getByIndex(i))
                  for i in range(selected.getCount())
                  if hasattr(selected.getByIndex(i), "getString")]
    else:
        result = [("selection/0", selected)] if hasattr(selected, "getString") else []
    if not result or not any(item.getString() for _, item in result):
        raise ValueError("Chọn đoạn chữ trong LibreOffice trước khi chạy AI.")
    return result


def snapshot(doc, selection):
    targets = units(doc, selection)
    if selection and doc.supportsService("com.sun.star.text.TextDocument"):
        frozen = []
        for key, item in targets:
            cursor = item.getText().createTextCursorByRange(item.getStart())
            cursor.gotoRange(item.getEnd(), True)
            frozen.append((key, cursor))
        targets = frozen
    texts = [{"id": key, "text": item.getString()} for key, item in targets]
    full = [(key, item.getString()) for key, item in all_units(doc)]
    revision = hashlib.sha256(json.dumps(full, ensure_ascii=False).encode()).hexdigest()
    if sum(len(item["text"]) for item in texts) > 60000:
        raise ValueError("Phạm vi quá dài. Chọn đoạn chữ hoặc đối tượng nhỏ hơn để xử lý.")
    token = uuid.uuid4().hex
    SNAPSHOTS[token] = (doc.getURL(), targets, revision)
    if len(SNAPSHOTS) > 128:
        del SNAPSHOTS[next(iter(SNAPSHOTS))]
    return {"units": texts, "revision": revision, "snapshot_id": token}


def cursor_length(text, drawing):
    # Writer counts Unicode characters; drawing text cursors count UTF-16 units.
    return len(text.encode("utf-16-le")) // 2 if drawing else len(text)


def impress_signature(doc):
    values = []
    for key, item in all_units(doc):
        entry = [key, item.getString()]
        if hasattr(item, "getText"):
            paragraphs = item.getText().createEnumeration()
            while paragraphs.hasMoreElements():
                paragraph = paragraphs.nextElement()
                if not hasattr(paragraph, "createEnumeration"):
                    continue
                portions = paragraph.createEnumeration()
                while portions.hasMoreElements():
                    portion = portions.nextElement()
                    if hasattr(portion, "getString"):
                        entry.append([portion.getString(), portion.CharFontName,
                                      portion.CharHeight, portion.CharWeight,
                                      str(portion.CharPosture), portion.CharColor])
        if item.supportsService("com.sun.star.drawing.GraphicObjectShape"):
            position, size = item.getPosition(), item.getSize()
            entry.append([position.X, position.Y, size.Width, size.Height])
        values.append(entry)
    return hashlib.sha256(json.dumps(values, ensure_ascii=False).encode()).hexdigest()


def replace_preserving_runs(target, old, new, drawing=False):
    # Edit only changed runs, from the end, preserving unaffected character styles.
    edits = difflib.SequenceMatcher(None, old, new, autojunk=False).get_opcodes()
    for tag, a, b, c, d in reversed(edits):
        if tag == "equal":
            continue
        cursor = target.getText().createTextCursorByRange(target.getStart())
        remaining = cursor_length(old[:a], drawing)
        while remaining:
            amount = min(remaining, 30000)
            cursor.goRight(amount, False)
            remaining -= amount
        remaining = cursor_length(old[a:b], drawing)
        while remaining:
            amount = min(remaining, 30000)
            cursor.goRight(amount, True)
            remaining -= amount
        cursor.setString(new[c:d])


def mutate(doc, title, operation, undo=None, redo=None):
    if doc.supportsService("com.sun.star.presentation.PresentationDocument"):
        # Impress does not expose reliable native undo for UNO text edits.
        # Keep PentaKuru undo in the live worker, tied to the document's content.
        try:
            result = operation()
            revision = impress_signature(doc)
        except Exception:
            if undo:
                undo()
            raise
        if undo:
            HISTORY.setdefault(doc.getURL(), []).append((undo, revision))
        return result
    manager = doc.getUndoManager()
    before = tuple(manager.getAllUndoActionTitles())
    manager.enterUndoContext("PentaKuru: " + title)
    try:
        result = operation()
    except Exception:
        manager.leaveUndoContext()
        if tuple(manager.getAllUndoActionTitles()) != before and manager.isUndoPossible():
            manager.undo()
        raise
    else:
        manager.leaveUndoContext()
        return result


def apply(doc, request):
    selected = request.get("selection", False)
    current = snapshot(doc, False)
    if current["revision"] != request["revision"]:
        raise ValueError("Tài liệu đã thay đổi trong lúc AI chạy; chạy lại với nội dung mới.")
    frozen = SNAPSHOTS.pop(request.get("snapshot_id", ""), None)
    if frozen:
        if frozen[0] != doc.getURL() or frozen[2] != request["revision"]:
            raise ValueError("Snapshot không khớp tài liệu.")
        targets = dict(frozen[1])
    elif selected:
        raise ValueError("Snapshot vùng chọn đã hết hạn; chạy lại AI.")
    else:
        targets = dict(units(doc, False))
    drawing = doc.supportsService("com.sun.star.presentation.PresentationDocument")
    changes = request["changes"]
    used = set()
    for change in changes:
        key = change["id"]
        if key in used or key not in targets or targets[key].getString() != change["old"]:
            raise ValueError("Phạm vi AI không còn khớp tài liệu đang mở.")
        if not isinstance(change["new"], str) or len(change["new"]) > 120000:
            raise ValueError("Nội dung AI không hợp lệ.")
        used.add(key)
    def update():
        count = 0
        # Reverse order protects adjacent paragraph ranges during text insertion.
        for change in reversed(changes):
            if change["old"] != change["new"]:
                replace_preserving_runs(targets[change["id"]], change["old"], change["new"], drawing)
                count += 1
        return {"changed": count, "dirty": doc.isModified()}
    def restore():
        for change in reversed(changes):
            replace_preserving_runs(targets[change["id"]], change["new"], change["old"], drawing)
    return mutate(doc, "Sửa nội dung", update, restore)


def style(doc, request):
    font = request.get("font", "Liberation Serif")
    size = float(request.get("font_size", 13))
    if not 6 <= size <= 96 or not font or len(font) > 100:
        raise ValueError("Font/cỡ chữ không hợp lệ.")
    targets = units(doc, request.get("selection", False))
    originals = []
    def save_runs(text):
        enumeration = text.createEnumeration()
        while enumeration.hasMoreElements():
            paragraph = enumeration.nextElement()
            if hasattr(paragraph, "createEnumeration"):
                portions = paragraph.createEnumeration()
                while portions.hasMoreElements():
                    portion = portions.nextElement()
                    if hasattr(portion, "getString"):
                        cursor = portion.getText().createTextCursorByRange(portion.getStart())
                        cursor.gotoRange(portion.getEnd(), True)
                        originals.append((cursor, cursor.CharFontName, cursor.CharHeight))
    for _, item in targets:
        if doc.supportsService("com.sun.star.presentation.PresentationDocument"):
            save_runs(item.getText())
        else:
            cursor = item.getText().createTextCursorByRange(item.getStart())
            cursor.gotoRange(item.getEnd(), True)
            originals.append((cursor, cursor.CharFontName, cursor.CharHeight))
    def update():
        for _, item in targets:
            cursor = item.getText().createTextCursorByRange(item.getStart())
            cursor.gotoRange(item.getEnd(), True)
            cursor.CharFontName, cursor.CharHeight = font, size
        return {"changed": len(targets), "dirty": doc.isModified()}
    def restore():
        for cursor, old_font, old_size in originals:
            cursor.CharFontName, cursor.CharHeight = old_font, old_size
    return mutate(doc, "Kiểu chữ", update, restore)


def align_images(doc, request):
    selected = request.get("selection", False)
    if doc.supportsService("com.sun.star.text.TextDocument"):
        graphics = doc.getGraphicObjects()
        if selected:
            item = doc.getCurrentController().getSelection()
            images = [item] if item and item.supportsService("com.sun.star.text.TextGraphicObject") else []
        else:
            images = [graphics.getByName(name) for name in graphics.getElementNames()]
        def update():
            for image in images:
                image.AnchorType = uno.Enum("com.sun.star.text.TextContentAnchorType", "AS_CHARACTER")
                image.getAnchor().ParaAdjust = 3  # ParagraphAdjust.CENTER
            return {"changed": len(images), "dirty": doc.isModified()}
    elif doc.supportsService("com.sun.star.presentation.PresentationDocument"):
        pages = doc.getDrawPages()
        chosen = doc.getCurrentController().getSelection() if selected else None
        images = []
        for i in range(pages.getCount()):
            page = pages.getByIndex(i)
            for _, shape in shapes(page, ""):
                if shape.supportsService("com.sun.star.drawing.GraphicObjectShape"):
                    if not selected or (chosen and any(chosen.getByIndex(j) == shape for j in range(chosen.getCount()))):
                        images.append((page, shape))
        def update():
            for page, image in images:
                size, position = image.getSize(), image.getPosition()
                position.X = max(0, (page.Width - size.Width) // 2)
                image.setPosition(position)
            return {"changed": len(images), "dirty": doc.isModified()}
    else:
        raise ValueError("Căn ảnh hiện hỗ trợ Writer và Impress.")
    if not images:
        raise ValueError("Không có ảnh trong phạm vi được chọn.")
    if doc.supportsService("com.sun.star.presentation.PresentationDocument"):
        originals = [(image, image.getPosition()) for _, image in images]
        def restore():
            for image, position in originals:
                image.setPosition(position)
        return mutate(doc, "Căn ảnh", update, restore)
    return mutate(doc, "Căn ảnh", update)


def execute(request):
    office = desktop(request)
    command = request["command"]
    if command == "shutdown_fixture":
        if not request["pipe"].startswith("pentakuruofficenativecheck") or ".build" not in Path(request["profile"]).parts:
            raise ValueError("Chỉ dừng instance kiểm thử.")
        components = office.getComponents().createEnumeration()
        while components.hasMoreElements():
            component = components.nextElement()
            if hasattr(component, "setModified"):
                component.setModified(False)
            component.dispose()
        office.terminate()
        return {"closed": True}
    if command == "create_fixture":
        if ".build" not in Path(request["path"]).parts:
            raise ValueError("Fixture chỉ được tạo trong .build.")
        def stage(name):
            (Path(request["path"]).parent / "stages.log").write_text(name,encoding="utf-8")
        stage("load factory")
        doc = office.loadComponentFromURL(request["factory"], "_blank", 0, (prop("Hidden", False),prop("InteractionHandler",INTERACTION)))
        stage("set text")
        if request["factory"] == "private:factory/swriter":
            text = doc.getText()
            cursor = text.createTextCursor()
            for index, line in enumerate(["🎓 Học sinh tìm hiễu nội dung.","🎓 Học sinh tìm hiễu nội dung.","Đoạn cuối giữ nguyên."]):
                cursor.gotoEnd(False)
                if index:
                    text.insertControlCharacter(cursor, 0, False)
                    cursor.gotoEnd(False)
                text.insertString(cursor, line, False)
        else:
            shape = doc.createInstance("com.sun.star.drawing.TextShape")
            doc.getDrawPages().getByIndex(0).add(shape)
            size = uno.createUnoStruct("com.sun.star.awt.Size")
            size.Width, size.Height = 18000, 3000
            shape.setSize(size)
            shape.setString("🎓 Bài giảng: tìm hiễu nội dung")
        png = Path(request["path"]).parent / "fixture.png"
        stage("create image")
        png.write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aPZ0AAAAASUVORK5CYII="))
        image = doc.createInstance("com.sun.star.text.TextGraphicObject" if request["factory"] == "private:factory/swriter" else "com.sun.star.drawing.GraphicObjectShape")
        image.GraphicURL = uno.systemPathToFileUrl(str(png))
        stage("insert image")
        if request["factory"] == "private:factory/swriter":
            image.AnchorType = uno.Enum("com.sun.star.text.TextContentAnchorType", "AS_CHARACTER")
            image.Width, image.Height = 5000, 2500
            cursor = doc.getText().createTextCursor()
            cursor.gotoEnd(False)
            doc.getText().insertTextContent(cursor, image, False)
        else:
            doc.getDrawPages().getByIndex(0).add(image)
            size = uno.createUnoStruct("com.sun.star.awt.Size")
            size.Width, size.Height = 5000, 2500
            image.setSize(size)
            position = uno.createUnoStruct("com.sun.star.awt.Point")
            position.X, position.Y = 1000, 2000
            image.setPosition(position)
        stage("store document")
        doc.storeAsURL(uno.systemPathToFileUrl(str(Path(request["path"]).resolve())),
                       (prop("FilterName", request["filter"]), prop("Overwrite", True),prop("InteractionHandler",INTERACTION)))
        stage("stored")
        return {"created": True}
    doc = document(office, request)
    if command == "select_fixture":
        if ".build" not in Path(request["path"]).parts:
            raise ValueError("Chỉ chọn vùng trong tài liệu thử.")
        item = dict(all_units(doc))[request["unit"]]
        cursor = item.getText().createTextCursorByRange(item.getStart())
        cursor.gotoRange(item.getEnd(), True)
        doc.getCurrentController().select(cursor)
        return {"selected": True}
    if command in ("open", "focus"):
        frame = doc.getCurrentController().getFrame()
        frame.activate()
        frame.getContainerWindow().setVisible(True)
        frame.getContainerWindow().toFront()
        return {"dirty": doc.isModified(), "readonly": doc.isReadonly()}
    if doc.isReadonly():
        raise ValueError("Tài liệu chỉ đọc.")
    if command == "read":
        return snapshot(doc, request.get("selection", False))
    if command == "apply":
        return apply(doc, request)
    if command == "style":
        return style(doc, request)
    if command == "images":
        return align_images(doc, request)
    if command == "undo":
        if doc.supportsService("com.sun.star.presentation.PresentationDocument"):
            history = HISTORY.get(doc.getURL(), [])
            if not history:
                raise ValueError("Không còn thao tác PentaKuru để hoàn tác.")
            callback, revision = history[-1]
            if impress_signature(doc) != revision:
                raise ValueError("Tài liệu đã thay đổi ngoài tác vụ; không hoàn tác chồng lên nội dung mới.")
            callback()
            history.pop()
            return {"dirty": doc.isModified()}
        manager = doc.getUndoManager()
        if not manager.isUndoPossible() or not manager.getCurrentUndoActionTitle().startswith("PentaKuru:"):
            raise ValueError("Không còn thao tác PentaKuru ở đầu lịch sử hoàn tác: " + repr(manager.getAllUndoActionTitles()))
        manager.undo()
        return {"dirty": doc.isModified()}
    if command == "save":
        doc.store()
        return {"dirty": doc.isModified()}
    if command == "close_fixture":
        if ".build" not in Path(request["path"]).parts:
            raise ValueError("Chỉ đóng cưỡng chế tài liệu kiểm thử trong .build.")
        if doc.isModified():
            doc.store()
        HISTORY.pop(doc.getURL(), None)
        deadline = time.monotonic() + 8
        while True:
            try:
                doc.close(True)
                break
            except Exception:
                if time.monotonic() > deadline:
                    doc.dispose()
                    break
                time.sleep(.2)
        return {"closed": True}
    raise ValueError("Lệnh Office không được hỗ trợ.")


if __name__ == "__main__":
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    for line in sys.stdin:
        try:
            result = {"ok": True, "data": execute(json.loads(line))}
        except Exception as exc:
            result = {"ok": False, "error": str(exc)}
        sys.stdout.write(json.dumps(result, ensure_ascii=True) + "\n")
        sys.stdout.flush()
