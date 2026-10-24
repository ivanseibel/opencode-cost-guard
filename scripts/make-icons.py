#!/usr/bin/env python3
"""Generate Cost Guard icons (run from the repo root: python3 scripts/make-icons.py).

After regenerating, refresh the macOS bundle icon with:

    cd crates/app && cargo tauri icon icons/icon.png

The Tauri CLI writes icon.icns plus platform extras; keep only icon.icns and
icon.png in crates/app/icons.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "crates" / "app" / "icons"


def shield(cx: float, top: float, width: float, height: float) -> list[tuple[float, float]]:
    left, right = cx - width / 2, cx + width / 2
    shoulder = top + height * 0.55
    return [
        (left, top),
        (right, top),
        (right, shoulder),
        (cx, top + height),
        (left, shoulder),
    ]


def app_icon() -> Image.Image:
    image = Image.new("RGBA", (1024, 1024), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((64, 64, 960, 960), radius=200, fill=(15, 23, 42, 255))

    outer = shield(512, 230, 460, 540)
    mask = Image.new("L", (1024, 1024), 0)
    ImageDraw.Draw(mask).polygon(outer, fill=255)
    image.paste(Image.new("RGBA", (1024, 1024), (248, 250, 252, 255)), (0, 0), mask)

    inner = shield(512, 286, 340, 420)
    inner_mask = Image.new("L", (1024, 1024), 0)
    ImageDraw.Draw(inner_mask).polygon(inner, fill=255)
    split = Image.new("RGBA", (1024, 1024), (34, 197, 94, 255))
    ImageDraw.Draw(split).rectangle((0, 520, 1024, 1024), fill=(239, 68, 68, 255))
    image.paste(split, (0, 0), inner_mask)

    return image


def tray_template() -> Image.Image:
    image = Image.new("RGBA", (44, 44), (0, 0, 0, 0))
    ImageDraw.Draw(image).polygon(shield(22, 5, 28, 34), fill=(0, 0, 0, 255))
    return image


def tray_state(color: tuple[int, int, int]) -> Image.Image:
    image = Image.new("RGBA", (44, 44), (0, 0, 0, 0))
    ImageDraw.Draw(image).polygon(shield(22, 5, 28, 34), fill=(*color, 255))
    return image


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    app_icon().save(ICONS / "icon.png")
    tray_template().save(ICONS / "tray-template.png")
    tray_state((34, 197, 94)).save(ICONS / "tray-fallback-off.png")
    tray_state((239, 68, 68)).save(ICONS / "tray-fallback-on.png")
    print("icons written to", ICONS)


if __name__ == "__main__":
    main()
