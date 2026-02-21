use clap::Parser;
use notify::{Event, RecursiveMode, Watcher, recommended_watcher};
use std::{path::Path, process::{self, Command}, sync::mpsc};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let command = OnSaveCommand {
        cmd: match args.run.get(0) {
            Some(x) => x.clone(),
            None => {
                eprintln!("Error: Please provide a command to run");
                process::exit(1);
            }
        },
        args: match args.run.get(1..) {
            Some(x) => x.to_vec(),
            None => vec![],
        },
    };
    let paths: Vec<&Path> = args.watch.iter().map(|x| Path::new(x)).collect();

    println!("Checking for paths existence..");
    for path in &paths {
        if !path.exists() {
            eprintln!("Error: \'{}\' doesn't exist", path.display());
            process::exit(1);
        }
    }

    println!("Checking for the command existence..");
    if which::which(&command.cmd).is_err() {
        eprintln!("Error: command \'{}\' not found", command.cmd);
        process::exit(1);
    }

    print!("\x1B[2J\x1B[1;1H");
    
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = recommended_watcher(tx)?;

    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    for event in rx {
        match event {
            Ok(e) => {
                println!("{} changed",e.paths[0].display());
                Command::new(&command.cmd)
                    .args(&command.args)
                    .spawn()
                    .expect("Failed to spawn")
                    .wait()
                    .expect("Failed to wait");
            },
            Err(e) => eprintln!("watch error: {:#?}", e),
        }
        println!("______________________________________");
    };

    Ok(())
}
