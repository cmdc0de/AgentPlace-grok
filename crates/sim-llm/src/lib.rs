//! OpenAI-compatible chat client for action selection.
//!
//! Local: Ollama at `http://localhost:11434`
//! Frontier: xAI at `https://api.x.ai/v1` with `XAI_API_KEY` (default model grok-4.6)

use serde::Deserialize;
use sim_core::action::ChosenAction;
use sim_core::llm::{ActionChooser, ChooseError, parse_choice_json};
use sim_core::observation::Observation;
use std::time::Duration;

const DEFAULT_XAI_MODEL: &str = "grok-4.6";
const DEFAULT_OLLAMA_MODEL: &str = "llama3.2:3b";

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
                if base.is_empty() {
                    base = "http://localhost:11434".into();
                }
                if model.is_empty() {
                    model = DEFAULT_OLLAMA_MODEL.into();
                }
                // Ollama OpenAI compat is under /v1
                if !base.contains("/v1") {
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
        parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(ChooseError::Malformed)
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
}

impl ActionChooser for OpenAiCompatClient {
    fn choose(&self, call_seed: u64, obs: &Observation) -> Result<ChosenAction, ChooseError> {
        let prompt = build_prompt(obs);
        let mut temp = self.temperature;
        let mut last = ChooseError::Malformed;
        let attempts = self.max_retries.saturating_add(1);
        for i in 0..attempts {
            match self.post_once(call_seed, &prompt, temp) {
                Ok(text) => {
                    return parse_choice_json(&text, &obs.legal, &self.species);
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

fn build_prompt(obs: &Observation) -> String {
    let legal: Vec<String> = obs.legal.iter().map(|a| format!("{a:?}")).collect();
    let heard: Vec<String> = obs
        .heard
        .iter()
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
        .map(|p| {
            format!(
                "#{} {:?} yes={} no={} you_support={} {:?}",
                p.id, p.status, p.support, p.oppose, p.you_support, p.rule
            )
        })
        .collect();
    format!(
        "Agent {} at ({}, {}). Vision {}. Goals: [{}]\n\
         Board: [{}]\n\
         Heard: [{}]\n\
         Legal primary actions (you MUST pick one of these):\n{}\n\
         Reply JSON: {{\"action\":\"Wait|Rest|Drink|Hunt|Fish|Gather|Eat|Farm|Craft|MoveRelative|Propose|Support|Oppose\",\"target\":\"species or item\",\"dx\":0,\"dy\":0,\"recipe\":\"spear\",\"text\":\"proposal text\",\"proposal_id\":0,\"rule\":{{\"kind\":\"BanEatSpecies|BanGatherSpecies|MaxGatherPerTick\",\"species\":\"mushroom\",\"n\":1}},\"speak\":{{\"to\":\"broadcast\",\"shout\":false,\"text\":\"...\"}}}}\n\
         Prefer a structured rule when banning a species. Unknown rule kind waits. Omit speak if silent.",
        obs.agent_id.0,
        obs.x,
        obs.y,
        obs.vision,
        goals.join(" | "),
        board.join(" ; "),
        heard.join(" | "),
        legal.join("\n"),
    )
}

pub fn chooser_from_config(cfg: &sim_core::ExperimentConfig) -> Result<sim_core::Chooser, String> {
    match cfg.llm.provider.as_str() {
        "" | "mock" => Ok(sim_core::Chooser::Mock),
        "wait" => Ok(sim_core::Chooser::Wait),
        "ollama" | "openai_compatible" | "xai" | "spacexai" => {
            let client = OpenAiCompatClient::from_config(cfg);
            Ok(sim_core::Chooser::Custom(std::sync::Arc::new(client)))
        }
        other => Err(format!("unknown llm provider '{other}'")),
    }
}
