#!/usr/bin/env python3
"""Draw Facet's app icon (a hexagon cut into six facets, one per account colour)
as PNG files, using only the Python standard library.

Run: python3 -I app/scripts/make_icons.py
Writes app/src-tauri/icons/{32x32,128x128,128x128@2x,icon}.png and
app/public/facet.svg. Colours come from docs/05-design/tokens.css.
"""
import math
import re
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOKENS = (ROOT / "docs/05-design/tokens.css").read_text()


def token(name):
    m = re.search(rf"--{re.escape(name)}:\s*(#[0-9A-Fa-f]{{6}})", TOKENS)
    if not m:
        raise SystemExit(f"token {name} not found")
    h = m.group(1)[1:]
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


FACETS = [token(f"facet-{i}") for i in range(1, 7)]
INK = token("ink")


def hexagon(cx, cy, r):
    return [(cx + r * math.cos(math.radians(-90 + 60 * k)), cy + r * math.sin(math.radians(-90 + 60 * k))) for k in range(6)]


def inside(px, py, poly):
    hit = False
    n = len(poly)
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        if (y1 > py) != (y2 > py) and px < (x2 - x1) * (py - y1) / (y2 - y1) + x1:
            hit = not hit
    return hit


def render(size, ss=4):
    c = size / 2
    outer = hexagon(c, c, size * 0.46)
    inner_r = size * 0.10
    tris = [[(c, c), outer[k], outer[(k + 1) % 6]] for k in range(6)]
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            acc = [0, 0, 0, 0]
            for sy in range(ss):
                for sx in range(ss):
                    px, py = x + (sx + 0.5) / ss, y + (sy + 0.5) / ss
                    if math.hypot(px - c, py - c) < inner_r:
                        col = INK
                    else:
                        col = None
                        for k, t in enumerate(tris):
                            if inside(px, py, t):
                                col = FACETS[k]
                                break
                    if col:
                        acc[0] += col[0]; acc[1] += col[1]; acc[2] += col[2]; acc[3] += 255
            n = ss * ss
            a = acc[3] / n
            if a:
                row += bytes([round(acc[0] * 255 / acc[3]), round(acc[1] * 255 / acc[3]), round(acc[2] * 255 / acc[3]), round(a)])
            else:
                row += bytes([0, 0, 0, 0])
        rows.append(bytes(row))
    raw = b"".join(rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def svg():
    c, r = 64, 59
    pts = hexagon(c, c, r)
    parts = []
    for k in range(6):
        a, b = pts[k], pts[(k + 1) % 6]
        col = "#%02X%02X%02X" % FACETS[k]
        parts.append(f'<path d="M{c} {c}L{a[0]:.2f} {a[1]:.2f}L{b[0]:.2f} {b[1]:.2f}Z" fill="{col}"/>')
    ink = "#%02X%02X%02X" % INK
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128">{"".join(parts)}<circle cx="{c}" cy="{c}" r="12.8" fill="{ink}"/></svg>\n'


def main():
    out = ROOT / "app/src-tauri/icons"
    out.mkdir(parents=True, exist_ok=True)
    for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
        (out / name).write_bytes(render(size))
    pub = ROOT / "app/public"
    pub.mkdir(parents=True, exist_ok=True)
    (pub / "facet.svg").write_text(svg())
    print("icons written to", out)


if __name__ == "__main__":
    main()
