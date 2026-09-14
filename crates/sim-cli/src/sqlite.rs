//! Hash-neutral sqlite sink for the same facts JSONL records. sim-cli only.

use rusqlite::{Connection, params};
use sim_core::action::Recipe;
use sim_core::agent::ItemId;
use sim_core::decision_log::DecisionRecord;
use sim_core::event_log::{SimEvent, SimEventKind};
use sim_core::objects::CatalogEntry;
use sim_core::timing::TickTiming;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS ticks (
  tick INTEGER PRIMARY KEY,
  wall_ns INTEGER NOT NULL,
  world_ns INTEGER NOT NULL,
  board_ns INTEGER NOT NULL,
  incentive_ns INTEGER NOT NULL,
  agents_ns INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS agent_timing (
  tick INTEGER NOT NULL,
  agent INTEGER NOT NULL,
  perceive_ns INTEGER NOT NULL,
  retrieve_ns INTEGER NOT NULL,
  reflect_ns INTEGER NOT NULL,
  plan_ns INTEGER NOT NULL,
  select_ns INTEGER NOT NULL,
  execute_ns INTEGER NOT NULL,
  remember_ns INTEGER NOT NULL,
  PRIMARY KEY (tick, agent)
);
CREATE TABLE IF NOT EXISTS decisions (
  tick INTEGER NOT NULL,
  agent INTEGER NOT NULL,
  chooser TEXT NOT NULL,
  policy_branch TEXT NOT NULL,
  call_seed INTEGER NOT NULL,
  prompt_hash TEXT NOT NULL,
  primary_action TEXT NOT NULL,
  speak INTEGER NOT NULL,
  reasoning TEXT,
  PRIMARY KEY (tick, agent)
);
CREATE TABLE IF NOT EXISTS decision_legal (
  tick INTEGER NOT NULL,
  agent INTEGER NOT NULL,
  action TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
  tick INTEGER NOT NULL,
  agent INTEGER NOT NULL,
  kind TEXT NOT NULL,
  from_x INTEGER,
  from_y INTEGER,
  to_x INTEGER,
  to_y INTEGER,
  x INTEGER,
  y INTEGER,
  item TEXT,
  qty INTEGER,
  success INTEGER,
  species INTEGER,
  toxic INTEGER,
  shout INTEGER,
  broadcast INTEGER,
  text TEXT,
  proposal_id INTEGER,
  reason TEXT,
  incentive_id TEXT,
  detail TEXT,
  hunger_zero INTEGER,
  thirst_zero INTEGER,
  target INTEGER,
  damage INTEGER,
  parent_a INTEGER,
  parent_b INTEGER,
  invent_kind INTEGER,
  pipeline_stages INTEGER
);
CREATE INDEX IF NOT EXISTS events_tick ON events (tick);
CREATE INDEX IF NOT EXISTS events_kind ON events (kind);
CREATE INDEX IF NOT EXISTS events_agent_tick ON events (agent, tick);
"#;

pub struct SqliteLog {
    conn: Connection,
}

impl SqliteLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn insert_timing(&mut self, t: &TickTiming) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO ticks
             (tick, wall_ns, world_ns, board_ns, incentive_ns, agents_ns)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                t.tick as i64,
                t.wall_ns as i64,
                t.world_ns as i64,
                t.board_ns as i64,
                t.incentive_ns as i64,
                t.agents_ns as i64
            ],
        )?;
        for a in &t.agents {
            tx.execute(
                "INSERT OR REPLACE INTO agent_timing
                 (tick, agent, perceive_ns, retrieve_ns, reflect_ns, plan_ns,
                  select_ns, execute_ns, remember_ns)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    t.tick as i64,
                    a.agent as i64,
                    a.perceive_ns as i64,
                    a.retrieve_ns as i64,
                    a.reflect_ns as i64,
                    a.plan_ns as i64,
                    a.select_ns as i64,
                    a.execute_ns as i64,
                    a.remember_ns as i64
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_decisions(
        &mut self,
        recs: &[DecisionRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if recs.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        for r in recs {
            tx.execute(
                "INSERT OR REPLACE INTO decisions
                 (tick, agent, chooser, policy_branch, call_seed, prompt_hash,
                  primary_action, speak, reasoning)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    r.tick as i64,
                    r.agent as i64,
                    r.chooser,
                    r.policy_branch,
                    r.call_seed as i64,
                    r.prompt_hash,
                    primary_action_name(&r.primary),
                    if r.speak.is_some() { 1i64 } else { 0 },
                    r.reasoning.as_deref()
                ],
            )?;
            tx.execute(
                "DELETE FROM decision_legal WHERE tick = ?1 AND agent = ?2",
                params![r.tick as i64, r.agent as i64],
            )?;
            for action in &r.legal {
                tx.execute(
                    "INSERT INTO decision_legal (tick, agent, action) VALUES (?1, ?2, ?3)",
                    params![r.tick as i64, r.agent as i64, action],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn insert_events(
        &mut self,
        events: &[SimEvent],
        catalog: &[CatalogEntry],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if events.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        for e in events {
            let row = EventRow::from_event(e, catalog);
            tx.execute(
                "INSERT INTO events (
                   tick, agent, kind, from_x, from_y, to_x, to_y, x, y, item, qty,
                   success, species, toxic, shout, broadcast, text, proposal_id, reason,
                   incentive_id, detail, hunger_zero, thirst_zero, target, damage,
                   parent_a, parent_b, invent_kind, pipeline_stages
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                   ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29
                 )",
                params![
                    row.tick,
                    row.agent,
                    row.kind,
                    row.from_x,
                    row.from_y,
                    row.to_x,
                    row.to_y,
                    row.x,
                    row.y,
                    row.item,
                    row.qty,
                    row.success,
                    row.species,
                    row.toxic,
                    row.shout,
                    row.broadcast,
                    row.text,
                    row.proposal_id,
                    row.reason,
                    row.incentive_id,
                    row.detail,
                    row.hunger_zero,
                    row.thirst_zero,
                    row.target,
                    row.damage,
                    row.parent_a,
                    row.parent_b,
                    row.invent_kind,
                    row.pipeline_stages
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

pub const SQL_MEDIAN_WALL: &str =
    "SELECT wall_ns FROM ticks ORDER BY wall_ns LIMIT 1 OFFSET (SELECT COUNT(*) FROM ticks) / 2";
pub const SQL_MEDIAN_AGENT: &str = "SELECT (perceive_ns+retrieve_ns+select_ns+execute_ns+remember_ns) AS agent_ns FROM agent_timing ORDER BY 1 LIMIT 1 OFFSET (SELECT COUNT(*) FROM agent_timing) / 2";
pub const SQL_EVENTS_BY_KIND: &str =
    "SELECT kind, COUNT(*) AS n FROM events GROUP BY kind ORDER BY n DESC, kind";
pub const SQL_CRAFTS: &str =
    "SELECT item, COUNT(*) AS n FROM events WHERE kind = 'craft' GROUP BY item ORDER BY n DESC, item";

pub fn metrics_json(conn: &Connection) -> Result<serde_json::Value, rusqlite::Error> {
    use rusqlite::OptionalExtension;
    let median_wall_ns: Option<i64> = conn
        .query_row(SQL_MEDIAN_WALL, [], |r| r.get(0))
        .optional()?;
    let median_agent_ns: Option<i64> = conn
        .query_row(SQL_MEDIAN_AGENT, [], |r| r.get(0))
        .optional()?;
    let mut events_by_kind = Vec::new();
    let mut stmt = conn.prepare(SQL_EVENTS_BY_KIND)?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "kind": r.get::<_, String>(0)?,
            "n": r.get::<_, i64>(1)?,
        }))
    })?;
    for row in rows {
        events_by_kind.push(row?);
    }
    let mut crafts = Vec::new();
    let mut stmt = conn.prepare(SQL_CRAFTS)?;
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "item": r.get::<_, Option<String>>(0)?.unwrap_or_default(),
            "n": r.get::<_, i64>(1)?,
        }))
    })?;
    for row in rows {
        crafts.push(row?);
    }
    Ok(serde_json::json!({
        "median_wall_ns": median_wall_ns,
        "median_agent_ns": median_agent_ns,
        "events_by_kind": events_by_kind,
        "crafts": crafts,
    }))
}

