//! OpenAI-compatible chat client for action selection.
//!
//! Local: Ollama at `http://spark-bcce.hlab:11434` (config default). Empty `base_url` ⇒ mock.
//! Frontier: xAI at `https://api.x.ai/v1` with `XAI_API_KEY` (default model grok-4.6)

use serde::Deserialize;
use sim_core::action::ChosenAction;
use sim_core::llm::{ActionChooser, ChooseError, extract_json_payload, parse_choice_json};
use sim_core::observation::Observation;
use std::time::Duration;

const DEFAULT_XAI_MODEL: &str = "grok-4.6";
const DEFAULT_OLLAMA_MODEL: &str = "nemotron3:33b";

pub struct OpenAiCompatClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub timeout: Duration,
    pub temperature: f32,
    pub max_retries: u32,
    pub species: sim_core::species::SpeciesTables,
}

impl OpenAiCompatClient {
    pub fn from_config(cfg: &sim_core::ExperimentConfig) -> Self {
        let provider = cfg.llm.provider.as_str();
        let mut base = cfg.llm.base_url.clone();
        let mut model = cfg.llm.model.clone();
        let mut api_key = None;
        match provider {
            "openai_compatible" | "xai" | "spacexai" => {
                if base.is_empty() || base.contains("11434") {
                    base = "https://api.x.ai/v1".into();
                }
                if model.is_empty() {
                    model = DEFAULT_XAI_MODEL.into();
                }
                api_key = std::env::var(&cfg.llm.api_key_env).ok();
            }
            _ => {
                if model.is_empty() {
                    model = DEFAULT_OLLAMA_MODEL.into();
                }
                // Ollama OpenAI compat is under /v1. Empty URL is rejected in chooser_from_config.
                if !base.is_empty() && !base.contains("/v1") {
                    base = format!("{}/v1", base.trim_end_matches('/'));
                }
            }
        }
        Self {
            base_url: base.trim_end_matches('/').into(),
            api_key,
            model,
            timeout: Duration::from_millis(cfg.llm.timeout_ms.max(1)),
            temperature: cfg.llm.action_temperature as f32,
            max_retries: cfg.llm.max_retries,
            species: cfg.world.species.clone(),
        }
    }

    fn post_once(&self, seed: u64, prompt: &str, temperature: f32) -> Result<String, ChooseError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = serde_json::json!({
            "model": self.model,
            "temperature": temperature,
            "messages": [
                {"role": "system", "content": "You are a simulation agent. Reply with a single JSON object only."},
                {"role": "user", "content": prompt}
            ],
            "seed": seed,
        });
        // Nemotron-class thinking models: skip the reasoning channel when the server honors it.
        if !self.model.contains("grok") {
            body["think"] = serde_json::json!(false);
        }
        if self.model.contains("grok") {
            body["response_format"] = serde_json::json!({"type": "json_object"});
        }
        let mut req = ureq::post(&url).timeout(self.timeout);
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {key}"));
        }
        let resp = req.send_json(body).map_err(|e| match e {
            ureq::Error::Status(_, _) => ChooseError::Unreachable,
            ureq::Error::Transport(t) if format!("{t:?}").to_lowercase().contains("time") => {
                ChooseError::Timeout
            }
            _ => ChooseError::Unreachable,
        })?;
        let parsed: ChatResponse = resp.into_json().map_err(|_| ChooseError::Malformed)?;
        let msg = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or(ChooseError::Malformed)?;
        payload_from_message(&msg)
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    #[serde(default)]
    message: Msg,
}
#[derive(Deserialize, Default)]
struct Msg {
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
}

fn payload_from_message(msg: &Msg) -> Result<String, ChooseError> {
    for candidate in [msg.content.as_deref(), msg.reasoning.as_deref()]
        .into_iter()
        .flatten()
    {
        let extracted = extract_json_payload(candidate);
        if extracted.starts_with('{') {
            return Ok(extracted);
        }
    }
    Err(ChooseError::Malformed)
}

