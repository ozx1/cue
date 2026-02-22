use clap::{Parser, Subcommand};
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use serde::{Deserialize, Serialize};
use shell_words::split;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{
    path::Path,
    process::{self, Command},
    sync::mpsc,
};
use terminal_size::{Width, terminal_size};

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

    #[arg(short, long, num_args = 1.., required = false)]
    watch: Vec<String>,

    #[arg(short, long, required = false)]
    run: Option<String>,
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

struct OnSaveCommand {
    cmd: String,
    args: Vec<String>,
}

fn color_path(path: &Path) -> ColoredString {
    path.display().to_string().cyan()
}

fn parse_command(run: &str, err: impl Fn() -> ColoredString) -> OnSaveCommand {
    let cmd = match split(run) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{} failed to parse the command\n {}", err(), e);
            process::exit(1);
        }
    };

    OnSaveCommand {
        cmd: match cmd.get(0) {
            Some(x) => x.clone(),
            None => {
                eprintln!("{} empty command", err());
                process::exit(1);
            }
        },
        args: cmd.get(1..).unwrap_or_default().to_vec(),
    }
}

fn validate_paths(paths: &[&Path], err: impl Fn() -> ColoredString, cue: impl Fn() -> ColoredString) {
    println!("{} Checking for paths existence..", cue());
    for path in paths {
        if path.exists() {
            println!("{} {}", color_path(path), "exists".green());
        } else {
            eprintln!("{} '{}' doesn't exist", err(), color_path(path));
            process::exit(1);
        }
    }
}

fn validate_command(command: &OnSaveCommand, err: impl Fn() -> ColoredString, cue: impl Fn() -> ColoredString) {
    println!("{} Checking for the command existence..", cue());
    if which::which(&command.cmd).is_err() {
        eprintln!("{} command '{}' not found", err(), command.cmd);
        process::exit(1);
    } else {
        println!("The command '{}' {}", command.cmd, "exists".green());
    }
}

fn start_watcher(
    paths: Vec<&Path>,
    command: OnSaveCommand,
    run_str: &str,
    cue: impl Fn() -> ColoredString,
    err: impl Fn() -> ColoredString,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = terminal_size().map(|(Width(w), _)| w).unwrap_or(40) as usize / 2;

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(tx)?;

    println!(
        "{} the command '{}' will run when the files are modified",
        cue(),
        run_str
    );

    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    let mut child: Option<std::process::Child> = None;
    let mut last_run = Instant::now();

    for event in rx {
        match event {
            Ok(e) => {
                if last_run.elapsed() > Duration::from_millis(150) {
                    last_run = Instant::now();
                    if matches!(e.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        if let Some(mut c) = child.take() {
                            c.kill().ok();
                            c.wait().ok();
                        }
                        clearscreen::clear().unwrap();
                        let changed_path = match e.paths.get(0) {
                            Some(x) => dunce::canonicalize(x.clone()).unwrap_or(x.clone()),
                            None => PathBuf::new(),
                        };
                        println!("{} {} changed", cue(), color_path(&changed_path));
                        println!("{}", "_".repeat(width));
                        child = Some(
                            Command::new(&command.cmd)
                                .args(&command.args)
                                .spawn()
                                .expect("Failed to spawn"),
                        );
                    }
                }
            }
            Err(e) => eprintln!("{} watch error: {:#?}", err(), e),
        }
    }

    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let cue = || "[cue]".green();
    let err = || "Error:".red();

    match args.command {
        Some(Commands::Task { action }) => {
            let mut config: CueConfig = confy::load("cue", None)?;

            match action {
                TaskAction::Add { name, watch, run } => {
                    config.tasks.insert(name.clone(), Task { watch, run });
                    confy::store("cue", None, config)?;
                    println!("{} task '{}' saved", cue(), name);
                }
                TaskAction::Remove { name } => {
                    if config.tasks.remove(&name).is_some() {
                        confy::store("cue", None, config)?;
                        println!("{} task '{}' removed", cue(), name);
                    } else {
                        eprintln!("{} task '{}' not found", err(), name);
                        process::exit(1);
                    }
                }
                TaskAction::List => {
                    if config.tasks.is_empty() {
                        println!("{} no saved tasks", cue());
                    } else {
                        println!("{} saved tasks:", cue());
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
            watch: watch_override,
            run: run_override,
        }) => {
            let config: CueConfig = confy::load("cue", None)?;

            let task = match config.tasks.get(&name) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("{} task '{}' not found", err(), name);
                    process::exit(1);
                }
            };

            let watch_strs = watch_override.unwrap_or(task.watch);
            let run_str = run_override.unwrap_or(task.run);

            let paths: Vec<&Path> = watch_strs.iter().map(|x| Path::new(x)).collect();
            let command = parse_command(&run_str, &err);

            validate_paths(&paths, &err, &cue);
            validate_command(&command, &err, &cue);
            start_watcher(paths, command, &run_str, cue, err)?;
        }

        None => {
            let watch = if args.watch.is_empty() {
                eprintln!("{} please provide paths to watch with -w", err());
                process::exit(1);
            } else {
                args.watch
            };

            let run_str = match args.run {
                Some(r) => r,
                None => {
                    eprintln!("{} please provide a command to run with -r", err());
                    process::exit(1);
                }
            };

            let paths: Vec<&Path> = watch.iter().map(|x| Path::new(x)).collect();
            let command = parse_command(&run_str, &err);

            validate_paths(&paths, &err, &cue);
            validate_command(&command, &err, &cue);
            start_watcher(paths, command, &run_str, cue, err)?;
        }
    }

    Ok(())
}