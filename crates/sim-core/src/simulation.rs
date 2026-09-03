use crate::action::{ChosenAction, Speak, SpeakTarget};
use crate::agent::{Abilities, Agent, AgentId, Needs, Personality};
use crate::board::{Goal, PublicBoard};
use crate::config::{ExperimentConfig, SpawnMode};
use crate::decision_log::{self, DecisionRecord};
use crate::error::SimError;
use crate::event_log::{EventLog, SimEvent, SimEventKind, hash_kind};
use crate::execute::{apply_heard_memories, execute_primary};
use crate::haul::StorageParams;
use crate::incentive::{self, IncentiveSchedule};
use crate::llm::{
    ActionChooser, ChooseError, Chooser, LLM_SKIP_SENTINEL, LLM_WAIT_SENTINEL, REPLAY_CALL_CHOOSE,
    REPLAY_CALL_IMPORTANCE, REPLAY_CALL_PLAN, REPLAY_CALL_REFLECT, REPLAY_CALL_REFLECT_EVICT,
    ReplayRecord, ReplayTable, chosen_to_json, importance_record_json, insight_record_json,
    is_llm_wait_response, is_skip_response, parse_choice_json, parse_importance_json,
    parse_insight_json, parse_plan_json, plan_record_json, prompt_hash, try_parse_plan_step,
};
use crate::memory::{MemoryEntry, MemoryKind};
use crate::observation;
use crate::policy::{avoid_toxic, mock_choose};
use crate::seeding::{RngBank, derive_seed, resolve_seed};
use crate::timing::{self, AgentTiming, TickTiming};
use crate::voting::{CouncilTally, VoteAccept, VoteWeight, VotingParams};
use crate::world::World;
use rand::Rng;
use rand::seq::SliceRandom;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// SHA-256 of canonical simulation state.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateHash(pub [u8; 32]);

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl fmt::Debug for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StateHash({})", self)
    }
}

#[derive(Clone, Debug)]
pub struct Simulation {
    pub config: ExperimentConfig,
    pub tick: u64,
    pub world: World,
    pub agents: BTreeMap<AgentId, Agent>,
    pub rngs: RngBank,
    pub events: EventLog,
    pub board: PublicBoard,
    pub chooser: Chooser,
    pub replay: Option<ReplayTable>,
    pub record_path: Option<PathBuf>,
    pub last_tick_decisions: Vec<DecisionRecord>,
    pub incentives: IncentiveSchedule,
    pub incentive_toml: String,
    pub incentive_active: BTreeSet<String>,
    /// Wall-clock of the last tick. Not hashed.
    pub last_tick_timing: Option<TickTiming>,
    /// Overlay/constants. Not in ExperimentConfig postcard.
    pub storage: StorageParams,
    /// Overlay. Not in ExperimentConfig postcard.
    pub voting: VotingParams,
    /// Runtime meta-rule overrides (from adopted rules). Not ExperimentConfig.
    pub meta_lifetime: Option<u64>,
    pub meta_threshold_milli: Option<u32>,
    pub meta_vote_weight: Option<VoteWeight>,
    pub meta_vote_accept: Option<VoteAccept>,
    pub meta_council: Option<Vec<AgentId>>,
    pub meta_council_tally: Option<CouncilTally>,
    /// Incentive id → agents who already received one-shots.
    pub incentive_oneshot: BTreeMap<String, BTreeSet<AgentId>>,
    /// Overlay `[llm] barrier`. Not hashed, not checkpointed.
    pub llm_barrier: bool,
    /// Extra choose attempts after the first when `llm_barrier`. Default 3.
    pub llm_barrier_retries: u32,
    /// Overlay `[llm] reflect_on_evict`. Not hashed, not checkpointed.
    pub llm_reflect_on_evict: bool,
    /// Overlay `[llm] reflect_every_n_ticks`. 0 = off. Not hashed.
    pub llm_reflect_every_n: u64,
    /// Overlay `[llm] plan_every_n_ticks`. 0 = off. Not hashed.
    pub llm_plan_every_n: u64,
    /// Overlay `[llm] plan_length`. Default 4. Not hashed.
    pub llm_plan_length: u32,
    /// Overlay `[llm] execute_plan`. Not hashed.
    pub llm_execute_plan: bool,
    /// Overlay `[llm] reflect_importance`. Not hashed.
    pub llm_reflect_importance: bool,
    /// Overlay `[conflict] enabled`. Not hashed.
    pub conflict_enabled: bool,
    /// Overlay `[conflict] death_enabled`. Not hashed.
    pub conflict_death_enabled: bool,
    /// Overlay `[agents.sheet] enabled`. Not hashed.
    pub sheet_enabled: bool,
    /// Overlay `[population] reproduction`. Not hashed.
    pub reproduction_enabled: bool,
    /// Next AgentId to assign on birth. Checkpointed in the board blob.
    pub next_agent_id: u64,
    /// Overlay `[population] aging`. Not hashed.
    pub aging_enabled: bool,
    /// Overlay `[population] childhood_ticks`. Default 80. Not hashed.
    pub childhood_ticks: u64,
    /// Overlay `[population] founder_age_ticks`. Default 200. Not hashed.
    pub founder_age_ticks: u64,
    /// Next household id. Checkpointed in the board blob.
    pub next_household_id: u64,
    /// Overlay `[population] household_crates`. Not hashed.
    pub household_crates_enabled: bool,
    /// Household id → home cell. Checkpointed. Hashed when non-empty.
    pub household_home: BTreeMap<u64, (u32, u32)>,
    /// Overlay `[population] culture`. Not hashed.
    pub culture_enabled: bool,
    /// Overlay `[population] culture_count`. Default 4. Not hashed.
    pub culture_count: u8,
    /// Overlay `[inventions] enabled`. Not hashed.
    pub inventions_enabled: bool,
    /// Overlay `[inventions] share_delay_ticks`. Default 8. Not hashed.
    pub invention_share_delay: u64,
    /// Invention table. Checkpointed. Hashed when non-empty.
    pub inventions: BTreeMap<u64, crate::inventions::Invention>,
    pub next_invention_id: u64,
}

