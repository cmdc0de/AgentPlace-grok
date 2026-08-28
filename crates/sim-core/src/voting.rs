//! Vote tally mode. Overlay-overridable; not ExperimentConfig.

use crate::agent::AgentId;
use crate::error::SimError;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VoteWeight {
    #[default]
    Equal,
    Influence,
    Respect,
}

impl VoteWeight {
    pub fn parse(s: &str) -> Result<Self, SimError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "equal" => Ok(Self::Equal),
            "influence" => Ok(Self::Influence),
            "respect" => Ok(Self::Respect),
            other => Err(SimError::Config(format!(
                "unknown voting weight {other:?} (use equal, influence, or respect)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Influence => "influence",
            Self::Respect => "respect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VoteAccept {
    #[default]
    Majority,
    Unanimous,
    Council,
}

impl VoteAccept {
    pub fn parse(s: &str) -> Result<Self, SimError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "majority" => Ok(Self::Majority),
            "unanimous" => Ok(Self::Unanimous),
            "council" => Ok(Self::Council),
            other => Err(SimError::Config(format!(
                "unknown voting accept {other:?} (use majority, unanimous, or council)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Majority => "majority",
            Self::Unanimous => "unanimous",
            Self::Council => "council",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CouncilTally {
    #[default]
    Unanimous,
    Majority,
}

impl CouncilTally {
    pub fn parse(s: &str) -> Result<Self, SimError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "unanimous" => Ok(Self::Unanimous),
            "majority" => Ok(Self::Majority),
            other => Err(SimError::Config(format!(
                "unknown council_tally {other:?} (use unanimous or majority)"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unanimous => "unanimous",
            Self::Majority => "majority",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VotingParams {
    pub weight: VoteWeight,
    pub accept: VoteAccept,
    pub council: Vec<AgentId>,
    pub council_tally: CouncilTally,
}

impl VotingParams {
    pub fn equal() -> Self {
        Self::default()
    }

    pub fn influence() -> Self {
        Self {
            weight: VoteWeight::Influence,
            ..Self::default()
        }
    }

    pub fn respect() -> Self {
        Self {
            weight: VoteWeight::Respect,
            ..Self::default()
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
            accept: Option<String>,
            #[serde(default)]
            council: Vec<u64>,
            council_tally: Option<String>,
        }
        let slice: Slice = toml::from_str(s).unwrap_or_default();
        let mut p = Self::default();
        if let Some(w) = slice.voting.weight {
            p.weight = VoteWeight::parse(&w)?;
        }
        if let Some(a) = slice.voting.accept {
            p.accept = VoteAccept::parse(&a)?;
        }
        p.council = slice.voting.council.into_iter().map(AgentId).collect();
        if let Some(t) = slice.voting.council_tally {
            p.council_tally = CouncilTally::parse(&t)?;
            if p.accept != VoteAccept::Council {
                return Err(SimError::Config(
                    "council_tally requires voting accept = \"council\"".into(),
                ));
            }
        }
        if p.accept == VoteAccept::Council && p.council.is_empty() {
            return Err(SimError::Config(
                "voting accept = \"council\" requires a non-empty council list".into(),
            ));
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
        assert_eq!(VoteWeight::parse("respect").unwrap(), VoteWeight::Respect);
    }

    #[test]
    fn config_toml_influence() {
        let p = VotingParams::from_config_toml("[voting]\nweight = \"influence\"\n").unwrap();
        assert_eq!(p.weight, VoteWeight::Influence);
    }

    #[test]
    fn config_toml_respect() {
        let p = VotingParams::from_config_toml("[voting]\nweight = \"respect\"\n").unwrap();
        assert_eq!(p.weight, VoteWeight::Respect);
    }

    #[test]
    fn config_toml_unknown_errors() {
        let err = VotingParams::from_config_toml("[voting]\nweight = \"maybe\"\n").unwrap_err();
        assert!(err.to_string().contains("unknown"), "{err}");
    }

    #[test]
    fn omit_accept_is_majority() {
        let p = VotingParams::from_config_toml("[voting]\nweight = \"equal\"\n").unwrap();
        assert_eq!(p.accept, VoteAccept::Majority);
        assert!(p.council.is_empty());
    }

    #[test]
    fn config_toml_unanimous() {
        let p = VotingParams::from_config_toml("[voting]\naccept = \"unanimous\"\n").unwrap();
        assert_eq!(p.accept, VoteAccept::Unanimous);
    }

    #[test]
    fn config_toml_council() {
        let p =
            VotingParams::from_config_toml("[voting]\naccept = \"council\"\ncouncil = [0, 1]\n")
                .unwrap();
        assert_eq!(p.accept, VoteAccept::Council);
        assert_eq!(p.council, vec![AgentId(0), AgentId(1)]);
    }

    #[test]
    fn council_without_list_errors() {
        let err = VotingParams::from_config_toml("[voting]\naccept = \"council\"\n").unwrap_err();
        assert!(err.to_string().contains("council"), "{err}");
    }

    #[test]
    fn unknown_accept_errors() {
        let err = VotingParams::from_config_toml("[voting]\naccept = \"maybe\"\n").unwrap_err();
        assert!(err.to_string().contains("accept"), "{err}");
    }

    #[test]
    fn omit_council_tally_is_unanimous() {
        let p =
            VotingParams::from_config_toml("[voting]\naccept = \"council\"\ncouncil = [0, 1]\n")
                .unwrap();
        assert_eq!(p.council_tally, CouncilTally::Unanimous);
    }

    #[test]
    fn config_toml_council_tally_majority() {
        let p = VotingParams::from_config_toml(
            "[voting]\naccept = \"council\"\ncouncil = [0, 1, 2]\ncouncil_tally = \"majority\"\n",
        )
        .unwrap();
        assert_eq!(p.council_tally, CouncilTally::Majority);
    }

    #[test]
    fn unknown_council_tally_errors() {
        let err = VotingParams::from_config_toml(
            "[voting]\naccept = \"council\"\ncouncil = [0]\ncouncil_tally = \"maybe\"\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("council_tally"), "{err}");
    }

    #[test]
    fn council_tally_without_council_accept_errors() {
        let err =
            VotingParams::from_config_toml("[voting]\ncouncil_tally = \"majority\"\n").unwrap_err();
        assert!(err.to_string().contains("council_tally"), "{err}");
    }
}
