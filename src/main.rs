use clap::Parser;
use colored::*;
use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::{
    path::Path,
    process::{self, Command},
    sync::mpsc,
};
use terminal_size::{terminal_size,Width};


#[derive(Parser)]
#[command(
    name = "cue",
    version,
    about = "Watches files and runs commands on save"
)]
struct Cli {
    #[arg(short, long, num_args = 1.. , required = true)]
    watch: Vec<String>,

    #[arg(short, long, num_args = 1.., required = true)]
    run: Vec<String>,
}

struct OnSaveCommand {
    cmd: String,
    args: Vec<String>,
}

fn color_path(path: &Path) -> ColoredString {
    path.display().to_string().cyan()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

let width = terminal_size()
    .map(|(Width(w), _)| w)
    .unwrap_or(40) as usize / 2;

    let cue = || "[cue]".green();
    let err = || "Error:".red();
    let exists = || "exists".green();
    
    let command = OnSaveCommand {
        cmd: match args.run.get(0) {
            Some(x) => x.clone(),
            None => {
                eprintln!("{} Please provide a command to run", err());
                process::exit(1);
            }
        },
        args: match args.run.get(1..) {
            Some(x) => x.to_vec(),
            None => vec![],
        },
    };
    let paths: Vec<&Path> = args.watch.iter().map(|x| Path::new(x)).collect();

    println!("{} Checking for paths existence..", cue());
    for path in &paths {
        if path.exists() {
            println!("{} {}", color_path(path), exists())
        } else {
            eprintln!("{} \'{}\' doesn't exist", err(), color_path(path));
            process::exit(1);
        }
    }

    println!("{} Checking for the command existence..", cue());
    if which::which(&command.cmd).is_err() {
        eprintln!("{} command \'{}\' not found", err(), command.cmd);
        process::exit(1);
    } else {
        println!("The command \'{}\' {}", command.cmd, exists())
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = recommended_watcher(tx)?;

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
                        println!("{}","_".repeat(width));
                        child = Some(
                            Command::new(&command.cmd)
                                .args(&command.args)
                                .spawn()
                                .expect("Failed to spawn"),
                        );
                    }
                }
            }
            Err(e) => eprintln!("{} watch error: {:#?}", cue(), e),
        }
    }
    Ok(())
}