impl ActionChooser for OpenAiCompatClient {
    fn choose(
        &self,
        call_seed: u64,
        obs: &Observation,
    ) -> Result<(ChosenAction, String), ChooseError> {
        let prompt = build_prompt(obs, &self.species);
        let mut temp = self.temperature;
        let mut last = ChooseError::Malformed;
        let attempts = self.max_retries.saturating_add(1);
        for i in 0..attempts {
            match self.post_once(call_seed, &prompt, temp) {
                Ok(text) => {
                    let payload = extract_json_payload(&text);
                    let choice = parse_choice_json(&payload, &obs.legal, &self.species)?;
                    return Ok((choice, payload));
                }
                Err(ChooseError::Timeout) => return Err(ChooseError::Timeout),
                Err(e) => {
                    last = e;
                    temp = (temp * 0.7).max(0.0);
                    if i + 1 >= attempts {
                        break;
                    }
                }
            }
        }
        Err(last)
    }
}

const PROMPT_LINE_CAP: usize = 16;

pub fn build_prompt(obs: &Observation, species: &sim_core::species::SpeciesTables) -> String {
    let legal: Vec<String> = obs
        .legal
        .iter()
        .map(|a| sim_core::observation::format_primary(a, species))
        .collect();
    let heard: Vec<String> = obs
        .heard
        .iter()
        .take(PROMPT_LINE_CAP)
        .map(|h| {
            format!(
                "{}: {}",
                h.speaker
                    .map(|id| id.0.to_string())
                    .unwrap_or_else(|| "?".into()),
                h.text
            )
        })
        .collect();
    let goals: Vec<String> = obs.goals.iter().map(|g| g.text.clone()).collect();
    let board: Vec<String> = obs
        .board
        .iter()
        .take(PROMPT_LINE_CAP)
        .map(|p| {
            format!(
                "#{} {:?} yes={} no={} you_support={} {:?}",
                p.id, p.status, p.support, p.oppose, p.you_support, p.rule
            )
        })
        .collect();
    let rels: Vec<String> = obs
        .relationships
        .iter()
        .take(PROMPT_LINE_CAP)
        .map(|r| {
            format!(
                "#{} trust={:.1} aff={:.1} resp={:.1} fear={:.1}",
                r.id.0,
                r.trust as f64 / 100.0,
                r.affinity as f64 / 100.0,
                r.respect as f64 / 100.0,
                r.fear as f64 / 100.0
            )
        })
        .collect();
    let inventory: Vec<String> = obs
        .inventory
        .iter()
        .map(|i| format!("{}×{}", i.item, i.qty))
        .collect();
    let incentives: Vec<String> = obs
        .incentives
        .iter()
        .map(|i| {
            let window = match i.end_tick {
                Some(e) => format!("{}–{}", i.start_tick, e),
                None => format!("{}–", i.start_tick),
            };
            if i.description.is_empty() {
                format!("{} ({window})", i.id)
            } else {
                format!("{} ({window}): {}", i.id, i.description)
            }
        })
        .collect();
    let illness = if obs.illness_ticks > 0 {
        format!("yes ({} ticks)", obs.illness_ticks)
    } else {
        "no".into()
    };
    let visible = sim_core::observation::visible_summary(obs, species, 12);
    format!(
        "Agent {} at ({}, {}). Vision {}.\n\
         Needs (0–100): hunger={} thirst={} energy={} illness={}\n\
         Inventory: [{}]\n\
         Allergies: [{}]\n\
         Known toxins: [{}]\n\
         Active incentives: [{}]\n\
         Goals: [{}]\n\
         Visible: {}\n\
         Board: [{}]\n\
         Relationships: [{}]\n\
         Heard: [{}]\n\
         Legal primary actions (you MUST pick one of these):\n{}\n\
         Reply JSON: {{\"action\":\"Wait|Rest|Drink|Hunt|Fish|Gather|Eat|Farm|Craft|MoveRelative|Propose|Support|Oppose|Transfer|Store|Retrieve\",\"target\":\"species, item, or agent id\",\"dx\":0,\"dy\":0,\"qty\":1,\"recipe\":\"spear\",\"text\":\"proposal text\",\"proposal_id\":0,\"rule\":{{\"kind\":\"BanEatSpecies|BanGatherSpecies|MaxGatherPerTick\",\"species\":\"mushroom\",\"n\":1}},\"speak\":{{\"to\":\"broadcast\",\"shout\":false,\"text\":\"...\"}}}}\n\
         Prefer a structured rule when banning a species. Unknown rule kind waits. Omit speak if silent. Drink if thirsty and water is legal; Eat if hungry and food is legal. Store surplus food; Retrieve from a stockpile when hungry.",
        obs.agent_id.0,
        obs.x,
        obs.y,
        obs.vision,
        obs.hunger,
        obs.thirst,
        obs.energy,
        illness,
        inventory.join(", "),
        obs.allergies.join(", "),
        obs.toxins.join(", "),
        incentives.join(" ; "),
        goals.join(" | "),
        if visible.is_empty() {
            "(none)".into()
        } else {
            visible
        },
        board.join(" ; "),
        rels.join(" ; "),
        heard.join(" | "),
        legal.join("\n"),
    )
}

