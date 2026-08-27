mod network;
mod overlay;
mod server;

use sim_core::{
    ExperimentConfig, Simulation, append_decisions_jsonl, append_events_jsonl, append_timing_jsonl,
    compare_csv, compare_markdown, compare_runs, experiment_id, load_compare_pair, report_markdown,
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
    let mut listen: Vec<String> = Vec::new();
    let mut allow_control = false;
    let mut token: Option<String> = None;
    let mut incentives_path: Option<PathBuf> = None;
    let mut inject_path: Option<PathBuf> = None;
    let mut compare: Vec<PathBuf> = Vec::new();
    let mut csv = false;
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
            "--listen" => {
                i += 1;
                listen.push(
                    args.get(i)
                        .ok_or("--listen requires tcp:// or ws:// URL")?
                        .clone(),
                );
            }
            "--allow-control" => allow_control = true,
            "--token" => {
                i += 1;
                token = Some(args.get(i).ok_or("--token requires a value")?.clone());
            }
            "--incentives" => {
                i += 1;
                incentives_path = Some(PathBuf::from(
                    args.get(i).ok_or("--incentives requires a path")?,
                ));
            }
            "--inject" => {
                i += 1;
                inject_path = Some(PathBuf::from(
                    args.get(i).ok_or("--inject requires a path")?,
                ));
            }
            "--compare" => {
                i += 1;
                let a = args.get(i).ok_or("--compare requires two paths")?;
                i += 1;
                let b = args.get(i).ok_or("--compare requires two paths")?;
                compare = vec![PathBuf::from(a), PathBuf::from(b)];
            }
            "--csv" => csv = true,
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
        i += 1;
    }

    if compare.len() == 2 {
        return run_compare(&compare[0], &compare[1], csv);
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

    let net = network::NetworkParams::from_path(&config_path);
    allow_control = allow_control || net.allow_control;
    if token.is_none() && !net.token.is_empty() {
        token = Some(net.token.clone());
    }
    let listen = server::merge_listen(&net, &listen);
    let overlay = overlay::OverlayFile::from_path(&config_path);
    if overlay.storage.slot_cap.is_some()
        || overlay.storage.weight_cap.is_some()
        || overlay.storage.haul.is_some()
    {
        let mut p = sim.storage;
        if let Some(n) = overlay.storage.slot_cap {
            p.slot_cap = n.max(1);
        }
        if let Some(w) = overlay.storage.weight_cap {
            p.weight_cap_milli = sim_core::species::f64_to_milli(w).max(1);
        }
        if let Some(h) = overlay.storage.haul {
            p.haul_milli = sim_core::species::f64_to_milli(h).max(1);
        }
        sim.storage = p;
    }
    if let Some(w) = overlay.voting.weight.as_deref() {
        sim.voting.weight = sim_core::VoteWeight::parse(w)?;
    }
    if incentives_path.is_none() && !overlay.incentives.schedule.is_empty() {
        incentives_path = Some(PathBuf::from(overlay.incentives.schedule.clone()));
    }
    let schedule_path = inject_path.or(incentives_path);
    if let Some(path) = &schedule_path {
        let text = std::fs::read_to_string(path)?;
        sim.inject_schedule_toml(&text)?;
    }

    if checkpoint_every.is_some() && out_dir.is_none() {
        out_dir = Some(PathBuf::from(&sim.config.checkpoint.directory));
    }
    let write_timing = overlay.metrics.timing.unwrap_or(out_dir.is_some());

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

    if !listen.is_empty() && !(summarize || report) {
        return server::serve(server::ServeOpts {
            sim,
            ticks: n,
            listen,
            allow_control,
            token,
            quiet,
            out_dir,
            checkpoint_every,
            write_timing,
        });
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
    let mut decisions_path = None;
    let mut timing_path = None;
    let mut last_event = sim.events.events.len();
    let mut tick_ns_sum = 0u128;
    let mut tick_ns_n = 0u64;
    let mut tick_ns_last = 0u64;
    if let Some(dir) = &out_dir {
        std::fs::create_dir_all(dir)?;
        let id = experiment_id(&sim.config_hash()?);
        jsonl_path = Some(dir.join(format!("{id}_events.jsonl")));
        decisions_path = Some(dir.join(format!("{id}_decisions.jsonl")));
        if write_timing {
            timing_path = Some(dir.join(format!("{id}_timing.jsonl")));
        }
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
        if let Some(path) = &decisions_path {
            append_decisions_jsonl(path, &sim.last_tick_decisions)?;
        }
        if let Some(t) = &sim.last_tick_timing {
            tick_ns_sum += u128::from(t.wall_ns);
            tick_ns_n += 1;
            tick_ns_last = t.wall_ns;
            if let Some(path) = &timing_path {
                append_timing_jsonl(path, t)?;
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
    if tick_ns_n > 0 {
        eprintln!(
            "tick_ns_last={tick_ns_last} tick_ns_mean={}",
            tick_ns_sum / u128::from(tick_ns_n)
        );
    }
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
          [--listen tcp://HOST:PORT] [--listen ws://HOST:PORT]
          [--allow-control] [--token SECRET]
          [--incentives PATH] [--inject PATH]
          [--compare DIR_OR_CKPT DIR_OR_CKPT] [--csv]

Options:
  -c, --config PATH         Experiment TOML (default: configs/default.toml)
  -n, --ticks N             Number of ticks to run (default: 100; 0 with --load to inspect)
  -q, --quiet               Only print final_tick and final_hash
  -o, --out-dir DIR         Write checkpoints, Markdown summaries, and JSONL events
      --checkpoint-every K  Checkpoint every K ticks (implies --out-dir from config if omitted)
      --load PATH           Restore a .ckpt and continue
      --summarize           Print the Markdown world summary
      --report              Write food-economy report (md/csv); prints markdown if no --out-dir
      --llm PROVIDER        mock | wait | ollama | openai_compatible (empty base_url ⇒ mock)
      --listen URL          Repeatable. tcp://host:port and/or ws://host:port (no TLS)
      --allow-control       Accept pause/play/step/save/report/summarize from clients
      --token SECRET        Require matching token on Hello (LAN auth, not TLS)
      --incentives PATH     Apply incentive TOML from tick 0
      --inject PATH         Replace schedule (typical with --load)
      --compare A B         Diff two --out-dir folders or .ckpt files (markdown)
      --csv                 With --compare, also print CSV
  -h, --help                Show this help"
    );
}

fn run_compare(a: &Path, b: &Path, csv: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (sa, sb) = load_compare_pair(a, b)?;
    let report = compare_runs(&sa, &a.display().to_string(), &sb, &b.display().to_string());
    print!("{}", compare_markdown(&report));
    if csv {
        print!("{}", compare_csv(&report));
    }
    Ok(())
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