/// Serve GET/OPTIONS `/metrics` until `running` is false. Prints `sqlite_http=` on stderr.
pub fn spawn_metrics_http(
    bind: &str,
    db_path: PathBuf,
    running: Arc<AtomicBool>,
) -> Result<String, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(bind)?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}/metrics");
    eprintln!("sqlite_http={url}");
    thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = handle_metrics_http(stream, &db_path);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(_) => break,
            }
        }
    });
    Ok(url)
}

fn handle_metrics_http(mut stream: TcpStream, db_path: &Path) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let cors = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\n";
    if first.starts_with("OPTIONS ") {
        let resp = format!("HTTP/1.1 204 No Content\r\n{cors}\r\n");
        stream.write_all(resp.as_bytes())?;
        return Ok(());
    }
    if !first.starts_with("GET /metrics") && !first.starts_with("GET / ") {
        let body = b"not found";
        let resp = format!(
            "HTTP/1.1 404 Not Found\r\n{cors}Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        stream.write_all(resp.as_bytes())?;
        stream.write_all(body)?;
        return Ok(());
    }
    let body = match Connection::open(db_path).and_then(|c| metrics_json(&c)) {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\n{cors}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

fn primary_action_name(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => map
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "Wait".into()),
        serde_json::Value::String(s) => s.clone(),
        _ => "Wait".into(),
    }
}