impl Simulation {
    pub fn new(config: ExperimentConfig) -> Result<Self, SimError> {
        let master = config.master_seed;
        let mut rngs = RngBank::new(master);

        let world_seed = resolve_seed(config.world.seed, master, "world", rngs.stream("master"));
        rngs.set_resolved("world", world_seed);

        let spawn_seed = resolve_seed(
            config.agents.spawn_seed,
            master,
            "agent_init",
            rngs.stream("master"),
        );
        rngs.set_resolved("agent_init", spawn_seed);

        let world = World::generate(&config.world, world_seed, rngs.stream("world"));
        let agents = spawn_agents(&config, &world, rngs.stream("agent_init"))?;

        for agent in agents.values() {
            let _ = rngs.agent_stream(agent.id);
        }
        let next_agent_id = agents.keys().map(|id| id.0).max().unwrap_or(0).saturating_add(1);

        let chooser = if config.llm.provider == "wait" {
            Chooser::Wait
        } else {
            Chooser::Mock
        };
        let (replay, record_path) = replay_or_record(&config);

        Ok(Self {
            config,
            tick: 0,
            world,
            agents,
            rngs,
            events: EventLog::default(),
            board: PublicBoard::default(),
            chooser,
            replay,
            record_path,
            last_tick_decisions: Vec::new(),
            incentives: IncentiveSchedule::default(),
            incentive_toml: String::new(),
            incentive_active: BTreeSet::new(),
            last_tick_timing: None,
            storage: StorageParams::default(),
            voting: VotingParams::default(),
            meta_lifetime: None,
            meta_threshold_milli: None,
            meta_vote_weight: None,
            meta_vote_accept: None,
            meta_council: None,
            meta_council_tally: None,
            incentive_oneshot: BTreeMap::new(),
            llm_barrier: false,
            llm_barrier_retries: 3,
            llm_reflect_on_evict: false,
            llm_reflect_every_n: 0,
            llm_plan_every_n: 0,
            llm_plan_length: 4,
            llm_execute_plan: false,
            llm_reflect_importance: false,
            conflict_enabled: false,
            conflict_death_enabled: false,
            sheet_enabled: false,
            reproduction_enabled: false,
            next_agent_id,
            aging_enabled: false,
            childhood_ticks: 80,
            founder_age_ticks: 200,
            next_household_id: 1,
            household_crates_enabled: false,
            household_home: BTreeMap::new(),
            culture_enabled: false,
            culture_count: 4,
            inventions_enabled: false,
            invention_share_delay: 8,
            inventions: BTreeMap::new(),
            next_invention_id: 1,
        })
    }

    /// Overlay on: roll unused founder sheets from `agent_init` (not the live spawn stream).
    pub fn enable_sheet(&mut self) {
        self.sheet_enabled = true;
        let spawn = self
            .rngs
            .derived_seeds
            .get("agent_init")
            .copied()
            .unwrap_or(self.config.master_seed);
        let ids: Vec<AgentId> = self.agents.keys().copied().collect();
        for id in ids {
            let unused = self
                .agents
                .get(&id)
                .is_some_and(|a| a.sheet.is_unused());
            if !unused {
                continue;
            }
            let seed = derive_seed(spawn, &format!("sheet_{}", id.0));
            let mut rng = crate::seeding::rng_from_seed(seed);
            if let Some(a) = self.agents.get_mut(&id) {
                a.sheet = crate::sheet::AbilitySheet::roll_3d6(&mut rng);
                if a.health == crate::agent::HEALTH_MAX {
                    a.health = a.sheet.health_max();
                }
            }
        }
    }

    pub fn enable_reproduction(&mut self) {
        self.reproduction_enabled = true;
        self.enable_sheet();
    }

    /// Overlay on: stamp founder age once if still 0. Do not re-stamp on `--load`.
    pub fn enable_aging(&mut self, childhood_ticks: u64, founder_age_ticks: u64) {
        self.aging_enabled = true;
        self.childhood_ticks = childhood_ticks;
        self.founder_age_ticks = founder_age_ticks;
        for a in self.agents.values_mut() {
            if a.age_ticks == 0 && a.kinship.parents.is_empty() {
                a.age_ticks = founder_age_ticks;
            }
        }
    }

    pub fn is_child(&self, agent: &crate::agent::Agent) -> bool {
        self.aging_enabled && agent.age_ticks < self.childhood_ticks
    }

    pub fn enable_household_crates(&mut self) {
        self.household_crates_enabled = true;
    }

    /// Overlay on: assign unused founder culture from `agent_init`. Do not re-roll on `--load`.
    pub fn enable_culture(&mut self, culture_count: u8) {
        self.culture_enabled = true;
        self.culture_count = culture_count.max(1);
        let spawn = self
            .rngs
            .derived_seeds
            .get("agent_init")
            .copied()
            .unwrap_or(self.config.master_seed);
        let count = u32::from(self.culture_count);
        let ids: Vec<AgentId> = self.agents.keys().copied().collect();
        for id in ids {
            let skip = self
                .agents
                .get(&id)
                .is_some_and(|a| a.culture != 0 || !a.kinship.parents.is_empty());
            if skip {
                continue;
            }
            let seed = derive_seed(spawn, &format!("culture_{}", id.0));
            let mut rng = crate::seeding::rng_from_seed(seed);
            let n: u32 = rng.random();
            if let Some(a) = self.agents.get_mut(&id) {
                a.culture = (1 + (n % count)) as u8;
            }
        }
    }

    pub fn enable_inventions(&mut self, share_delay_ticks: u64) {
        self.inventions_enabled = true;
        self.invention_share_delay = share_delay_ticks;
    }

    pub fn share_due_inventions(&mut self) {
        let delay = self.invention_share_delay;
        let tick = self.tick;
        for inv in self.inventions.values_mut() {
            if !inv.shared && tick >= inv.tick.saturating_add(delay) {
                inv.shared = true;
            }
        }
    }

    pub fn refresh_meta(&mut self) {
        self.meta_lifetime = None;
        self.meta_threshold_milli = None;
        self.meta_vote_weight = None;
        self.meta_vote_accept = None;
        self.meta_council = None;
        self.meta_council_tally = None;
        for r in &self.board.adopted {
            match &r.rule {
                Some(crate::board::StructuredRule::SetProposalLifetime { ticks }) => {
                    self.meta_lifetime = Some(*ticks);
                }
                Some(crate::board::StructuredRule::SetAcceptanceThreshold { milli }) => {
                    self.meta_threshold_milli = Some((*milli).clamp(100, 10_000));
                }
                Some(crate::board::StructuredRule::SetVoteWeight { weight }) => {
                    self.meta_vote_weight = Some(*weight);
                }
                Some(crate::board::StructuredRule::SetVoteAccept { accept }) => {
                    self.meta_vote_accept = Some(*accept);
                }
                Some(crate::board::StructuredRule::SetCouncil { ids }) => {
                    self.meta_council = Some(ids.clone());
                }
                Some(crate::board::StructuredRule::SetCouncilTally { tally }) => {
                    self.meta_council_tally = Some(*tally);
                }
                _ => {}
            }
        }
    }

