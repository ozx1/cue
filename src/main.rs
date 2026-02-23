use clap::{Parser, Subcommand};
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use serde::{Deserialize, Serialize};
use shell_words::split;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use terminal_size::{Width, terminal_size};
use chrono::Utc;

const CUE: &str = "[cue]";
const DEBOUNCE_MS: u64 = 150;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct CueConfig {
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
}

#[derive(Subcommand)]
enum Commands {
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    Run {
        name: String,
        #[arg(short, long, num_args = 1..)]
        watch: Option<Vec<String>>,
        #[arg(short, long)]
        run: Option<String>,
        #[arg(long, short, default_value_t = DEBOUNCE_MS)]
        debounce: u64,
    },
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

fn validate_paths(paths: &[&Path]) {
    println!("{} checking paths...", CUE.green());
    for path in paths {
        if path.exists() {
            println!(
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

fn validate_command(command: &ParsedCommand) {
    println!("{} checking command...", CUE.green());
    if which::which(&command.cmd).is_err() {
        eprintln!("{} command '{}' not found", "Error:".red(), command.cmd);
        process::exit(1);
    }
    println!("  '{}' {}", command.cmd, "found".green());
}

fn start_watcher(
    paths: Vec<&Path>,
    command: ParsedCommand,
    run_str: &str,
    debounce: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = terminal_size()
        .map(|(Width(w), _)| w as usize)
        .unwrap_or(80)
        / 2;

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(tx)?;

    println!(
        "{} watching — will run '{}' on changes",
        CUE.green(),
        run_str
    );

    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    let mut child: Option<std::process::Child> = None;
    let mut last_run = Instant::now();

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

                clearscreen::clear().unwrap();
                println!(
                    "{} {} changed at {}",
                    CUE.green(),
                    changed.display().to_string().cyan(),
                    Utc::now().format("%H:%M:%S")
                );
                println!("{}", "_".repeat(width));

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
            }
        }

        Some(Commands::Run {
            name,
            watch,
            run,
            debounce,
        }) => {
            let config: CueConfig = confy::load("cue", None)?;

            let task = config.tasks.get(&name).cloned().unwrap_or_else(|| {
                eprintln!("{} task '{}' not found", "Error:".red(), name);
                process::exit(1);
            });

            let watch_strs = watch.unwrap_or(task.watch);
            let run_str = run.unwrap_or(task.run);
            let paths: Vec<&Path> = watch_strs.iter().map(|s| Path::new(s)).collect();
            let command = parse_command(&run_str);

            validate_paths(&paths);
            validate_command(&command);
            start_watcher(paths, command, &run_str, debounce)?;
        }

        None => {
            if args.watch.is_empty() {
                eprintln!("{} please provide paths with -w", "Error:".red());
                process::exit(1);
            }
            let run_str = args.run.unwrap_or_else(|| {
                eprintln!("{} please provide a command with -r", "Error:".red());
                process::exit(1);
            });

            let paths: Vec<&Path> = args.watch.iter().map(|s| Path::new(s)).collect();
            let command = parse_command(&run_str);

            validate_paths(&paths);
            validate_command(&command);
            start_watcher(paths, command, &run_str, DEBOUNCE_MS)?;
        }
    }

    Ok(())
}
