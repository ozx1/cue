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

const CUE: &str = "[cue]";
const DEBOUNCE_MS: u64 = 150;

macro_rules! log {
    ($quiet:expr, $($arg:tt)*) => {
        if !$quiet {
            println!($($arg)*);
        }
    };
}

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct CueConfig {
    default: Option<String>,
    tasks: HashMap<String, Task>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    watch: Vec<String>,
    run: String,
}

// ─── CLI ──────────────────────────────────────────────────────────────────────

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

    #[arg(long, short, default_value_t = DEBOUNCE_MS)]
    debounce: u64,

    #[arg(long, short)]
    global: bool,

    #[arg(long)]
    quite: bool,

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

        #[arg(long, short, default_value_t = DEBOUNCE_MS)]
        debounce: u64,

        #[arg(long, short)]
        global: bool,

        #[arg(long, short)]
        quite: bool,

        #[arg(long, short)]
        no_clear: bool,
    },
    Init,
}

#[derive(Subcommand)]
enum TaskAction {
    Add {
        name: String,

        #[arg(short, long, num_args = 1.., required = true)]
        watch: Vec<String>,

        #[arg(short, long, required = true)]
        run: String,
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
    },
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

fn validate_paths(paths: &[&Path], quite: bool) {
    log!(quite, "{} checking paths...", CUE.green());
    for path in paths {
        if path.exists() {
            log!(
                quite,
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

fn validate_command(command: &ParsedCommand, quite: bool) {
    log!(quite, "{} checking command...", CUE.green());
    if which::which(&command.cmd).is_err() {
        eprintln!("{} command '{}' not found", "Error:".red(), command.cmd);
        process::exit(1);
    }
    log!(quite, "  '{}' {}", command.cmd, "found".green());
}

fn start_watcher(
    paths: Vec<&Path>,
    command: ParsedCommand,
    run_str: &str,
    debounce: u64,
    quite: bool,
    no_clear: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
        / 2;

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(tx)?;

    log!(
        quite,
        "{} watching — will run '{}' on changes",
        CUE.green(),
        run_str
    );

    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    let mut child: Option<std::process::Child> = None;
    let mut last_run = Instant::now();

    if quite {
        child = Some(
            Command::new(&command.cmd)
                .args(&command.args)
                .spawn()
                .expect("failed to spawn command"),
        )
    };

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

                if no_clear {
                    log!(quite, "{}", "_".repeat(width));
                }else {
                    clearscreen::clear().unwrap()
                }
                log!(
                    quite,
                    "{} {} changed at {}",
                    CUE.green(),
                    changed.display().to_string().cyan(),
                    Utc::now().format("%H:%M:%S")
                );
                log!(quite, "{}", "_".repeat(width));

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

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::Task { action }) => {
            let mut config: CueConfig = confy::load("cue", None)?;

            match action {
                TaskAction::Add { name, watch, run } => {
                    config.tasks.insert(name.clone(), Task { watch, run });
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
                                "  {} — watch: {:?} | run: \"{}\"",
                                name.cyan(),
                                task.watch,
                                task.run
                            );
                        }
                    }
                }
                TaskAction::Edit { name, watch, run } => {
                    let task = config.tasks.get_mut(&name).unwrap_or_else(|| {
                        eprintln!("{} task '{}' not found", "Error:".red(), name);
                        process::exit(1);
                    });

                    if let Some(x) = run {
                        task.run = x
                    }
                    if !watch.is_empty() {
                        task.watch = watch
                    }

                    confy::store("cue", None, config)?;
                    println!("{} task '{}' updated", CUE.green(), name);
                }
            }
        }

        Some(Commands::Run {
            name,
            watch,
            run,
            debounce,
            global,
            quite,
            no_clear,
        }) => {
            let config: CueConfig = if Path::new("cue.toml").exists() && !global {
                log!(
                    quite,
                    "{} found 'cue.toml' — loading tasks from it",
                    CUE.green()
                );
                let content = fs::read_to_string("cue.toml").unwrap_or_else(|_| {
                    eprintln!("{} failed to read cue.toml", "Error:".red());
                    process::exit(1);
                });

                toml::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("{} invalid cue.toml: {}", "Error:".red(), e);
                    process::exit(1);
                })
            } else {
                log!(quite, "{} loading global tasks", CUE.green());
                confy::load("cue", None)?
            };

