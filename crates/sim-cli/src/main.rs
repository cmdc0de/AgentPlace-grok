use sim_core::{ExperimentConfig, Simulation};
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut config_path = default_config_path();
    let mut ticks: Option<u64> = None;
    let mut quiet = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                i += 1;
                config_path = PathBuf::from(
                    args.get(i).ok_or("--config requires a path")?,
                );
            }
            "--ticks" | "-n" => {
                i += 1;
                ticks = Some(
                    args.get(i)
                        .ok_or("--ticks requires a number")?
                        .parse()?,
                );
            }
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        i += 1;
    }

    let config = ExperimentConfig::load_path(&config_path)?;
    let n = ticks.unwrap_or(100);
    let mut sim = Simulation::new(config)?;

    if !quiet {
        println!(
            "master_seed={} world={}x{} agents={} ticks={}",
            sim.config.master_seed,
            sim.world.width,
            sim.world.height,
            sim.agents.len(),
            n
        );
        println!("world_hash={}", sim.world_hash());
        for (label, seed) in &sim.rngs.derived_seeds {
            println!("derived.{label}={seed}");
        }
    }

    for _ in 0..n {
        if !sim.tick() {
            break;
        }
        if !quiet {
            println!("tick={} hash={}", sim.tick, sim.state_hash());
        }
    }

    println!("final_tick={}", sim.tick);
    println!("final_hash={}", sim.state_hash());
    if !quiet {
        for agent in sim.agents.values() {
            println!(
                "agent {} pos=({},{}) h={}",
                agent.id.0,
                agent.x,
                agent.y,
                sim.world.height_at(agent.x, agent.y)
            );
        }
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "\
sim-cli — headless AgentTown runner

Usage:
  sim-cli [--config PATH] [--ticks N] [--quiet]

Options:
  -c, --config PATH   Experiment TOML (default: configs/default.toml)
  -n, --ticks N       Number of ticks to run (default: 100)
  -q, --quiet         Only print final_tick and final_hash
  -h, --help          Show this help"
    );
}

fn default_config_path() -> PathBuf {
    let candidates = [
        Path::new("configs/default.toml"),
        Path::new("../configs/default.toml"),
        Path::new("../../configs/default.toml"),
    ];
    for path in candidates {
        if path.exists() {
            return path.to_path_buf();
        }
    }
    PathBuf::from("configs/default.toml")
}
