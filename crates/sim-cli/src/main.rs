use sim_core::{
    ExperimentConfig, Simulation, append_events_jsonl, experiment_id, report_markdown,
    summary_markdown, write_report, write_run_checkpoint,
};
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
    let mut out_dir: Option<PathBuf> = None;
    let mut checkpoint_every: Option<u64> = None;
    let mut load_path: Option<PathBuf> = None;
    let mut summarize = false;
    let mut report = false;
    let mut llm_override: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                i += 1;
                config_path = PathBuf::from(args.get(i).ok_or("--config requires a path")?);
            }
            "--ticks" | "-n" => {
                i += 1;
                ticks = Some(args.get(i).ok_or("--ticks requires a number")?.parse()?);
            }
            "--quiet" | "-q" => quiet = true,
            "--out-dir" | "-o" => {
                i += 1;
                out_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--out-dir requires a path")?,
                ));
            }
            "--checkpoint-every" => {
                i += 1;
                checkpoint_every = Some(
                    args.get(i)
                        .ok_or("--checkpoint-every requires a number")?
                        .parse()?,
                );
            }
            "--load" => {
                i += 1;
                load_path = Some(PathBuf::from(args.get(i).ok_or("--load requires a path")?));
            }
            "--summarize" => summarize = true,
            "--report" => report = true,
            "--llm" => {
                i += 1;
                let provider = args
                    .get(i)
                    .ok_or("--llm requires mock|wait|ollama|openai_compatible")?;
                llm_override = Some(provider.clone());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        i += 1;
    }

    let mut sim = if let Some(path) = &load_path {
        Simulation::load_checkpoint(path)?
    } else {
        let config = ExperimentConfig::load_path(&config_path)?;
        Simulation::new(config)?
    };
    if let Some(provider) = &llm_override {
        sim.config.llm.provider = provider.clone();
    }
    sim.chooser = sim_llm::chooser_from_config(&sim.config)?;

    if checkpoint_every.is_some() && out_dir.is_none() {
        out_dir = Some(PathBuf::from(&sim.config.checkpoint.directory));
    }

    let n = ticks.unwrap_or(if (summarize || report) && load_path.is_some() {
        0
    } else {
        100
    });

    if !quiet {
        println!(
            "master_seed={} world={}x{} agents={} ticks={} start_tick={}",
            sim.config.master_seed,
            sim.world.width,
            sim.world.height,
            sim.agents.len(),
            n,
            sim.tick
        );
        println!("world_hash={}", sim.world_hash());
        for (label, seed) in &sim.rngs.derived_seeds {
            println!("derived.{label}={seed}");
        }
        println!(
            "water={} vegetation={} minerals={}",
            sim.world.water_count(),
            sim.world.vegetation_count(),
            sim.world.mineral_count()
        );
    }

    if (summarize || report) && n == 0 {
        if summarize {
            print!("{}", summary_markdown(&sim)?);
        }
        if report {
            emit_report(&sim, out_dir.as_deref())?;
        }
        println!("final_tick={}", sim.tick);
        println!("final_hash={}", sim.state_hash());
        return Ok(());
    }

    let mut jsonl_path = None;
    let mut last_event = sim.events.events.len();
    if let Some(dir) = &out_dir {
        std::fs::create_dir_all(dir)?;
        let id = experiment_id(&sim.config_hash()?);
        jsonl_path = Some(dir.join(format!("{id}_events.jsonl")));
        if let Some(path) = &jsonl_path {
            append_events_jsonl(path, &sim.events.events)?;
            last_event = sim.events.events.len();
        }
        if sim.tick > 0 {
            write_run_checkpoint(&sim, dir)?;
        }
    }

    let interval = checkpoint_every.unwrap_or(sim.config.checkpoint.auto_interval_ticks);

    for _ in 0..n {
        if !sim.tick() {
            break;
        }
        if let Some(path) = &jsonl_path {
            let events = &sim.events.events;
            if events.len() > last_event {
                append_events_jsonl(path, &events[last_event..])?;
                last_event = events.len();
            }
        }
        if let Some(dir) = &out_dir {
            if interval > 0 && sim.tick % interval == 0 {
                write_run_checkpoint(&sim, dir)?;
            }
        }
        if !quiet {
            println!("tick={} hash={}", sim.tick, sim.state_hash());
        }
    }

    if let Some(dir) = &out_dir {
        write_run_checkpoint(&sim, dir)?;
    }

    if summarize {
        print!("{}", summary_markdown(&sim)?);
    }
    if report {
        emit_report(&sim, out_dir.as_deref())?;
    }

    println!("final_tick={}", sim.tick);
    println!("final_hash={}", sim.state_hash());
    if !quiet {
        for agent in sim.agents.values() {
            println!(
                "agent {} pos=({},{}) h={} land={}",
                agent.id.0,
                agent.x,
                agent.y,
                sim.world.height_at(agent.x, agent.y),
                sim.world.is_land(agent.x, agent.y)
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
          [--out-dir DIR] [--checkpoint-every K]
          [--load PATH] [--summarize] [--report]

Options:
  -c, --config PATH         Experiment TOML (default: configs/default.toml)
  -n, --ticks N             Number of ticks to run (default: 100; 0 with --load to inspect)
  -q, --quiet               Only print final_tick and final_hash
  -o, --out-dir DIR         Write checkpoints, Markdown summaries, and JSONL events
      --checkpoint-every K  Checkpoint every K ticks (implies --out-dir from config if omitted)
      --load PATH           Restore a .ckpt and continue
      --summarize           Print the Markdown world summary
      --report              Write food-economy report (md/csv); prints markdown if no --out-dir
      --llm PROVIDER        mock | wait | ollama | openai_compatible (xAI)
  -h, --help                Show this help"
    );
}

fn emit_report(sim: &Simulation, out_dir: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = out_dir {
        let (md, csv) = write_report(sim, dir)?;
        println!("report={}", md.display());
        if let Some(csv) = csv {
            println!("report_csv={}", csv.display());
        }
    } else {
        let built = sim_core::build_report(sim)?;
        print!("{}", report_markdown(&built));
    }
    Ok(())
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