fn item_label(item: ItemId, catalog: &[CatalogEntry]) -> String {
    match item {
        ItemId::Food(_) => "food".into(),
        ItemId::Wood => "wood".into(),
        ItemId::Fiber => "fiber".into(),
        ItemId::Stone => "stone".into(),
        ItemId::Basket => "basket".into(),
        ItemId::Spear => "spear".into(),
        ItemId::FishingRod => "fishing_rod".into(),
        ItemId::Backpack => "backpack".into(),
        ItemId::Catalog(n) => catalog
            .iter()
            .find(|e| e.item == ItemId::Catalog(n))
            .map(|e| e.slug.clone())
            .unwrap_or_else(|| format!("catalog:{n}")),
    }
}

fn recipe_label(recipe: Recipe, catalog: &[CatalogEntry]) -> String {
    match recipe {
        Recipe::Basket => "basket".into(),
        Recipe::Spear => "spear".into(),
        Recipe::FishingRod => "fishing_rod".into(),
        Recipe::Backpack => "backpack".into(),
        Recipe::Catalog(n) => item_label(ItemId::Catalog(n), catalog),
    }
}

fn b01(v: bool) -> i64 {
    if v { 1 } else { 0 }
}

struct EventRow {
    tick: i64,
    agent: i64,
    kind: String,
    from_x: Option<i64>,
    from_y: Option<i64>,
    to_x: Option<i64>,
    to_y: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
    item: Option<String>,
    qty: Option<i64>,
    success: Option<i64>,
    species: Option<i64>,
    toxic: Option<i64>,
    shout: Option<i64>,
    broadcast: Option<i64>,
    text: Option<String>,
    proposal_id: Option<i64>,
    reason: Option<String>,
    incentive_id: Option<String>,
    detail: Option<String>,
    hunger_zero: Option<i64>,
    thirst_zero: Option<i64>,
    target: Option<i64>,
    damage: Option<i64>,
    parent_a: Option<i64>,
    parent_b: Option<i64>,
    invent_kind: Option<i64>,
    pipeline_stages: Option<i64>,
}

