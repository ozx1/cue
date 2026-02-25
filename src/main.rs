use chrono::Utc;
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::Select;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use serde::{Deserialize, Serialize};
use shell_words::split;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use terminal_size::{Width, terminal_size};
use walkdir::WalkDir;

const CUE: &str = "[cue]";
const DEBOUNCE_MS: u64 = 150;

macro_rules! log {
    ($quiet:expr, $($arg:tt)*) => {
        if !$quiet {
            println!($($arg)*);
        }
    };
}

#[derive(Serialize, Deserialize, Default)]
struct CueConfig {
    default: Option<String>,
    tasks: HashMap<String, Task>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    watch: Vec<String>,
    run: Option<String>,
    extensions: Option<Vec<String>>,
}

#[derive(Parser)]
#[command(
    name = "cue",
    version,
    about = "Automate your workflow — watch files, run commands, stay in flow."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, num_args = 1..)]
    watch: Vec<String>,
    #[arg(short, long)]
    run: Option<String>,
    #[arg(short, long, num_args = 1..)]
    extensions: Option<Vec<String>>,
    #[arg(long, short, default_value_t = DEBOUNCE_MS)]
    debounce: u64,
    #[arg(long, short)]
    global: bool,
    #[arg(long, short)]
    quiet: bool,
    #[arg(long, short)]
    no_clear: bool,
}

#[derive(Subcommand)]
enum Commands {
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    Run {
        name: Option<String>,
        #[arg(short, long, num_args = 1..)]
        watch: Option<Vec<String>>,
        #[arg(short, long)]
        run: Option<String>,
        #[arg(short, long, num_args = 1..)]
        extensions: Option<Vec<String>>,
        #[arg(long, short, default_value_t = DEBOUNCE_MS)]
        debounce: u64,
        #[arg(long, short)]
        global: bool,
        #[arg(long, short)]
        quiet: bool,
        #[arg(long, short)]
        no_clear: bool,
    },
    Init {
        template: Option<String>,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    #[command(group = clap::ArgGroup::new("source").required(true).multiple(true))]
    Add {
        name: String,
        #[arg(short, long, num_args = 1.., group = "source")]
        watch: Vec<String>,
        #[arg(short, long)]
        run: String,
        #[arg(short, long, num_args = 1.., group = "source")]
        extensions: Option<Vec<String>>,
    },
    Remove {
        name: String,
    },
    List,
    #[command(group = clap::ArgGroup::new("edit_fields").required(true).multiple(true))]
    Edit {
        name: String,
        #[arg(short, long, num_args = 1.., group = "edit_fields")]
        watch: Vec<String>,
        #[arg(short, long, group = "edit_fields")]
        run: Option<String>,
        #[arg(short, long, num_args = 1.., group = "edit_fields")]
        extensions: Option<Vec<String>>,
    },
    Rename {
        name: String,
        new_name: String,
    },
}

struct ParsedCommand {
    cmd: String,
    args: Vec<String>,
}

fn parse_command(run: &str) -> ParsedCommand {
    let parts = split(run).unwrap_or_else(|e| {
        eprintln!("{} failed to parse command: {}", "Error:".red(), e);
        process::exit(1);
    });
    if parts.is_empty() {
        eprintln!("{} empty command", "Error:".red());
        process::exit(1);
    }
    ParsedCommand {
        cmd: parts[0].clone(),
        args: parts[1..].to_vec(),
    }
}

fn load_config(from_global: bool) -> CueConfig {
    if from_global {
        confy::load::<CueConfig>("cue", None).unwrap_or_else(|_| {
            eprintln!("{} failed to read config", "Error:".red());
            process::exit(1);
        })
    } else {
        let content = fs::read_to_string("cue.toml").unwrap_or_else(|_| {
            eprintln!("{} failed to read cue.toml", "Error:".red());
            process::exit(1);
        });
        toml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("{} invalid cue.toml: {}", "Error:".red(), e);
            process::exit(1);
        })
    }
}

fn resolve_config(global: bool, quiet: bool) -> CueConfig {
    if global {
        log!(quiet, "{} loading global tasks", CUE.green());
        load_config(true)
    } else if Path::new("cue.toml").exists() {
        log!(quiet, "{} loading tasks from 'cue.toml'", CUE.green());
        load_config(false)
    } else {
        log!(quiet, "{} loading global tasks", CUE.green());
        load_config(true)
    }
}

