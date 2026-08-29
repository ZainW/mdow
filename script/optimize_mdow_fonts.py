#!/usr/bin/env python3
"""Build the Mdow reading cuts from pinned OFL sources.

The reader paints 400-700, italic emphasis, and tabular figures. This script
instances variable axes to that range, pins Inter optical size to 16px, and
subsets glyphs to Latin, Greek, Cyrillic, and the punctuation markdown uses.
Georgia and SF Mono stay system lookups. They cannot be redistributed.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path(os.environ.get("MDOW_FONT_CACHE", "/tmp/mdow-font-src"))
GPUI_FONTS = ROOT / "apps/gpui/assets/fonts"
DESKTOP_FONTS = ROOT / "apps/desktop/src/renderer/src/assets/fonts"
WEB_FONTS = ROOT / "apps/web/src/assets/fonts"
LICENSE_DIR = GPUI_FONTS / "licenses"

SOURCES = {
    "inter": (
        "https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip",
        "inter.zip",
    ),
    "charis": (
        "https://github.com/silnrsi/font-charis/releases/download/v7.000/Charis-7.000.zip",
        "charis.zip",
    ),
    "jetbrains": (
        "https://github.com/JetBrains/JetBrainsMono/releases/download/v2.304/JetBrainsMono-2.304.zip",
        "jetbrains.zip",
    ),
    "geist": (
        "https://github.com/vercel/geist-font/archive/refs/tags/1.8.0.zip",
        "geist.zip",
    ),
}

READER_UNICODES = ",".join(
    [
        "U+0020-007E",
        "U+00A0-00FF",
        "U+0100-024F",
        "U+0370-03FF",
        "U+0400-04FF",
        "U+2000-206F",
        "U+2070-209F",
        "U+20A0-20CF",
        "U+2100-214F",
        "U+2190-21FF",
        "U+2200-22FF",
        "U+25A0-25FF",
        "U+2600-26FF",
        "U+2713",
        "U+2717",
        "U+FEFF",
    ]
)
MONO_UNICODES = f"{READER_UNICODES},U+2500-259F"
FEATURES = "kern,liga,calt,tnum,locl,case"


def run(args: list[str]) -> None:
    subprocess.check_call(args)


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 0:
        return
    print(f"download {url}")
    with urllib.request.urlopen(url) as response, dest.open("wb") as out:
        shutil.copyfileobj(response, out)


def unzip(archive: Path, dest: Path) -> Path:
    dest.mkdir(parents=True, exist_ok=True)
    marker = dest / ".extracted"
    if not marker.exists():
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(dest)
        marker.write_text("ok\n")
    return dest


def find_file(root: Path, name: str) -> Path:
    matches = [path for path in root.rglob("*") if path.is_file() and path.name == name]
    if len(matches) != 1:
        raise FileNotFoundError(f"{name} in {root}: {matches}")
    return matches[0]


def instance(src: Path, dest: Path, **axes: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    args = [sys.executable, "-m", "fontTools.varLib.instancer", str(src)]
    args.extend(f"{tag}={value}" for tag, value in axes.items())
    args.extend(["-o", str(dest)])
    run(args)


def subset(src: Path, dest: Path, unicodes: str) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    run(
        [
            sys.executable,
            "-m",
            "fontTools.subset",
            str(src),
            f"--output-file={dest}",
            f"--unicodes={unicodes}",
            f"--layout-features={FEATURES}",
            "--glyph-names",
            "--recommended-glyphs",
            "--name-IDs=*",
            "--recalc-bounds",
            "--recalc-average-width",
        ]
    )


def to_woff2(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    run(
        [
            sys.executable,
            "-m",
            "fontTools.ttLib.woff2",
            "compress",
            str(src),
            "-o",
            str(dest),
        ]
    )


def set_family(path: Path, family: str, subfamily: str) -> None:
    from fontTools.ttLib import TTFont

    font = TTFont(path)
    table = font["name"]
    full = family if subfamily == "Regular" else f"{family} {subfamily}"
    postscript = f"{family.replace(' ', '')}-{subfamily.replace(' ', '')}"
    replacements = {
        1: family,
        2: subfamily,
        4: full,
        6: postscript,
        16: family,
        17: subfamily,
    }
    seen: set[tuple[int, int, int, int]] = set()
    for rec in list(table.names):
        if rec.nameID in replacements:
            table.setName(
                replacements[rec.nameID],
                rec.nameID,
                rec.platformID,
                rec.platEncID,
                rec.langID,
            )
            seen.add((rec.nameID, rec.platformID, rec.platEncID, rec.langID))
    for name_id, value in replacements.items():
        if not any(item[0] == name_id for item in seen):
            table.setName(value, name_id, 3, 1, 0x409)
    font.save(path)


def write_outputs(ttf: Path, desktop_name: str | None, web_name: str | None) -> None:
    if desktop_name:
        to_woff2(ttf, DESKTOP_FONTS / desktop_name)
    if web_name:
        to_woff2(ttf, WEB_FONTS / web_name)


def copy_license(src: Path, dest_name: str) -> None:
    for directory in (LICENSE_DIR, DESKTOP_FONTS / "licenses"):
        directory.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, directory / dest_name)


def optimize_variable(
    src: Path,
    gpui_name: str,
    unicodes: str,
    desktop_name: str | None = None,
    web_name: str | None = None,
    **axes: str,
) -> Path:
    staged = CACHE / "staged" / gpui_name
    instanced = staged.with_suffix(".instanced.ttf")
    instance(src, instanced, **axes)
    dest = GPUI_FONTS / gpui_name
    subset(instanced, dest, unicodes)
    write_outputs(dest, desktop_name, web_name)
    return dest


def optimize_static(
    src: Path,
    gpui_name: str,
    family: str,
    subfamily: str,
    unicodes: str,
    desktop_name: str | None = None,
) -> Path:
    dest = GPUI_FONTS / gpui_name
    subset(src, dest, unicodes)
    set_family(dest, family, subfamily)
    write_outputs(dest, desktop_name, None)
    return dest


def ensure_fonttools() -> None:
    try:
        import fontTools  # noqa: F401
    except ImportError:
        sys.stderr.write(
            "fontTools is required. Create a venv and install fonttools[woff] brotli, then rerun.\n"
        )
        raise SystemExit(1)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    ensure_fonttools()

    CACHE.mkdir(parents=True, exist_ok=True)
    extracted: dict[str, Path] = {}
    for name, (url, filename) in SOURCES.items():
        archive = CACHE / filename
        download(url, archive)
        extracted[name] = unzip(archive, CACHE / name)

    inter = extracted["inter"]
    geist = extracted["geist"]
    charis = extracted["charis"]
    jetbrains = extracted["jetbrains"]

    optimize_variable(
        find_file(inter, "InterVariable.ttf"),
        "InterVariable.ttf",
        READER_UNICODES,
        desktop_name="InterVariable.woff2",
        web_name="InterVariable.woff2",
        wght="400:700",
        opsz="16",
    )
    optimize_variable(
        find_file(inter, "InterVariable-Italic.ttf"),
        "InterVariable-Italic.ttf",
        READER_UNICODES,
        desktop_name="InterVariable-Italic.woff2",
        web_name="InterVariable-Italic.woff2",
        wght="400:700",
        opsz="16",
    )
    optimize_variable(
        find_file(geist, "GeistMono[wght].ttf"),
        "GeistMono-Variable.ttf",
        MONO_UNICODES,
        desktop_name="GeistMono-Variable.woff2",
        web_name="GeistMono-Variable.woff2",
        wght="400:700",
    )
    optimize_variable(
        find_file(geist, "GeistMono-Italic[wght].ttf"),
        "GeistMono-Italic-Variable.ttf",
        MONO_UNICODES,
        desktop_name="GeistMono-Italic-Variable.woff2",
        web_name="GeistMono-Italic-Variable.woff2",
        wght="400:700",
    )

    for src_name, dest_name, subfamily, woff2 in (
        ("Charis-Regular.ttf", "Charter-Regular.ttf", "Regular", "Charter-Regular.woff2"),
        ("Charis-Italic.ttf", "Charter-Italic.ttf", "Italic", "Charter-Italic.woff2"),
        ("Charis-Bold.ttf", "Charter-Bold.ttf", "Bold", "Charter-Bold.woff2"),
        (
            "Charis-BoldItalic.ttf",
            "Charter-BoldItalic.ttf",
            "Bold Italic",
            "Charter-BoldItalic.woff2",
        ),
    ):
        optimize_static(
            find_file(charis, src_name),
            dest_name,
            "Charter",
            subfamily,
            READER_UNICODES,
            desktop_name=woff2,
        )

    optimize_variable(
        find_file(jetbrains, "JetBrainsMono[wght].ttf"),
        "JetBrainsMono-Variable.ttf",
        MONO_UNICODES,
        desktop_name="JetBrainsMono-Variable.woff2",
        wght="400:700",
    )
    optimize_variable(
        find_file(jetbrains, "JetBrainsMono-Italic[wght].ttf"),
        "JetBrainsMono-Italic-Variable.ttf",
        MONO_UNICODES,
        desktop_name="JetBrainsMono-Italic-Variable.woff2",
        wght="400:700",
    )

    copy_license(find_file(inter, "LICENSE.txt"), "Inter-OFL.txt")
    copy_license(find_file(charis, "OFL.txt"), "Charis-OFL.txt")
    copy_license(find_file(jetbrains, "OFL.txt"), "JetBrainsMono-OFL.txt")
    geist_license = next(
        path
        for path in extracted["geist"].rglob("*")
        if path.is_file() and path.name.upper() in {"OFL.TXT", "LICENSE.TXT", "LICENSE"}
    )
    copy_license(geist_license, "Geist-OFL.txt")

    print("wrote optimized fonts to")
    print(f"  {GPUI_FONTS}")
    print(f"  {DESKTOP_FONTS}")
    print(f"  {WEB_FONTS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
