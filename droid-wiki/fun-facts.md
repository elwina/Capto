# Fun facts

## Latin name for a capture tool

"Capto" is a Latin verb meaning roughly "I take / seize / try to catch", which suits a screen recorder nicely. The project positions itself in the README as a spiritual successor to Captura, but it is an MIT-licensed clean-room implementation rather than a fork. The Latin reading is not stated anywhere in the repo, so it is a plausible gloss rather than a documented origin.

## The binary-name war: `capto` vs `capto-app`

Both the CLI and the desktop app might naturally be called "capto", but they cannot be. Cargo would write two `target/debug/capto.exe` files, and Windows paths are case-insensitive, so `Capto.exe` and `capto.exe` would collide anyway. The CLI owns the `capto` name while the desktop crate is `capto-app`, and the installer places the CLI at `cli\capto.exe` rather than as a separate download. The whole reasoning is spelled out in `docs/CLI.md`.

## Alt+F5..F8: a compact cluster dodging Alt+F4

The default recording hotkeys live in the Alt+F5 through Alt+F8 cluster. The choice is deliberate: Alt+F4 is reserved by Windows for closing windows, and the source comment notes that "F5-F8 is a compact cluster". A `normalize_hotkeys` step silently migrates older Ctrl+Shift defaults to the new cluster, so people upgrading don't have to reconfigure by hand. See `crates/capto-hooks/src/lib.rs`.

## Three NSIS commits in a row to protect the user PATH

On August 12 the installer's PATH handling went through three consecutive commits: "Fix NSIS PATH hooks to never overwrite the user PATH on failed reads or empty values", "Fix NSIS IfErrors usage in PATH hooks to native NSIS syntax", then "Use EnVar NSIS plugin for PATH to survive long user PATH values." That last one matters, because the classic Windows PATH registry trick silently truncates when the value is too long. Capto chased this hard so its installer could set up the `capto` command without ever clobbering the user's existing PATH.

## A debugging breadcrumb born from dropped frames

Inside the recording loop there is a deliberately noisy log line: if a single write to the FFmpeg process's stdin blocks for 250ms or more, Capto logs "slow ffmpeg write: capture outrunning encoder". It is profiling instrumentation that exists because users reported dropped frames, and it turned an intermittent symptom into a measurable threshold. It lives in `crates/capto-core/src/session.rs`.

## Related pages

- [Lore](lore.md) for the full timeline behind these details
- [Glossary](overview/glossary.md) for the terminology
- [Design decisions](background/design-decisions.md) for the rationale behind the choices
