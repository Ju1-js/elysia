use std::{env, path::PathBuf};

use anyhow::{Result, anyhow};

use backend::settings::GlobalSettings;

#[tokio::main]
async fn main() {
    let settings = match GlobalSettings::load() {
        Ok(mut settings) => {
            settings.validate();
            settings
        }
        Err(_) => {
            let mut settings = GlobalSettings::default();
            settings.validate();
            if let Err(err) = settings.save() {
                eprintln!("Failed to save settings: {}", err);
            }
            settings
        }
    };

    let args: Vec<String> = env::args().skip(1).collect();

    match args.first() {
        Some(arg) => {
            match run_verb(&settings, arg.trim(), &args[1..]).await {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Error: {}", err);
                }
            };
        }
        None => {
            println!("No arguments provided");
        }
    }
}

async fn run_verb(settings: &GlobalSettings, verb: &str, args: &[String]) -> Result<()> {
    match verb {
        "help" => {
            run_help();
        }
        "scan" => {
            run_scan(settings, args.first()).await?;
        }
        _ => {
            println!("Unknown verb: {}", verb);
        }
    }

    Ok(())
}

fn run_help() {
    println!("Usage: elysia-cli [verb] <arguments>");
    println!("Verbs:");
    println!("  help");
    println!("      Display this help message");
    println!("  scan <path>");
    println!("      Scan a directory for games");
}

async fn run_scan(settings: &GlobalSettings, path: Option<&String>) -> Result<()> {
    let path = if let Some(path) = path {
        PathBuf::from(path)
    } else {
        env::current_dir().unwrap()
    };

    let games = backend::game_providers::hoyoplay::scan_dir(settings, &path)
        .await
        .map_err(|e| anyhow!(e))?;

    if games.is_empty() {
        println!("No games found");
    } else {
        for (exe, id, version) in games {
            println!("{} -> {}: {}", exe, id, version);
        }
    }

    Ok(())
}
