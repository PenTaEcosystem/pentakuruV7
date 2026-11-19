"""Package PentaKuru Full & Lite executables (and clean source archive), without secrets."""
from pathlib import Path
import argparse
import shutil
import subprocess
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--name", default="PentaKuru-Full")
parser.add_argument("--edition", choices=["full", "lite"], default="full")
parser.add_argument("--binary", type=Path, default=Path("target/release/pentakuru.exe"))
parser.add_argument("--watch", type=Path, help="Optional built Zepp .zab to include with Windows package")
parser.add_argument("--skip-source", action="store_true", help="Skip generating the source archive")
parser.add_argument("--source-name", default="PentaKuru-v0.2.0")
args = parser.parse_args()

for label, val in [("name", args.name), ("source-name", args.source_name)]:
    if not val or any(
        char not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."
        for char in val
    ):
        parser.error(f"{label} must contain only letters, digits, dots, hyphens and underscores")

root = Path(__file__).resolve().parent.parent
dist = root / "dist"
dist.mkdir(exist_ok=True)
exe = dist / f"{args.name}.exe"
shutil.copy2(root / args.binary, exe)

edition_notes = (
    "PentaKuru Full (v0.2.0)\n"
    "- Bao gom day du tinh nang: Notes, Kanban, Tim kiem, Chat AI, Edge TTS, Sua chinh ta, Sally, Penta Music, Penta Clip, Watch Server, va Office (LibreOffice).\n"
    "- Da loai bo UI-TARS; lich nhac nho thiet lap qua Chat AI va tu dong tam dung khi nhan tin voi AI.\n"
    if args.edition == "full"
    else "PentaKuru Lite (v0.2.0)\n"
    "- Phien ban gon nhe KHONG bao gom LibreOffice / Office.\n"
    "- Giu nguyen toan bo cac tinh nang con lai: Notes, Kanban, Tim kiem, Chat AI, Edge TTS, Sua chinh ta, Sally, Penta Music, Penta Clip, Watch Server.\n"
    "- Da loai bo UI-TARS; lich nhac nho thiet lap qua Chat AI va tu dong tam dung khi nhan tin voi AI.\n"
)

notices = [
    "README.md",
    "LICENSE",
    "LICENSE-PENTEYE-MIT",
    "THIRD-PARTY-NOTICES.md",
    "assets/fonts/OFL.txt",
    "docs/PLATFORM-SUPPORT.md",
    "docs/SALLY-INTEGRATION.md",
    "docs/SALLY-AUDIO-FIX.md",
    "vendor/penta-bip/README.md",
    "docs/INSTALLER.md",
    "docs/CONNECTED-WORKSPACE.md",
]
if args.edition == "full":
    notices.append("docs/OFFICE-MEDIA-INTEGRATION.md")

zip_path = dist / f"{args.name}-Windows-x64.zip"
with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as out:
    out.write(exe, exe.name)
    out.writestr("EDITION.txt", edition_notes)
    if args.watch:
        out.write(root / args.watch, "PentaKuru-Watch.zab")
    for name in notices:
        file_path = root / name
        if file_path.is_file():
            out.write(file_path, name)

if not args.skip_source:
    source_roots = [
        "src",
        "tests",
        "assets",
        "scripts",
        "docs",
        "vendor/Sally/src",
        "vendor/Sally/src-tauri/src",
        "vendor/Sally/src-tauri/capabilities",
        "vendor/Sally/src-tauri/icons",
        "vendor/penta-bip/app-side",
        "vendor/penta-bip/common",
        "vendor/penta-bip/page",
        "vendor/penta-bip/setting",
        "vendor/penta-bip/assets",
        "vendor/penta-bip/watchface",
        "vendor/penta-bip/tests",
    ]
    source_files = notices + [
        "docs/OFFICE-MEDIA-INTEGRATION.md",
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        ".gitignore",
        ".gitattributes",
        "rust-toolchain.toml",
        "vendor/Sally/LICENSE",
        "vendor/Sally/README.md",
        "vendor/Sally/.env.example",
        "vendor/Sally/package.json",
        "vendor/Sally/package-lock.json",
        "vendor/Sally/index.html",
        "vendor/Sally/tsconfig.json",
        "vendor/Sally/vite.config.ts",
        "vendor/Sally/src-tauri/Cargo.toml",
        "vendor/Sally/src-tauri/Cargo.lock",
        "vendor/Sally/src-tauri/build.rs",
        "vendor/Sally/src-tauri/tauri.conf.json",
        "vendor/Sally/src-tauri/tauri.pentakuru.conf.json",
        "vendor/penta-bip/app.js",
        "vendor/penta-bip/app.json",
        "vendor/penta-bip/app-ai.json",
        "vendor/penta-bip/package.json",
        "vendor/penta-bip/package-lock.json",
    ]
    tracked = None
    try:
        res = subprocess.run(
            ["git", "ls-files"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        )
        tracked = {line.strip() for line in res.stdout.splitlines() if line.strip()}
    except Exception:
        tracked = None

    files = {root / name for name in source_files if (root / name).is_file()}
    for name in source_roots:
        base = root / name
        if base.is_dir():
            files.update(path for path in base.rglob("*") if path.is_file())
    source_zip = dist / f"{args.source_name}-Source.zip"
    with zipfile.ZipFile(source_zip, "w", zipfile.ZIP_DEFLATED) as out:
        for path in sorted(files):
            rel = path.relative_to(root).as_posix()
            if tracked is not None and rel not in tracked:
                continue
            if path.name != ".env" and "__pycache__" not in path.parts:
                out.write(path, rel)
    print(f"Created {source_zip}")

print(f"Created {zip_path}")
