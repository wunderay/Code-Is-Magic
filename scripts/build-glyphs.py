#!/usr/bin/env python3
"""Compile measured vector outlines from the bundled, licensed font subsets.

Run with fontTools installed. No download or runtime font dependency is needed.
The manifest owns the selected Unicode repertoire; procedural bind sigils are
original closed geometry generated below. --check verifies the committed asset.
"""
import argparse
import json
import math
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.svgLib.path import parse_path
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent


def measured(path, family, units):
    pen = BoundsPen(None)
    parse_path(path, pen)
    if pen.bounds is None:
        raise ValueError("Empty glyph")
    left, bottom, right, top = pen.bounds
    if right <= left or top <= bottom:
        raise ValueError("Degenerate glyph")
    return {
        "path": path,
        "cx": round((left + right) / 2, 6),
        "cy": round((bottom + top) / 2, 6),
        "units": units,
        "width": round(right - left, 6),
        "height": round(top - bottom, 6),
        "family": family,
    }


def stroke(x1, y1, x2, y2, thickness=42):
    length = math.hypot(x2 - x1, y2 - y1)
    nx, ny = -(y2 - y1) * thickness / (2 * length), (x2 - x1) * thickness / (2 * length)
    points = [(x1 + nx, y1 + ny), (x2 + nx, y2 + ny), (x2 - nx, y2 - ny), (x1 - nx, y1 - ny)]
    return "M" + "L".join(f"{x:.4f} {y:.4f}" for x, y in points) + "Z"


def build():
    manifest = json.loads((ROOT / "scripts/glyph-sources.json").read_text())
    fonts = {Path(item["path"]).name: TTFont(ROOT / item["path"]) for item in manifest["fonts"]}
    result = {}
    for item in manifest["glyphs"]:
        symbol = item["symbol"]
        if symbol in result:
            raise ValueError("Duplicate glyph")
        if "font" in item:
            font = fonts[item["font"]]
            glyphset = font.getGlyphSet()
            name = font.getBestCmap()[ord(symbol)]
            pen = SVGPathPen(glyphset)
            glyphset[name].draw(pen)
            path, units = pen.getCommands(), font["head"].unitsPerEm
        else:
            path, units = item["path"], item["units"]
        result[symbol] = measured(path, item["family"], units)
    # Twenty-four distinct angular bind sigils, with bars, prongs and lozenges.
    for i in range(24):
        path = stroke(0, -410, 0, 410, 46)
        for side in [-1, 1]:
            for level in range(3):
                mask = 1 << (level + (3 if side == 1 else 0))
                if (i * 13 + 19) & mask:
                    y = -260 + level * 260
                    end_y = y + (135 if (i + level) % 2 else -135)
                    path += stroke(0, y, side * (195 + i % 3 * 30), end_y)
                    if (i + level) % 4 == 0:
                        path += stroke(side * 150, y, side * 150, end_y)
        if i % 3 == 0:
            for a, b in [((0, -115), (115, 0)), ((115, 0), (0, 115)), ((0, 115), (-115, 0)), ((-115, 0), (0, -115))]:
                path += stroke(*a, *b, 32)
        result[f"bind-{i:02}"] = measured(path, "bind-sigils", 1000)
    binds = [value["path"] for key, value in result.items() if key.startswith("bind-")]
    if len(set(binds)) != len(binds):
        raise ValueError("Duplicate bind sigils")
    return json.dumps(result, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = ROOT / "src/glyphs.json"
    data = build()
    if args.check:
        if output.read_text() != data:
            raise SystemExit("Glyph asset differs; run python3 scripts/build-glyphs.py")
    else:
        output.write_text(data)
    print(f"{len(json.loads(data))} measured glyphs verified")
