# OpenCode Cost Guard

A macOS menu-bar utility for OpenCode Go users: shows the current price state of models (peak/off-peak), recommends a model for the user's task, and surfaces relevant catalog changes. This file is the project's glossary — vocabulary only, no implementation details.

## Catalog and plans

**Catalog**:
The full set of Model Entries the app knows about, with pricing and capabilities.
_Avoid_: model list, model registry

**Model Entry**:
A single model known to the app, with its Provider, capabilities, limits and prices.
_Avoid_: model, model record

**Provider**:
The organization that offers a Model Entry (DeepSeek, Xiaomi, Z.ai, …).
_Avoid_: vendor, brand

**Plan**:
A subscription that grants access to a Roster with Allowances (OpenCode Go, OpenCode Go Plus).
_Avoid_: tier, subscription

**Roster**:
The set of Model Entries included in a Plan.
_Avoid_: catalog (the Catalog is everything the app knows; the Roster is what a Plan includes)

**Catalog Change**:
A detected difference between Catalog Snapshots: additions, removals, price or capability changes.
_Avoid_: update, news

## Pricing

**Pricing Window**:
A time interval during which a Provider charges a distinct price for its models. DeepSeek currently has two states: Off-Peak (default) and Peak.
_Avoid_: schedule, tariff

**Peak Window**:
A recurring weekday interval, in UTC, when DeepSeek charges Peak Pricing (currently 2× Off-Peak).
_Avoid_: rush hour, surge

**Off-Peak Window**:
Every moment that is not inside a Peak Window.
_Avoid_: discount hour, night rate

**Chinese Public Holiday**:
A public holiday in mainland China, excluded from DeepSeek Peak Windows even on weekdays. Adjusted working weekends (makeup days) never create Peak Windows.
_Avoid_: holiday (qualify it), CN holiday

**List Price**:
A Provider's published price for using its model directly via API.
_Avoid_: official price, direct price

**Plan Price**:
The token price a Plan uses to meter Allowance consumption.
_Avoid_: net price, effective price

**Allowance**:
The spending budget a Plan grants for a Model Entry, expressed in dollars per period.
_Avoid_: quota, credits, limit

**Allowance Window**:
One of the metering periods against which Allowance consumption is measured (5-hour, weekly, monthly).
_Avoid_: reset window

## Task and recommendation

**Task Profile**:
The category of work the user declares for the model recommendation (Simple coding, Complex coding, Visual/UI, Agentic/Tools/MCP, Large refactor).
_Avoid_: use case, workload

**Recommendation**:
A suggested Model Entry for the active Task Profile, with a short, explainable justification.
_Avoid_: best model, suggestion

**Recommendation Engine**:
The deterministic, explainable component that produces Recommendations from the Catalog, the Pricing Window state and the active Task Profile.
_Avoid_: scoring, ranking

## App state

**Catalog Sync**:
The update of the local Catalog from the remote source.
_Avoid_: fetch, download

**Catalog Snapshot**:
The Catalog as it was at a point in time; used to detect Catalog Changes.
_Avoid_: dump, backup

**Overlay**:
Project-maintained curated data layered over the upstream Catalog: pricing windows, holiday calendar, plan limits and capability scores.
_Avoid_: patch, extension
