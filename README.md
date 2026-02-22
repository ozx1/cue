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


## Usage

### Watch mode

```
cue -w <files or dirs> -r "<command>"
```

| Flag | Description |
|------|-------------|
| `-w` | One or more files or directories to watch |
| `-r` | The command to run when a change is detected |

> **Note:** Always wrap your command in quotes so flags  are passed to your command, not to cue.

### Tasks

Save a command as a task so you can run it with a single word:

```
cue task add <name> -w <files or dirs> -r "<command>"
cue task remove <name>
cue task list
cue run <name>
```

You can also override the watch paths or command when running a task:

```
cue run <name> -w <other paths>
cue run <name> -r "<other command>"
```

Tasks are saved globally on your machine so they're available from any directory.


## Examples

**One-off watch**
```bash
cue -w src -r "cargo run"
cue -w src tests -r "cargo test"
cue -w ./app -r "go run . --port 8080"
```

**Save a task and run it**
```bash
cue task add build -w src -r "cargo build --release"
cue run build
```

**List saved tasks**
```bash
cue task list
```

**Remove a task**
```bash
cue task remove build
```


## How it works

1. cue starts watching all the paths you provide
2. When any file changes, cue runs your command
3. If the command is still running from a previous save, cue kills it and starts fresh

This means you always get the latest version running without any manual intervention.


## Contributing

Found a bug or have an idea? Open an issue or submit a pull request — contributions are welcome.


## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.