pub fn chooser_from_config(cfg: &sim_core::ExperimentConfig) -> Result<sim_core::Chooser, String> {
    match cfg.llm.provider.as_str() {
        "" | "mock" => Ok(sim_core::Chooser::Mock),
        "wait" => Ok(sim_core::Chooser::Wait),
        "ollama" | "openai_compatible" | "xai" | "spacexai" => {
            if cfg.llm.base_url.trim().is_empty() {
                return Ok(sim_core::Chooser::Mock);
            }
            let client = OpenAiCompatClient::from_config(cfg);
            Ok(sim_core::Chooser::Custom(std::sync::Arc::new(client)))
        }
        other => Err(format!("unknown llm provider '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::action::PrimaryAction;
    use sim_core::observation::{IncentiveView, Observation};
    use sim_core::{Chooser, ExperimentConfig};

    fn cfg(provider: &str, url: &str) -> ExperimentConfig {
        let mut c = ExperimentConfig::from_toml_str(
            r#"
master_seed = 1
[world]
width = 32
height = 32
max_height = 8
[agents]
count = 2
"#,
        )
        .unwrap();
        c.llm.provider = provider.into();
        c.llm.base_url = url.into();
        c
    }

    #[test]
    fn empty_url_ollama_is_mock() {
        let ch = chooser_from_config(&cfg("ollama", "")).unwrap();
        assert!(matches!(ch, Chooser::Mock));
    }

    #[test]
    fn empty_url_xai_is_mock() {
        let ch = chooser_from_config(&cfg("xai", "  ")).unwrap();
        assert!(matches!(ch, Chooser::Mock));
    }

    #[test]
    fn mock_provider_ignores_url() {
        let ch = chooser_from_config(&cfg("mock", "http://spark-bcce.hlab:11434")).unwrap();
        assert!(matches!(ch, Chooser::Mock));
    }

    #[test]
    fn ollama_with_url_is_custom() {
        let ch = chooser_from_config(&cfg("ollama", "http://spark-bcce.hlab:11434")).unwrap();
        assert!(matches!(ch, Chooser::Custom(_)));
    }

    #[test]
    fn prompt_contains_needs_and_incentive() {
        let mut obs = Observation::default();
        obs.hunger = 40;
        obs.thirst = 22;
        obs.energy = 80;
        obs.incentives.push(IncentiveView {
            id: "coop_food".into(),
            description: "bonus for supporters".into(),
            start_tick: 0,
            end_tick: Some(3000),
        });
        obs.legal = vec![PrimaryAction::Drink, PrimaryAction::Gather { species: 1 }];
        let p = build_prompt(&obs, &sim_core::species::SpeciesTables::default());
        let lower = p.to_ascii_lowercase();
        assert!(lower.contains("hunger=40"), "{p}");
        assert!(lower.contains("thirst=22"), "{p}");
        assert!(p.contains("coop_food"), "{p}");
        assert!(p.contains("Drink"), "{p}");
        assert!(p.contains("berry_bush"), "{p}");
    }

    #[test]
    fn extract_think_and_reasoning_json() {
        let wrapped = "<think>planning</think>\n```json\n{\"action\":\"Drink\"}\n```";
        let p = extract_json_payload(wrapped);
        assert!(p.contains("Drink"), "{p}");
        let from_reason = extract_json_payload("Sure.\n{\"action\":\"Wait\"}");
        assert_eq!(from_reason, "{\"action\":\"Wait\"}");
        let msg = Msg {
            content: Some("<think>x</think>".into()),
            reasoning: Some("{\"action\":\"Rest\"}".into()),
        };
        let got = payload_from_message(&msg).unwrap();
        assert!(got.contains("Rest"), "{got}");
    }
}
