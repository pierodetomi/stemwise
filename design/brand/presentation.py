"""Builds final/presentation.html: one self-contained page (SVGs inlined as data URIs, fonts from Google Fonts)."""

import base64

ORIGINAL = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">'
    '<path d="M16 25H43Q50 25 55 32L64 43Q68 48 76 48H84" fill="none" stroke="#CEE654" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>'
    '<path d="M16 75H43Q50 75 55 68L64 57Q68 52 76 52H84" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>'
    '<path d="M75 39L86 50L75 61" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/></svg>'
)


def build(out, g):
    C = dict(
        bg=g.BG,
        panel=g.PANEL,
        well=g.WELL,
        line=g.LINE,
        bgm=g.BG_M,
        panelm=g.PANEL_M,
        text=g.TEXT,
        dim=g.DIM,
        lime=g.LIME,
        lilac=g.LILAC,
    )

    def read(name):
        return (out / name).read_text()

    def uri(svg_text):
        return "data:image/svg+xml;base64," + base64.b64encode(svg_text.encode()).decode()

    def img(name, w, h=None, cls="", alt=""):
        h = h or w
        c = f' class="{cls}"' if cls else ""
        return f'<img src="{uri(read(name))}" width="{w}" height="{h}" alt="{alt}"{c}>'

    def mark(p, mix, master, size, vb):
        return (
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {vb} {vb}" width="{size}" height="{size}" '
            f'aria-hidden="true">{g.mark_groups(p, mix, master, ids=False)}</svg>'
        )

    def dim(acc, bg):
        return f"color-mix(in srgb, {acc} 40%, {bg})"

    def bar(phase, lit=True):
        m = phase == "mix"
        acc, bg = (g.LIME, g.BG) if m else (g.LILAC, g.BG_M)
        if lit:
            mix, master = (acc, dim(acc, bg)) if m else (dim(acc, bg), acc)
        else:
            mix = master = acc
        return (
            f'<div class="bar" style="background:{bg};--acc:{acc}">{mark(g.P20, mix, master, 20, 20)}'
            f'<span class="proj">Paper Lanterns</span><span class="seg"><b class="{"on" if m else ""}">Mix</b>'
            f'<b class="{"" if m else "on"}">Master</b></span><span class="fill"></span>'
            f'<span class="btn">{"Revise mix" if m else "Revise master"}</span></div>'
        )

    lockup = read("lockup.svg")
    icon = read("icon-1024.svg")

    # Construction drawing on the 100 grid.
    B = g.BASE
    grid = "".join(
        f'<path d="M{i} 0V100M0 {i}H100" stroke="{C["line"]}" stroke-width=".25"/>'
        for i in range(0, 101, 10)
    )
    guides = "".join(
        f'<path d="M0 {y}H100" stroke="{C["dim"]}" stroke-width=".3" stroke-dasharray="1.2 1.2"/>'
        for y in (B["top"][0], B["top"][1], B["bot"][1], B["bot"][0])
    )
    vguides = "".join(
        f'<path d="M{x} 0V100" stroke="{C["dim"]}" stroke-width=".3" stroke-dasharray="1.2 1.2"/>'
        for x in (B["x0"], B["x1"], B["pill"][0], B["pill"][2])
    )
    ctrl = "".join(
        f'<circle cx="{x}" cy="{y}" r="1.1" fill="none" stroke="{C["text"]}" stroke-width=".35"/>'
        for x, y in (
            (B["c1"], B["top"][0]),
            (B["c2"], B["top"][1]),
            (B["c1"], B["bot"][0]),
            (B["c2"], B["bot"][1]),
        )
    )
    spine = (
        f'<path d="{g.lane(B, *B["top"])}{g.lane(B, *B["bot"])}" fill="none" stroke="{C["text"]}" '
        'stroke-width=".35"/>'
    )
    lbl = lambda x, y, t, a="start": (
        f'<text x="{x}" y="{y}" font-size="2.6" fill="{C["dim"]}" text-anchor="{a}" '
        f'font-family="Figtree">{t}</text>'
    )
    construction = (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-14 -6 128 112" width="100%">'
        f'<rect x="0" y="0" width="100" height="100" fill="{C["bg"]}"/>{grid}'
        f'<g opacity=".55">{g.mark_groups(B, g.LIME, g.LILAC, ids=False)}</g>{guides}{vguides}{spine}{ctrl}'
        + lbl(-2, B["top"][0] + 1, "20", "end")
        + lbl(-2, B["top"][1] + 1, "45.5", "end")
        + lbl(-2, B["bot"][1] + 1, "54.5", "end")
        + lbl(-2, B["bot"][0] + 1, "80", "end")
        + lbl(B["x0"], 104, "9", "middle")
        + lbl(B["x1"], 104, "53", "middle")
        + lbl(B["pill"][0], 104, "65", "middle")
        + lbl(B["pill"][2], 104, "95", "middle")
        + lbl(59, 33, "gap 7.5", "middle")
        + lbl(80, 34, "30 × 26, r 10", "middle")
        + lbl(14, 14, "stroke 9", "middle")
        + "</svg>"
    )

    css = f"""
*{{box-sizing:border-box}}
:root{{color-scheme:dark}}
html,body{{background:{C["bg"]}}}
body{{margin:0;color:{C["text"]};font:15px/1.6 Figtree,system-ui,sans-serif}}
main{{max-width:1120px;margin:0 auto;padding:64px 28px 120px}}
h1,h2,h3{{font-family:'Bricolage Grotesque',Figtree,sans-serif;font-weight:600;letter-spacing:-.01em;margin:0}}
h1{{font-size:52px;line-height:1.05}}h2{{font-size:30px;margin:6px 0 12px}}h3{{font-size:17px;font-weight:500;margin:30px 0 12px}}
p{{margin:0 0 10px;max-width:760px}}.dim{{color:{C["dim"]}}}
.eyebrow{{color:{C["lime"]};letter-spacing:.14em;text-transform:uppercase;font-size:11px;font-weight:600}}
section{{border-top:1px solid {C["line"]};margin-top:64px;padding-top:44px}}
figure{{margin:0}}figcaption,.cap{{color:{C["dim"]};font-size:12px;margin-top:8px}}
.card{{background:{C["panel"]};border-radius:14px;padding:22px}}
.well{{background:{C["well"]};border-radius:14px;padding:22px}}
.row{{display:flex;gap:24px;align-items:flex-end;flex-wrap:wrap}}
.grid2{{display:grid;grid-template-columns:1fr 1fr;gap:20px}}
.grid3{{display:grid;grid-template-columns:repeat(3,1fr);gap:16px}}
.hero{{display:grid;grid-template-columns:1.1fr 1fr;gap:36px;align-items:center;margin-top:8px}}
.hero .lock{{background:{C["well"]};border-radius:18px;padding:56px 40px;display:grid;place-items:center}}
.hero .lock img{{width:100%;height:auto;max-width:420px}}
.ic{{filter:none}}
table{{border-collapse:collapse;width:100%;font-size:14px}}
td,th{{text-align:left;vertical-align:top;padding:10px 12px;border-top:1px solid {C["line"]}}}
th{{color:{C["dim"]};font-weight:500;font-size:12px;letter-spacing:.06em;text-transform:uppercase;border-top:0}}
td:first-child{{color:{C["dim"]};width:30px}}
.before-after{{display:grid;grid-template-columns:1fr 40px 1fr;align-items:center;gap:10px}}
.before-after .well{{display:grid;place-items:center;padding:30px}}
.arrowtxt{{color:{C["dim"]};text-align:center;font-size:22px}}
.sizes{{display:flex;gap:34px;align-items:flex-end;flex-wrap:wrap}}
.sizes figure{{display:flex;flex-direction:column;align-items:center}}
.big-icon img{{max-width:100%;height:auto}}
.dockbg{{background:linear-gradient(160deg,#3b2f45,#1b1726 60%,#2a2235);border-radius:14px;padding:48px 30px 20px;display:inline-block}}
.dock{{display:inline-flex;gap:10px;padding:10px 12px;border-radius:22px;background:rgba(58,50,62,.75);border:1px solid rgba(245,240,246,.12)}}
.zoom{{display:flex;gap:28px;flex-wrap:wrap;align-items:flex-end}}
.px{{position:relative;background:{C["bg"]};border-radius:8px;overflow:hidden}}
.px .pg{{position:absolute;inset:0;background-image:linear-gradient(to right,rgba(245,240,246,.09) 1px,transparent 1px),linear-gradient(to bottom,rgba(245,240,246,.09) 1px,transparent 1px)}}
.bar{{height:52px;display:flex;align-items:center;gap:12px;padding:0 16px;border:1px solid {C["line"]};border-radius:10px;margin-bottom:10px}}
.bar svg{{flex:none}}
.proj{{font:500 15px 'Bricolage Grotesque',sans-serif}}
.seg{{display:inline-flex;background:{C["well"]};border-radius:9px;padding:3px;margin-left:6px}}
.seg b{{font-weight:500;font-size:12px;color:{C["dim"]};padding:4px 11px;border-radius:7px}}
.seg b.on{{background:{C["panel"]};color:var(--acc);box-shadow:inset 0 0 0 1px {C["line"]}}}
.fill{{flex:1}}.btn{{font-size:12px;color:{C["dim"]};border:1px solid {C["line"]};border-radius:10px;padding:5px 12px}}
.welcome{{background:{C["bg"]};border:1px solid {C["line"]};border-radius:14px;display:grid;grid-template-columns:1.2fr 1fr;min-height:360px;overflow:hidden}}
.welcome .l{{padding:56px 48px;display:flex;flex-direction:column;justify-content:center}}
.welcome .name{{font:600 40px/1.1 'Bricolage Grotesque',sans-serif;letter-spacing:-.01em;margin:18px 0 10px}}
.welcome .slogan{{font-size:14px;color:{C["dim"]};max-width:360px}}
.welcome .cta{{display:flex;gap:10px;margin-top:26px}}
.welcome .cta span{{font-size:13px;border-radius:10px;padding:8px 14px;border:1px solid {C["line"]}}}
.welcome .cta span:first-child{{background:{C["lime"]};color:{C["bg"]};border-color:{C["lime"]};font-weight:600}}
.welcome .r{{background:{C["panel"]};padding:30px 26px;border-left:1px solid {C["line"]}}}
.welcome .r h4{{margin:0 0 12px;font:500 12px Figtree;color:{C["dim"]};letter-spacing:.08em;text-transform:uppercase}}
.recent{{display:flex;justify-content:space-between;padding:11px 12px;border-radius:10px;font-size:14px}}
.recent:first-of-type{{background:{C["well"]}}}.recent small{{color:{C["dim"]};font-size:12px}}
.connect{{max-width:460px;margin:0 auto;background:{C["panel"]};border:1px solid {C["line"]};border-radius:14px;overflow:hidden}}
.connect header{{display:flex;align-items:center;gap:10px;padding:16px 20px;border-bottom:1px solid {C["line"]};font:600 16px 'Bricolage Grotesque'}}
.connect div{{padding:20px;font-size:14px;color:{C["dim"]}}}
.mono{{display:grid;grid-template-columns:1fr 1fr;gap:16px}}
.mono>div{{border-radius:14px;padding:36px 28px;display:flex;flex-direction:column;align-items:center;gap:26px}}
.mono .d{{background:{C["well"]}}}.mono .lt{{background:#E9E4EA}}.mono .lt .cap{{color:#5D5462}}
.mono .lk{{width:100%;max-width:300px;height:auto}}
.clear{{display:grid;grid-template-columns:1fr 1fr;gap:20px}}
.dos{{display:grid;grid-template-columns:repeat(4,1fr);gap:14px}}
.dos figure{{background:{C["panel"]};border-radius:14px;padding:18px;display:flex;flex-direction:column;align-items:center}}
.dos .t{{height:120px;width:100%;border-radius:10px;display:grid;place-items:center}}
.dos figcaption{{text-align:center}}.no{{color:#F2A7A7}}.ok{{color:{C["lime"]}}}
.crit td:first-child{{width:160px;color:{C["text"]}}}
@media (max-width:900px){{.hero,.grid2,.welcome,.clear,.mono{{grid-template-columns:1fr}}.grid3,.dos{{grid-template-columns:1fr 1fr}}.before-after{{grid-template-columns:1fr}}}}
"""

    defects = [
        (
            "It reads as the generic “merge” icon.",
            "The two lines no longer end in a common arrow: they come close and hand the song to a separate, solid shape. The mark is a bracket that gathers, followed by a room.",
        ),
        (
            "The two lines arrive together, like two equal inputs.",
            "The two trajectories are now both the Mix (lime): many parts that come together. The Master (lilac) comes after, beyond a gap, and reads from left to right.",
        ),
        (
            "The arrowhead is a cliché and at 32 px it becomes a blotch.",
            "The arrow is removed. In its place the Master room, a solid rounded rectangle, separated from the lines by a gap of 7.5/100.",
        ),
        (
            "The mark occupies half the height.",
            "The mark spans 15.5 to 84.5 in height and 4.5 to 95 in width: two thirds of the square, instead of half.",
        ),
        (
            "In monochrome the two rooms disappear.",
            "The Mix is made of lines, the Master is a solid shape: two rooms recognizable even in a single color. In the bar the room of the current phase lights up.",
        ),
        (
            "The lockup lettering is not Bricolage Grotesque.",
            "The name is Bricolage Grotesque 600, tracking −0.01 em, converted to outlines from the same font file the app includes.",
        ),
        (
            "The icon background is not a squircle.",
            "The background is the macOS continuous-corner squircle: an 824 px body centered on the 1024 square, with the shadow in the margin.",
        ),
    ]

    dock_neutral = [
        (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><defs><linearGradient id="a" x2="0" y2="1">'
            '<stop offset="0" stop-color="#E7E3E8"/><stop offset="1" stop-color="#BDB7C0"/></linearGradient></defs>'
            f'<path d="{g.BODY}" fill="url(#a)"/><rect x="300" y="330" width="424" height="364" rx="40" fill="none" stroke="#6F6873" stroke-width="44"/>'
            '<path d="M300 430H724" stroke="#6F6873" stroke-width="44"/></svg>'
        ),
        (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><defs><linearGradient id="a" x2="0" y2="1">'
            '<stop offset="0" stop-color="#8E8893"/><stop offset="1" stop-color="#5F5964"/></linearGradient></defs>'
            f'<path d="{g.BODY}" fill="url(#a)"/><circle cx="512" cy="512" r="190" fill="none" stroke="#E6E1E8" stroke-width="48"/>'
            '<path d="M512 512V400M512 512L590 560" stroke="#E6E1E8" stroke-width="44" stroke-linecap="round"/></svg>'
        ),
        (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><defs><linearGradient id="a" x2="0" y2="1">'
            '<stop offset="0" stop-color="#6E7A86"/><stop offset="1" stop-color="#46505B"/></linearGradient></defs>'
            f'<path d="{g.BODY}" fill="url(#a)"/><path d="M330 360H694M330 512H694M330 664H560" stroke="#E6E1E8" stroke-width="48" stroke-linecap="round"/></svg>'
        ),
        (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024"><defs><linearGradient id="a" x2="0" y2="1">'
            '<stop offset="0" stop-color="#E7E3E8"/><stop offset="1" stop-color="#BDB7C0"/></linearGradient></defs>'
            f'<path d="{g.BODY}" fill="url(#a)"/><path d="M290 380Q290 330 340 330H460L510 380H690Q740 380 740 430V640Q740 690 690 690H340Q290 690 290 640Z" fill="#8A8390"/></svg>'
        ),
    ]
    dimg = lambda s, w=64: f'<img src="{uri(s)}" width="{w}" height="{w}" alt="">'
    dock = (
        f'<div class="dockbg"><div class="dock">{dimg(dock_neutral[0])}{dimg(dock_neutral[1])}'
        f"{img('icon-1024.svg', 64, alt='Stemwise')}{dimg(dock_neutral[2])}{dimg(dock_neutral[3])}</div></div>"
    )

    def zoom(name, px, k, label):
        s = px * k
        return (
            f'<figure><div class="px" style="width:{s}px;height:{s}px">{img(name, s)}'
            f'<div class="pg" style="background-size:{k}px {k}px"></div></div>'
            f"<figcaption>{label} · {px} px, magnified {k}×</figcaption></figure>"
        )

    m20_lime = read("mark-20.svg").replace("currentColor", g.LIME)

    # clear space: x = half the Master room height = 13/100 of the mark side
    x = 13
    clear_svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{-4.5 + 4.5 - x - 6} {15.5 - x - 6} {90.5 + 2 * x + 12} {69 + 2 * x + 12}" width="100%">'
        f'<rect x="{4.5 - x}" y="{15.5 - x}" width="{90.5 + 2 * x}" height="{69 + 2 * x}" fill="none" stroke="{C["dim"]}" stroke-width=".4" stroke-dasharray="1.5 1.5"/>'
        f'<rect x="4.5" y="15.5" width="90.5" height="69" fill="none" stroke="{C["line"]}" stroke-width=".4"/>'
        + g.mark_groups(B, g.LIME, g.LILAC, ids=False)
        + f'<rect x="{4.5 - x}" y="44" width="{x}" height="12" fill="{C["lilac"]}" fill-opacity=".18"/>'
        f'<text x="{4.5 - x / 2}" y="52" font-size="4" text-anchor="middle" fill="{C["dim"]}" font-family="Figtree">x</text>'
        f'<rect x="65" y="{15.5 - x}" width="30" height="{x}" fill="{C["lilac"]}" fill-opacity=".18"/>'
        f'<text x="80" y="{15.5 - x / 2 + 1.4}" font-size="4" text-anchor="middle" fill="{C["dim"]}" font-family="Figtree">x = ½ room</text>'
        "</svg>"
    )

    def tile(content, bg, cap, good):
        cls = "ok" if good else "no"
        sign = "Yes" if good else "No"
        return (
            f'<figure><div class="t" style="background:{bg}">{content}</div>'
            f'<figcaption><b class="{cls}">{sign}</b> · {cap}</figcaption></figure>'
        )

    mk = lambda mix=g.LIME, mas=g.LILAC, style="": (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="84" height="84" style="{style}">'
        f"{g.mark_groups(B, mix, mas, ids=False)}</svg>"
    )
    dos = "".join(
        [
            tile(mk(), C["bg"], "in color on aubergine", True),
            tile(mk(), C["bgm"], "in color on indigo", True),
            tile(mk(C["text"], C["text"]), C["well"], "light monochrome on dark", True),
            tile(mk(C["bg"], C["bg"]), "#E9E4EA", "dark monochrome on light", True),
            tile(mk(style="transform:rotate(-18deg)"), C["bg"], "do not rotate", False),
            tile(mk("#F28A3C", "#3C9AF2"), C["bg"], "do not recolor with other hues", False),
            tile(
                mk(style="transform:scaleX(1.45)"), C["bg"], "do not stretch or squash", False
            ),
            tile(mk(), "#F2C744", "do not use in color on light or colored backgrounds", False),
        ]
    )

    html = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Stemwise — mark</title>
