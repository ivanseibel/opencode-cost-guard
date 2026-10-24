# MVP — v0.1.0 (macOS)

Agreed 2026-10-23. Scope is deliberately small. Everything here is v0.1 unless marked otherwise. Vocabulary: [CONTEXT.md](../CONTEXT.md). Decision records: [adr/](./adr/).

## Must

1. **Menu-bar tray**
   - Title: `🟢 off` / `🔴 2×` (DeepSeek Pricing Window state) + `⚠` appended while a Catalog Change is unacknowledged; the menu header shows the full `🟢 DS off` / `🔴 DS 2×` label.
   - Title updates only on state transitions (mitigates tauri#12060). No styling API — text/emoji only.
2. **Menu**
   - DeepSeek state + next Pricing Window transition (local time; UTC shown in parentheses);
   - current Plan Prices (input / output / cache read), peak-aware;
   - active Task Profile with a switcher;
   - Recommendation + one-line reason, plus top alternatives;
   - Sync Catalog (manual trigger), Settings, Quit.
3. **Notifications**
   - On entering Peak and on returning Off-Peak (individually toggleable). Notified at most once per transition.
   - No pre-peak warning in v0.1 (v0.2).
4. **Catalog Sync**
   - Fetch `https://models.opencode.ai/api.json` at start, every 6 h, and on manual trigger; ETag-aware.
   - Schema validation; on failure fall back to last valid snapshot, then to the bundled snapshot. Never crash on bad remote data.
   - Snapshot diff → Catalog Change set → `⚠` badge (dismiss clears it).
5. **Catalog + Overlays**
   - Full OpenCode Go Roster (~35–37 Model Entries).
   - Curated TOML Overlays with `source` + `lastVerified`: DeepSeek peak windows + 2× multiplier, CN holiday calendar (makeup-weekend rule), Go monthly limits, per-profile capability scores.
   - Capability scores: coarse tier + flags (tools/vision/reasoning/context) for the whole Roster; detailed scores for commonly used models. Promo/free models flagged with terms (e.g. "data may be used") and surfaced in reasons.
6. **Recommendation Engine v1**
   - Deterministic and explainable. Hard filters per Task Profile (e.g. Agentic/Tools/MCP requires tool support; Visual/UI requires vision), then weighted score: peak-adjusted Plan Price (normalized across Roster), curated capability score, static allowance headroom, context fit.
   - Output: top pick + short reason, plus top-3 alternatives. Ties break toward cheaper.
7. **Settings**
   - Config file + minimal window: autostart toggle, notifications toggles, default Task Profile, sync interval, ignored models list.
8. **Platform behavior**
   - `LSUIElement` in bundled builds: no Dock icon, no Cmd-Tab entry. Autostart via LaunchAgent.
   - Core crate has zero Tauri/macOS dependencies (crate boundary).
9. **Privacy/security**
   - Zero credentials; no OpenCode files, DB or CLI reads; no telemetry; all data local.

## Config keys (v0.1)

```json
{
  "autostart": false,
  "notifications": { "peakEnter": true, "peakExit": true },
  "defaultProfile": "simple-coding",
  "syncIntervalHours": 6,
  "ignoredModels": []
}
```

## Acceptance criteria

1. **Peak state correct at every edge**: window entry/exit, weekends, CN holidays, makeup weekends, month/year boundaries — unit-tested with a fixed clock, including UTC↔local display conversion.
2. **Glanceable**: peak state readable from the menu bar in under 2 seconds, zero clicks.
3. **Live updates**: state changes without restart (timer-driven; no busy polling).
4. **Notifications**: delivered on enter/exit Peak when enabled; never spammed.
5. **Recommendation**: each of the 5 Task Profiles yields a pick with a visible reason; table-driven tests assert expected winners per profile × peak/off-peak (e.g. at peak, Simple coding favors cheaper off-peak alternatives over DeepSeek).
6. **Catalog Sync**: updates without recompiling; invalid remote payload never breaks the app (fallback + log).
7. **Change badge**: `⚠` appears after a simulated Catalog Change and clears when dismissed.
8. **Idle footprint**: < 100 MB RSS and < 0.5 % CPU after the first week of use (document the measured floor; adjust only with evidence).
9. **Portability proof**: core crate builds and tests on Ubuntu in CI.
10. **Dogfooding gate**: the author uses it daily for 2 weeks without disabling it and with no trust-breaking incident (wrong peak state, crash, notification spam, price lie).
11. **Autostart + Quit**: autostart survives reboot; Quit is clean; no Dock icon ever in the bundled build.

## Out of scope (v0.1)

- **v0.2 (Should)**: pre-peak warning ("X min before Peak"), catalog-change notifications, styled settings window, favorites, app icon.
- **Phase 2+**: history, allowance tracking/estimates, budget alerts, compare view, OpenCode integration, Linux, smart routing.
