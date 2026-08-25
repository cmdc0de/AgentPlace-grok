use crate::action::{ChosenAction, PrimaryAction, Speak, SpeakTarget};
use crate::agent::ItemId;
use crate::memory::knows_toxin;
use crate::observation::Observation;
use crate::species::SpeciesTables;
use rand::Rng;
use rand::seq::IndexedRandom;
use rand_chacha::ChaCha20Rng;

pub fn mock_choose(
    obs: &Observation,
    rng: &mut ChaCha20Rng,
    thirst: u32,
    hunger: u32,
    energy: u32,
    thirst_max: u32,
    hunger_max: u32,
    energy_max: u32,
    memory: &[crate::memory::MemoryEntry],
    last_warn_tick: u64,
    tick: u64,
    warn_cooldown: u64,
    species: &SpeciesTables,
    identified_others: bool,
) -> ChosenAction {
    if let Some(gov) = choose_governance(obs, memory, species) {
        let speak = maybe_warn(
            obs,
            memory,
            last_warn_tick,
            tick,
            warn_cooldown,
            species,
            identified_others,
        );
        return ChosenAction {
            primary: gov,
            speak,
        };
    }
    let primary = choose_primary(
        obs, rng, thirst, hunger, energy, thirst_max, hunger_max, energy_max,
    );
    let speak = maybe_warn(
        obs,
        memory,
        last_warn_tick,
        tick,
        warn_cooldown,
        species,
        identified_others,
    );
    ChosenAction { primary, speak }
}

fn choose_governance(
    obs: &Observation,
    memory: &[crate::memory::MemoryEntry],
    species: &SpeciesTables,
) -> Option<PrimaryAction> {
    use crate::board::{ProposalStatus, StructuredRule};
    for (i, spec) in species.vegetation.iter().enumerate() {
        let tag = (i + 1) as u8;
        if !knows_toxin(memory, tag) {
            continue;
        }
        if let Some(p) = obs.board.iter().find(|p| {
            p.status == ProposalStatus::Open
                && matches!(p.rule, Some(StructuredRule::BanEatSpecies { species: s }) if s == tag)
                && !p.you_support
        }) {
            return Some(PrimaryAction::Support { proposal_id: p.id });
        }
        let already = obs.board.iter().any(|p| {
            matches!(p.rule, Some(StructuredRule::BanEatSpecies { species: s }) if s == tag)
                && p.status != ProposalStatus::Rejected
                && p.status != ProposalStatus::Expired
        });
        if !already
            && obs
                .legal
                .iter()
                .any(|a| matches!(a, PrimaryAction::Propose { .. }))
        {
            return Some(PrimaryAction::Propose {
                text: format!("do not eat {}", spec.id),
                rule: Some(StructuredRule::BanEatSpecies { species: tag }),
            });
        }
    }
    None
}

fn choose_primary(
    obs: &Observation,
    rng: &mut ChaCha20Rng,
    thirst: u32,
    hunger: u32,
    energy: u32,
    thirst_max: u32,
    hunger_max: u32,
    energy_max: u32,
) -> PrimaryAction {
    let thirsty = thirst < thirst_max / 2;
    let hungry = hunger < hunger_max / 2;
    let tired = energy < energy_max / 3;

    if thirsty {
        if obs.legal.iter().any(|a| matches!(a, PrimaryAction::Drink)) {
            return PrimaryAction::Drink;
        }
        if let Some(mv) = move_toward(obs, rng, |t| t.water) {
            return mv;
        }
    }
    if hungry {
        for a in &obs.legal {
            if matches!(a, PrimaryAction::Eat { .. }) {
                return a.clone();
            }
        }
        for a in &obs.legal {
            if matches!(a, PrimaryAction::Gather { species } if *species != 0) {
                return a.clone();
            }
        }
        if let Some(PrimaryAction::Hunt) =
            obs.legal.iter().find(|a| matches!(a, PrimaryAction::Hunt))
        {
            return PrimaryAction::Hunt;
        }
        if let Some(PrimaryAction::Fish) =
            obs.legal.iter().find(|a| matches!(a, PrimaryAction::Fish))
        {
            return PrimaryAction::Fish;
        }
        if let Some(mv) = move_toward(obs, rng, |t| {
            t.vegetation != 0 || t.animals > 0 || t.fish > 0
        }) {
            return mv;
        }
    }
    if tired && obs.legal.iter().any(|a| matches!(a, PrimaryAction::Rest)) {
        return PrimaryAction::Rest;
    }
    let moves: Vec<_> = obs
        .legal
        .iter()
        .filter(|a| matches!(a, PrimaryAction::MoveRelative { .. }))
        .cloned()
        .collect();
    if !moves.is_empty() && rng.random_bool(0.6) {
        return moves.choose(rng).unwrap().clone();
    }
    PrimaryAction::Wait
}

fn move_toward(
    obs: &Observation,
    rng: &mut ChaCha20Rng,
    pred: impl Fn(&crate::observation::TileView) -> bool,
) -> Option<PrimaryAction> {
    let target = obs
        .tiles
        .iter()
        .find(|t| pred(t) && (t.x != obs.x || t.y != obs.y))?;
    let dx = (target.x as i32 - obs.x as i32).signum();
    let dy = (target.y as i32 - obs.y as i32).signum();
    let cand = if dx != 0 && dy != 0 {
        if rng.random_bool(0.5) {
            PrimaryAction::MoveRelative { dx, dy: 0 }
        } else {
            PrimaryAction::MoveRelative { dx: 0, dy }
        }
    } else {
        PrimaryAction::MoveRelative { dx, dy }
    };
    obs.legal.iter().find(|a| *a == &cand).cloned()
}

fn maybe_warn(
    obs: &Observation,
    memory: &[crate::memory::MemoryEntry],
    last_warn_tick: u64,
    tick: u64,
    warn_cooldown: u64,
    species: &SpeciesTables,
    identified_others: bool,
) -> Option<Speak> {
    if !identified_others {
        return None;
    }
    if tick.saturating_sub(last_warn_tick) < warn_cooldown && last_warn_tick != 0 {
        return None;
    }
    for (i, spec) in species.vegetation.iter().enumerate() {
        let tag = (i + 1) as u8;
        if knows_toxin(memory, tag) {
            return Some(Speak {
                to: SpeakTarget::Broadcast,
                shout: false,
                text: format!("{} is toxic", spec.id),
            });
        }
    }
    let _ = obs;
    None
}

/// Filter eat/gather using agent memory (called from simulation).
pub fn avoid_toxic(obs: &Observation, memory: &[crate::memory::MemoryEntry]) -> Observation {
    let mut obs = obs.clone();
    obs.legal.retain(|a| match a {
        PrimaryAction::Eat {
            item: ItemId::Food(tag),
        } => !knows_toxin(memory, *tag),
        PrimaryAction::Gather { species } if *species != 0 => !knows_toxin(memory, *species),
        _ => true,
    });
    obs
}