fn pick_task(config: &CueConfig, name: Option<String>, quiet: bool) -> String {
    if let Some(n) = name {
        return n;
    }
    if let Some(d) = &config.default {
        log!(quiet, "{} default task '{}' — running it", CUE.green(), d);
        return d.clone();
    }
    let tasks: Vec<&String> = config.tasks.keys().collect();
    let choice = Select::new()
        .with_prompt("which task do you want to run?")
        .items(&tasks)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("{} cancelled", "Error:".red());
            process::exit(1);
        });
    tasks[choice].to_string()
}

fn validate_paths(paths: &[&Path], quiet: bool) {
    log!(quiet, "{} checking paths...", CUE.green());
    for path in paths {
        if path.exists() {
            log!(
                quiet,
                "  {} {}",
                path.display().to_string().cyan(),
                "exists".green()
            );
        } else {
            eprintln!("{} '{}' doesn't exist", "Error:".red(), path.display());
            process::exit(1);
        }
    }
}

fn validate_command(command: &ParsedCommand, quiet: bool) {
    log!(quiet, "{} checking command...", CUE.green());
    if which::which(&command.cmd).is_err() {
        eprintln!("{} command '{}' not found", "Error:".red(), command.cmd);
        process::exit(1);
    }
    log!(quiet, "  '{}' {}", command.cmd, "found".green());
}

fn find_by_extensions(extensions: &[String]) -> Vec<PathBuf> {
    WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            p.extension()
                .map(|e| extensions.iter().any(|ext| ext.as_str() == e))
                .unwrap_or(false)
        })
        .collect()
}

fn resolve_paths(watch: Vec<String>, extensions: Option<Vec<String>>) -> Vec<String> {
    match extensions {
        Some(exts) if !exts.is_empty() => find_by_extensions(&exts)
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        _ => watch,
    }
}

fn run_task(
    config: &CueConfig,
    name: Option<String>,
    watch_override: Option<Vec<String>>,
    run_override: Option<String>,
    extensions_override: Option<Vec<String>>,
    debounce: u64,
    quiet: bool,
    no_clear: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = pick_task(config, name, quiet);
    let task = config.tasks.get(&name).cloned().unwrap_or_else(|| {
        eprintln!("{} task '{}' not found", "Error:".red(), name);
        process::exit(1);
    });

    let extensions = extensions_override.or(task.extensions);
    let watch_strs = resolve_paths(watch_override.unwrap_or(task.watch), extensions);
    let run_str = run_override.or(task.run).unwrap_or_else(|| {
        eprintln!(
            "{} task has no run command — provide one with -r",
            "Error:".red()
        );
        process::exit(1);
    });

    let paths: Vec<&Path> = watch_strs.iter().map(|s| Path::new(s)).collect();
    let command = parse_command(&run_str);
    validate_paths(&paths, quiet);
    validate_command(&command, quiet);
    start_watcher(paths, command, &run_str, debounce, quiet, no_clear)
}

