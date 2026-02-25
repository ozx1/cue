# Changelog

All notable changes to cue will be documented in this file.

---

## [Unreleased]

> Features that are currently in development.

---

## [0.3.0] - 2026-02-25

## Added

- **Watching files by extensions** watch file that uses an certain extensions using `-e`
- **Rename tasks** 
-**Add templates** make `cue.toml` templates by `cue init <template>`
  supported languages:
    Rust
    C
    C++
    Go (Golang)
    Zig
    Swift
    Haskell
    Node.js (JavaScript/TypeScript)
    Python
    Ruby
    PHP
    Lua
    Elixir
    Java
    Kotlin
    CSS / SCSS (Sass)
    Shell (Bash/Sh)


## [0.2.4] - 2026-02-24

### Added

- **`cue init`** — generates a starter `cue.toml` in the current directory
- **`cue task edit`** — edit tasks in the global config file
- **Task Selection** — when no default is set, cue shows an interactive menu to choose a task when running `cue` and `cue run`
- **Quite mode** adding quite mode that stops cue logs using `--quite` \ `-q`
- **No clear** add `--no-clear flag`

## [0.2.3] - 2026-02-24

### Added

- **Zero-config mode** — run `cue` with no flags and it automatically loads tasks from `cue.toml`
- **Default task** — set a `default` field in `cue.toml` to skip the picker and run a task automatically
- **Task Selection** — when no default is set, cue shows an interactive menu to choose a task when running `cue` only
- **`cue init`** — generates a starter `cue.toml` in the current directory
- **`--global` / `-g` flag** — force loading global tasks even when a `cue.toml` exists

---

## [0.2.2] - 2026-02-23

### Added

- **Local project config** — `cue run` now looks for a `cue.toml` in the current directory before falling back to global tasks

### Fixed

- Debounce not applying correctly in all modes

---

## [0.2.1] - 2026-02-23

### Added

- **`--debounce` / `-d` flag** — configure the debounce window in milliseconds (default: 150ms)

---

## [0.2.0] - 2026-02-22

### Added

- **Tasks** — save a watch + command pair as a named task with `cue task add`
- **`cue task list`** — list all saved tasks
- **`cue task remove`** — remove a saved task
- **`cue run`** — run a saved task by name
- **Task overrides** — override a task's paths or command on the fly with `-w` or `-r`
- **Global task storage** — tasks are saved globally and available from any directory

### Changed

- Refactored `main.rs` into helper functions to reduce code duplication

---

## [0.1.0] - 2026-02-21

### Added

- **Watch mode** — watch files or directories and run a command on every save
- **Kill and restart** — if the previous run is still going, cue kills it and starts fresh
- **Debounce** — waits 150ms after the last change before running to avoid duplicate runs
- **Path and command validation** — cue checks that paths exist and the command is found before watching
- **Cross-platform install scripts** — install via curl on macOS/Linux or PowerShell on Windows
- **CI pipeline** — automatically builds binaries for Linux, macOS, and Windows on every release
