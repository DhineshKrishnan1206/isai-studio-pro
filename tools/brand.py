#!/usr/bin/env python3
"""Turns the logo artwork in crates/app/assets/brand/source/ into the
transparent images the app uses.

Each source is black ink on a plain light background. For every source this
writes `<name>-light.png` (light ink, for dark themes) and `<name>-dark.png`
(dark ink, for light themes) next to the source folder, cropped tight.

Replace a source with a higher-resolution export and run:
    python3 tools/brand.py
Needs Pillow (pip install pillow).
"""
from pathlib import Path

from PIL import Image, ImageFilter

BRAND = Path(__file__).resolve().parent.parent / "crates/app/assets/brand"
LIGHT_INK = (237, 239, 245)
DARK_INK = (18, 20, 26)
TARGET_HEIGHT = 600  # small sources are upscaled so they stay smooth on high-DPI screens


def ink_mask(src: Image.Image) -> Image.Image:
    # Transparent sources are flattened onto white first, so "no ink" is light either way.
    if src.mode in ("RGBA", "LA", "P"):
        src = src.convert("RGBA")
        page = Image.new("RGBA", src.size, (255, 255, 255, 255))
        page.alpha_composite(src)
        src = page
    grey = src.convert("L")
    # The page colour is whatever the corners are; anything darker is ink.
    page = min(grey.getpixel((2, 2)), grey.getpixel((grey.width - 3, grey.height - 3))) - 6
    ink = 40
    alpha = grey.point(lambda v: 0 if v >= page else 255 if v <= ink else int((page - v) * 255 / (page - ink)))
    box = alpha.point(lambda v: 255 if v > 24 else 0).getbbox()
    pad = max(4, alpha.width // 100)
    box = (max(0, box[0] - pad), max(0, box[1] - pad), min(alpha.width, box[2] + pad), min(alpha.height, box[3] + pad))
    alpha = alpha.crop(box)
    if alpha.height < TARGET_HEIGHT:
        scale = TARGET_HEIGHT / alpha.height
        alpha = alpha.resize((round(alpha.width * scale), TARGET_HEIGHT), Image.LANCZOS)
        alpha = alpha.filter(ImageFilter.GaussianBlur(scale * 0.3))
        alpha = alpha.point(lambda v: 0 if v < 40 else 255 if v > 215 else int((v - 40) * 255 / 175))
    return alpha


def main() -> None:
    for source in sorted((BRAND / "source").glob("*.png")):
        alpha = ink_mask(Image.open(source))
        for suffix, rgb in (("light", LIGHT_INK), ("dark", DARK_INK)):
            out = Image.new("RGBA", alpha.size, rgb + (0,))
            out.putalpha(alpha)
            path = BRAND / f"{source.stem}-{suffix}.png"
            out.save(path, optimize=True)
            print(f"{path.relative_to(BRAND.parent.parent.parent.parent)}  {out.width}x{out.height}")


if __name__ == "__main__":
    main()
