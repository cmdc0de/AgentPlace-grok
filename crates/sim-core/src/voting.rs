//! Vote tally mode. Overlay-overridable; not ExperimentConfig.

use crate::error::SimError;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoteWeight {
    #[default]
    Equal,
    Influence,
}

impl VoteWeight {
    pub fn parse(s: &str) -> Result<Self, SimError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "equal" => Ok(Self::Equal),
            "influence" => Ok(Self::Influence),
            other => Err(SimError::Config(format!(
                "unknown voting weight {other:?} (use equal or influence)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Influence => "influence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VotingParams {
    pub weight: VoteWeight,
}

impl VotingParams {
    pub fn equal() -> Self {
        Self {
            weight: VoteWeight::Equal,
        }
    }

    pub fn influence() -> Self {
        Self {
            weight: VoteWeight::Influence,
        }
    }

    /// Read `[voting]` from an experiment TOML. Unknown tables ignored.
    pub fn from_config_toml(s: &str) -> Result<Self, SimError> {
        #[derive(Default, Deserialize)]
        struct Slice {
            #[serde(default)]
            voting: Table,
        }
        #[derive(Default, Deserialize)]
        struct Table {
            weight: Option<String>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        let mut p = Self::default();
        if let Some(w) = slice.voting.weight {
            p.weight = VoteWeight::parse(&w)?;
        }
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_weight_errors() {
        let err = VoteWeight::parse("maybe").unwrap_err();
        assert!(err.to_string().contains("unknown"), "{err}");
    }

    #[test]
    fn omit_and_equal_parse() {
        assert_eq!(VoteWeight::parse("").unwrap(), VoteWeight::Equal);
        assert_eq!(VoteWeight::parse("equal").unwrap(), VoteWeight::Equal);
        assert_eq!(
            VoteWeight::parse("influence").unwrap(),
            VoteWeight::Influence
        );
    }

    #[test]
    fn config_toml_influence() {
        let p = VotingParams::from_config_toml("[voting]\nweight = \"influence\"\n").unwrap();
        assert_eq!(p.weight, VoteWeight::Influence);
    }

    #[test]
    fn config_toml_unknown_errors() {
        let err = VotingParams::from_config_toml("[voting]\nweight = \"maybe\"\n").unwrap_err();
        assert!(err.to_string().contains("unknown"), "{err}");
    }
}
