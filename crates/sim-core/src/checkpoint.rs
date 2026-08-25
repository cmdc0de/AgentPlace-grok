//! Versioned binary checkpoints plus derived Markdown summaries.

use crate::agent::Agent;
use crate::board::{Goal, PublicBoard as RichBoard};
use crate::config::ExperimentConfig;
use crate::error::SimError;
use crate::event_log::{EventLog, SimEvent, SimEventKind};
use crate::memory::MemoryMeta;
use crate::seeding::RngBank;
use crate::simulation::Simulation;
use crate::social::RelationshipSummary;
use crate::world::World;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

pub const CHECKPOINT_MAGIC: [u8; 4] = *b"AGTN";
pub const CHECKPOINT_FORMAT_VERSION: u32 = 2;

/// Wire type kept binary-compatible with M3 (`entries: Vec<String>`).
/// M4 stores a hex postcard blob of board + goals in `entries[0]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublicBoard {
    #[serde(default)]
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BoardBlob {
    #[serde(default)]
    board: RichBoard,
    #[serde(default)]
    goals: BTreeMap<u64, Vec<Goal>>,
    #[serde(default)]
    relationships: BTreeMap<u64, BTreeMap<u64, RelationshipSummary>>,
    #[serde(default)]
    memory_meta: BTreeMap<u64, Vec<MemoryMeta>>,
    #[serde(default)]
    next_memory_id: BTreeMap<u64, u64>,
    #[serde(default)]
    influence: BTreeMap<u64, u32>,
}

fn board_to_wire(sim: &Simulation) -> PublicBoard {
    let blob = BoardBlob {
        board: sim.board.clone(),
        goals: sim
            .agents
            .iter()
            .map(|(id, a)| (id.0, a.goals.clone()))
            .collect(),
        relationships: sim
            .agents
            .iter()
            .map(|(id, a)| {
                (
                    id.0,
                    a.relationships
                        .iter()
                        .map(|(oid, r)| (oid.0, r.clone()))
                        .collect(),
                )
            })
            .collect(),
        memory_meta: sim
            .agents
            .iter()
            .map(|(id, a)| {
                (
                    id.0,
                    a.memory
                        .iter()
                        .map(|e| MemoryMeta {
                            id: e.id,
                            participants: e.participants.clone(),
                            valence: e.valence,
                        })
                        .collect(),
                )
            })
            .collect(),
        next_memory_id: sim
            .agents
            .iter()
            .map(|(id, a)| (id.0, a.next_memory_id))
            .collect(),
        influence: sim
            .agents
            .iter()
            .map(|(id, a)| (id.0, a.influence_factor))
            .collect(),
    };
    match postcard::to_allocvec(&blob) {
        Ok(bytes) => PublicBoard {
            entries: vec![hex::encode(bytes)],
        },
        Err(_) => PublicBoard::default(),
    }
}