<link rel="icon" href="{uri(read("favicon-32.svg"))}">
<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,500;12..96,600&family=Figtree:wght@400;500;600&display=swap" rel="stylesheet">
<style>{css}</style></head><body><main>

<div class="hero"><div>
<p class="eyebrow">Stemwise · mark · refined direction D</p>
<h1>Two rooms</h1>
<p class="dim" style="margin-top:16px">The two trajectories of proposal D are the Mix: many parts that approach and come together. Beyond a gap is the Master, a single solid shape: the finished song. First the lines, then the room; first the lime, then the lilac.</p>
<p class="dim">The mark is made of two separable parts, <code>room-mix</code> and <code>room-master</code>: in the top bar the room of the phase you are in lights up.</p>
</div><div class="lock">{img("lockup.svg", 420, 71, alt="Stemwise")}</div></div>

<section><p class="eyebrow">1 · From proposal to refinement</p><h2>Same trajectories, another story</h2>
<p class="dim">The curves are those of D: horizontal run, curve, straight diagonal, curve, horizontal run, with continuous tangents. What changes is what they do: they come close without merging, and the arrow gives way to the second room.</p>
<div class="before-after" style="margin:22px 0 26px">
<figure><div class="well"><img src="{uri(ORIGINAL)}" width="200" height="200" alt=""></div><figcaption>Starting proposal</figcaption></figure>
<div class="arrowtxt">→</div>
<figure><div class="well">{img("mark.svg", 200)}</div><figcaption>Refinement</figcaption></figure></div>
<table><tr><th></th><th>The flaw</th><th>How it is solved</th></tr>
{"".join(f"<tr><td>{i + 1}</td><td>{a}</td><td>{b}</td></tr>" for i, (a, b) in enumerate(defects))}
</table></section>

