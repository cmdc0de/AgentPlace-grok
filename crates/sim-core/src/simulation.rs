use crate::agent::{Agent, AgentId};
use crate::config::ExperimentConfig;
use crate::error::SimError;
use crate::event_log::{EventLog, SimEvent, SimEventKind};
use crate::seeding::{resolve_seed, RngBank};
use crate::world::World;
use rand::seq::SliceRandom;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

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
}

impl Simulation {
    pub fn new(config: ExperimentConfig) -> Result<Self, SimError> {
        let master = config.master_seed;
        let mut rngs = RngBank::new(master);

        let world_seed = resolve_seed(
            config.world.seed,
            master,
            "world",
            rngs.stream("master"),
        );
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

        Ok(Self {
            config,
            tick: 0,
            world,
            agents,
            rngs,
            events: EventLog::default(),
        })
    }

    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    /// Advance one discrete tick. Returns false if max_ticks has been reached.
    pub fn tick(&mut self) -> bool {
        if self.config.simulation.max_ticks > 0 && self.tick >= self.config.simulation.max_ticks {
            return false;
        }
        self.tick += 1;

        let mut order: Vec<AgentId> = self.agents.keys().copied().collect();
        order.shuffle(self.rngs.stream("turn_order"));

        for id in order {
            self.step_agent(id);
        }
        true
    }

    pub fn run_ticks(&mut self, n: u64) {
        for _ in 0..n {
            if !self.tick() {
                break;
            }
        }
    }

    fn step_agent(&mut self, id: AgentId) {
        let current = *self.agents.get(&id).expect("agent exists");
        let wait = self.rngs.agent_stream(id).random_bool(0.2);
        if wait {
            self.events.push(SimEvent {
                tick: self.tick,
                agent: id,
                kind: SimEventKind::Wait,
            });
            return;
        }

        let dir = self.rngs.agent_stream(id).random_range(0u8..4);
        let (dx, dy) = match dir {
            0 => (0i32, -1),
            1 => (0, 1),
            2 => (-1, 0),
            _ => (1, 0),
        };
        let nx = current.x as i32 + dx;
        let ny = current.y as i32 + dy;
        if !self.world.in_bounds(nx, ny) {
            self.events.push(SimEvent {
                tick: self.tick,
                agent: id,
                kind: SimEventKind::Wait,
            });
            return;
        }
        let to_x = nx as u32;
        let to_y = ny as u32;
        if let Some(agent) = self.agents.get_mut(&id) {
            agent.x = to_x;
            agent.y = to_y;
        }
        self.events.push(SimEvent {
            tick: self.tick,
            agent: id,
            kind: SimEventKind::Move {
                from_x: current.x,
                from_y: current.y,
                to_x,
                to_y,
            },
        });
    }

    pub fn world_hash(&self) -> StateHash {
        StateHash(self.world.hash_bytes())
    }

    pub fn state_hash(&self) -> StateHash {
        let mut hasher = Sha256::new();
        hasher.update(self.tick.to_le_bytes());
        hasher.update(self.config.master_seed.to_le_bytes());
        hasher.update(self.world.hash_bytes());
        for (id, agent) in &self.agents {
            hasher.update(id.0.to_le_bytes());
            hasher.update(agent.x.to_le_bytes());
            hasher.update(agent.y.to_le_bytes());
        }
        for (label, a, b, seed) in self.rngs.fingerprint() {
            hasher.update(label.as_bytes());
            hasher.update(a.to_le_bytes());
            hasher.update(b.to_le_bytes());
            hasher.update(seed.to_le_bytes());
        }
        for event in &self.events.events {
            hasher.update(event.tick.to_le_bytes());
            hasher.update(event.agent.0.to_le_bytes());
            match event.kind {
                SimEventKind::Wait => hasher.update([0u8]),
                SimEventKind::Move {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                } => {
                    hasher.update([1u8]);
                    hasher.update(from_x.to_le_bytes());
                    hasher.update(from_y.to_le_bytes());
                    hasher.update(to_x.to_le_bytes());
                    hasher.update(to_y.to_le_bytes());
                }
            }
        }
        StateHash(hasher.finalize().into())
    }
}

fn spawn_agents(
    config: &ExperimentConfig,
    world: &World,
    rng: &mut rand_chacha::ChaCha20Rng,
) -> Result<BTreeMap<AgentId, Agent>, SimError> {
    let count = config.agents.count;
    let mut occupied: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut agents = BTreeMap::new();
    let cells = (world.width as u64) * (world.height as u64);
    let max_attempts = (count as u64).saturating_mul(16).max(cells);

    for i in 0..count {
        let id = AgentId(i as u64);
        let mut pos = None;
        for _ in 0..max_attempts {
            let x = rng.random_range(0..world.width);
            let y = rng.random_range(0..world.height);
            if occupied.insert((x, y)) {
                pos = Some((x, y));
                break;
            }
        }
        let (x, y) = pos.unwrap_or_else(|| {
            let x = rng.random_range(0..world.width);
            let y = rng.random_range(0..world.height);
            (x, y)
        });
        agents.insert(id, Agent::new(id, x, y));
    }
    Ok(agents)
}
