#!/usr/bin/env python3
"""Builds design/brand/final/: the refined "D · Due stanze" mark, icon, lockups and presentation.

Requires fontTools and uharfbuzz (MIT / Apache-2.0), used only here to turn Bricolage Grotesque 600
(the OFL file the app already ships, from @fontsource) into paths. Run from the repository root:

    python3 design/brand/build-final.py
"""

import io
import math
import pathlib

import uharfbuzz as hb
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "design/brand/final"
FONT = (
    ROOT
    / "app/ui/node_modules/@fontsource/bricolage-grotesque/files/bricolage-grotesque-latin-600-normal.woff"
)

BG, PANEL, WELL, LINE = "#19121B", "#241C26", "#110B12", "#433946"
BG_M, PANEL_M = "#141120", "#1E1B2C"
TEXT, DIM = "#F5F0F6", "#B3AAB5"
LIME, LILAC = "#CEE654", "#CDAFFF"


def n(v):
    """Compact number: at most 2 decimals, no trailing zeros."""
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


# ── Geometry ────────────────────────────────────────────────────────────────────────────────
# The mark on its 100 × 100 grid. Each lane is the curve of the original D (horizontal run,
# rounded bend, straight diagonal, rounded bend, horizontal run), with tangent continuity.
BASE = dict(
    w=9, x0=9, a=23, c1=31, c2=45, x1=53,
    top=(20, 45.5), bot=(80, 54.5),
    pill=(65, 37, 95, 63), r=10,
)


def lane(p, y0, y1, t=0.3, s=1.0, ox=0.0, oy=0.0):
    X = lambda x: ox + s * x
    Y = lambda y: oy + s * y
    dx, dy = p["c2"] - p["c1"], y1 - y0
    p1 = (p["c1"] + t * dx, y0 + t * dy)
    p2 = (p["c2"] - t * dx, y1 - t * dy)
    return (
        f"M{n(X(p['x0']))} {n(Y(y0))}H{n(X(p['a']))}"
        f"Q{n(X(p['c1']))} {n(Y(y0))} {n(X(p1[0]))} {n(Y(p1[1]))}"
        f"L{n(X(p2[0]))} {n(Y(p2[1]))}"
        f"Q{n(X(p['c2']))} {n(Y(y1))} {n(X(p['x1']))} {n(Y(y1))}"
    )


def pill(p, s=1.0, ox=0.0, oy=0.0):
    """The Master room: a solid rounded rectangle."""
    x0, y0, x1, y1 = (ox + s * p["pill"][0], oy + s * p["pill"][1], ox + s * p["pill"][2], oy + s * p["pill"][3])
    r = p["r"] * s
    A = f"A{n(r)} {n(r)} 0 0 1"
    return (f"M{n(x0 + r)} {n(y0)}H{n(x1 - r)}{A} {n(x1)} {n(y0 + r)}V{n(y1 - r)}{A} {n(x1 - r)} {n(y1)}"
            f"H{n(x0 + r)}{A} {n(x0)} {n(y1 - r)}V{n(y0 + r)}{A} {n(x0 + r)} {n(y0)}Z")


def mark_groups(p, mix, master, s=1.0, ox=0.0, oy=0.0, ids=True):
    w = n(p["w"] * s)
    d = lane(p, *p["top"], s=s, ox=ox, oy=oy) + lane(p, *p["bot"], s=s, ox=ox, oy=oy)
    gi = (' id="room-mix"', ' id="room-master"') if ids else ("", "")
    return (
        f'<g{gi[0]}><path d="{d}" fill="none" stroke="{mix}" stroke-width="{w}" '
        f'stroke-linecap="round" stroke-linejoin="round"/></g>'
        f'<g{gi[1]}><path d="{pill(p, s, ox, oy)}" fill="{master}"/></g>'
    )


# Optical versions, drawn on their own pixel grid.
P20 = dict(w=2, x0=1.75, a=4.4, c1=6, c2=8.8, x1=10.5, top=(4, 9), bot=(16, 11), pill=(13, 7, 19.25, 13), r=2)
P32 = dict(w=2, x0=6.5, a=9.5, c1=11.5, c2=14.75, x1=16.5, top=(9, 15), bot=(23, 17), pill=(19.5, 13, 26.5, 19), r=2.5)
P16 = dict(w=1.5, x0=2.75, a=4.4, c1=5.35, c2=7, x1=8, top=(4.25, 7.25), bot=(11.75, 8.75),
           pill=(10.25, 6, 14, 10), r=1.5)