<section><p class="eyebrow">2 · Construction</p><h2>100 grid, stroke 9</h2>
<div class="grid2" style="align-items:start"><div class="card">{construction}</div><div>
<h3 style="margin-top:0">The two Mix lines</h3>
<p class="dim">They start at 20 and 80 and arrive at 45.5 and 54.5: at the end they touch without overlapping (stroke 9, rounded caps and joins). Each curve has its control points at x = 31 and x = 45; the diagonal segment takes the central 40 % of the path between the two control points.</p>
<h3>The Master room</h3>
<p class="dim">A solid rectangle of 30 × 26 with radius 10, centered on the axis: taller than the two lines side by side (18) by almost one stroke, so that at a glance it weighs as much as the Mix. The radius is soft like that of the app's cards. The gap between the lines and the room is 7.5.</p>
<h3>Footprint</h3>
<p class="dim">From 4.5 to 95 in width, from 15.5 to 84.5 in height: the mark is centered in the square, wider than tall like the left-to-right reading it tells.</p>
<h3>Optical versions</h3>
<p class="dim">At 20, 32 and 16 px the mark is redrawn on the pixel grid: horizontal runs on whole rows, 2 px stroke (1.5 at 16), gap of at least 1.5 px, room with a 2 px radius.</p>
</div></div></section>

