# cue

cue is a lightweight CLI tool that watches your files and automatically runs a command every time you save. No config needed to get started.

> **Note:** cue is still under active development — usable and tested ,but features may change.

---

## Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Watch Mode](#watch-mode)
- [Watch file by extensions](#watch-file-by-extensions)
- [Tasks](#tasks)
- [Local Project Config](#local-project-config)
- [Debounce](#debounce)
- [Zero config mode](#zero-config-mode)

---

## Demo

**Initialize a project config**

<img src="assets/cue_init.png" width="400"/>

**Pick a task to run**

<img src="assets/cue.png" width="400"/>

**Watch and run on every save**

<img src="assets/cue_watch.png" width="400"/>

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

| Flag         | Short | Description                                  |
| ------------ | ----- | -------------------------------------------- |
| `--watch`    | `-w`  | Files or directories to watch                |
| `--run`      | `-r`  | Command to run on change                     |
| `--debounce` | `-d`  | Set the [Debounce](#debounce) value (ms)     |
| `--quite`    | `-q`  | Enables quite mode                           |
| `--no-clear` | -     | Disables clearing the screen after every run |

---

## Watch file by extensions

 Watch file by extensions using `-e` \ `--extensions`

 ```bash
cue -e rs -r "cargo run"
 ```

> **Note** this feature is not fully tested and may take lot of memory when there is a lot of files with the provided extensions

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

### Edit a task

```bash
cue task edit <name> -w <files or dirs>
cue task edit <name> -r "<command>"
cue task edit <name> -w <files or dirs> -r "<command>"
```

## Rename a task

```bash
cue task rename <current_name> <new_name>
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

| Flag         | Short | Description                                                             |
| ------------ | ----- | ----------------------------------------------------------------------- |
| `--watch`    | `-w`  | Override the task's watch paths                                         |
| `--run`      | `-r`  | Override the task's command                                             |
| `--debounce` | `-d`  | Set the [Debounce](#debounce) value (ms)                                |
| `--global`   | `-g`  | Force using global tasks instead of `cue.toml` in the working directory |
| `--quite`    | `-q`  | Enables quite mode (only when running tasks)                            |
| `--no-clear` | -     | Disables clearing the screen after every run                            |

---

## Local Project Config

By default, `cue run` looks for a `cue.toml` file in your current directory. If found, tasks are loaded from it instead of your global tasks. This lets you commit your cue setup alongside your project so your whole team can use the same commands.

You can use `cue init` to create a cue.toml

```bash
cue init
```

### cue.toml format

```toml
[tasks.build]
watch = ["src"]
run = "cargo build --release"

[tasks.test]
watch = ["src", "tests"]
run = "cargo test"
```

## templates 

You can make cue.toml templates for your programming language using `cue init <template>`

```bash
cue init rust
cue init go
cue init cpp
```

**supported languages:**\
      - Rust\
      - C\
      - C++\
      - Go (Golang)\
      - Zig\
      - Swift\
      - Haskell\
      - Node.js (JavaScript/TypeScript)\
      - Python\
      - Ruby\
      - PHP\
      - Lua\
      - Elixir\
      - Java\
      - Kotlin\
      - CSS / SCSS (Sass)\
      - Shell (Bash/Sh)

### How cue works

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

## Quite mode

Quite mode stops cue logs from appearing and just run the command when using `cue`, `cue run`, `cue run <taskname>` or `cue -w <files or dirs> -r "<command>"`

also quite mode doesn't stop errors or logs from `task` add, remove, edit or list

enable it with `--quite` / `-q`

```
cue -q
cue run -q
cue run my_task
cue -w src -r "cargo run" -q
```

## Zero-Config Mode

You can run `cue` or `cue run` with no flags

```bash
cue
cue run
```

If a `cue.toml` exists in your current directory, cue loads it

**If a default task is set**, cue runs it immediately:

```toml
"default" = "test"
```

```
[cue] loading tasks from 'cue.toml'
[cue] default task 'build' — running it
```

**If no default is set**, cue shows a picker so you can choose:

```
[cue] loading tasks from 'cue.toml'
? which task do you want to run?
> build
  test
  lint
```

Use `--global` / `-g` to skip `cue.toml` and load from your global tasks instead:

```bash
cue --global
```

> **Note:** If there is no `cue.toml` and no flags are provided, cue will show an error and ask you to use `-w` and `-r` directly or you can use the `-g` flag to load global tasks.

## How It Works

1. cue starts watching all the paths you provide
2. A file is saved — cue waits for the debounce window to pass
3. If the previous run is still going, cue kills it
4. cue runs your command fresh

## Contributing

Found a bug or have an idea? Open an issue or submit a pull request — contributions are welcome.

## License

MIT — [LICENSE](LICENSE)