def squircle(x, y, w, h, r):
    """macOS continuous-corner rounded rectangle (the iOS 7+ bezier approximation)."""
    X0, Y0, X1, Y1 = x, y, x + w, y + h
    a, b, c, d, e, g, k = (
        1.52866 * r,
        1.08849 * r,
        0.8684 * r,
        0.66993 * r,
        0.06414 * r,
        0.37357 * r,
        0.18591 * r,
    )
    P = lambda *v: " ".join(n(q) for q in v)
    return "".join(
        [
            f"M{P(X0 + a, Y0)}",
            f"H{n(X1 - a)}",
            f"C{P(X1 - b, Y0, X1 - c, Y0, X1 - d, Y0 + e)}",
            f"C{P(X1 - g, Y0 + k, X1 - k, Y0 + g, X1 - e, Y0 + d)}",
            f"C{P(X1, Y0 + c, X1, Y0 + b, X1, Y0 + a)}",
            f"V{n(Y1 - a)}",
            f"C{P(X1, Y1 - b, X1, Y1 - c, X1 - e, Y1 - d)}",
            f"C{P(X1 - k, Y1 - g, X1 - g, Y1 - k, X1 - d, Y1 - e)}",
            f"C{P(X1 - c, Y1, X1 - b, Y1, X1 - a, Y1)}",
            f"H{n(X0 + a)}",
            f"C{P(X0 + b, Y1, X0 + c, Y1, X0 + d, Y1 - e)}",
            f"C{P(X0 + g, Y1 - k, X0 + k, Y1 - g, X0 + e, Y1 - d)}",
            f"C{P(X0, Y1 - c, X0, Y1 - b, X0, Y1 - a)}",
            f"V{n(Y0 + a)}",
            f"C{P(X0, Y0 + b, X0, Y0 + c, X0 + e, Y0 + d)}",
            f"C{P(X0 + k, Y0 + g, X0 + g, Y0 + k, X0 + d, Y0 + e)}",
            f"C{P(X0 + c, Y0, X0 + b, Y0, X0 + a, Y0)}",
            "Z",
        ]
    )


BODY = squircle(100, 100, 824, 824, 185.4)  # Apple's macOS icon body on the 1024 canvas
ICON_S = 5.8  # mark scale inside the 1024 icon
ICON_O = 512 - 50 * ICON_S  # keeps the mark centred on the body

BG_GRAD = (
    '<linearGradient id="bg" x1="0.15" y1="0.1" x2="0.85" y2="0.95">'
    f'<stop offset="0" stop-color="#271E2A"/><stop offset=".55" stop-color="{BG}"/>'
    '<stop offset="1" stop-color="#171223"/></linearGradient>'
)


def svg(vb, body, w=None, h=None, extra=""):
    size = f' width="{w}" height="{h}"' if w else ""
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}"{size}{extra}>{body}</svg>\n'


# ── Wordmark: Bricolage Grotesque 600 to paths ──────────────────────────────────────────────
def wordmark(text, size, x, baseline, tracking=-0.01):
    tt = TTFont(str(FONT))
    tt.flavor = None
    buf = io.BytesIO()
    tt.save(buf)
    data = buf.getvalue()
    upm = tt["head"].unitsPerEm
    font = hb.Font(hb.Face(data))
    b = hb.Buffer()
    b.add_str(text)
    b.guess_segment_properties()
    hb.shape(font, b, {"kern": True, "liga": True})
    gs = tt.getGlyphSet()
    order = tt.getGlyphOrder()
    sc = size / upm
    pen_x = 0.0
    parts = []
    for i, (info, pos) in enumerate(zip(b.glyph_infos, b.glyph_positions)):
        name = order[info.codepoint]
        pen = SVGPathPen(gs, ntos=n)
        gx = x + (pen_x + pos.x_offset) * sc
        gs[name].draw(TransformPen(pen, (sc, 0, 0, -sc, gx, baseline - pos.y_offset * sc)))
        parts.append(pen.getCommands())
        pen_x += pos.x_advance + (tracking * upm if i < len(b.glyph_infos) - 1 else 0)
    cap = tt["OS/2"].sCapHeight * sc
    return "".join(parts), x + pen_x * sc, cap


# ── Lockup geometry ────────────────────────────────────────────────────────────────────────
VIS = (4.5, 15.5, 95, 84.5)  # visible bounds of the 100-grid mark (strokes included)