<section><p class="eyebrow">3 · App icon</p><h2>Continuous-corner squircle, 824 body</h2>
<p class="dim">Sizes in screen pixels: on a Retina Mac the 1024 icon takes 512 points, the 512 one takes 256.</p>
<div class="grid2" style="align-items:end;margin-top:18px">
<figure class="card big-icon" style="text-align:center">{img("icon-1024.svg", 512)}<figcaption>1024 px (512 pt)</figcaption></figure>
<div class="card"><div class="sizes">
<figure>{img("icon-1024.svg", 256)}<figcaption>512 px (256 pt)</figcaption></figure>
<figure>{img("icon-1024.svg", 128)}<figcaption>128</figcaption></figure>
<figure>{img("icon-1024.svg", 32)}<figcaption>32</figcaption></figure>
<figure>{img("icon-1024.svg", 16)}<figcaption>16</figcaption></figure>
</div></div></div>
<h3>In the Dock</h3>{dock}<p class="cap">Dock at 64 pt (the icon at 128 px on Retina), among four neutral icons.</p>
<h3>Designed for small sizes</h3>
<div class="zoom card">{zoom("favicon-32.svg", 32, 8, "favicon-32")}{zoom("favicon-16.svg", 16, 12, "favicon-16")}
<figure><div class="px" style="width:160px;height:160px"><img src="{uri(m20_lime)}" width="160" height="160" alt=""><div class="pg" style="background-size:8px 8px"></div></div><figcaption>mark-20 · 20 px, magnified 8×</figcaption></figure>
<figure style="display:flex;flex-direction:column;align-items:center;gap:14px">{img("favicon-32.svg", 32)}{img("favicon-16.svg", 16)}<figcaption>at real size</figcaption></figure>
</div></section>

