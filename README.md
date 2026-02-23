# cue

cue is a lightweight CLI tool that watches your files and automatically runs a command every time you save. No config needed to get started.

---

## Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Watch Mode](#watch-mode)
- [Tasks](#tasks)
- [Local Project Config](#local-project-config)
- [Debounce](#debounce)

---

## Installation

**macOS / Linux**
```bash
curl -sSf https://raw.githubusercontent.com/ozx1/cue/master/install.sh | sh
```

**Windows (PowerShell)**
```powershell
iwr https://raw.githubusercontent.com/ozx1/cue/master/install.ps1 -UseBasicParsing | iex
```

**From source**
```bash
cargo install --path .
```

---

## Quick Start

```bash
cue -w src -r "cargo run"
```

That's it. Every time you save a file inside `src`, cue runs `cargo run`.

---

## Watch Mode

Watch one or more files or directories and run a command on every save.

```bash
cue -w <files or dirs> -r "<command>"
```

**Examples**
```bash
# Watch a directory
cue -w src -r "cargo run"

# Watch multiple directories
cue -w src tests -r "cargo test"

# Watch a single file
cue -w main.go -r "go run main.go"
```

> **Tip:** Always wrap your command in quotes so its flags go to your command, not to cue.

**Flags**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--watch` | `-w` | Files or directories to watch | — |
| `--run` | `-r` | Command to run on change | — |
| `--debounce` | `-d` | Milliseconds to wait before running | `150` |

---

## Tasks

Save a watch + command pair as a named task so you can run it with a single word. Tasks are saved globally on your machine and available from any directory.

### Add a task
```bash
cue task add <name> -w <files or dirs> -r "<command>"
```

### Run a task
```bash
cue run <name>
```

### List all tasks
```bash
cue task list
```

### Remove a task
```bash
cue task remove <name>
```

### Examples
```bash
# Save tasks
cue task add build -w src -r "cargo build --release"
cue task add test -w src tests -r "cargo test"

# Run them
cue run build
cue run test
```

### Override a task

Run a saved task with a different path or command without editing it:

```bash
# Different path
cue run build -w src/main.rs

# Different command
cue run build -r "cargo build"
```

**Flags**

| Flag | Short | Description |
|------|-------|-------------|
| `--watch` | `-w` | Override the task's watch paths |
| `--run` | `-r` | Override the task's command |
| `--debounce` | `-d` | Set the debounce value (ms) |
| `--global` | `-g` | Force using global tasks instead of `cue.toml` in the working directory |

---

## Local Project Config

By default, `cue run` looks for a `cue.toml` file in your current directory. If found, tasks are loaded from it instead of your global tasks. This lets you commit your cue setup alongside your project so your whole team can use the same commands.

### cue.toml format

```toml
[tasks.build]
watch = ["src"]
run = "cargo build --release"

[tasks.test]
watch = ["src", "tests"]
run = "cargo test"
```

### How it works

- If `cue.toml` exists in the current directory tasks load from it
- If not tasks load from global config
- Use `--global` / `-g` to force global tasks even when a `cue.toml` exists

```bash
# Uses cue.toml if present (default)
cue run build

# Forces global tasks
cue run build --global
```

---

## Debounce

When you save a file, your editor often writes to disk multiple times in quick succession. Without debounce, cue would run your command 3–5 times per save.

cue waits **150ms** after the last detected change before running — so you always get exactly one run per save.

change it with `--debounce` / `-d`:

```bash
cue -w src -r "cargo build" -d 500
```

---

## How It Works

1. cue starts watching all the paths you provide
2. A file is saved — cue waits for the debounce window to pass
3. If the previous run is still going, cue kills it
4. cue runs your command fresh



## Contributing

Found a bug or have an idea? Open an issue or submit a pull request — contributions are welcome.


## License

MIT — [LICENSE](LICENSE)