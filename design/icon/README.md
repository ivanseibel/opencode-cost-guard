# Icon concept explorations

Three vector directions for the app icon and menu-bar glyph, following `../icon-brief.md`. Each folder holds `icon.svg` (1024, transparent corners, tile inside the 824 px safe area), `glyph.svg` (18 pt grid, black, template-ready), PNGs at 1024/256/128/32/16 and `notes.md`. PNGs are transparent; `qlmanage` output was alpha-masked afterwards because it renders SVGs on white.

| # | Concept | Shapes | Colour | Idea |
|---|---------|--------|--------|------|
| 1 | Price window | tile + 2 fields | navy, green, red | Rounded window cut by a diagonal slit into off-peak (green) and peak (red). |
| 2 | Beacon | tile + dot + arcs | navy, green, off-white | A live dot with two broadcast arcs: monitoring and alerts. |
| 3 | Gate | tile + monolith + door | off-white, navy, green | A solid block with one lit arched threshold. |

## Comparison

- **Meaning fit:** 1 is closest to the product (peak/off-peak price windows). 2 says "monitoring" but not price. 3 is abstract and architectural.
- **Distinctiveness:** 1 and 3 are original-looking. 2 resembles the common RSS/Wi-Fi glyph, so it is the weakest on ownership.
- **Glyph:** 3 is the cleanest menu-bar silhouette. 1 works but its slit is the only fine detail. 2 has the thinnest strokes.
- **Light/dark:** 1 and 2 use navy tiles and hold on both. 3 uses an off-white tile that disappears on light backgrounds.

## 16 px verdict

**Concept 1 (Price window) survives 16 px best as an app icon:** two large colour masses stay readable even when the slit blurs. Concept 3 has the best monochrome glyph silhouette but a weak light tile at small sizes. Concept 2's arcs fall to about 1 px strokes and are the first to break.

A sensible next step if concept 1 is chosen: reuse the gate's simple-silhouette discipline for the glyph and consider widening the slit slightly for 16 px.