fn board_from_wire(
    wire: &PublicBoard,
    agents: &mut BTreeMap<crate::agent::AgentId, Agent>,
) -> RichBoard {
    let Some(hex_str) = wire.entries.first() else {
        return RichBoard::default();
    };
    let Ok(bytes) = hex::decode(hex_str) else {
        return RichBoard::default();
    };
    let blob = match postcard::from_bytes::<BoardBlob>(&bytes) {
        Ok(b) => b,
        Err(_) => {
            #[derive(Deserialize)]
            struct BoardBlobM4 {
                #[serde(default)]
                board: RichBoard,
                #[serde(default)]
                goals: BTreeMap<u64, Vec<Goal>>,
            }
            match postcard::from_bytes::<BoardBlobM4>(&bytes) {
                Ok(old) => BoardBlob {
                    board: old.board,
                    goals: old.goals,
                    ..BoardBlob::default()
                },
                Err(_) => return RichBoard::default(),
            }
        }
    };
    for (id, goals) in blob.goals {
        if let Some(agent) = agents.get_mut(&crate::agent::AgentId(id)) {
            agent.goals = goals;
        }
    }
    for (id, rels) in blob.relationships {
        if let Some(agent) = agents.get_mut(&crate::agent::AgentId(id)) {
            agent.relationships = rels
                .into_iter()
                .map(|(oid, r)| (crate::agent::AgentId(oid), r))
                .collect();
        }
    }
    for (id, meta) in blob.memory_meta {
        if let Some(agent) = agents.get_mut(&crate::agent::AgentId(id)) {
            for (entry, m) in agent.memory.iter_mut().zip(meta.into_iter()) {
                entry.id = m.id;
                entry.participants = m.participants;
                entry.valence = m.valence;
            }
        }
    }
    for (id, nid) in blob.next_memory_id {
        if let Some(agent) = agents.get_mut(&crate::agent::AgentId(id)) {
            agent.next_memory_id = nid.max(1);
        }
    }
    for (id, inf) in blob.influence {
        if let Some(agent) = agents.get_mut(&crate::agent::AgentId(id)) {
            agent.influence_factor = inf;
        }
    }
    blob.board
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncentiveState {
    #[serde(default)]
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsState {
    #[serde(default)]
    pub values: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointBody {
    pub tick: u64,
    pub master_seed: u64,
    pub config_hash: [u8; 32],
    /// TOML of `ExperimentConfig` so SeedSpec can round-trip (postcard cannot
    /// `deserialize_any`, which the TOML integer-or-string visitor needs).
    pub config_toml: String,
    pub world: World,
    pub agents: BTreeMap<crate::agent::AgentId, Agent>,
    pub rngs: RngBank,
    pub events: Vec<SimEvent>,
    #[serde(default)]
    pub public_board: PublicBoard,
    #[serde(default)]
    pub active_incentives: IncentiveState,
    #[serde(default)]
    pub metrics: MetricsState,
}

pub fn config_hash(config: &ExperimentConfig) -> Result<[u8; 32], SimError> {
    let bytes = postcard::to_allocvec(config)
        .map_err(|e| SimError::Checkpoint(format!("failed to serialize config for hash: {e}")))?;
    Ok(Sha256::digest(&bytes).into())
}

pub fn experiment_id(hash: &[u8; 32]) -> String {
    hex::encode(&hash[..8])
}

impl Simulation {
    pub fn config_hash(&self) -> Result<[u8; 32], SimError> {
        config_hash(&self.config)
    }

    pub fn to_checkpoint(&self) -> Result<CheckpointBody, SimError> {
        let config_toml = toml::to_string(&self.config)
            .map_err(|e| SimError::Checkpoint(format!("config toml encode failed: {e}")))?;
        Ok(CheckpointBody {
            tick: self.tick,
            master_seed: self.config.master_seed,
            config_hash: config_hash(&self.config)?,
            config_toml,
            world: self.world.clone(),
            agents: self.agents.clone(),
            rngs: self.rngs.clone(),
            events: self.events.events.clone(),
            public_board: board_to_wire(self),
            active_incentives: IncentiveState {
                entries: if self.incentive_toml.is_empty() {
                    Vec::new()
                } else {
                    vec![self.incentive_toml.clone()]
                },
            },
            metrics: MetricsState::default(),
        })
    }

    pub fn from_checkpoint(body: CheckpointBody) -> Result<Self, SimError> {
        let config = ExperimentConfig::from_toml_str(&body.config_toml)?;
        let expected = config_hash(&config)?;
        if expected != body.config_hash && !body.public_board.entries.is_empty() {
            return Err(SimError::Checkpoint(
                "config hash in checkpoint does not match serialized config".into(),
            ));
        }
        if body.master_seed != config.master_seed {
            return Err(SimError::Checkpoint(
                "master_seed in checkpoint does not match config".into(),
            ));
        }
        let replay = crate::simulation::replay_or_record(&config).0;
        let mut agents = body.agents;
        let board = board_from_wire(&body.public_board, &mut agents);
        let inf = config.influence_milli();
        for a in agents.values_mut() {
            if a.influence_factor == 0 {
                a.influence_factor = inf;
            }
            if a.next_memory_id == 0 {
                a.next_memory_id = 1;
            }
        }
        let incentive_toml = body
            .active_incentives
            .entries
            .first()
            .cloned()
            .unwrap_or_default();
        let incentives = if incentive_toml.is_empty() {
            crate::incentive::IncentiveSchedule::default()
        } else {
            crate::incentive::IncentiveSchedule::from_toml_str(&incentive_toml).unwrap_or_default()
        };
        let incentive_active = crate::incentive::reconstruct_active(&body.events);
        Ok(Self {
            config,
            tick: body.tick,
            world: body.world,
            agents,
            rngs: body.rngs,
            events: EventLog {
                events: body.events,
            },
            board,
            chooser: crate::llm::Chooser::Mock,
            replay,
            record_path: None,
            last_tick_decisions: Vec::new(),
            incentives,
            incentive_toml,
            incentive_active,
            last_tick_timing: None,
        })
    }

    pub fn encode_checkpoint(&self) -> Result<Vec<u8>, SimError> {
        encode_checkpoint(&self.to_checkpoint()?)
    }

    pub fn decode_checkpoint(bytes: &[u8]) -> Result<Self, SimError> {
        Self::from_checkpoint(decode_checkpoint(bytes)?)
    }

    pub fn save_checkpoint(&self, path: impl AsRef<Path>) -> Result<(), SimError> {
        save_checkpoint_bytes(path, &self.encode_checkpoint()?)
    }

    pub fn load_checkpoint(path: impl AsRef<Path>) -> Result<Self, SimError> {
        let bytes = fs::read(path)?;
        Self::decode_checkpoint(&bytes)
    }
}

pub fn encode_checkpoint(body: &CheckpointBody) -> Result<Vec<u8>, SimError> {
    let payload = postcard::to_allocvec(body)
        .map_err(|e| SimError::Checkpoint(format!("postcard encode failed: {e}")))?;
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&CHECKPOINT_MAGIC);
    out.extend_from_slice(&CHECKPOINT_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

pub fn decode_checkpoint(bytes: &[u8]) -> Result<CheckpointBody, SimError> {
    if bytes.len() < 8 {
        return Err(SimError::Checkpoint("checkpoint file is truncated".into()));
    }
    if bytes[0..4] != CHECKPOINT_MAGIC {
        return Err(SimError::Checkpoint(format!(
            "invalid magic (expected AGTN, got {:?})",
            &bytes[0..4]
        )));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != CHECKPOINT_FORMAT_VERSION {
        return Err(SimError::Checkpoint(format!(
            "unsupported checkpoint format_version {version} (this binary supports {CHECKPOINT_FORMAT_VERSION})"
        )));
    }
    postcard::from_bytes(&bytes[8..])
        .map_err(|e| SimError::Checkpoint(format!("postcard decode failed: {e}")))
}

pub fn save_checkpoint_bytes(path: impl AsRef<Path>, bytes: &[u8]) -> Result<(), SimError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let tmp = path.with_extension("ckpt.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn save_checkpoint_with_retries(
    sim: &Simulation,
    path: impl AsRef<Path>,
    retries: u32,
) -> Result<(), SimError> {
    let bytes = sim.encode_checkpoint()?;
    let path = path.as_ref();
    let attempts = retries.max(1);
    let mut last_err = None;
    for _ in 0..attempts {
        match save_checkpoint_bytes(path, &bytes) {
            Ok(()) => return Ok(()),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err
        .unwrap_or_else(|| SimError::Checkpoint("checkpoint write failed with no error".into())))
}

pub fn cell_label(world: &World, x: u32, y: u32) -> &'static str {
    if world.is_water(x, y) {
        "water"
    } else if world.has_mineral(x, y) {
        "mineral"
    } else if world.has_vegetation(x, y) {
        "vegetation"
    } else {
        "land"
    }
}

pub fn summary_markdown(sim: &Simulation) -> Result<String, SimError> {
    let hash = sim.config_hash()?;
    let id = experiment_id(&hash);
    let veg: u32 = sim.agents.values().map(|a| a.consumption.vegetation).sum();
    let animal: u32 = sim.agents.values().map(|a| a.consumption.animal).sum();
    let fish: u32 = sim.agents.values().map(|a| a.consumption.fish).sum();
    let toxic: u32 = sim
        .agents
        .values()
        .map(|a| a.consumption.toxic_events)
        .sum();
    let open = sim.board.open().count();
    Ok(format!(
        "# Checkpoint summary\n\n\
         - experiment_id: `{id}`\n\
         - tick: {}\n\
         - master_seed: {}\n\
         - config_hash: `{}`\n\
         - world_hash: `{}`\n\
         - state_hash: `{}`\n\
         - map: {}x{} (max_height {})\n\
         - agents: {}\n\
         - water_cells: {}\n\
         - vegetation_patches: {}\n\
         - mineral_nodes: {}\n\
         - animals: {}\n\
         - fish: {}\n\
         - events: {}\n\
         - consumption: veg {} animal {} fish {} toxic {}\n\
         - board: open {} accepted {} rejected {} expired {} adopted {}\n",
        sim.tick,
        sim.config.master_seed,
        hex::encode(hash),
        sim.world_hash(),
        sim.state_hash(),
        sim.world.width,
        sim.world.height,
        sim.world.max_height,
        sim.agents.len(),
        sim.world.water_count(),
        sim.world.vegetation_count(),
        sim.world.mineral_count(),
        sim.world.animal_total(),
        sim.world.fish_total(),
        sim.events.events.len(),
        veg,
        animal,
        fish,
        toxic,
        open,
        sim.board.accepted_count,
        sim.board.rejected_count,
        sim.board.expired_count,
        sim.board.adopted.len(),
    ))
}

pub fn agents_markdown(sim: &Simulation) -> String {
    let mut out = String::from("# Agent summaries\n\n");
    for agent in sim.agents.values() {
        out.push_str(&format!(
            "## Agent {}\n\n\
             - position: ({}, {})\n\
             - height: {}\n\
             - cell: {}\n\
             - needs: hunger {:.1} thirst {:.1} energy {:.1}\n\
             - illness_ticks: {}\n\
             - inventory_items: {}\n\
             - consumption: veg {} animal {} fish {} toxic {}\n\
             - goals: {}\n\n",
            agent.id.0,
            agent.x,
            agent.y,
            sim.world.height_at(agent.x, agent.y),
            cell_label(&sim.world, agent.x, agent.y),
            agent.needs.hunger as f64 / 100.0,
            agent.needs.thirst as f64 / 100.0,
            agent.needs.energy as f64 / 100.0,
            agent.illness_ticks,
            agent.inventory_count(),
            agent.consumption.vegetation,
            agent.consumption.animal,
            agent.consumption.fish,
            agent.consumption.toxic_events,
            agent
                .goals
                .iter()
                .map(|g| g.text.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    out
}

pub fn event_to_jsonl(event: &SimEvent) -> String {
    let kind = match &event.kind {
        SimEventKind::Wait => "{\"type\":\"wait\"}".to_string(),
        SimEventKind::Rest => "{\"type\":\"rest\"}".to_string(),
        SimEventKind::Move {
            from_x,
            from_y,
            to_x,
            to_y,
        } => format!(
            "{{\"type\":\"move\",\"from_x\":{from_x},\"from_y\":{from_y},\"to_x\":{to_x},\"to_y\":{to_y}}}"
        ),
        SimEventKind::Gather { species, qty, .. } => {
            format!("{{\"type\":\"gather\",\"species\":{species},\"qty\":{qty}}}")
        }
        SimEventKind::Drink => "{\"type\":\"drink\"}".to_string(),
        SimEventKind::Eat { toxic, .. } => format!("{{\"type\":\"eat\",\"toxic\":{toxic}}}"),
        SimEventKind::Hunt { success } => format!("{{\"type\":\"hunt\",\"success\":{success}}}"),
        SimEventKind::Fish { success } => format!("{{\"type\":\"fish\",\"success\":{success}}}"),
        SimEventKind::Farm { species, x, y } => {
            format!("{{\"type\":\"farm\",\"species\":{species},\"x\":{x},\"y\":{y}}}")
        }
        SimEventKind::Craft { success, .. } => {
            format!("{{\"type\":\"craft\",\"success\":{success}}}")
        }
        SimEventKind::Speak {
            shout,
            text,
            broadcast,
            ..
        } => {
            let t = text.replace('\\', "\\\\").replace('"', "\\\"");
            format!(
                "{{\"type\":\"speak\",\"shout\":{shout},\"broadcast\":{broadcast},\"text\":\"{t}\"}}"
            )
        }
        SimEventKind::LlmWait => "{\"type\":\"llm_wait\"}".to_string(),
        SimEventKind::Propose { proposal_id } => {
            format!("{{\"type\":\"propose\",\"id\":{proposal_id}}}")
        }
        SimEventKind::Support { proposal_id } => {
            format!("{{\"type\":\"support\",\"id\":{proposal_id}}}")
        }
        SimEventKind::Oppose { proposal_id } => {
            format!("{{\"type\":\"oppose\",\"id\":{proposal_id}}}")
        }
        SimEventKind::RuleBlocked { reason } => {
            let r = reason.replace('\\', "\\\\").replace('"', "\\\"");
            format!("{{\"type\":\"rule_blocked\",\"reason\":\"{r}\"}}")
        }
        SimEventKind::IncentiveApplied { id, detail } => {
            let i = id.replace('\\', "\\\\").replace('"', "\\\"");
            let d = detail.replace('\\', "\\\\").replace('"', "\\\"");
            format!("{{\"type\":\"incentive_applied\",\"id\":\"{i}\",\"detail\":\"{d}\"}}")
        }
        SimEventKind::IncentiveEnded { id } => {
            let i = id.replace('\\', "\\\\").replace('"', "\\\"");
            format!("{{\"type\":\"incentive_ended\",\"id\":\"{i}\"}}")
        }
        SimEventKind::Died {
            hunger_zero,
            thirst_zero,
        } => format!(
            "{{\"type\":\"died\",\"hunger_zero\":{hunger_zero},\"thirst_zero\":{thirst_zero}}}"
        ),
    };
    format!(
        "{{\"tick\":{},\"agent\":{},\"kind\":{kind}}}",
        event.tick, event.agent.0
    )
}

pub fn append_events_jsonl(path: impl AsRef<Path>, events: &[SimEvent]) -> Result<(), SimError> {
    if events.is_empty() {
        return Ok(());
    }
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for event in events {
        writeln!(file, "{}", event_to_jsonl(event))?;
    }
    Ok(())
}

pub fn write_markdown_summaries(sim: &Simulation, dir: impl AsRef<Path>) -> Result<(), SimError> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let id = experiment_id(&sim.config_hash()?);
    let stem = format!("{id}_tick_{}", sim.tick);
    fs::write(
        dir.join(format!("{stem}_summary.md")),
        summary_markdown(sim)?,
    )?;
    fs::write(dir.join(format!("{stem}_agents.md")), agents_markdown(sim))?;
    Ok(())
}

pub fn checkpoint_stem(sim: &Simulation) -> Result<String, SimError> {
    Ok(format!(
        "{}_tick_{}",
        experiment_id(&sim.config_hash()?),
        sim.tick
    ))
}

pub fn prune_old_checkpoints(dir: impl AsRef<Path>, keep_last_n: u32) -> Result<(), SimError> {
    if keep_last_n == 0 {
        return Ok(());
    }
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(());
    }
    let mut ckpts: Vec<(u64, std::path::PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ckpt") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if let Some(tick) = name.rsplit("_tick_").nth(0).and_then(|s| s.parse().ok()) {
            ckpts.push((tick, path));
        }
    }
    ckpts.sort_by_key(|(tick, _)| *tick);
    let keep = keep_last_n as usize;
    if ckpts.len() <= keep {
        return Ok(());
    }
    let drop_n = ckpts.len() - keep;
    for (_, path) in ckpts.into_iter().take(drop_n) {
        let _ = fs::remove_file(&path);
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let dir = path.parent().unwrap_or(Path::new("."));
            let _ = fs::remove_file(dir.join(format!("{stem}_summary.md")));
            let _ = fs::remove_file(dir.join(format!("{stem}_agents.md")));
        }
    }
    Ok(())
}

/// Write binary checkpoint, optional markdown, and return the ckpt path.
pub fn write_run_checkpoint(
    sim: &Simulation,
    dir: impl AsRef<Path>,
) -> Result<std::path::PathBuf, SimError> {
    let dir = dir.as_ref();
    fs::create_dir_all(dir)?;
    let stem = checkpoint_stem(sim)?;
    let ckpt_path = dir.join(format!("{stem}.ckpt"));
    save_checkpoint_with_retries(sim, &ckpt_path, sim.config.checkpoint.write_retries)?;
    if sim.config.checkpoint.write_markdown_summaries {
        let id_tick = stem.as_str();
        fs::write(
            dir.join(format!("{id_tick}_summary.md")),
            summary_markdown(sim)?,
        )?;
        fs::write(
            dir.join(format!("{id_tick}_agents.md")),
            agents_markdown(sim),
        )?;
    }
    prune_old_checkpoints(dir, sim.config.checkpoint.keep_last_n)?;
    Ok(ckpt_path)
}
