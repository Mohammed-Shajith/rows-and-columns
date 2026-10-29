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

### D12 · Stop using GridCraft itself; keep reading its fixes
**2026-10-27.** Chosen by the user: Rows&Columns is the only app. The official GridCraft app was
moved to the Trash, and pushing to GridCraft's repository is blocked in the local checkout
(`upstream` push URL disabled). GridCraft's code is still *read*: newer upstream commits are merged
in when they bring fixes (55 on 2026-10-27), because that's free work. Every change we make goes
only to this repository.

### D13 · How upstream updates are merged
**2026-10-27.** Upstream reorganises code often (on 2026-10-27 it split large files into modules).
When a merge conflicts because code moved, we take upstream's new layout and re-apply our change in
the new place, rather than keeping our old copy of the file. Our changes are small and listed per
version in `VERSION_LOG.xlsx`, which keeps that easy. A merge is part of the next version, not a
version of its own.

### D14 · Data-loss bugs jump the queue
**2026-10-27.** G09 (big XLSX sheets silently truncated at ~71k rows) was fixed before the planned
colour work. Anything that can lose the user's data or crash goes first, whatever the agreed order.

### D15 · Dark theme colours come from measuring Excel
**2026-10-27.** The dark palette is set from pixel samples of Excel for Mac screenshots (frame,
ribbon, headings, selection, dialogs, gridlines), not from guesses. Colour values describe a look,
not Microsoft's artwork; no icons, images or theme files were copied. One new colour setting,
`header_all_text`, was needed because Excel uses dark letters on a fully selected heading and light
letters on a partly selected one. Every text/background pair stays in the WCAG contrast test.

### D16 · The filter menu: Excel's layout, our own file
**2026-10-29.** The AutoFilter dropdown (G06, G12) follows Excel for Mac's popover: column name
as title, side-by-side sort, "Choose One" conditions, search, values, Auto Apply, Apply Filter and
Clear Filter. Choices made:
- **Auto Apply is on by default**, as in Excel, and remembered while the app is open.
- **Conditions open inline** inside the menu rather than as a separate dropdown, which the app
  would treat as a click outside and close the menu.
- **Date columns become a Year › Month › Day tree** when every non-blank value is a date. Ticks
  still apply as an ordinary value filter, so files stay readable by Excel and GridCraft.
- **Values are read once per opening** and drawn a screenful at a time, so huge columns stay
  smooth.
- **The code lives in a new file** (`grid/filter_popup.rs`), with GridCraft's menu function handing
  over to it, so GridCraft's later changes to the old menu merge with little conflict (D13).
- Not yet: filter and sort by colour.

### D17 · Our version numbers stay ours
**2026-10-29.** GridCraft moved to 0.13.0. Rows&Columns keeps its own numbering (1.2 now); each
merge resolves the version line in `Cargo.toml` in our favour, and `VERSION_LOG.xlsx` records which
GridCraft version a release includes.

### D18 · Day-month-year dates are decided per column
**2026-10-29.** A single date like `03/04/2025` can't say whether it's day or month first, but a
column comes from one source in one format. On CSV opening, a column is read **day first** when some
date only works day first (`16-08-2023`) and none only month first (`08/16/2023`). Then its
ambiguous dates follow (`7/8/2024` = 7 August). Columns with no evidence, or evidence both ways, read
as before. The user's region (DMY for India) could later decide the unclear ones. Text to Columns
gets Excel's explicit choice (Date: DMY…) for everything else, and re-reads dates *as shown*, which
fixes columns already mis-read. Impossible dates never roll over (`DATE()` does; reading text must
not).

### D19 · Long work moves off the screen thread one piece at a time
**2026-10-29.** G07 asked for heavy work in the background with progress and Esc to cancel.
Measured first on a 350k × 12 file: opening (~2 s) and Remove Duplicates (~7 s) were the worst.
- Remove Duplicates was mostly wasted work (a #SPILL! scan per changed cell), so it was **made
  fast** (~2 s) rather than moved.
- **Opening** is split into a session-free read-and-parse that runs on a worker thread and a quick
  adoption step (`file.openPrepared`) that runs as a normal command, under the crash guard. Only
  files of 2 MB or more take the background path, so small files and every scripted, CLI or
  control-channel open behave exactly as before. Esc stops waiting; the worker's result is dropped.
- **Saving** stays on the screen thread for now (~1 s at this size): a background save must be
  finished before quitting, which touches the Save / Don't Save / Cancel flow. It's next.

### D20 · Our own checkbox and primary button; icons chosen with the owner
**2026-10-29.** egui can't fill a checkbox by its ticked state, and its theme hook for that is
experimental, so Rows&Columns draws its own (`rc_widgets.rs`) with the same accessibility role,
label and state, and every checkbox uses it. Primary buttons get Excel's measured green with white
text. The 121 code-drawn icons are not redrawn yet: their style is a matter of taste, so the owner
picks a direction from samples first (redraw ours in a richer style, or adopt an open-source set
under a licence that allows it; never Microsoft's).

### D21 · Icons: our own drawings, made richer
**2026-10-30.** The owner chose to keep Rows&Columns' own code-drawn icons and make them livelier,
rather than adopt an open-source set (Phosphor, Tabler) or anything resembling Microsoft's. Two
layers: the shared drawing helpers give every coloured shape subtle depth and draw lines slightly
bolder (lifts all icons at once), and the flattest icons (thin outlines in the text colour) are
redrawn with filled colour, paper and badges. An icon sheet test (`RC_ICON_SHEET`) renders them in
both themes for review. More icons can follow the same pattern.

### D22 · Background saving keeps "unsaved" honest
**2026-10-30.** Big workbooks (200,000+ cells) save on a worker thread. Copy-on-write makes the
snapshot free, and the snapshot (not the live workbook) becomes the saved version, so anything
typed during the write is still marked unsaved. Saves that something waits for stay immediate: the
quit prompt's Save (the close right after must see the saved state) and scripted saves. Quitting
while a save runs waits for it, then goes through the normal unsaved-changes check.

### D23 · A missing narrow font gets a narrow substitute
**2026-10-30.** Excel files default to Aptos Narrow, which a Mac without Office fonts doesn't have.
Falling back to a regular-width font made numbers overflow (2E+06) and text clip early. A missing
font whose name says Narrow or Condensed now draws with an installed narrow font (Arial Narrow
ships with macOS). Microsoft's Aptos files themselves are never read or bundled.
