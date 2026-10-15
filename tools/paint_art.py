"""Paints Isai's backdrop art: soft colour fields in the Meadow palette,
brushed sideways, on canvas with grain and paint speckles. Every image is
generated here, so the art is ours to ship (GPL like the rest).

Needs Pillow. Usage:
    python3 tools/paint_art.py OUT.jpg WIDTH HEIGHT SEED [ANGLE] [field|dusk]

The shipped images (crates/app/assets/art/):
    meadow-field.jpg  1600 1000 23 0 field
    meadow-dusk.jpg   1600 1000 11 -18 dusk
    meadow-lake.jpg   1600 1000 4 0 field
"""
import random
import sys

from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter

FOREST = (0, 50, 30)        # #00321E
GREEN = (17, 98, 82)        # #116252
TEAL = (86, 160, 150)       # sky / water
LEAF = (110, 170, 70)
YELLOW = (245, 220, 83)     # #F5DC53
PEACH = (244, 178, 113)     # #F4B271
ORANGE = (205, 97, 56)      # #CD6138
CREAM = (246, 236, 214)

out, W, H, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
angle = float(sys.argv[5]) if len(sys.argv) > 5 else 0.0
scene = sys.argv[6] if len(sys.argv) > 6 else "field"
rng = random.Random(seed)

# Work on a bigger, rotated canvas so diagonal streaks still fill the frame.
P = int(max(W, H) * 1.5)
img = Image.new("RGB", (P, P), GREEN)
d = ImageDraw.Draw(img)

# 1. Broad bands, top to bottom.
bands = {
    "field": [(0.0, GREEN), (0.28, TEAL), (0.5, TEAL), (0.62, LEAF), (0.72, YELLOW), (0.8, LEAF), (1.0, FOREST)],
    "dusk": [(0.0, FOREST), (0.3, GREEN), (0.48, TEAL), (0.6, TEAL), (0.68, LEAF), (0.82, ORANGE), (1.0, (120, 45, 30))],
}[scene]
for y in range(P):
    t = y / P
    for (a, ca), (b, cb) in zip(bands, bands[1:]):
        if a <= t <= b:
            k = (t - a) / (b - a)
            d.line([(0, y), (P, y)], fill=tuple(int(ca[i] + (cb[i] - ca[i]) * k) for i in range(3)))
            break

# 2. Soft colour fields.
blobs = Image.new("RGBA", (P, P), (0, 0, 0, 0))
bd = ImageDraw.Draw(blobs)
choices = [ORANGE, ORANGE, PEACH, YELLOW, TEAL, GREEN, FOREST, LEAF]
for i in range(14):
    # Always some warm orange and peach among the greens.
    c = [ORANGE, PEACH, ORANGE][i] if i < 3 else rng.choice(choices)
    cx, cy = rng.uniform(0, P), rng.uniform(0.15 * P, 0.95 * P)
    rx, ry = rng.uniform(0.2, 0.5) * P, rng.uniform(0.035, 0.09) * P
    bd.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=c + (rng.randint(190, 240),))
blobs = blobs.filter(ImageFilter.GaussianBlur(P * 0.018))
# Ragged, brushed edges: every few rows slide sideways by a wandering amount.
brushed = Image.new("RGBA", (P, P), (0, 0, 0, 0))
off = 0.0
for y in range(0, P, 3):
    off = off * 0.9 + rng.uniform(-1, 1) * P * 0.02
    brushed.paste(blobs.crop((0, y, P, y + 3)), (int(off), y))
img.paste(brushed, (0, 0), brushed)

# 3. Brush streaks: smear sideways, then add row-by-row light and dark lines.
img = img.resize((max(8, P // 30), P), Image.BILINEAR).resize((P, P), Image.BICUBIC)
rows = Image.new("L", (1, P))
v = 128
for y in range(P):
    v += rng.uniform(-9, 9)
    v = min(158, max(98, v * 0.97 + 128 * 0.03))
    rows.putpixel((0, y), int(v))
rows = rows.resize((P, P), Image.NEAREST)
cols = Image.new("L", (P, 1))
for x in range(P):
    cols.putpixel((x, 0), int(128 + 22 * rng.uniform(-1, 1)))
cols = cols.resize((P, P), Image.NEAREST).filter(ImageFilter.BoxBlur(P // 60))
streak = ImageChops.blend(rows, cols, 0.35).filter(ImageFilter.GaussianBlur(1.2))
img = ImageChops.overlay(img, Image.merge("RGB", (streak, streak, streak)))

if angle:
    img = img.rotate(angle, resample=Image.BICUBIC)
img = img.crop(((P - W) // 2, (P - H) // 2, (P - W) // 2 + W, (P - H) // 2 + H))

# Bolder colour, and a deep-green vignette like the posters.
img = ImageEnhance.Color(img).enhance(1.12)
img = ImageEnhance.Contrast(img).enhance(1.06)
vig = Image.new("L", (W, H), 0)
ImageDraw.Draw(vig).ellipse([-W * 0.15, -H * 0.1, W * 1.15, H * 1.1], fill=255)
vig = vig.filter(ImageFilter.GaussianBlur(min(W, H) * 0.12))
img = Image.composite(img, Image.new("RGB", (W, H), FOREST), vig)

# 4. Canvas weave.
tile = Image.new("L", (6, 6), 128)
for x in range(6):
    for y in range(6):
        tile.putpixel((x, y), 128 + (18 if (x // 3 + y // 3) % 2 == 0 else -18) + (10 if (x % 3 == 1 or y % 3 == 1) else 0))
weave = Image.new("L", (W, H))
for x in range(0, W, 6):
    for y in range(0, H, 6):
        weave.paste(tile, (x, y))
weave = weave.filter(ImageFilter.GaussianBlur(0.6))
img = ImageChops.blend(img, ImageChops.overlay(img, Image.merge("RGB", (weave,) * 3)), 0.55)

# 5. Grain.
noise = Image.effect_noise((W, H), 38).filter(ImageFilter.GaussianBlur(0.4))
img = ImageChops.blend(img, ImageChops.overlay(img, Image.merge("RGB", (noise,) * 3)), 0.5)

# 6. Paint speckles.
sp = ImageDraw.Draw(img)
for _ in range(int(W * H / 9000)):
    x, y = rng.uniform(0, W), rng.uniform(0, H)
    if rng.random() < 0.6 and 0.25 * W < x < 0.75 * W:
        continue
    c = rng.choice([YELLOW, YELLOW, PEACH, ORANGE, TEAL, CREAM, (200, 90, 120)])
    r = rng.uniform(0.9, 2.8)
    sp.ellipse([x - r, y - r * rng.uniform(1, 2.6), x + r, y + r], fill=c)

img.save(out, quality=84, optimize=True)
print(out, img.size)
