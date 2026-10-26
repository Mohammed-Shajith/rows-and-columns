# Decisions

Why Rows&Columns is the way it is. Newest at the bottom; each entry says what was decided, why,
and what it means in practice. Maintained alongside [`VERSION_LOG.xlsx`](VERSION_LOG.xlsx).

---

### D01 · Build on GridCraft instead of writing a spreadsheet from scratch
**2026-10-17.** The goal is an Excel-like app for personal use that is fast on big analytics files
and has good filters (other alternatives, OnlyOffice and Zoho Sheet, felt laggy). The first plan was
a from-scratch web app (hand-written formula engine and grid). GridCraft, an open-source (MIT /
Apache-2.0), native Rust spreadsheet with 500+ functions, XLSX, filters, PivotTables and charts,
already existed. **Why:** months of work saved; effort goes into what matters to the user (speed,
filters, look). **Means:** the code is Rust, and the work is fixing and improving GridCraft.

### D02 · GenOffice is not the base either
**2026-10-24.** GenOffice (Electron + Univer) was checked as an alternative. **Why not:** it fully
loads only up to 1,000,000 cells and sorts at most 200,000; the user's files are ~1.0M and ~4.2M
cells. It does have things worth learning from (save prompt on close, condition filters).

### D03 · Desktop app for macOS (Apple silicon) first
**2026-10-17.** Offline, opens real `.xlsx`/`.csv` files on disk, no account or server. **Means:**
releases are built for macOS arm64 first; GridCraft's Windows, Linux and web builds still exist
in the code but aren't tested by us yet.

### D04 · Flaws are logged first, then fixed in priority order
**2026-10-17.** Every problem found while using the app gets an ID (G01, G02…) with severity,
the cause found in the code, what Excel does, and a fix idea. Blockers first. **Before fixing,
check whether upstream GridCraft already fixed it**: GridCraft moves fast (0.7.0 → 0.12.1 in nine
days), and two of the first eight flaws (G01, G08) were fixed upstream before we started.
The flaw log itself stays private: its evidence includes screenshots of real business data.

### D05 · Name: Rows&Columns
**2026-10-26.** Chosen by the user. GitHub repository names can't contain `&`, so the repository
is `rows-and-columns`. In macOS menus the name is written `Rows&&Columns` in code, because the menu
library treats a single `&` as a keyboard-mnemonic marker and hides it.

### D06 · Rename what users see; keep GridCraft's internal names
**2026-10-26.** Window title, menus, About, messages, the `.app`, DMG, bundle id
(`io.github.mohammed-shajith.rows-and-columns`) and settings folder
(`~/Library/Application Support/Rows&Columns`) are Rows&Columns. Crates, binaries and code
identifiers stay `gridcraft-*`. **Why:** renaming internals would touch thousands of lines and make
every future merge of GridCraft's fixes conflict. **Means:** `cargo run -p gridcraft`, and the CLI
is still `gridcraft-cli`. Own bundle id and settings folder let Rows&Columns and GridCraft run side
by side without sharing settings.

### D07 · ArtCraft logos removed; GridCraft credited
**2026-10-26.** GridCraft's brand terms require any published modified version to remove the
ArtCraft name and logos and not present itself as an ArtCraft product. `docs/brand/` is deleted,
the About window and Help menu link to this repository and to GridCraft's, and README, NOTICE and
About credit GridCraft in plain text. GridCraft's copyright and both licence files are kept
(required by MIT and Apache-2.0); NOTICE adds a line for Rows&Columns' modifications.

### D08 · The update check points here, and doesn't run at start
**2026-10-26.** GridCraft's built-in updater looked at GridCraft's releases and offers "Update and
Restart", which would have replaced Rows&Columns (and its fixes) with stock GridCraft. It now
looks at this repository, and the automatic check at start is off until Rows&Columns publishes
releases. Help ▸ Check for Updates… still works.

### D09 · Versioning: 1.0 now, minor 1.x, major 2.0
**2026-10-26.** Chosen by the user. 1.0 is the first release (GridCraft 0.12.1 + first fixes + the
rename). Small changes are minor versions (1.1, 1.2…); big, visible steps are major (2.0, 3.0).
Each version is a row in `VERSION_LOG.xlsx` (rules on its second sheet) and a git tag (`v1.1.0`).
The app's own version number (`Cargo.toml`, About) follows the same numbers, independent of
GridCraft's; the log records which GridCraft version each release is based on.

### D10 · Repository: public, full history, GridCraft as `upstream`
**2026-10-26.** Public, as the user asked; MIT OR Apache-2.0 like GridCraft. GridCraft's full git
history is kept (credit, and it makes merging their later fixes possible); our work sits on top.
Locally the remotes are `origin` (this repository) and `upstream` (storytold/gridcraft).
GitHub Actions are turned off: GridCraft's workflows need ArtCraft's signing secrets and would
fail. Builds are made locally with `packaging/macos/package.sh`.

### D11 · G03: AutoFit reads the data once
**2026-10-26.** AutoFit on a whole-sheet selection scanned every used row for each of 16,384
columns (≈5.7 billion lookups on a 350k × 12 file; ~250 s). Now it clamps to the used columns and
makes one row-major pass keeping a maximum per column (~2 s, Excel's speed). Measurement and
padding are unchanged, so results equal per-column AutoFit (tested).