    pub fn effective_lifetime(&self) -> u64 {
        self.meta_lifetime
            .unwrap_or(self.config.proposals.proposal_lifetime_ticks)
    }

    pub fn effective_vote_weight(&self) -> VoteWeight {
        self.meta_vote_weight.unwrap_or(self.voting.weight)
    }

    pub fn effective_vote_accept(&self) -> VoteAccept {
        self.meta_vote_accept.unwrap_or(self.voting.accept)
    }

    pub fn effective_council(&self) -> &[AgentId] {
        self.meta_council
            .as_deref()
            .unwrap_or(self.voting.council.as_slice())
    }

    pub fn effective_council_tally(&self) -> CouncilTally {
        self.meta_council_tally.unwrap_or(self.voting.council_tally)
    }

    pub fn give_item(
        &mut self,
        id: crate::agent::AgentId,
        item: crate::agent::ItemId,
        qty: u32,
    ) -> Result<u32, SimError> {
        if qty == 0 {
            return Err(SimError::Config("give qty must be > 0".into()));
        }
        let Some(agent) = self.agents.get_mut(&id) else {
            return Err(SimError::Config(format!("no agent {}", id.0)));
        };
        let added = agent.try_add_item(item, qty);
        self.events.push(SimEvent {
            tick: self.tick,
            agent: id,
            kind: SimEventKind::Give { item, qty: added },
        });
        Ok(added)
    }

    /// Display 0–100 → millipoints `N * 100` clamped to 0..=10_000.
    pub fn set_display_field(
        &mut self,
        id: crate::agent::AgentId,
        field: &str,
        display: u32,
    ) -> Result<u32, SimError> {
        let milli = display.saturating_mul(100).min(10_000);
        let Some(agent) = self.agents.get_mut(&id) else {
            return Err(SimError::Config(format!("no agent {}", id.0)));
        };
        match field {
            "hunger" => agent.needs.hunger = milli,
            "thirst" => agent.needs.thirst = milli,
            "energy" => agent.needs.energy = milli,
            "influence" => agent.influence_factor = milli,
            other => {
                return Err(SimError::Config(format!(
                    "unknown set field {other:?} (use hunger, thirst, energy, influence, or respect)"
                )));
            }
        }
        Ok(milli)
    }

    /// Display 0–100 → millipoints `N * 100` on the `id → toward` respect edge.
    pub fn set_respect(
        &mut self,
        id: crate::agent::AgentId,
        toward: crate::agent::AgentId,
        display: u32,
    ) -> Result<u32, SimError> {
        let milli = display.saturating_mul(100).min(10_000);
        if !self.agents.contains_key(&id) {
            return Err(SimError::Config(format!("no agent {}", id.0)));
        }
        if !self.agents.contains_key(&toward) {
            return Err(SimError::Config(format!("no agent {}", toward.0)));
        }
        let Some(agent) = self.agents.get_mut(&id) else {
            return Err(SimError::Config(format!("no agent {}", id.0)));
        };
        let row = agent.relationships.entry(toward).or_default();
        row.respect = milli as i16;
        Ok(milli)
    }

    pub fn inject_schedule_toml(&mut self, toml: &str) -> Result<(), SimError> {
        let sched = IncentiveSchedule::from_toml_str(toml)?;
        self.incentives = sched;
        self.incentive_toml = toml.to_string();
        Ok(())
    }

    /// Test/helper: write a memory through the live remember path (may reflect-on-evict).
    pub fn remember_entry(&mut self, id: AgentId, entry: MemoryEntry) {
        crate::execute::remember_agent(self, id, entry);
    }

    pub(crate) fn reflect_on_evict(&mut self, id: AgentId, dropped: Vec<String>) {
        if dropped.is_empty() || !self.llm_reflect_on_evict {
            return;
        }
        let llm_base = self.rngs.derived_seeds.get("llm").copied().unwrap_or(0);
        let seed = derive_seed(
            llm_base,
            &format!("tick_{}_agent_{}_reflect_0", self.tick, id.0),
        );
        let summary = if self.replay.is_some() {
            let Some(raw) = self.replay_call(id, REPLAY_CALL_REFLECT_EVICT) else {
                return;
            };
            let Some(summary) = parse_insight_json(&raw) else {
                return;
            };
            summary
        } else {
            let Chooser::Custom(ch) = &self.chooser else {
                return;
            };
            let ch = Arc::clone(ch);
            match ch.reflect(seed, &dropped) {
                Ok(s) => {
                    let s = s.trim().to_string();
                    if s.is_empty() {
                        self.record_replay(
                            id,
                            seed,
                            String::new(),
                            LLM_SKIP_SENTINEL.into(),
                            REPLAY_CALL_REFLECT_EVICT,
                        );
                        return;
                    }
                    self.record_replay(
                        id,
                        seed,
                        String::new(),
                        insight_record_json(&s),
                        REPLAY_CALL_REFLECT_EVICT,
                    );
                    s
                }
                Err(_) => {
                    self.record_replay(
                        id,
                        seed,
                        String::new(),
                        LLM_SKIP_SENTINEL.into(),
                        REPLAY_CALL_REFLECT_EVICT,
                    );
                    return;
                }
            }
        };
        self.insert_reflection(id, summary);
    }

    fn replay_call(&self, id: AgentId, call: &str) -> Option<String> {
        self.replay
            .as_ref()?
            .get_call(self.tick, id.0, call)
            .filter(|raw| !is_skip_response(raw))
            .map(str::to_string)
    }

