mod client;
mod network;
mod overlay;
mod server;
mod sqlite;

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
    let mut connect: Option<String> = None;
    let mut start_paused = false;
    let mut llm_barrier = false;
    let mut llm_barrier_retries: Option<u32> = None;
    let mut lockstep = false;
    let mut lockstep_timeout_ms: Option<u64> = None;
    let mut llm_reflect_on_evict = false;
    let mut llm_reflect_every: Option<u64> = None;
    let mut llm_plan_every: Option<u64> = None;
    let mut llm_execute_plan = false;
    let mut conflict = false;
    let mut conflict_death = false;
    let mut sheet = false;
    let mut reproduction = false;
    let mut aging = false;
    let mut household_crates = false;
    let mut culture = false;
    let mut llm_reflect_importance = false;
    let mut inventions = false;
    let mut pipeline_events = false;
    let mut catalog = false;
    let mut telemetry = false;
    let mut objects_path: Option<PathBuf> = None;
    let mut sqlite_path: Option<PathBuf> = None;
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
            "--sqlite" => {
                i += 1;
                sqlite_path = Some(PathBuf::from(
                    args.get(i).ok_or("--sqlite requires a path")?,
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
            "--connect" => {
                i += 1;
                connect = Some(
                    args.get(i)
                        .ok_or("--connect requires tcp:// or ws:// URL")?
                        .clone(),
                );
            }
            "--allow-control" => allow_control = true,
            "--start-paused" => start_paused = true,
            "--lockstep" => lockstep = true,
            "--lockstep-timeout-ms" => {
                i += 1;
                lockstep_timeout_ms = Some(
                    args.get(i)
                        .ok_or("--lockstep-timeout-ms requires a number")?
                        .parse()?,
                );
            }
            "--llm-reflect-on-evict" => llm_reflect_on_evict = true,
            "--llm-reflect-every" => {
                i += 1;
                llm_reflect_every = Some(
                    args.get(i)
                        .ok_or("--llm-reflect-every requires a number")?
                        .parse()?,
                );
            }
            "--llm-plan-every" => {
                i += 1;
                llm_plan_every = Some(
                    args.get(i)
                        .ok_or("--llm-plan-every requires a number")?
                        .parse()?,
                );
            }
            "--llm-execute-plan" => llm_execute_plan = true,
            "--conflict" => conflict = true,
            "--conflict-death" => {
                conflict = true;
                conflict_death = true;
            }
            "--sheet" => sheet = true,
            "--reproduction" => {
                reproduction = true;
                sheet = true;
            }
            "--aging" => aging = true,
            "--household-crates" => household_crates = true,
            "--culture" => culture = true,
            "--inventions" => inventions = true,
            "--pipeline-events" => pipeline_events = true,
            "--catalog" => catalog = true,
            "--telemetry" => telemetry = true,
            "--objects" => {
                i += 1;
                objects_path = Some(PathBuf::from(
                    args.get(i).ok_or("--objects requires a directory")?,
                ));
            }
            "--llm-reflect-importance" => llm_reflect_importance = true,
            "--llm-barrier" => llm_barrier = true,
            "--llm-barrier-retries" => {
                i += 1;
                llm_barrier = true;
                llm_barrier_retries = Some(
                    args.get(i)
                        .ok_or("--llm-barrier-retries requires a number")?
                        .parse()?,
                );
            }
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
    if let Some(url) = &connect {
        if !listen.is_empty() {
            return Err("--listen and --connect are mutually exclusive".into());
        }
        return client::log_tail(url, token, quiet, allow_control);
    }

    let objects_dir = objects_path
        .clone()
        .or_else(sim_core::objects::default_objects_dir);
    let mut sim = if let Some(path) = &load_path {
        Simulation::load_checkpoint(path)?
    } else {
        let mut config = ExperimentConfig::load_path(&config_path)?;
        if let Some(dir) = &objects_dir {
            let defs = sim_core::load_object_defs(dir)?;
            sim_core::apply_species_defs(&mut config.world.species, &defs);
        }
        Simulation::new(config)?
    };
    if let Some(provider) = &llm_override {
        sim.config.llm.provider = provider.clone();
    }
    sim.chooser = sim_llm::chooser_from_config(&sim.config)?;

    let net = network::NetworkParams::from_path(&config_path);
    allow_control = allow_control || net.allow_control;
    lockstep = lockstep || net.lockstep;
    let lockstep_timeout_ms = lockstep_timeout_ms.unwrap_or(net.lockstep_timeout_ms);
    if token.is_none() && !net.token.is_empty() {
        token = Some(net.token.clone());
    }
    let listen = server::merge_listen(&net, &listen);
    if start_paused {
        if listen.is_empty() {
            return Err("--start-paused requires --listen".into());
        }
        if !allow_control {
            return Err("--start-paused requires --allow-control".into());
        }
    }
    let overlay = overlay::OverlayFile::from_path(&config_path);
    {
        let text = std::fs::read_to_string(&config_path).unwrap_or_default();
        sim.storage = sim_core::StorageParams::from_config_toml(&text)?;
        sim.voting = sim_core::VotingParams::from_config_toml(&text)?;
        let mut barrier = sim_core::LlmBarrierParams::from_config_toml(&text);
        if llm_barrier {
            barrier.barrier = true;
        }
        if let Some(n) = llm_barrier_retries {
            barrier.barrier = true;
            barrier.retries = n;
        }
        sim.llm_barrier = barrier.barrier;
        sim.llm_barrier_retries = barrier.retries;
        sim.llm_reflect_on_evict = llm_reflect_on_evict || barrier.reflect_on_evict;
        sim.llm_reflect_every_n = llm_reflect_every.unwrap_or(barrier.reflect_every_n_ticks);
        sim.llm_plan_every_n = llm_plan_every.unwrap_or(barrier.plan_every_n_ticks);
        sim.llm_plan_length = barrier.plan_length;
        sim.llm_execute_plan = llm_execute_plan || barrier.execute_plan;
        sim.llm_reflect_importance = llm_reflect_importance || barrier.reflect_importance;
        let cp = sim_core::ConflictParams::from_config_toml(&text);
        sim.conflict_enabled = conflict || cp.enabled;
        sim.conflict_death_enabled = conflict_death || cp.death_enabled;
        if sheet || sim_core::SheetParams::from_config_toml(&text).enabled {
            sim.enable_sheet();
        }
        let pop = sim_core::PopulationParams::from_config_toml(&text);
        if reproduction || pop.reproduction {
            sim.enable_reproduction();
        }
        if aging || pop.aging {
            sim.enable_aging(pop.childhood_ticks, pop.founder_age_ticks);
        }
        if household_crates || pop.household_crates {
            sim.enable_household_crates();
        }
        if culture || pop.culture {
            sim.enable_culture(pop.culture_count);
        }
        let inv = sim_core::InventionsParams::from_config_toml(&text);
        if inventions || inv.enabled {
            sim.enable_inventions(inv.share_delay_ticks);
            sim.invention_tree = inv.tree;
            sim.invention_patent_ticks = inv.patent_ticks;
        }
        let pipe = sim_core::PipelineParams::from_config_toml(&text);
        sim.pipeline_hash_events = pipeline_events || pipe.hash_events;
        let cat = sim_core::CatalogParams::from_config_toml(&text);
        if let Some(dir) = &objects_dir {
            let _ = sim.apply_objects_dir(dir)?;
        } else if catalog || cat.enabled {
            sim.enable_catalog(Vec::new());
        }
        let tel = sim_core::TelemetryParams::from_config_toml(&text);
        sim.telemetry_enabled = telemetry || tel.enabled;
        sim.telemetry_otlp_endpoint = tel.otlp_endpoint;
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
    let write_timing = overlay
        .metrics
        .timing
        .unwrap_or(out_dir.is_some() || sqlite_path.is_some());

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

    if !listen.is_empty() {
        return server::serve(server::ServeOpts {
            sim,
            ticks: n,
            listen,
            allow_control,
            start_paused,
            token,
            quiet,
            out_dir,
            checkpoint_every,
            write_timing,
            lockstep,
            lockstep_timeout_ms,
            summarize,
            report,
            sqlite: sqlite_path,
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

    let mut sqlite = match &sqlite_path {
        Some(p) => Some(sqlite::SqliteLog::open(p)?),
        None => None,
    };
    if let Some(db) = &mut sqlite {
        db.insert_events(&sim.events.events, &sim.catalog)?;
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
            }
        }
        if let Some(db) = &mut sqlite {
            let events = &sim.events.events;
            if events.len() > last_event {
                db.insert_events(&events[last_event..], &sim.catalog)?;
            }
            db.insert_decisions(&sim.last_tick_decisions)?;
            if write_timing {
                if let Some(t) = &sim.last_tick_timing {
                    db.insert_timing(t)?;
                }
            }
        }
        last_event = sim.events.events.len();
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
          [--out-dir DIR] [--sqlite PATH] [--checkpoint-every K]
          [--load PATH] [--summarize] [--report]
          [--listen tcp://HOST:PORT] [--listen ws://HOST:PORT]
          [--connect tcp://HOST:PORT]
          [--allow-control] [--start-paused] [--lockstep] [--lockstep-timeout-ms N]
          [--token SECRET]
          [--llm-barrier] [--llm-barrier-retries N] [--llm-reflect-on-evict]
          [--llm-reflect-every N] [--llm-plan-every N]
          [--llm-execute-plan] [--llm-reflect-importance]
          [--conflict] [--conflict-death]
          [--sheet] [--reproduction] [--aging]
          [--household-crates] [--culture] [--inventions]
          [--pipeline-events] [--catalog] [--objects DIR] [--telemetry]
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
      --report              Write food-economy report (md/csv); prints markdown if no --out-dir. With --listen, emit when the session ends
      --sqlite PATH         Write events/decisions/timing as sqlite columns (extra sink; JSONL unchanged)
      --llm PROVIDER        mock | wait | ollama | openai_compatible (empty base_url ⇒ mock)
      --listen URL          Repeatable. tcp://host:port and/or ws://host:port (no TLS). Always binds; --report/--summarize do not skip it
      --connect URL         Welcome/Tick hash tail. With --allow-control, stdin slash commands send Control
      --allow-control       Listen: accept Control. Connect: send Control from stdin
      --start-paused        Listen without ticking until a client sends Play (needs --listen and --allow-control)
      --lockstep            After each tick, wait for AckTick from every subscriber (overlay [network] lockstep)
      --lockstep-timeout-ms N  Give up waiting for AckTick after N ms (0 = forever; overlay lockstep_timeout_ms)
      --llm-reflect-on-evict   Summarise dropped memories via LLM (overlay [llm] reflect_on_evict)
      --llm-reflect-every N Periodic insight every N ticks (0 = off; overlay reflect_every_n_ticks)
      --llm-plan-every N    Short-term plan every N ticks (0 = off; overlay plan_every_n_ticks)
      --llm-execute-plan    Execute plan[0] when it is legal action JSON (overlay [llm] execute_plan)
      --conflict            Enable Attack/Flee (overlay [conflict] enabled)
      --conflict-death      0 health removes the agent (implies --conflict; overlay death_enabled)
      --sheet               Roll founder STR/DEX/CON/INT/WIS/CHA (overlay [agents.sheet] enabled)
      --reproduction        PairBond/Reproduce (implies --sheet; overlay [population] reproduction)
      --aging               Accrue age_ticks; childhood gates PairBond/Reproduce/Attack
      --household-crates    Members Store/Retrieve at household home (Chebyshev ≤ 1)
      --culture             Assign founder culture ids; children copy a parent
      --inventions          Invent GatherBonus/MoveBonus/SenseBonus; inventor then society after share_delay_ticks
      --pipeline-events     Hash one Pipeline stage bitmask per living agent per tick (overlay [pipeline] hash_events)
      --catalog             Hash [sim] catalog items from --objects (does not imply --sheet)
      --objects DIR         Object definition TOML directory (default: configs/objects if present)
      --telemetry           Hash-neutral tick aggregates + RSS (overlay [telemetry] enabled)
      --llm-reflect-importance  Extra LLM call after retrieve may rewrite memory importance
      --llm-barrier         Retry timeout/parse (default 3 extra attempts) then Wait; overlay [llm] barrier
      --llm-barrier-retries N  Extra attempts after the first (implies --llm-barrier; 0 = one attempt)
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
