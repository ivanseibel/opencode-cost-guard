# Icon design brief — OpenCode Cost Guard

## What the product is

A macOS menu-bar utility for OpenCode Go users. It shows whether DeepSeek is in
peak or off-peak pricing, recommends a model for the current task profile, and
flags catalog changes. The name is "Cost Guard", but the product is really about
**timing and price windows**, not security.

## What we need

Three genuinely distinct **vector** concept directions for the app icon, plus a
monochrome menu-bar glyph derived from each. The current placeholder (a generic
shield with a green/red split) is rejected as too generic.

## Hard constraints

- App icon artwork: 1024×1024 SVG, transparent background, safe margin for the
  macOS squircle mask (keep the mark inside ~824×824 centered).
- Must read at **32 px** and **16 px**; avoid thin strokes, tiny text, fine
  texture, and more than 2–3 shapes.
- Menu-bar glyph: single-color (black) SVG with transparency, ~18×18 pt design
  grid, legible at 16–18 px; macOS renders it as a template (auto light/dark).
- No lettering, no wordmarks, no dollar signs, no shields, no robots/brains, no
  magnifying glasses, no generic "AI" sparkles.
- Flat or near-flat fills; at most one subtle gradient. Work on light and dark
  macOS backgrounds.
- Original work only. Do not imitate known app icons.

## Suggested territory (explore, don't just copy)

1. **Price window** — an abstract portal/slit split into two fields (green/red
   for off-peak/peak), maybe forming a subtle "C" or "G" in the negative space.
2. **Beacon** — a dot with one or two arcs, implying monitoring and alerts;
   strong silhouette at 16 px.
3. **Gate** — a minimal vertical aperture/threshold, architectural, no shield.

Each concept may use the palette: deep navy `#0B1220`, off-white `#F6F7F9`,
green `#22C55E`, red `#EF4444`, or a monochrome variant. Color is optional for
the app icon; the menu-bar glyph must be monochrome.

## Deliverables (per concept)

```
design/icon/concept-<n>/
├── icon.svg          # 1024 viewBox, transparent
├── glyph.svg         # monochrome menu-bar mark
├── icon-1024.png
├── icon-256.png
├── icon-128.png
├── icon-32.png
├── icon-16.png
└── notes.md          # one paragraph: idea, why it is distinctive, 16 px check
```

Plus `design/icon/README.md` comparing the three concepts and naming the one
that survives 16 px best.

## Rendering

`qlmanage` and `sips` are available; `cairosvg` is not. Example:

```bash
qlmanage -t -s 1024 -o design/icon/concept-1 design/icon/concept-1/icon.svg
mv design/icon/concept-1/icon.svg.png design/icon/concept-1/icon-1024.png
sips -z 256 256 design/icon/concept-1/icon-1024.png --out design/icon/concept-1/icon-256.png
# repeat for 128, 32, 16
```

## Out of scope

- No changes to `crates/app` code, `tauri.conf.json`, or release files.
- No icon integration, no rebuild, no commit outside the design branch.
- No publishing, tagging or pushing.