    fn record_replay(
        &self,
        id: AgentId,
        call_seed: u64,
        prompt_hash: String,
        response: String,
        call: &str,
    ) {
        let Some(path) = &self.record_path else {
            return;
        };
        let rec = ReplayRecord {
            tick: self.tick,
            agent: id.0,
            call_seed,
            prompt_hash,
            response,
            call: if call == REPLAY_CALL_CHOOSE {
                String::new()
            } else {
                call.to_string()
            },
        };
        if let Ok(line) = serde_json::to_string(&rec) {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{line}")
                });
        }
    }

    fn insert_reflection(&mut self, id: AgentId, summary: String) {
        let tick = self.tick;
        let cap = self.config.memory_capacity();
        let policy = self.config.agents.memory.eviction_policy;
        let bonus = self.config.social_bonus_milli();
        let persist = self.config.agents.memory.persistent_relationships;
        if let Some(a) = self.agents.get_mut(&id) {
            let _ = a.remember(
                cap,
                policy,
                bonus,
                persist,
                MemoryEntry {
                    tick,
                    kind: MemoryKind::Reflection,
                    text: summary,
                    importance: 200,
                    last_accessed: tick,
                    species_tag: 0,
                    id: 0,
                    participants: Vec::new(),
                    valence: 0,
                    ..Default::default()
                },
            );
        }
    }

    fn every_n_fires(n: u64, tick: u64) -> bool {
        n > 0 && tick > 0 && tick % n == 0
    }

    fn step_importance(&mut self, id: AgentId, retrieved_ids: &[u64]) {
        if !self.llm_reflect_importance {
            return;
        }
        let llm_base = self.rngs.derived_seeds.get("llm").copied().unwrap_or(0);
        let seed = derive_seed(
            llm_base,
            &format!("tick_{}_agent_{}_importance_0", self.tick, id.0),
        );
        let raw = if self.replay.is_some() {
            let Some(raw) = self.replay_call(id, REPLAY_CALL_IMPORTANCE) else {
                return;
            };
            raw
        } else {
            let Chooser::Custom(ch) = &self.chooser else {
                return;
            };
            let ch = Arc::clone(ch);
            let hints: Vec<(u64, u8, String)> = self
                .agents
                .get(&id)
                .map(|a| {
                    retrieved_ids
                        .iter()
                        .filter_map(|mid| {
                            a.memory
                                .iter()
                                .find(|e| e.id == *mid)
                                .map(|e| (e.id, e.importance, e.text.clone()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            if hints.is_empty() {
                return;
            }
            match ch.importance(seed, &hints) {
                Ok(s) => match parse_importance_json(s.trim()) {
                    Some((mem_id, imp)) => {
                        let rec = importance_record_json(mem_id, imp);
                        self.record_replay(
                            id,
                            seed,
                            String::new(),
                            rec.clone(),
                            REPLAY_CALL_IMPORTANCE,
                        );
                        rec
                    }
                    None => {
                        self.record_replay(
                            id,
                            seed,
                            String::new(),
                            LLM_SKIP_SENTINEL.into(),
                            REPLAY_CALL_IMPORTANCE,
                        );
                        return;
                    }
                },
                Err(_) => {
                    self.record_replay(
                        id,
                        seed,
                        String::new(),
                        LLM_SKIP_SENTINEL.into(),
                        REPLAY_CALL_IMPORTANCE,
                    );
                    return;
                }
            }
        };
        let Some((mem_id, imp)) = parse_importance_json(&raw) else {
            return;
        };
        if let Some(a) = self.agents.get_mut(&id) {
            if let Some(e) = a.memory.iter_mut().find(|e| e.id == mem_id) {
                e.importance = imp;
            }
        }
    }

    fn step_insight(&mut self, id: AgentId, obs: &observation::Observation) {
        let forced = crate::incentive::force_reflect(self, id);
        if !Self::every_n_fires(self.llm_reflect_every_n, self.tick) && !forced {
            return;
        }
        let llm_base = self.rngs.derived_seeds.get("llm").copied().unwrap_or(0);
        let seed = derive_seed(
            llm_base,
            &format!("tick_{}_agent_{}_insight_0", self.tick, id.0),
        );
        let hash = prompt_hash(obs);
        let summary = if self.replay.is_some() {
            let Some(raw) = self.replay_call(id, REPLAY_CALL_REFLECT) else {
                return;
            };
            let Some(summary) = parse_insight_json(&raw) else {
                return;
            };
            summary
        } else {
            let Chooser::Custom(ch) = &self.chooser else {
                return;
            };
            let ch = Arc::clone(ch);
            match ch.insight(seed, obs) {
                Ok(s) => {
                    let s = s.trim().to_string();
                    if s.is_empty() {
                        self.record_replay(
                            id,
                            seed,
                            hash,
                            LLM_SKIP_SENTINEL.into(),
                            REPLAY_CALL_REFLECT,
                        );
                        return;
                    }
                    self.record_replay(
                        id,
                        seed,
                        hash,
                        insight_record_json(&s),
                        REPLAY_CALL_REFLECT,
                    );
                    s
                }
                Err(_) => {
                    self.record_replay(
                        id,
                        seed,
                        hash,
                        LLM_SKIP_SENTINEL.into(),
                        REPLAY_CALL_REFLECT,
                    );
                    return;
                }
            }
        };
        crate::execute::remember_agent(
            self,
            id,
            MemoryEntry {
                tick: self.tick,
                kind: MemoryKind::Reflection,
                text: summary,
                importance: 200,
                last_accessed: self.tick,
                species_tag: 0,
                id: 0,
                participants: Vec::new(),
                valence: 0,
                ..Default::default()
            },
        );
    }

    fn step_plan(&mut self, id: AgentId, obs: &mut observation::Observation) {
        if !Self::every_n_fires(self.llm_plan_every_n, self.tick) {
            return;
        }
        let max_len = self.llm_plan_length as usize;
        let llm_base = self.rngs.derived_seeds.get("llm").copied().unwrap_or(0);
        let seed = derive_seed(
            llm_base,
            &format!("tick_{}_agent_{}_plan_0", self.tick, id.0),
        );
        let hash = prompt_hash(obs);
        let steps = if self.replay.is_some() {
            let Some(raw) = self.replay_call(id, REPLAY_CALL_PLAN) else {
                return;
            };
            let Some(steps) = parse_plan_json(&raw, max_len) else {
                return;
            };
            steps
        } else {
            let Chooser::Custom(ch) = &self.chooser else {
                return;
            };
            let ch = Arc::clone(ch);
            match ch.plan(seed, obs, max_len) {
                Ok(s) => {
                    let steps: Vec<String> = s
                        .into_iter()
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .take(max_len)
                        .collect();
                    if steps.is_empty() {
                        self.record_replay(
                            id,
                            seed,
                            hash,
                            LLM_SKIP_SENTINEL.into(),
                            REPLAY_CALL_PLAN,
                        );
                        return;
                    }
                    self.record_replay(
                        id,
                        seed,
                        hash,
                        plan_record_json(&steps),
                        REPLAY_CALL_PLAN,
                    );
                    steps
                }
                Err(_) => {
                    self.record_replay(
                        id,
                        seed,
                        hash,
                        LLM_SKIP_SENTINEL.into(),
                        REPLAY_CALL_PLAN,
                    );
                    return;
                }
            }
        };
        if let Some(a) = self.agents.get_mut(&id) {
            a.plan = steps;
        }
        obs.plan = self
            .agents
            .get(&id)
            .map(|a| a.plan.clone())
            .unwrap_or_default();
    }

    fn try_execute_plan(
        &mut self,
        id: AgentId,
        obs: &observation::Observation,
    ) -> Option<ChosenAction> {
        if !self.llm_execute_plan {
            return None;
        }
        let step = self.agents.get(&id)?.plan.first()?.clone();
        let choice = try_parse_plan_step(&step, &obs.legal, &self.config.world.species)?;
        if let Some(a) = self.agents.get_mut(&id) {
            if !a.plan.is_empty() {
                a.plan.remove(0);
            }
        }
        Some(choice)
    }

    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    pub fn vote_weight_of(&self, id: AgentId) -> u64 {
        match self.effective_vote_weight() {
            VoteWeight::Equal => 1,
            VoteWeight::Influence => self
                .agents
                .get(&id)
                .map(|a| a.sheet.influence_vote_weight(a.influence_factor))
                .unwrap_or(1),
            VoteWeight::Respect => self.incoming_respect_sum(id).max(1),
        }
    }

    /// Sum of `max(other.relationships[id].respect, 0)` over other living agents.
    pub fn incoming_respect_sum(&self, id: AgentId) -> u64 {
        let mut sum = 0u64;
        for (oid, other) in &self.agents {
            if *oid == id {
                continue;
            }
            let r = other
                .relationships
                .get(&id)
                .map(|row| row.respect)
                .unwrap_or(0);
            if r > 0 {
                sum += r as u64;
            }
        }
        sum
    }

    pub fn vote_weight_map(&self) -> BTreeMap<AgentId, u64> {
        self.agents
            .keys()
            .copied()
            .map(|id| (id, self.vote_weight_of(id)))
            .collect()
    }

    pub fn living_vote_total(&self) -> u64 {
        self.vote_weight_map().values().copied().sum()
    }

    pub fn vote_need(&self) -> u64 {
        let th = incentive::proposal_threshold(self);
        ((th * self.living_vote_total() as f64).ceil() as u64).max(1)
    }

    pub fn proposal_yes_no_weight(&self, p: &crate::board::Proposal) -> (u64, u64) {
        let yes: u64 = p.supporters.iter().map(|id| self.vote_weight_of(*id)).sum();
        let no: u64 = p.opposers.iter().map(|id| self.vote_weight_of(*id)).sum();
        (yes, no)
    }

    pub fn tick(&mut self) -> bool {
        if self.config.simulation.max_ticks > 0 && self.tick >= self.config.simulation.max_ticks {
            return false;
        }
        if self.config.simulation.pause_when_empty && self.agents.is_empty() {
            return false;
        }
        let wall0 = Instant::now();
        self.tick += 1;
        self.share_due_inventions();
        self.last_tick_decisions.clear();
        let aging = self.aging_enabled;
        for a in self.agents.values_mut() {
            a.gathers_this_tick = 0;
            if aging {
                a.age_ticks = a.age_ticks.saturating_add(1);
            }
        }
        if self.config.agents.social.track_relationships {
            let step = self.config.influence_decay_milli();
            for a in self.agents.values_mut() {
                crate::social::decay_map(&mut a.relationships, step);
            }
        }
        let inc0 = Instant::now();
        incentive::sync(self);
        let incentive_ns = timing::ns_since(inc0);
        let world0 = Instant::now();
        self.world_step();
        self.reap_dead();
        let world_ns = timing::ns_since(world0);
        let board0 = Instant::now();
        self.refresh_meta();
        let th = incentive::proposal_threshold(self);
        let life = self.effective_lifetime();
        let tick = self.tick;
        match self.effective_vote_accept() {
            VoteAccept::Majority => {
                let weights = self.vote_weight_map();
                let total: u64 = weights.values().copied().sum();
                self.board.tick_lifecycle(
                    total,
                    |id| weights.get(&id).copied().unwrap_or(1),
                    th,
                    life,
                    tick,
                );
            }
            VoteAccept::Unanimous => {
                let living: Vec<AgentId> = self.agents.keys().copied().collect();
                self.board.tick_stance_complete(&living, life, tick);
            }
            VoteAccept::Council => {
                let living: Vec<AgentId> = self
                    .effective_council()
                    .iter()
                    .copied()
                    .filter(|id| self.agents.contains_key(id))
                    .collect();
                match self.effective_council_tally() {
                    CouncilTally::Unanimous => {
                        self.board.tick_stance_complete(&living, life, tick);
                    }
                    CouncilTally::Majority => {
                        let weights = self.vote_weight_map();
                        let total: u64 = living
                            .iter()
                            .map(|id| weights.get(id).copied().unwrap_or(1))
                            .sum();
                        let council: BTreeSet<AgentId> = living.iter().copied().collect();
                        self.board.tick_lifecycle(
                            total,
                            |id| {
                                if council.contains(&id) {
                                    weights.get(&id).copied().unwrap_or(1)
                                } else {
                                    0
                                }
                            },
                            th,
                            life,
                            tick,
                        );
                    }
                }
            }
        }
        let board_ns = timing::ns_since(board0);
        let agents0 = Instant::now();
        let mut order: Vec<AgentId> = self.agents.keys().copied().collect();
        order.shuffle(self.rngs.stream("turn_order"));
        let mut agent_times = Vec::with_capacity(order.len());
        for id in order {
            agent_times.push(self.step_agent(id));
        }
        let agents_ns = timing::ns_since(agents0);
        for a in self.agents.values_mut() {
            a.gathers_this_tick = 0;
        }
        self.last_tick_timing = Some(TickTiming {
            tick: self.tick,
            wall_ns: timing::ns_since(wall0),
            world_ns,
            board_ns,
            incentive_ns,
            agents_ns,
            agents: agent_times,
        });
        true
    }

    pub fn run_ticks(&mut self, n: u64) {
        for _ in 0..n {
            if !self.tick() {
                break;
            }
        }
    }

    fn reap_dead(&mut self) {
        if !self.config.needs.death_enabled {
            return;
        }
        let dead: Vec<(crate::agent::AgentId, bool, bool)> = self
            .agents
            .iter()
            .filter(|(_, a)| a.needs.hunger == 0 || a.needs.thirst == 0)
            .map(|(id, a)| (*id, a.needs.hunger == 0, a.needs.thirst == 0))
            .collect();
        for (id, hunger_zero, thirst_zero) in dead {
            self.events.push(SimEvent {
                tick: self.tick,
                agent: id,
                kind: SimEventKind::Died {
                    hunger_zero,
                    thirst_zero,
                },
            });
            self.agents.remove(&id);
        }
    }

    fn world_step(&mut self) {
        let hunger_d = self.config.hunger_decay_milli();
        let thirst_d = self.config.thirst_decay_milli();
        let energy_d = self.config.energy_decay_milli();
        for agent in self.agents.values_mut() {
            agent.needs.hunger = agent.needs.hunger.saturating_sub(hunger_d);
            agent.needs.thirst = agent.needs.thirst.saturating_sub(thirst_d);
            let extra = if agent.illness_ticks > 0 { energy_d } else { 0 };
            agent.needs.energy = agent.needs.energy.saturating_sub(energy_d + extra);
            if agent.illness_ticks > 0 {
                agent.illness_ticks -= 1;
            }
        }
        let tick = self.tick;
        let species = self.config.world.species.clone();
        let mut ready = Vec::new();
        for (&pos, crop) in &self.world.crops {
            let grow = species
                .veg(crop.species_tag)
                .map(|s| s.grow_ticks)
                .unwrap_or(40);
            if tick >= crop.planted_tick + grow {
                ready.push((pos, crop.species_tag));
            }
        }
        for (pos, tag) in ready {
            self.world.crops.remove(&pos);
            self.world.set_vegetation(pos.0, pos.1, tag);
        }
        let regen = self.rngs.stream("event").random_bool(0.05);
        if regen {
            let land = self.world.land_cells();
            if !land.is_empty() {
                let idx = self.rngs.stream("event").random_range(0..land.len());
                let (x, y) = land[idx];
                if self.world.animal_count_at(x, y) < 3 {
                    self.world.add_animal(x, y, 1);
                }
            }
        }
    }

    fn step_agent(&mut self, id: AgentId) -> AgentTiming {
        let mut timing = AgentTiming::new(id);
        if self.agents.get(&id).is_none() {
            return timing;
        }
        if self
            .agents
            .get(&id)
            .is_some_and(|a| a.incapacitated)
        {
            execute_primary(self, id, &crate::action::PrimaryAction::Wait);
            return timing;
        }
        let p0 = Instant::now();
        let mut obs = observation::build(self, id);
        apply_heard_memories(self, id, &obs.heard);
        timing.perceive_ns = timing::ns_since(p0);
        let r0 = Instant::now();

        let llm_base = self.rngs.derived_seeds.get("llm").copied().unwrap_or(0);
        let call_seed = derive_seed(
            llm_base,
            &format!("tick_{}_agent_{}_call_0", self.tick, id.0),
        );
        let mut allow_speak = true;
        let mut chooser_name = "mock";
        let mut policy_branch;
        let retrieved_ids: Vec<u64> = {
            let bonus = self.config.social_bonus_milli();
            let k = self.config.agents.memory.retrieval_k as usize;
            let query = if self.config.agents.memory.enable_embeddings {
                let mut s = format!("h{} t{} e{}", obs.hunger, obs.thirst, obs.energy);
                for h in &obs.heard {
                    s.push(' ');
                    s.push_str(&h.text);
                }
                for t in &obs.toxins {
                    s.push(' ');
                    s.push_str(t);
                }
                Some(s)
            } else {
                None
            };
            self.agents
                .get(&id)
                .map(|a| {
                    crate::memory::retrieve(&a.memory, k, bonus, query.as_deref())
                        .into_iter()
                        .map(|e| e.id)
                        .collect()
                })
                .unwrap_or_default()
        };
        timing.retrieve_ns = timing::ns_since(r0);
        self.step_importance(id, &retrieved_ids);
        let rf0 = Instant::now();
        self.step_insight(id, &obs);
        timing.reflect_ns = timing::ns_since(rf0);
        let pl0 = Instant::now();
        self.step_plan(id, &mut obs);
        timing.plan_ns = timing::ns_since(pl0);
        let hash = prompt_hash(&obs);
        let relation_ids: Vec<u64> = obs.relationships.iter().map(|r| r.id.0).collect();

        let s0 = Instant::now();
        let mut record_raw: Option<String> = None;
        let mut chosen = if let Some(c) = self.try_execute_plan(id, &obs) {
            chooser_name = "plan";
            policy_branch = "execute_plan";
            record_raw = Some(chosen_to_json(&c));
            c
        } else if let Some(raw) = self
            .replay
            .as_ref()
            .and_then(|t| t.get(self.tick, id.0).map(|s| s.to_string()))
        {
            chooser_name = "replay";
            if is_llm_wait_response(&raw) {
                policy_branch = "llm_wait";
                allow_speak = false;
                self.events.push(SimEvent {
                    tick: self.tick,
                    agent: id,
                    kind: SimEventKind::LlmWait,
                });
                ChosenAction::wait()
            } else {
                policy_branch = "replay";
                parse_choice_json(&raw, &obs.legal, &self.config.world.species)
                    .unwrap_or_else(|_| ChosenAction::wait())
            }
        } else {
            match &self.chooser {
                Chooser::Wait => {
                    chooser_name = "wait";
                    policy_branch = "wait";
                    allow_speak = false;
                    record_raw = Some(LLM_WAIT_SENTINEL.into());
                    self.events.push(SimEvent {
                        tick: self.tick,
                        agent: id,
                        kind: SimEventKind::LlmWait,
                    });
                    ChosenAction::wait()
                }
                Chooser::Custom(chooser) => {
                    chooser_name = "llm";
                    match self.choose_custom(chooser.as_ref(), call_seed, &obs) {
                        Ok((c, raw)) => {
                            policy_branch = "llm";
                            record_raw = Some(raw);
                            c
                        }
                        Err(_) => {
                            policy_branch = "llm_wait";
                            allow_speak = false;
                            record_raw = Some(LLM_WAIT_SENTINEL.into());
                            self.events.push(SimEvent {
                                tick: self.tick,
                                agent: id,
                                kind: SimEventKind::LlmWait,
                            });
                            ChosenAction::wait()
                        }
                    }
                }
                Chooser::Mock => {
                    let Some(agent) = self.agents.get(&id) else {
                        return timing;
                    };
                    let filtered = avoid_toxic(&obs, &agent.memory);
                    let identified = filtered.agents.iter().any(|a| a.id.is_some());
                    let thirst = agent.needs.thirst;
                    let hunger = agent.needs.hunger;
                    let energy = agent.needs.energy;
                    let memory = agent.memory.clone();
                    let last_warn = agent.last_warn_tick;
                    let influence = agent.influence_factor;
                    let agree = agent.personality.agreeableness;
                    let rels = agent.relationships.clone();
                    let thresh = self.config.trust_threshold_milli();
                    let rng = self.rngs.agent_stream(id);
                    let (action, branch) = mock_choose(
                        &filtered,
                        rng,
                        thirst,
                        hunger,
                        energy,
                        self.config.thirst_max_milli(),
                        self.config.hunger_max_milli(),
                        self.config.energy_max_milli(),
                        &memory,
                        last_warn,
                        self.tick,
                        self.config.communication.warn_cooldown_ticks,
                        &self.config.world.species,
                        identified,
                        &rels,
                        influence,
                        thresh,
                        agree,
                    );
                    policy_branch = branch;
                    action
                }
            }
        };

        if !observation::is_legal_choice(&obs.legal, &chosen.primary) {
            chosen.primary = crate::action::PrimaryAction::Wait;
            if policy_branch != "llm_wait" && chooser_name != "wait" {
                policy_branch = "wait";
            }
        }
        if !allow_speak {
            chosen.speak = None;
        }
        timing.select_ns = timing::ns_since(s0);

        self.last_tick_decisions.push(decision_log::record(
            self.tick,
            id,
            chooser_name,
            policy_branch,
            call_seed,
            hash.clone(),
            retrieved_ids,
            relation_ids,
            &obs.legal,
            &chosen,
            None,
        ));

        self.record_replay(
            id,
            call_seed,
            hash,
            record_raw.unwrap_or_else(|| chosen_to_json(&chosen)),
            REPLAY_CALL_CHOOSE,
        );

        let e0 = Instant::now();
        execute_primary(self, id, &chosen.primary);
        timing.execute_ns = timing::ns_since(e0);
        let m0 = Instant::now();
        if allow_speak {
            if let Some(speak) = chosen.speak {
                self.execute_speak(id, speak);
            }
        }
        timing.remember_ns = timing::ns_since(m0);
        timing
    }

    fn choose_custom(
        &self,
        chooser: &dyn ActionChooser,
        call_seed: u64,
        obs: &observation::Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        let attempts = if self.llm_barrier {
            1u32.saturating_add(self.llm_barrier_retries)
        } else {
            1
        };
        let mut last = ChooseError::Unreachable;
        for _ in 0..attempts {
            match chooser.choose(call_seed, obs) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last = e;
                    if !self.llm_barrier {
                        return Err(e);
                    }
                }
            }
        }
        Err(last)
    }

    fn execute_speak(&mut self, id: AgentId, mut speak: Speak) {
        let max_len = self.config.communication.max_message_length as usize;
        if speak.text.chars().count() > max_len {
            speak.text = speak.text.chars().take(max_len).collect();
        }
        if speak.text.is_empty() {
            return;
        }
        let Some(speaker) = self.agents.get(&id).cloned() else {
            return;
        };
        let mut shout = speak.shout;
        if shout {
            let cost = self.config.shout_energy_milli();
            if speaker.needs.energy < cost {
                shout = false;
            } else if let Some(a) = self.agents.get_mut(&id) {
                a.needs.energy -= cost;
            }
        }
        let tick = self.tick;
        if let Some(a) = self.agents.get_mut(&id) {
            a.last_warn_tick = tick;
        }
        crate::execute::remember_agent(
            self,
            id,
            MemoryEntry {
                tick,
                kind: MemoryKind::Utterance,
                text: format!("said: {}", speak.text),
                importance: 40,
                last_accessed: tick,
                species_tag: 0,
                id: 0,
                participants: Vec::new(),
                valence: 0,
                ..Default::default()
            },
        );
        let (broadcast, targets) = match speak.to {
            SpeakTarget::Broadcast => (true, Vec::new()),
            SpeakTarget::Directed(ids) => {
                let ident = crate::observation::perceive_range(
                    self.config.observation.base_agent_identity_range,
                    speaker.personality.perceptiveness,
                    speaker.sheet.wisdom,
                );
                let valid: Vec<AgentId> = ids
                    .into_iter()
                    .filter(|tid| {
                        self.agents.get(tid).is_some_and(|t| {
                            crate::observation::chebyshev(speaker.x, speaker.y, t.x, t.y) <= ident
                        })
                    })
                    .collect();
                (false, valid)
            }
        };
        if self.config.agents.social.track_relationships {
            let partners: Vec<AgentId> = if broadcast {
                let hear = crate::observation::perceive_range(
                    self.config
                        .communication
                        .base_speech_range
                        .max(self.config.observation.base_hearing_range),
                    speaker.personality.perceptiveness,
                    speaker.sheet.wisdom,
                );
                self.agents
                    .values()
                    .filter(|t| t.id != id)
                    .filter(|t| {
                        let dist = crate::observation::chebyshev(speaker.x, speaker.y, t.x, t.y);
                        let ident = crate::observation::perceive_range(
                            self.config.observation.base_agent_identity_range,
                            t.personality.perceptiveness,
                            t.sheet.wisdom,
                        );
                        dist <= hear && dist <= ident
                    })
                    .map(|t| t.id)
                    .collect()
            } else {
                targets.clone()
            };
            let (fwd, back) = if shout {
                (crate::social::SHOUT, crate::social::SHOUT_BACK)
            } else {
                (crate::social::SPEAK, crate::social::SPEAK_BACK)
            };
            for pid in &partners {
                if let Some(a) = self.agents.get_mut(&id) {
                    crate::social::apply_delta(
                        &mut a.relationships,
                        *pid,
                        tick,
                        fwd.0,
                        fwd.1,
                        fwd.2,
                        fwd.3,
                        None,
                    );
                }
                if let Some(a) = self.agents.get_mut(pid) {
                    crate::social::apply_delta(
                        &mut a.relationships,
                        id,
                        tick,
                        back.0,
                        back.1,
                        back.2,
                        back.3,
                        None,
                    );
                }
            }
        }
        self.events.push(SimEvent {
            tick: self.tick,
            agent: id,
            kind: SimEventKind::Speak {
                shout,
                text: speak.text,
                broadcast,
                targets,
            },
        });
    }

    pub fn apply_speak(&mut self, id: AgentId, speak: Speak) {
        self.execute_speak(id, speak);
    }

    pub fn world_hash(&self) -> StateHash {
        StateHash(self.world.hash_bytes())
    }

    pub fn state_hash(&self) -> StateHash {
        let mut hasher = Sha256::new();
        hasher.update(self.tick.to_le_bytes());
        hasher.update(self.config.master_seed.to_le_bytes());
        hasher.update(self.world.hash_bytes());
        self.board.hash_into(&mut hasher);
        for agent in self.agents.values() {
            agent.hash_bytes(&mut hasher);
        }
        if !self.household_home.is_empty() {
            for (hid, (x, y)) in &self.household_home {
                hasher.update(hid.to_le_bytes());
                hasher.update(x.to_le_bytes());
                hasher.update(y.to_le_bytes());
            }
        }
        crate::inventions::hash_table(&self.inventions, &mut hasher);
        for (label, a, b, seed) in self.rngs.fingerprint() {
            hasher.update(label.as_bytes());
            hasher.update(a.to_le_bytes());
            hasher.update(b.to_le_bytes());
            hasher.update(seed.to_le_bytes());
        }
        for event in &self.events.events {
            hasher.update(event.tick.to_le_bytes());
            hasher.update(event.agent.0.to_le_bytes());
            hash_kind(&event.kind, &mut hasher);
        }
        StateHash(hasher.finalize().into())
    }
}

pub(crate) fn replay_or_record(
    config: &ExperimentConfig,
) -> (Option<ReplayTable>, Option<PathBuf>) {
    let path = config.llm.replay_file.trim();
    if path.is_empty() {
        return (None, None);
    }
    let p = PathBuf::from(path);
    if p.is_file() {
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        (Some(ReplayTable::from_jsonl(&text)), None)
    } else {
        (None, Some(p))
    }
}

fn spawn_agents(
    config: &ExperimentConfig,
    world: &World,
    rng: &mut rand_chacha::ChaCha20Rng,
) -> Result<BTreeMap<AgentId, Agent>, SimError> {
    let count = config.agents.count as usize;
    let mut land = world.land_cells();
    if land.len() < count {
        return Err(SimError::Config(format!(
            "not enough land cells ({}) for {} agents",
            land.len(),
            count
        )));
    }

    let chosen = match config.agents.spawn_mode {
        SpawnMode::Scattered => {
            land.shuffle(rng);
            land
        }
        SpawnMode::Clustered => cluster_cells(world, &land, count, rng),
        SpawnMode::FixedList => {
            return Err(SimError::Config(
                "spawn_mode=fixed_list requires agents.spawn_list (not implemented in M2)".into(),
            ));
        }
    };

    let mut agents = BTreeMap::new();
    let mut occupied: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (i, &(x, y)) in chosen.iter().take(count).enumerate() {
        if !occupied.insert((x, y)) {
            continue;
        }
        let id = AgentId(i as u64);
        let mut agent = Agent::new(id, x, y);
        agent.inventory_cap = config.agents.inventory_capacity;
        if config.agents.start_with_basic_needs {
            agent.needs = Needs::maxed(
                config.hunger_max_milli(),
                config.thirst_max_milli(),
                config.energy_max_milli(),
            );
        }
        sample_body(&mut agent, config, rng);
        agent.influence_factor = config.influence_milli();
        if config.agents.start_with_basic_needs {
            let mut goals = default_goals();
            let cap = config.agents.goals.max_personal_goals as usize;
            if goals.len() > cap {
                goals.truncate(cap);
            }
            agent.goals = goals;
        }
        agents.insert(id, agent);
    }
    if agents.len() < count {
        return Err(SimError::Config(format!(
            "failed to place {} unique land spawns (placed {})",
            count,
            agents.len()
        )));
    }
    Ok(agents)
}

fn default_goals() -> Vec<Goal> {
    vec![
        Goal {
            id: 0,
            text: "stay fed".into(),
            priority: 80,
            source: "spawn".into(),
        },
        Goal {
            id: 1,
            text: "avoid known toxins".into(),
            priority: 70,
            source: "spawn".into(),
        },
    ]
}

fn sample_body(agent: &mut Agent, config: &ExperimentConfig, rng: &mut rand_chacha::ChaCha20Rng) {
    if config.agents.archetypes.is_empty() {
        agent.abilities = Abilities::default();
        agent.personality = Personality::default();
    } else {
        let total: f64 = config
            .agents
            .archetypes
            .iter()
            .map(|a| a.weight.max(0.0))
            .sum();
        let mut pick = rng.random::<f64>() * total.max(0.0001);
        let mut chosen = &config.agents.archetypes[0];
        for arch in &config.agents.archetypes {
            pick -= arch.weight.max(0.0);
            if pick <= 0.0 {
                chosen = arch;
                break;
            }
        }
        agent.abilities = chosen.abilities;
        agent.personality = chosen.personality.clone();
    }
    // ~10% allergic to nightshade / solanaceae
    if rng.random_bool(0.1)
        && !agent
            .personality
            .allergy_tags
            .iter()
            .any(|t| t == "solanaceae")
    {
        agent.personality.allergy_tags.push("solanaceae".into());
    }
}

fn cluster_cells(
    world: &World,
    land: &[(u32, u32)],
    count: usize,
    rng: &mut rand_chacha::ChaCha20Rng,
) -> Vec<(u32, u32)> {
    let origin = land[rng.random_range(0..land.len())];
    let (ox, oy) = origin;
    let mut chosen = Vec::new();
    let mut seen: BTreeSet<(u32, u32)> = BTreeSet::new();
    let max_r = world.width.max(world.height);
    for r in 0..=max_r {
        let mut ring = Vec::new();
        let x0 = ox.saturating_sub(r);
        let x1 = (ox + r).min(world.width.saturating_sub(1));
        let y0 = oy.saturating_sub(r);
        let y1 = (oy + r).min(world.height.saturating_sub(1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as i32 - ox as i32).unsigned_abs();
                let dy = (y as i32 - oy as i32).unsigned_abs();
                if dx.max(dy) != r {
                    continue;
                }
                if world.is_land(x, y) && seen.insert((x, y)) {
                    ring.push((x, y));
                }
            }
        }
        ring.shuffle(rng);
        chosen.extend(ring);
        if chosen.len() >= count {
            break;
        }
    }
    if chosen.len() < count {
        let mut rest: Vec<(u32, u32)> =
            land.iter().copied().filter(|c| !seen.contains(c)).collect();
        rest.shuffle(rng);
        chosen.extend(rest);
    }
    chosen
}