fn start_watcher(
    paths: Vec<&Path>,
    command: ParsedCommand,
    run_str: &str,
    debounce: u64,
    quiet: bool,
    no_clear: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
        / 2;
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(tx)?;

    log!(
        quiet,
        "{} watching — will run '{}' on changes",
        CUE.green(),
        run_str
    );

    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    let mut last_run = Instant::now();
    let mut child = Some(
        Command::new(&command.cmd)
            .args(&command.args)
            .spawn()
            .expect("failed to spawn command"),
    );

    for event in rx {
        match event {
            Ok(e) if matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_)) => {
                if last_run.elapsed() < Duration::from_millis(debounce) {
                    continue;
                }
                last_run = Instant::now();

                if let Some(mut c) = child.take() {
                    c.kill().ok();
                    c.wait().ok();
                }

                let changed = e
                    .paths
                    .first()
                    .map(|p| dunce::canonicalize(p).unwrap_or(p.clone()))
                    .unwrap_or(PathBuf::new());

                let file_name = changed
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or(changed.display().to_string());

                if no_clear {
                    log!(quiet, "{}", "_".repeat(width));
                } else {
                    clearscreen::clear().unwrap();
                }
                log!(
                    quiet,
                    "{} {} changed at {}",
                    CUE.green(),
                    file_name.cyan(),
                    Utc::now().format("%H:%M:%S")
                );
                log!(quiet, "{}", "_".repeat(width));

                child = Some(
                    Command::new(&command.cmd)
                        .args(&command.args)
                        .spawn()
                        .expect("failed to spawn command"),
                );
            }
            Err(e) => eprintln!("{} watch error: {:#?}", "Error:".red(), e),
            _ => {}
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Task { action }) => {
            let mut config: CueConfig = load_config(true);
            match action {
                TaskAction::Add {
                    name,
                    watch,
                    run,
                    extensions,
                } => {
                    config.tasks.insert(
                        name.clone(),
                        Task {
                            watch,
                            run: Some(run),
                            extensions,
                        },
                    );
                    confy::store("cue", None, config)?;
                    println!("{} task '{}' saved", CUE.green(), name);
                }
                TaskAction::Remove { name } => {
                    if config.tasks.remove(&name).is_some() {
                        confy::store("cue", None, config)?;
                        println!("{} task '{}' removed", CUE.green(), name);
                    } else {
                        eprintln!("{} task '{}' not found", "Error:".red(), name);
                        process::exit(1);
                    }
                }
                TaskAction::List => {
                    if config.tasks.is_empty() {
                        println!("{} no saved tasks", CUE.green());
                    } else {
                        println!("{} saved tasks:", CUE.green());
                        for (name, task) in &config.tasks {
                            println!(
                                "  {} — watch: {:?} | extensions: {:?} | run: \"{}\"",
                                name.cyan(),
                                task.watch,
                                task.extensions,
                                task.run.as_deref().unwrap_or("none")
                            );
                        }
                    }
                }
                TaskAction::Edit {
                    name,
                    watch,
                    run,
                    extensions,
                } => {
                    let task = config.tasks.get_mut(&name).unwrap_or_else(|| {
                        eprintln!("{} task '{}' not found", "Error:".red(), name);
                        process::exit(1);
                    });
                    if let Some(x) = run {
                        task.run = Some(x);
                    }
                    if let Some(x) = extensions {
                        task.extensions = Some(x);
                    }
                    if !watch.is_empty() {
                        task.watch = watch;
                    }
                    confy::store("cue", None, config)?;
                    println!("{} task '{}' updated", CUE.green(), name);
                }
                TaskAction::Rename { name, new_name } => {
                    let task = config.tasks.remove(&name).unwrap_or_else(|| {
                        eprintln!("{} task '{}' not found", "Error:".red(), name);
                        process::exit(1);
                    });
                    config.tasks.insert(new_name.clone(), task);
                    confy::store("cue", None, config)?;
                    println!("{} task '{}' renamed to '{}'", CUE.green(), name, new_name);
                }
            }
        }

        Some(Commands::Run {
            name,
            watch,
            run,
            extensions,
            debounce,
            global,
            quiet,
            no_clear,
        }) => {
            let config = resolve_config(global, quiet);
            run_task(
                &config, name, watch, run, extensions, debounce, quiet, no_clear,
            )?;
        }

        None => {
            if args.watch.is_empty() && args.run.is_none() && args.extensions.is_none() {
                let config = if args.global {
                    log!(args.quiet, "{} loading global tasks", CUE.green());
                    load_config(true)
                } else if Path::new("cue.toml").exists() {
                    log!(args.quiet, "{} loading tasks from 'cue.toml'", CUE.green());
                    load_config(false)
                } else {
                    eprintln!(
                        "{} no 'cue.toml' found — use -w/-e and -r to watch directly, or -g for global tasks",
                        "Error:".red()
                    );
                    process::exit(1);
                };
                run_task(
                    &config,
                    None,
                    None,
                    None,
                    None,
                    args.debounce,
                    args.quiet,
                    args.no_clear,
                )?;
            } else {
                if args.watch.is_empty() && args.extensions.is_none() {
                    eprintln!(
                        "{} please provide paths with -w or extensions with -e",
                        "Error:".red()
                    );
                    process::exit(1);
                }
                let run_str = args.run.unwrap_or_else(|| {
                    eprintln!("{} please provide a command with -r", "Error:".red());
                    process::exit(1);
                });
                let watch_strs = resolve_paths(args.watch, args.extensions);
                let paths: Vec<&Path> = watch_strs.iter().map(|s| Path::new(s)).collect();
                let command = parse_command(&run_str);
                validate_paths(&paths, args.quiet);
                validate_command(&command, args.quiet);
                start_watcher(
                    paths,
                    command,
                    &run_str,
                    args.debounce,
                    args.quiet,
                    args.no_clear,
                )?;
            }
        }

        Some(Commands::Init { template }) => {
            let template: &[u8] = match template {
    None => b"# optional: runs automatically in zero-config mode\n# default = \"build\"\n\n[tasks.build]\nwatch = [\"src\"]\nrun = \"your command here\"\n",
    Some(x) => match x.to_lowercase().as_str() {
        "rust" => b"default = \"run\"\n[tasks.run]\nwatch = [\"src\"]\nextensions = [\"rs\"]\nrun = \"cargo run\"\n[tasks.test]\nwatch = [\"src\", \"tests\"]\nextensions = [\"rs\"]\nrun = \"cargo test\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"rs\"]\nrun = \"cargo build --release\"\n[tasks.check]\nwatch = [\"src\"]\nextensions = [\"rs\"]\nrun = \"cargo check\"\n[tasks.lint]\nwatch = [\"src\"]\nextensions = [\"rs\"]\nrun = \"cargo clippy\"",
        "node" | "nodejs" => b"default = \"dev\"\n[tasks.dev]\nwatch = [\"src\"]\nextensions = [\"js\", \"ts\"]\nrun = \"node index.js\"\n[tasks.test]\nwatch = [\"src\", \"tests\"]\nextensions = [\"js\", \"ts\"]\nrun = \"npm test\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"ts\"]\nrun = \"tsc\"\n[tasks.lint]\nwatch = [\"src\"]\nextensions = [\"js\", \"ts\"]\nrun = \"eslint src\"\n[tasks.format]\nwatch = [\"src\"]\nextensions = [\"js\", \"ts\"]\nrun = \"prettier --write src\"",
        "go" => b"default = \"run\"\n[tasks.run]\nwatch = [\".\"]\nextensions = [\"go\"]\nrun = \"go run .\"\n[tasks.test]\nwatch = [\".\"]\nextensions = [\"go\"]\nrun = \"go test ./...\"\n[tasks.build]\nwatch = [\".\"]\nextensions = [\"go\"]\nrun = \"go build -o app .\"\n[tasks.lint]\nwatch = [\".\"]\nextensions = [\"go\"]\nrun = \"golangci-lint run\"\n[tasks.fmt]\nwatch = [\".\"]\nextensions = [\"go\"]\nrun = \"gofmt -w .\"",
        "c" => b"default = \"build\"\n[tasks.build]\nwatch = [\"src\", \"include\"]\nextensions = [\"c\", \"h\"]\nrun = \"gcc src/*.c -Iinclude -o app\"\n[tasks.run]\nwatch = [\"src\", \"include\"]\nextensions = [\"c\", \"h\"]\nrun = \"make && ./app\"\n[tasks.clean]\nwatch = [\"src\"]\nextensions = [\"c\", \"h\"]\nrun = \"make clean\"",
        "cpp" => b"default = \"build\"\n[tasks.build]\nwatch = [\"src\", \"include\"]\nextensions = [\"cpp\", \"hpp\", \"h\"]\nrun = \"g++ src/*.cpp -Iinclude -o app\"\n[tasks.run]\nwatch = [\"src\", \"include\"]\nextensions = [\"cpp\", \"hpp\", \"h\"]\nrun = \"make && ./app\"\n[tasks.test]\nwatch = [\"src\", \"tests\"]\nextensions = [\"cpp\", \"hpp\"]\nrun = \"ctest --output-on-failure\"",
        "ruby" => b"default = \"run\"\n[tasks.run]\nwatch = [\".\"]\nextensions = [\"rb\"]\nrun = \"ruby main.rb\"\n[tasks.test]\nwatch = [\".\"]\nextensions = [\"rb\"]\nrun = \"bundle exec rspec\"\n[tasks.lint]\nwatch = [\".\"]\nextensions = [\"rb\"]\nrun = \"rubocop\"",
        "php" => b"default = \"run\"\n[tasks.run]\nwatch = [\".\"]\nextensions = [\"php\"]\nrun = \"php index.php\"\n[tasks.test]\nwatch = [\".\"]\nextensions = [\"php\"]\nrun = \"phpunit\"\n[tasks.lint]\nwatch = [\".\"]\nextensions = [\"php\"]\nrun = \"php -l index.php\"",
        "java" => b"default = \"build\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"java\"]\nrun = \"javac src/*.java -d out\"\n[tasks.run]\nwatch = [\"src\"]\nextensions = [\"java\"]\nrun = \"java -cp out Main\"\n[tasks.test]\nwatch = [\"src\", \"test\"]\nextensions = [\"java\"]\nrun = \"mvn test\"",
        "kotlin" => b"default = \"run\"\n[tasks.run]\nwatch = [\"src\"]\nextensions = [\"kt\"]\nrun = \"kotlinc src/*.kt -include-runtime -d app.jar && java -jar app.jar\"\n[tasks.test]\nwatch = [\"src\", \"test\"]\nextensions = [\"kt\"]\nrun = \"gradle test\"",
        "swift" => b"default = \"run\"\n[tasks.run]\nwatch = [\"Sources\"]\nextensions = [\"swift\"]\nrun = \"swift run\"\n[tasks.test]\nwatch = [\"Sources\", \"Tests\"]\nextensions = [\"swift\"]\nrun = \"swift test\"\n[tasks.build]\nwatch = [\"Sources\"]\nextensions = [\"swift\"]\nrun = \"swift build\"",
        "zig" => b"default = \"run\"\n[tasks.run]\nwatch = [\"src\"]\nextensions = [\"zig\"]\nrun = \"zig run src/main.zig\"\n[tasks.test]\nwatch = [\"src\"]\nextensions = [\"zig\"]\nrun = \"zig test src/main.zig\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"zig\"]\nrun = \"zig build\"",
        "elixir" => b"default = \"run\"\n[tasks.run]\nwatch = [\"lib\"]\nextensions = [\"ex\", \"exs\"]\nrun = \"mix run\"\n[tasks.test]\nwatch = [\"lib\", \"test\"]\nextensions = [\"ex\", \"exs\"]\nrun = \"mix test\"\n[tasks.compile]\nwatch = [\"lib\"]\nextensions = [\"ex\"]\nrun = \"mix compile\"",
        "haskell" => b"default = \"run\"\n[tasks.run]\nwatch = [\"src\"]\nextensions = [\"hs\"]\nrun = \"cabal run\"\n[tasks.test]\nwatch = [\"src\", \"test\"]\nextensions = [\"hs\"]\nrun = \"cabal test\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"hs\"]\nrun = \"cabal build\"",
        "css" | "scss" => b"default = \"build\"\n[tasks.build]\nwatch = [\"src\"]\nextensions = [\"scss\", \"sass\"]\nrun = \"sass src/main.scss dist/style.css\"\n[tasks.watch]\nwatch = [\"src\"]\nextensions = [\"css\", \"scss\"]\nrun = \"sass --watch src:dist\"",
        "lua" => b"default = \"run\"\n[tasks.run]\nwatch = [\".\"]\nextensions = [\"lua\"]\nrun = \"lua main.lua\"\n[tasks.test]\nwatch = [\".\"]\nextensions = [\"lua\"]\nrun = \"busted\"",
        "shell" | "sh" => b"default = \"run\"\n[tasks.run]\nwatch = [\".\"]\nextensions = [\"sh\"]\nrun = \"bash main.sh\"\n[tasks.lint]\nwatch = [\".\"]\nextensions = [\"sh\"]\nrun = \"shellcheck *.sh\"",
        _ => b"# optional: runs automatically in zero-config mode\n# default = \"build\"\n\n[tasks.build]\nwatch = [\"src\"]\nrun = \"your command here\"\n"
    },
};

            if Path::new("cue.toml").exists() {
                log!(args.quiet, "{} cue.toml already exists", CUE.green());
            } else {
                let mut file = File::create("cue.toml")?;
                file.write_all(template)?;
                log!(
                    args.quiet,
                    "{} cue.toml created — edit it then run cue",
                    CUE.green()
                );
            }
        }
    }

    Ok(())
}
