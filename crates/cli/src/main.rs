use std::{env, path::PathBuf};

use anyhow::{Result, anyhow};

use backend::{
    settings::GlobalSettings,
    task_manager::{
        Task, TaskManager, TaskStatus,
        steps::{
            Step,
            zip_stream::{Archive, ZipStreamStep},
        },
    },
};
use reqwest::{Client, Url};

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
        "gf2" => {
            run_gf2(settings).await?;
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
    println!("  gf2");
    println!("      For testing: streaming extract the GF2: Exilum archive to temp");
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

async fn run_gf2(settings: &GlobalSettings) -> Result<()> {
    let mut mgr = TaskManager::new(Client::new());

    let dest = settings.temp_directory.join("test");

    let task = Task {
        steps: vec![Step::DownloadFromArchives(ZipStreamStep::new(
            dest,
            vec![Archive {
                url: Url::parse("https://gf2-us-cdn.sunborngame.com/prod/package/PCClient/2.0.3932/GF2_Exilium_Origin.zip").unwrap(),
                hash: None,
                size: 344730668,
            }]
        ))],
    };

    let _ = mgr.submit_tx.send(task).await;

    let _ = tokio::spawn(async move {
        while let Some(progress) = mgr.status_rx.recv().await {
            match progress {
                TaskStatus::Preparing {} => println!("Preparing task"),
                TaskStatus::Started {} => println!("Started task"),
                TaskStatus::Progress {
                    current,
                    total,
                    mb_s,
                } => {
                    println!(
                        "Progress: {:.3}/{:.3} MB ({:.2} MB/s)",
                        current as f64 / 1000000.0,
                        total as f64 / 1000000.0,
                        mb_s
                    );
                }
                TaskStatus::Finalizing {} => println!("Finalizing task"),
                TaskStatus::Done {} => println!("Finished task"),
                TaskStatus::Failed { error } => println!("Failed task: {}", error),
            }
        }
    })
    .await;

    Ok(())
}
