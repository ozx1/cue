# cue

> A file watcher that runs your command the moment you save.

No config files. No setup. Just tell cue what to watch and what to run.

```bash
cue -w ./src -r cargo run
```

That's it. Every time you save, your command runs.

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

```
cue -w <paths>... -r <command>...
```

**Watch a single file**
```bash
cue -w main.py -r python main.py
```

**Watch multiple files and directories**
```bash
cue -w src tests config.toml -r cargo test
```

**Works with any command**
```bash
cue -w ./styles -r npm run build
cue -w ./src -r cargo build --release
cue -w ./app -r go run .
```

---

## How it works

cue watches your files for changes and kills + restarts your command automatically. If your command is still running when you save again, cue kills it and starts fresh — so you're always running the latest version.

---

## Contributing

Found a bug or have an idea? Open an issue or submit a pull request — contributions are welcome.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