<section><p class="eyebrow">4 · Top bar</p><h2>The mark also says which phase you are in</h2>
<p class="dim">The 20 px mark (<code>mark-20.svg</code>) has two groups: the app lights the room of the current phase in its accent and brings the other to 40 %. In the Mix the lines are lit, in the Master the solid room: the phase reads from position as well as from color.</p>
<div class="grid2" style="margin-top:18px"><div><p class="cap">Adopted: lit room</p>{bar("mix")}{bar("master")}</div>
<div><p class="cap">Fallback: the whole mark in the phase accent</p>{bar("mix", False)}{bar("master", False)}</div></div></section>

<section><p class="eyebrow">5 · Start screen and connection page</p><h2>The lockup at work</h2>
<div class="welcome" style="margin-top:18px"><div class="l">{mark(g.BASE, g.LIME, g.LILAC, 48, 100)}
<div class="name">Stemwise</div>
<div class="slogan">Mix and master the songs you have already recorded. An assistant proposes, you listen and decide.</div>
<div class="cta"><span>New project</span><span>Open…</span></div></div>
<div class="r"><h4>Recent projects</h4>
<div class="recent"><span>Paper Lanterns</span><small>Mix · 2 h ago</small></div>
<div class="recent"><span>Northbound</span><small>Master · yesterday</small></div>
<div class="recent"><span>Glass Harbour</span><small>Mix · 3 days ago</small></div>
<div class="recent"><span>Small Hours</span><small>Master · last week</small></div></div></div>
<h3>Browser page for connecting the assistant</h3>
<div class="connect"><header>{mark(g.P20, g.LIME, g.LILAC, 20, 20)}Stemwise</header>
<div>Stemwise is now connected to your OpenRouter account. You can close this tab and go back to the app.</div></div>
</section>

