# cue

> Automate your workflow — watch files, run commands, stay in flow.

cue watches your files and automatically runs a command every time you save. Save tasks as shortcuts so you never have to type the same command twice.

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

## Usage

### Watch mode

```bash
cue -w <files or dirs> -r "<command>"
```

| Flag | Description |
|------|-------------|
| `-w` | One or more files or directories to watch |
| `-r` | The command to run when a change is detected |
| `--debounce` | Milliseconds to wait before running (default: 150) |

> **Note:** Always wrap your command in quotes so its flags go to your command, not to cue.

### Examples

```bash
cue -w src -r "cargo run"
cue -w src tests -r "cargo test"
cue -w ./app -r "go run . --port 8080"
```

---

### Debounce

When you save a file, your editor often writes to disk multiple times in rapid succession. Without debounce, cue would run your command 3–5 times per save.

cue waits **150ms** by default after the last detected change before running your command — so you always get exactly one run per save.

You can tune this with `--debounce`:

```bash
# Wait 500ms before running (useful for slow editors or heavy builds)
cue -w src -r "cargo build" --debounce 500

# React faster (useful for simple scripts)
cue -w src -r "echo changed" --debounce 50
```

---

### Tasks

Save a command as a task so you can run it with a single word:

```bash
# Save a task
cue task add <name> -w <files or dirs> -r "<command>"

# Run it
cue run <name>

# List all saved tasks
cue task list

# Remove a task
cue task remove <name>
```

Tasks are saved **globally** on your machine so they're available from any directory.

#### Examples

```bash
cue task add build -w src -r "cargo build --release"
cue task add test -w src tests -r "cargo test"

cue run build
cue run test
```

#### Override a task on the fly

You can run a saved task with different paths or a different command without editing it:

```bash
# Use a different path
cue run build -w src/main.rs

# Use a different command
cue run build -r "cargo build"
```

---

## How it works

1. cue starts watching all the paths you provide
2. When a file is saved, cue waits for your editor to finish writing (debounce)
3. cue runs your command — if a previous run is still going, it kills it first
4. You always get the latest version running without any manual intervention

---

## Contributing

Found a bug or have an idea? Open an issue or submit a pull request — contributions are welcome.

---

## License

MIT — see [LICENSE](LICENSE) for details.