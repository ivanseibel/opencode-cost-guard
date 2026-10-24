# Refined icon concepts

Three evolutions of concept 1 (price window), following `../../icon-brief.md` and the maintainer feedback: concept 2 (beacon) is dropped, concept 3's gate silhouette is reused for glyph discipline, and the off-white tile is replaced by navy. Each folder holds `icon.svg` (1024, transparent corners, tile in the 824 px safe area, tile + 2 shapes), `glyph.svg` (18 pt grid, black), PNGs at 1024/256/128/32/16, and `notes.md`. PNGs are alpha-masked after `qlmanage` rendering. See `contact-sheet.png` for all sizes on light and dark, plus the glyphs on light and dark strips.

| # | Folder | Idea | Glyph |
|---|--------|------|-------|
| 1 | `authored-slit/` | Slit becomes a flat–45°–flat step, like a price tick. | Window cut by the same step |
| 2 | `layered-panes/` | Green and red panes overlap with a thin navy gap. | L-shaped back pane plus front pane |
| 3 | `window-gate/` | Arched doorway cut from the window; slit starts at the arch. | Rounded block with arched door |

## Comparison

- **Meaning fit:** 1 keeps the price-line idea and makes it ownable. 2 says "two states" but not price. 3 adds the gate metaphor at the cost of colour area.
- **Distinctiveness:** 1's step is the most authored detail. 2's offset panes can look like a "copy" icon. 3's arch can read as the letter "n".
- **App icon at 16 px:** 1 and 2 keep two clear colour masses. 3 loses the slit and shows a small red pillar.
- **Glyph at 16 px:** 3 has the cleanest single silhouette. 1 is legible but its step can read as "S/Z". 2 is clear but generic.
- **Light/dark:** all three use the navy tile and solid template glyphs, so they work on both.

## Verdict

- **Strongest app icon: 1 (Authored slit).** It is the closest to the product, reads at 16 px, and has the only distinctive brand detail.
- **Strongest glyph: 3 (Window + gate).** It is one solid shape with no fine detail. Caveat: check the "n" read. If it bothers the maintainer, glyph 1 is the better-linked alternative and pairs directly with icon 1.

Suggested next step: take icon 1 and glyph 3's discipline (a single shape) into a combined final, for example by testing glyph 1 with a slightly wider gap at 16 px.
