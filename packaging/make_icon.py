#!/usr/bin/env python3
"""Draws the engine's icon (packaging/ultima1-engine.png), our own pixel art:
an island with a small keep and a golden ankh under a night sky.
CC-BY-SA 4.0, like the rest of the enhanced assets. Needs Pillow."""
from PIL import Image

ART = [
    "................................",
    "................................",
    "..........*.............*.......",
    "...*............................",
    "...............YYYY.............",
    "..............YYYYYY.........*..",
    "......*.......YY..YY............",
    "..............YY..YY............",
    "...............YYYY.............",
    "..........YYYYYYYYYYYYYY........",
    "..........YYYYYYYYYYYYYY........",
    "................YY..............",
    "......~~~~~.....YY.....~~~~.....",
    "~~~~~~~~~~~~~~~.YY.~~~~~~~~~~~~~",
    "~~~~~~~~~~~~~~..YY..~~~~~~~~~~~~",
    "~~~~~~~~~~~GGGGGYYGGGGG~~~~~~~~~",
    "~~~~~~~~GGGGGGGGYYGGGGGGGG~~~~~~",
    "~~~~~~GGGGGTTGGGGGGGGGGGGGGG~~~~",
    "~~~~~GGGGGTTTTGGGGG#.#.#GGGGGG~~",
    "~~~~GGGGGGTTTTGGGGG#####GGGGGG~~",
    "~~~~GGGGGGGbbGGGGGG##+##GGGGGGG~",
    "~~~GGGGGGGGGGGGGGGG##+##GGGGGG~~",
    "~~~GGGGMMGGGGGGGGGGGGGGGGGGGG~~~",
    "~~~~GGMMMMGGGGGGGGGGGGGTTGGG~~~~",
    "~~~~GMMMMMMGGGGGGGGGGGTTTTG~~~~~",
    "~~~~~GGGGGGGGGGGGGGGGGGbbGG~~~~~",
    "~~~~~~GGGGGGGGGGGGGGGGGGGG~~~~~~",
    "~~~~~~~~GGGGGGGGGGGGGGGG~~~~~~~~",
    "~~~~~~~~~~~GGGGGGGGGG~~~~~~~~~~~",
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
]
COL = {".": (12, 14, 48), "*": (230, 230, 255), "Y": (250, 200, 40), "~": (30, 80, 200),
       "G": (40, 150, 50), "T": (20, 90, 30), "b": (110, 70, 30), "#": (170, 170, 180),
       "+": (40, 40, 50), "M": (130, 120, 110)}
img = Image.new("RGB", (32, 32))
for y, row in enumerate(ART):
    assert len(row) == 32, (y, len(row))
    for x, c in enumerate(row):
        img.putpixel((x, y), COL[c])
img.resize((256, 256), Image.NEAREST).save(__file__.rsplit("/", 1)[0] + "/ultima1-engine.png", optimize=True)
