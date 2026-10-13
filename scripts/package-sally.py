"""Package the combined executable and its corresponding source, without secrets."""
from pathlib import Path
import argparse
import shutil
import zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--name", default="PentaKuru-Sally-Full")
parser.add_argument("--binary", type=Path, default=Path("target/release/pentakuru.exe"))
parser.add_argument("--watch", type=Path, help="Optional built Zepp .zab to include with Windows package")
args = parser.parse_args()
artifact_name = args.name
if not artifact_name or any(char not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_" for char in artifact_name):
    parser.error("name must contain only letters, digits, hyphens and underscores")
root = Path(__file__).resolve().parent.parent
dist = root / "dist"
dist.mkdir(exist_ok=True)
exe = dist / f"{artifact_name}.exe"
shutil.copy2(root / args.binary, exe)
notices = ["README.md", "LICENSE", "LICENSE-PENTEYE-MIT", "THIRD-PARTY-NOTICES.md",
           "assets/fonts/OFL.txt", "docs/SALLY-INTEGRATION.md", "docs/SALLY-AUDIO-FIX.md",
           "vendor/penta-bip/README.md", "docs/OFFICE-MEDIA-INTEGRATION.md", "docs/INSTALLER.md", "docs/DEEPTUTOR-ONECLICK.md"]
with zipfile.ZipFile(dist / f"{artifact_name}-Windows-x64.zip", "w", zipfile.ZIP_DEFLATED) as out:
    out.write(exe, exe.name)
    if args.watch:
        out.write(root / args.watch, "PentaKuru-Watch.zab")
    for name in notices:
        out.write(root / name, name)

source_roots = ["src", "assets", "scripts", "docs", "vendor/Sally/src",
                "vendor/Sally/src-tauri/src", "vendor/Sally/src-tauri/capabilities",
                "vendor/Sally/src-tauri/icons",
                "vendor/penta-bip/app-side", "vendor/penta-bip/common", "vendor/penta-bip/page",
                "vendor/penta-bip/setting", "vendor/penta-bip/assets", "vendor/penta-bip/watchface",
                "vendor/penta-bip/tests"]
source_files = notices + ["Cargo.toml", "Cargo.lock", "build.rs", ".gitignore", ".gitattributes", "rust-toolchain.toml",
    "vendor/Sally/LICENSE", "vendor/Sally/README.md", "vendor/Sally/.env.example",
    "vendor/Sally/package.json", "vendor/Sally/package-lock.json",
    "vendor/Sally/index.html", "vendor/Sally/tsconfig.json", "vendor/Sally/vite.config.ts",
    "vendor/Sally/src-tauri/Cargo.toml", "vendor/Sally/src-tauri/Cargo.lock",
    "vendor/Sally/src-tauri/build.rs", "vendor/Sally/src-tauri/tauri.conf.json",
    "vendor/Sally/src-tauri/tauri.pentakuru.conf.json",
    "vendor/penta-bip/app.js", "vendor/penta-bip/app.json", "vendor/penta-bip/app-ai.json",
    "vendor/penta-bip/package.json", "vendor/penta-bip/package-lock.json"]
files = {root / name for name in source_files}
for name in source_roots:
    files.update(path for path in (root / name).rglob("*") if path.is_file())
with zipfile.ZipFile(dist / f"{artifact_name}-Source.zip", "w", zipfile.ZIP_DEFLATED) as out:
    for path in sorted(files):
        if path.name != ".env" and "__pycache__" not in path.parts:
            out.write(path, path.relative_to(root).as_posix())
print(f"Created {exe}")
