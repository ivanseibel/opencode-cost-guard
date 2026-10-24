#!/usr/bin/env python3
"""Render Cost Guard icon assets from the selected design.

Selected design: design/icon/refined/authored-slit (2026-10-24).

Usage:
    python3 scripts/make-icons.py
    cd crates/app && cargo tauri icon icons/icon.png   # refresh icon.icns

Keep only icon.icns and icon.png plus the tray-* assets in crates/app/icons.
"""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "crates" / "app" / "icons"
SOURCE = ICONS / "source"


def render_glyph_mask() -> Image.Image:
    """Rasterize the selected tray glyph into a 44x44 alpha mask."""
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            ["qlmanage", "-t", "-s", "1024", "-o", tmp, str(SOURCE / "tray-glyph.svg")],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        rendered = Path(tmp) / "tray-glyph.svg.png"
        gray = Image.open(rendered).convert("L")

    alpha = ImageOps.invert(gray)
    bbox = alpha.getbbox()
    if bbox is None:
        raise SystemExit("tray glyph rendered empty")
    alpha = alpha.crop(bbox)

    canvas = Image.new("L", (44, 44), 0)
    scale = min(36 / alpha.width, 36 / alpha.height)
    size = (max(1, round(alpha.width * scale)), max(1, round(alpha.height * scale)))
    resized = alpha.resize(size, Image.LANCZOS)
    canvas.paste(resized, ((44 - size[0]) // 2, (44 - size[1]) // 2))
    return canvas


def tint(mask: Image.Image, rgb: tuple[int, int, int]) -> Image.Image:
    image = Image.new("RGBA", mask.size, (*rgb, 0))
    image.putalpha(mask)
    return image


def main() -> None:
    (ICONS / "icon.png").write_bytes((SOURCE / "app-icon-1024.png").read_bytes())

    mask = render_glyph_mask()
    tint(mask, (0, 0, 0)).save(ICONS / "tray-template.png")
    tint(mask, (34, 197, 94)).save(ICONS / "tray-fallback-off.png")
    tint(mask, (239, 68, 68)).save(ICONS / "tray-fallback-on.png")
    print("icons written; run `cd crates/app && cargo tauri icon icons/icon.png` for icon.icns")


if __name__ == "__main__":
    main()
