#!/usr/bin/env python3
"""Draws the launcher icon (our own: a gold ankh on night blue) into
app/src/main/res/mipmap-*/ic_launcher.png. CC-BY-SA 4.0, as the engine's
other drawn assets."""
from pathlib import Path
from PIL import Image, ImageDraw

def icon(n):
    s = 8  # drawn big, then scaled down smoothly
    N = n * s
    im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    d.rounded_rectangle([0, 0, N - 1, N - 1], radius=N // 6, fill=(16, 24, 72, 255))
    gold = (240, 192, 64, 255)
    w = N // 9
    cx = N // 2
    d.rectangle([cx - w // 2, N * 0.42, cx + w // 2, N * 0.86], fill=gold)
    d.rectangle([N * 0.24, N * 0.44, N * 0.76, N * 0.44 + w], fill=gold)
    d.ellipse([N * 0.33, N * 0.12, N * 0.67, N * 0.48], outline=gold, width=w)
    return im.resize((n, n), Image.LANCZOS)

root = Path(__file__).resolve().parent.parent / "app/src/main/res"
for name, n in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
    p = root / f"mipmap-{name}"
    p.mkdir(parents=True, exist_ok=True)
    icon(n).save(p / "ic_launcher.png")
