# Fallback text font

`unscii-8-subset.hex` holds the printable ASCII glyphs and the CP437
symbol glyphs (U+263A, U+25BA, ...) of **unscii-8** by Viznut
(<http://viznut.fi/unscii/>, <https://github.com/viznut/unscii>,
file `fontfiles/unscii-8.hex`), unchanged, in the Unifont `.hex` format.
Unscii-8 is in the public domain (the author: "You can consider it Public
Domain (or CC-0)"); the GPL'd Unifont-derived variants are not used.

The original game prints its text through the PC BIOS, so its glyphs are
the machine's ROM font. That font is **not** in this repository. `u1`
takes an 8x8 font at run time from `--font FILE` or `U1_FONT` (a raw dump
of 8-byte glyphs, a PSF1/PSF2 file with 8x8 glyphs, or a `.hex` file) and
falls back to this unscii-8 lookalike. See docs/SPEC.md §16.
