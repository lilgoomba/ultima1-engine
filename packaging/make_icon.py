#!/usr/bin/env python3
"""Draws the engine's icon (packaging/ultima1-engine.png): our own 16-bit
style pixel art, a stone keep with a red banner on a green hill at dusk,
a golden ankh in the sky, inside a dark-outlined rounded tile. Nothing in
it comes from the original game. CC-BY-SA 4.0, like the enhanced assets.
Needs Pillow."""
from PIL import Image

N = 32
img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
px = img.load()
OUT = (20, 16, 32, 255)

def inside(x, y, r=5):
    # A rounded square, corners of radius r.
    cx = min(max(x, r), N - 1 - r)
    cy = min(max(y, r), N - 1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r

# Dusk sky: four bands, dithered where they meet.
SKY = [(44, 32, 96), (92, 48, 128), (180, 76, 112), (240, 140, 90)]
for y in range(N):
    for x in range(N):
        if not inside(x, y):
            continue
        f = y / 20 * (len(SKY) - 1)
        i = min(int(f), len(SKY) - 2)
        frac = f - i
        c = SKY[i + 1] if (frac > 0.5) ^ ((x + y) % 2 == 0 and abs(frac - 0.5) < 0.2) else SKY[i]
        px[x, y] = c + (255,)
# Stars.
for x, y in [(4, 4), (9, 2), (25, 5), (28, 9), (20, 3)]:
    px[x, y] = (255, 250, 220, 255)

def put(x, y, c):
    if 0 <= x < N and 0 <= y < N and inside(x, y):
        px[x, y] = c + (255,) if len(c) == 3 else c

# Ankh (top left), outlined, with a highlight.
G1, G2, G3 = (255, 230, 110), (232, 176, 40), (150, 96, 20)
ankh = ["..###..", ".#...#.", ".#...#.", "..#.#..", "#######", "...#...", "...#...", "...#...", "...#..."]
for j, row in enumerate(ankh):
    for i, ch in enumerate(row):
        if ch == "#":
            for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
                if not (0 <= j + dy < len(ankh) and 0 <= i + dx < 7 and ankh[j + dy][i + dx] == "#"):
                    put(4 + i + dx, 5 + j + dy, OUT)
for j, row in enumerate(ankh):
    for i, ch in enumerate(row):
        if ch == "#":
            put(4 + i, 5 + j, G1 if (i < 3 or j < 2) else (G2 if j < 7 else G3))

# Hill: shaded green, a light rim.
for y in range(18, N):
    for x in range(N):
        top = 18 + int(((x - 17) / 9.0) ** 2)
        if y >= top:
            c = (60, 168, 72) if y < top + 1 else ((40, 128, 56) if x < 22 else (28, 96, 48))
            if y > 27:
                c = (24, 80, 44)
            put(x, y, c)
        elif y == top - 1:
            put(x, y, OUT)

# Keep: two towers and a hall, lit from the left.
ST1, ST2, ST3 = (200, 200, 212), (150, 150, 170), (96, 96, 120)
def block(x0, y0, x1, y1):
    for y in range(y0 - 1, y1 + 2):
        for x in range(x0 - 1, x1 + 2):
            put(x, y, OUT)
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(x, y, ST1 if x == x0 else (ST3 if x == x1 else ST2))
block(14, 15, 23, 24)   # hall
block(12, 11, 15, 24)   # left tower
block(22, 11, 25, 24)   # right tower
for x in (12, 14, 22, 24):  # battlements
    put(x, 10, OUT); put(x, 9, OUT)
for x in (13, 15, 23, 25):
    put(x, 10, OUT)
put(12, 10, ST1); put(14, 10, ST2); put(22, 10, ST1); put(24, 10, ST2)
for x in range(16, 22, 2):
    put(x, 14, ST2); put(x, 13, OUT)
# Door and windows.
for y in range(20, 25):
    for x in (18, 19):
        put(x, y, (56, 36, 24) if y > 20 else OUT)
for x, y in [(13, 14), (23, 14), (13, 18), (23, 18)]:
    put(x, y, (255, 214, 90))
# Banner on the right tower.
for y in range(4, 10):
    put(24, y, OUT)
for y, w in [(4, 3), (5, 4), (6, 3)]:
    for x in range(25, 25 + w):
        put(x, y, (220, 40, 48) if y != 6 else (150, 20, 36))

# The tile's outline.
for y in range(N):
    for x in range(N):
        if inside(x, y) and any(not inside(x + dx, y + dy) for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]):
            px[x, y] = OUT

img.resize((256, 256), Image.NEAREST).save(__file__.rsplit("/", 1)[0] + "/ultima1-engine.png", optimize=True)