            let name = if let Some(n) = name {
                n
            } else {
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
            };

            let task = config.tasks.get(&name).cloned().unwrap_or_else(|| {
                eprintln!("{} task '{}' not found", "Error:".red(), name);
                process::exit(1);
            });

            let watch_strs = watch.unwrap_or(task.watch);
            let run_str = run.unwrap_or(task.run);
            let paths: Vec<&Path> = watch_strs.iter().map(|s| Path::new(s)).collect();
            let command = parse_command(&run_str);

            validate_paths(&paths, quite);
            validate_command(&command, quite);
            start_watcher(paths, command, &run_str, debounce, quite, no_clear)?;
        }

        None => {
            if args.watch.is_empty() && args.run.is_none() {
                let config: CueConfig = if args.global {
                    log!(args.quite, "{} loading global tasks", CUE.green());
                    confy::load("cue", None).unwrap_or_else(|_| {
                        eprintln!("{} failed to read config", "Error:".red());
                        process::exit(1);
                    })
                } else if Path::new("cue.toml").exists() {
                    let content = fs::read_to_string("cue.toml").unwrap_or_else(|_| {
                        eprintln!("{} failed to read cue.toml", "Error:".red());
                        process::exit(1);
                    });

                    log!(
                        args.quite,
                        "{} loading tasks from \'cue.toml\'",
                        CUE.green()
                    );
                    toml::from_str(&content).unwrap_or_else(|e| {
                        eprintln!("{} invalid cue.toml: {}", "Error:".red(), e);
                        process::exit(1);
                    })
                } else {
                    eprintln!(
                        "{} no \'cue.toml\' found - please provide paths with -w and a command to run with -r (or use the -g flag to use global tasks)",
                        "Error:".red()
                    );
                    process::exit(1);
                };
                let name = if let Some(d) = config.default {
                    log!(args.quite, "{} found a default task [{}]", CUE.green(), d);
                    d
                } else {
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
                };
                let task = config.tasks.get(&name).unwrap_or_else(|| {
                    eprintln!("{} task \'{}\' not found", "Error:".red(), name);
                    process::exit(1);
                });
                let watch_strs = &task.watch;
                let run_str = &task.run;
                let paths: Vec<&Path> = watch_strs.iter().map(|s| Path::new(s)).collect();
                let command = parse_command(&run_str);

                validate_paths(&paths, args.quite);
                validate_command(&command, args.quite);
                start_watcher(
                    paths,
                    command,
                    &run_str,
                    args.debounce,
                    args.quite,
                    args.no_clear,
                )?;
            } else {
                let run_str = args.run.unwrap_or_else(|| {
                    eprintln!("{} please provide a command with -r", "Error:".red());
                    process::exit(1);
                });

                let paths: Vec<&Path> = args.watch.iter().map(|s| Path::new(s)).collect();
                let command = parse_command(&run_str);

                validate_paths(&paths, args.quite);
                validate_command(&command, args.quite);
                start_watcher(
                    paths,
                    command,
                    &run_str,
                    args.debounce,
                    args.quite,
                    args.no_clear,
                )?;
            }
        }
        Some(Commands::Init) => {
            if Path::new("cue.toml").exists() {
                log!(args.quite, "{} cue.toml already exists", CUE.green())
            } else {
                let mut file = File::create("cue.toml")?;
                file.write_all(b"\"default\" = \"taskname\" \n\n[tasks.taskname]\nwatch = [\"filename.txt\"]\nrun = \"cmd arg -f\"\n",)?;
                log!(
                    args.quite,
                    "{} cue.toml created — edit it then run cue",
                    CUE.green()
                );
            }
        }
    }

    Ok(())
}