impl EventRow {
    fn blank(tick: i64, agent: i64, kind: &str) -> Self {
        Self {
            tick,
            agent,
            kind: kind.into(),
            from_x: None,
            from_y: None,
            to_x: None,
            to_y: None,
            x: None,
            y: None,
            item: None,
            qty: None,
            success: None,
            species: None,
            toxic: None,
            shout: None,
            broadcast: None,
            text: None,
            proposal_id: None,
            reason: None,
            incentive_id: None,
            detail: None,
            hunger_zero: None,
            thirst_zero: None,
            target: None,
            damage: None,
            parent_a: None,
            parent_b: None,
            invent_kind: None,
            pipeline_stages: None,
        }
    }

    fn from_event(event: &SimEvent, catalog: &[CatalogEntry]) -> Self {
        let tick = event.tick as i64;
        let agent = event.agent.0 as i64;
        match &event.kind {
            SimEventKind::Wait => Self::blank(tick, agent, "wait"),
            SimEventKind::Rest => Self::blank(tick, agent, "rest"),
            SimEventKind::Move {
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                let mut r = Self::blank(tick, agent, "move");
                r.from_x = Some(*from_x as i64);
                r.from_y = Some(*from_y as i64);
                r.to_x = Some(*to_x as i64);
                r.to_y = Some(*to_y as i64);
                r
            }
            SimEventKind::Gather { species, item, qty } => {
                let mut r = Self::blank(tick, agent, "gather");
                r.species = Some(*species as i64);
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Drink => Self::blank(tick, agent, "drink"),
            SimEventKind::Eat { item, toxic } => {
                let mut r = Self::blank(tick, agent, "eat");
                r.item = Some(item_label(*item, catalog));
                r.toxic = Some(b01(*toxic));
                r
            }
            SimEventKind::Hunt { success } => {
                let mut r = Self::blank(tick, agent, "hunt");
                r.success = Some(b01(*success));
                r
            }
            SimEventKind::Fish { success } => {
                let mut r = Self::blank(tick, agent, "fish");
                r.success = Some(b01(*success));
                r
            }
            SimEventKind::Farm { species, x, y } => {
                let mut r = Self::blank(tick, agent, "farm");
                r.species = Some(*species as i64);
                r.x = Some(*x as i64);
                r.y = Some(*y as i64);
                r
            }
            SimEventKind::Craft { recipe, success } => {
                let mut r = Self::blank(tick, agent, "craft");
                r.item = Some(recipe_label(*recipe, catalog));
                r.success = Some(b01(*success));
                r
            }
            SimEventKind::Speak {
                shout,
                text,
                broadcast,
                ..
            } => {
                let mut r = Self::blank(tick, agent, "speak");
                r.shout = Some(b01(*shout));
                r.broadcast = Some(b01(*broadcast));
                r.text = Some(text.clone());
                r
            }
            SimEventKind::LlmWait => Self::blank(tick, agent, "llm_wait"),
            SimEventKind::Propose { proposal_id } => {
                let mut r = Self::blank(tick, agent, "propose");
                r.proposal_id = Some(*proposal_id as i64);
                r
            }
            SimEventKind::Support { proposal_id } => {
                let mut r = Self::blank(tick, agent, "support");
                r.proposal_id = Some(*proposal_id as i64);
                r
            }
            SimEventKind::Oppose { proposal_id } => {
                let mut r = Self::blank(tick, agent, "oppose");
                r.proposal_id = Some(*proposal_id as i64);
                r
            }
            SimEventKind::RuleBlocked { reason } => {
                let mut r = Self::blank(tick, agent, "rule_blocked");
                r.reason = Some(reason.clone());
                r
            }
            SimEventKind::IncentiveApplied { id, detail } => {
                let mut r = Self::blank(tick, agent, "incentive_applied");
                r.incentive_id = Some(id.clone());
                r.detail = Some(detail.clone());
                r
            }
            SimEventKind::IncentiveEnded { id } => {
                let mut r = Self::blank(tick, agent, "incentive_ended");
                r.incentive_id = Some(id.clone());
                r
            }
            SimEventKind::Died {
                hunger_zero,
                thirst_zero,
            } => {
                let mut r = Self::blank(tick, agent, "died");
                r.hunger_zero = Some(b01(*hunger_zero));
                r.thirst_zero = Some(b01(*thirst_zero));
                r
            }
            SimEventKind::Transfer { item, qty, to } => {
                let mut r = Self::blank(tick, agent, "transfer");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r.target = Some(to.0 as i64);
                r
            }
            SimEventKind::Store { item, qty } => {
                let mut r = Self::blank(tick, agent, "store");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Retrieve { item, qty } => {
                let mut r = Self::blank(tick, agent, "retrieve");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Give { item, qty } => {
                let mut r = Self::blank(tick, agent, "give");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Pack { item, qty } => {
                let mut r = Self::blank(tick, agent, "pack");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Unpack { item, qty } => {
                let mut r = Self::blank(tick, agent, "unpack");
                r.item = Some(item_label(*item, catalog));
                r.qty = Some(*qty as i64);
                r
            }
            SimEventKind::Attack { target, damage } => {
                let mut r = Self::blank(tick, agent, "attack");
                r.target = Some(target.0 as i64);
                r.damage = Some(*damage as i64);
                r
            }
            SimEventKind::Flee => Self::blank(tick, agent, "flee"),
            SimEventKind::Incapacitated { by } => {
                let mut r = Self::blank(tick, agent, "incapacitated");
                r.target = Some(by.0 as i64);
                r
            }
            SimEventKind::CombatDeath { by } => {
                let mut r = Self::blank(tick, agent, "combat_death");
                r.target = Some(by.0 as i64);
                r
            }
            SimEventKind::PairBonded { with } => {
                let mut r = Self::blank(tick, agent, "pair_bonded");
                r.target = Some(with.0 as i64);
                r
            }
            SimEventKind::Born { parent_a, parent_b } => {
                let mut r = Self::blank(tick, agent, "born");
                r.parent_a = Some(parent_a.0 as i64);
                r.parent_b = Some(parent_b.0 as i64);
                r
            }
            SimEventKind::Invented { inventor, kind } => {
                let mut r = Self::blank(tick, agent, "invented");
                r.target = Some(inventor.0 as i64);
                r.invent_kind = Some(*kind as u8 as i64);
                r
            }
            SimEventKind::Pipeline { stages } => {
                let mut r = Self::blank(tick, agent, "pipeline");
                r.pipeline_stages = Some(*stages as i64);
                r
            }
            SimEventKind::Placed { x, y, item } => {
                let mut r = Self::blank(tick, agent, "placed");
                r.x = Some(*x as i64);
                r.y = Some(*y as i64);
                r.item = Some(item_label(*item, catalog));
                r
            }
            SimEventKind::PickedUp { x, y, item } => {
                let mut r = Self::blank(tick, agent, "picked_up");
                r.x = Some(*x as i64);
                r.y = Some(*y as i64);
                r.item = Some(item_label(*item, catalog));
                r
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::AgentId;
    use sim_core::event_log::SimEvent;
    use sim_core::timing::AgentTiming;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    fn tmp_db() -> (SqliteLog, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "m50-sqlite-{}-{}.sqlite3",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_file(&path);
        (SqliteLog::open(&path).unwrap(), path)
    }

    #[test]
    fn sqlite_no_json_column() {
        let (log, _path) = tmp_db();
        for table in ["ticks", "agent_timing", "decisions", "decision_legal", "events"] {
            let mut stmt = log
                .conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            let names: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            assert!(
                names.iter().all(|n| n != "json"),
                "{table} has json column: {names:?}"
            );
        }
    }

    #[test]
    fn sqlite_ticks_and_agent_timing() {
        let (mut log, _path) = tmp_db();
        let t = TickTiming {
            tick: 3,
            wall_ns: 12_000,
            world_ns: 1,
            board_ns: 2,
            incentive_ns: 3,
            agents_ns: 4,
            agents: vec![AgentTiming {
                agent: 0,
                select_ns: 99,
                ..AgentTiming::default()
            }],
        };
        log.insert_timing(&t).unwrap();
        let wall: i64 = log
            .conn
            .query_row("SELECT wall_ns FROM ticks WHERE tick = 3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(wall, 12_000);
        let select: i64 = log
            .conn
            .query_row(
                "SELECT select_ns FROM agent_timing WHERE tick = 3 AND agent = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(select, 99);
    }

    #[test]
    fn sqlite_events_move_columns() {
        let (mut log, _path) = tmp_db();
        let ev = SimEvent {
            tick: 1,
            agent: AgentId(0),
            kind: SimEventKind::Move {
                from_x: 1,
                from_y: 2,
                to_x: 3,
                to_y: 4,
            },
        };
        log.insert_events(&[ev], &[]).unwrap();
        let (kind, from_x, to_y, item): (String, i64, i64, Option<String>) = log
            .conn
            .query_row(
                "SELECT kind, from_x, to_y, item FROM events WHERE tick = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(kind, "move");
        assert_eq!(from_x, 1);
        assert_eq!(to_y, 4);
        assert!(item.is_none());
    }

    #[test]
    fn sqlite_decisions_and_legal() {
        let (mut log, _path) = tmp_db();
        let rec = DecisionRecord {
            tick: 2,
            agent: 0,
            chooser: "mock".into(),
            policy_branch: "mock".into(),
            call_seed: 1,
            prompt_hash: "ab".into(),
            retrieved_memory_ids: vec![],
            relation_ids: vec![],
            legal: vec!["Wait".into(), "Rest".into()],
            primary: serde_json::json!({"Wait": null}),
            speak: None,
            reasoning: None,
        };
        log.insert_decisions(&[rec]).unwrap();
        let action: String = log
            .conn
            .query_row(
                "SELECT primary_action FROM decisions WHERE tick = 2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(action, "Wait");
        let n: i64 = log
            .conn
            .query_row(
                "SELECT COUNT(*) FROM decision_legal WHERE tick = 2 AND agent = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n >= 1, "legal rows={n}");
    }

    #[test]
    fn metrics_empty_db_null_medians() {
        let (log, _path) = tmp_db();
        let v = metrics_json(&log.conn).unwrap();
        assert!(v["median_wall_ns"].is_null(), "{v}");
        assert!(v["median_agent_ns"].is_null(), "{v}");
        assert_eq!(v["events_by_kind"], serde_json::json!([]));
        assert_eq!(v["crafts"], serde_json::json!([]));
    }

    #[test]
    fn metrics_fixture_median_and_kinds() {
        let (mut log, _path) = tmp_db();
        for wall in [10i64, 30, 20] {
            log.conn
                .execute(
                    "INSERT INTO ticks (tick, wall_ns, world_ns, board_ns, incentive_ns, agents_ns)
                     VALUES (?1, ?2, 0, 0, 0, 0)",
                    params![wall / 10, wall],
                )
                .unwrap();
        }
        log.insert_events(
            &[SimEvent {
                tick: 1,
                agent: AgentId(0),
                kind: SimEventKind::Wait,
            }],
            &[],
        )
        .unwrap();
        let v = metrics_json(&log.conn).unwrap();
        assert_eq!(v["median_wall_ns"], 20);
        let kinds = v["events_by_kind"].as_array().unwrap();
        assert_eq!(kinds[0]["kind"], "wait");
        assert_eq!(kinds[0]["n"], 1);
    }

    #[test]
    fn metrics_http_cors_and_json() {
        let (log, path) = tmp_db();
        drop(log);
        let running = Arc::new(AtomicBool::new(true));
        let url = spawn_metrics_http("127.0.0.1:0", path, Arc::clone(&running)).unwrap();
        let addr = url
            .trim_start_matches("http://")
            .trim_end_matches("/metrics");
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);
        let resp = String::from_utf8_lossy(&buf);
        assert!(resp.contains("Access-Control-Allow-Origin: *"), "{resp}");
        assert!(resp.contains("median_wall_ns"), "{resp}");
        running.store(false, Ordering::Relaxed);
    }
}