def lockup(colors_mix, colors_master, text_color):
    m = 0.5  # mark box 50 × 50
    size = 40  # wordmark size, as on the welcome screen
    gap = 15
    mx = -VIS[0] * m
    mark = mark_groups(BASE, colors_mix, colors_master, s=m, ox=mx, oy=0)
    tx = (VIS[2] - VIS[0]) * m + gap
    word, end, cap = wordmark("Stemwise", size, tx, 0)
    # centre the cap height on the mark's visible centre
    base = 25 + cap / 2
    word, end, cap = wordmark("Stemwise", size, tx, base)
    top, bottom = VIS[1] * m, VIS[3] * m
    y0 = min(top, base - cap) - 1
    y1 = max(bottom, base + 10.5)  # descender-free word: leave room for the baseline overshoot
    w, h = end + 1, y1 - y0
    body = mark + f'<path d="{word}" fill="{text_color}"/>'
    return svg(f"0 {n(y0)} {n(w)} {n(h)}", body, n(w * 4), n(h * 4))


def write(name, content):
    path = OUT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def build_files():
    # Marks
    write("mark.svg", svg("0 0 100 100", mark_groups(BASE, LIME, LILAC), 512, 512))
    write(
        "mark-mono.svg",
        svg("0 0 100 100", mark_groups(BASE, "currentColor", "currentColor"), 512, 512),
    )
    write("mark-20.svg", svg("0 0 20 20", mark_groups(P20, "currentColor", "currentColor"), 20, 20))
    write("mono-light.svg", svg("0 0 100 100", mark_groups(BASE, TEXT, TEXT), 512, 512))
    write("mono-dark.svg", svg("0 0 100 100", mark_groups(BASE, BG, BG), 512, 512))
    # Favicons, with their background
    write(
        "favicon-32.svg",
        svg(
            "0 0 32 32",
            f'<path d="{squircle(0, 0, 32, 32, 7.2)}" fill="{BG}"/>'
            + mark_groups(P32, LIME, LILAC),
            32,
            32,
        ),
    )
    write(
        "favicon-16.svg",
        svg(
            "0 0 16 16",
            f'<path d="{squircle(0, 0, 16, 16, 3.6)}" fill="{BG}"/>'
            + mark_groups(P16, LIME, LILAC),
            16,
            16,
        ),
    )
    # Icon, flat and complete
    shadow = (
        '<filter id="sh" x="-10%" y="-10%" width="120%" height="125%">'
        '<feGaussianBlur stdDeviation="14"/></filter>'
    )
    icon = (
        f"<defs>{BG_GRAD}{shadow}</defs>"
        f'<path d="{squircle(100, 112, 824, 824, 185.4)}" fill="#07040A" fill-opacity=".45" filter="url(#sh)"/>'
        f'<path d="{BODY}" fill="url(#bg)"/>'
        f'<path d="{squircle(101, 101, 822, 822, 185.2)}" fill="none" stroke="{TEXT}" stroke-opacity=".07" stroke-width="2"/>'
        + mark_groups(BASE, LIME, LILAC, s=ICON_S, ox=ICON_O, oy=ICON_O)
    )
    write("icon-1024.svg", svg("0 0 1024 1024", icon, 1024, 1024))
    # Icon Composer layers, all 1024 × 1024 and aligned
    write(
        "icon-layers/background.svg",
        svg(
            "0 0 1024 1024", f'<defs>{BG_GRAD}</defs><path d="{BODY}" fill="url(#bg)"/>', 1024, 1024
        ),
    )
    write(
        "icon-layers/background-fullbleed.svg",
        svg(
            "0 0 1024 1024",
            f'<defs>{BG_GRAD}</defs><rect width="1024" height="1024" fill="url(#bg)"/>',
            1024,
            1024,
        ),
    )
    d = lane(BASE, *BASE["top"], s=ICON_S, ox=ICON_O, oy=ICON_O) + lane(
        BASE, *BASE["bot"], s=ICON_S, ox=ICON_O, oy=ICON_O
    )
    write(
        "icon-layers/room-mix.svg",
        svg(
            "0 0 1024 1024",
            f'<path d="{d}" fill="none" stroke="{LIME}" '
            f'stroke-width="{n(BASE["w"] * ICON_S)}" stroke-linecap="round" '
            'stroke-linejoin="round"/>',
            1024,
            1024,
        ),
    )
    write(
        "icon-layers/room-master.svg",
        svg(
            "0 0 1024 1024",
            f'<path d="{pill(BASE, ICON_S, ICON_O, ICON_O)}" fill="{LILAC}"/>',
            1024,
            1024,
        ),
    )
    # Lockups
    write("lockup.svg", lockup(LIME, LILAC, TEXT))
    write("lockup-mono-light.svg", lockup(TEXT, TEXT, TEXT))
    write("lockup-mono-dark.svg", lockup(BG, BG, BG))


if __name__ == "__main__":
    import sys

    sys.path.insert(0, str(pathlib.Path(__file__).parent))
    import presentation  # sibling module: builds presentation.html from the geometry above

    build_files()
    presentation.build(OUT, sys.modules[__name__])
    print("built", OUT)