<section><p class="eyebrow">6 · Monochromes</p><h2>Two rooms even in a single color</h2>
<div class="mono" style="margin-top:18px">
<div class="d"><div class="row">{img("mono-light.svg", 120)}{mark(g.P20, g.TEXT, g.TEXT, 20, 20)}</div>{img("lockup-mono-light.svg", 300, 51, cls="lk")}<span class="cap">#F5F0F6 on dark</span></div>
<div class="lt"><div class="row">{img("mono-dark.svg", 120)}{mark(g.P20, g.BG, g.BG, 20, 20)}</div>{img("lockup-mono-dark.svg", 300, 51, cls="lk")}<span class="cap">#19121B on light</span></div>
</div></section>

<section><p class="eyebrow">7 · Clear space and minimum size</p><h2>Half a room all around</h2>
<div class="clear" style="margin-top:18px"><div class="card">{clear_svg}</div><div>
<p class="dim">The clear space is <b>x</b>, half the height of the Master room (13/100 of the mark's side), measured from the visible mark. In the lockup the same x applies, measured on the mark; between mark and name the distance is fixed (15 units with the name at 40).</p>
<h3>Minimum sizes</h3>
<table><tr><td></td><td>Mark</td><td>16 px. Under 24 px the optical drawings are used: <code>mark-20.svg</code> in the bar, <code>favicon-32.svg</code> and <code>favicon-16.svg</code> in the browser.</td></tr>
<tr><td></td><td>Lockup</td><td>110 px wide (name about 12 px tall). Smaller: the mark only.</td></tr>
<tr><td></td><td>Icon</td><td>All macOS sizes from 16 to 1024 from the same <code>icon-1024.svg</code>.</td></tr></table>
</div></div></section>

<section><p class="eyebrow">8 · Correct and incorrect uses</p><h2>Two colors, one shape, no variation</h2>
<div class="dos" style="margin-top:18px">{dos}</div></section>

<section><p class="eyebrow">9 · Verification</p><h2>The six criteria</h2>
<table class="crit">
<tr><td>Recognizability</td><td class="dim">A lime bracket that hands over a lilac room: two simple shapes and a gap, which are remembered. At 16 px, in the favicon and in the icon, lines, gap and room remain visible.</td></tr>
<tr><td>Relevance</td><td class="dim">It tells the two phases in order: many parts that come together (Mix), then a single solid thing (Master). It uses no audio symbols.</td></tr>
<tr><td>Without color</td><td class="dim">Lines against a solid shape: the two rooms remain two even in monochrome, and the top bar tells them apart by position.</td></tr>
<tr><td>Consistency</td><td class="dim">Rounded stroke like the app's icons, soft radii like cards and clips, phase accents, Bricolage Grotesque in the name.</td></tr>
<tr><td>Calm</td><td class="dim">Two colors on aubergine, no pure white or black, no effects; the gradient lives only in the icon background.</td></tr>
<tr><td>Durability</td><td class="dim">Essential geometry, with no references to fashions or technologies: it holds as long as the product has two phases.</td></tr>
</table></section>

</main></body></html>
"""
    (out / "presentation.html").write_text(html)
