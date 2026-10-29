# Rows&Columns

**A fast, Excel-style spreadsheet for Mac that stays smooth on big data files.**

Rows&Columns opens the CSV and XLSX exports that analytics work runs on (hundreds of thousands of
rows) and keeps them responsive: scrolling, filtering, sorting, formulas and resizing columns
should feel as quick as Excel. It's open source and runs natively, with no subscription and no
account.

> Rows&Columns is a modified version of **[GridCraft](https://github.com/storytold/gridcraft)**,
> the open-source, clean-room Excel-style spreadsheet written in Rust by the ArtCraft team and the
> GridCraft contributors. All of GridCraft's engine, 500+ worksheet functions, XLSX support,
> PivotTables and charts come from there. Rows&Columns adds fixes and changes aimed at speed and
> polish on large files. It is not affiliated with or endorsed by the ArtCraft Team.

## Status

**Version 1.3** (based on GridCraft 0.13.0 plus newer upstream fixes). Every change is listed in
[`VERSION_LOG.xlsx`](VERSION_LOG.xlsx), and the reasoning behind them in
[`DECISIONS.md`](DECISIONS.md).

What Rows&Columns changes from GridCraft so far:

| | What | Effect |
| --- | --- | --- |
| Speed | AutoFit column width (select all, double-click a column border) makes one pass over the data | 350,000 rows × 12 columns: **~250 s → ~2 s** |
| Filters | The cursor turns into a pointing hand over filter buttons | matches Excel |
| Name | Rows&Columns: its own app name, bundle id, settings folder and update channel | runs side by side with GridCraft |
| Big files | Sheets with more than ~1,000,000 cells load completely (were cut at ~71,000 rows) | no silent data loss (1.1) |
| Look | Dark theme colours measured from Excel for Mac | Excel-like dark mode (1.1) |
| Filters | Excel-style filter menu: Auto Apply, "Choose One" conditions (contains, greater than, between, Top 10…), date columns as Year › Month › Day | (1.2) |
| Dates | Text to Columns with Date: DMY/MDY/YMD…; CSV columns of day-month-year dates open correctly | (1.3) |

Coming next: keeping the window responsive during long operations (progress, Esc to cancel), and new
icons and dialogs.

## Build and run (macOS)

You need [Rust](https://rustup.rs) (stable).

```sh
cargo run --release -p gridcraft                      # run the app
cargo run --release -p gridcraft -- book.csv          # open a file
packaging/macos/package.sh --arch aarch64             # build Rows&Columns.app and a DMG in dist/release/
```

The internal crate and binary names are still `gridcraft*` (`gridcraft-engine`,
`gridcraft-cli`…), so changes from GridCraft can keep flowing in. The command-line tool is
`gridcraft-cli`; see [`docs/cli.md`](docs/cli.md).

## License and credits

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, like
GridCraft. Required notices, including GridCraft's copyright, are in [NOTICE](NOTICE). Bundled
assets and their licences are listed in [ATTRIBUTION.md](ATTRIBUTION.md).

- GridCraft: Copyright (c) 2026 ArtCraft Team and the GridCraft contributors.
- Rows&Columns modifications: Copyright (c) 2026 Mohammed Shajith.

<sub>Microsoft and Excel are trademarks of Microsoft Corporation. Rows&Columns is an independent
project, not affiliated with or endorsed by Microsoft. These names are used only to describe the
workflows it is compatible with.</sub>
