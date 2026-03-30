use serde::{Deserialize, Serialize};

/// Message priority levels.
///
/// Higher priority messages may be processed first in queue-based systems,
/// and some providers support mapping these to native priority fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Lowest priority — informational, non-urgent.
    Low,
    /// Normal priority (default).
    Normal,
    /// Higher than normal — should be noticed promptly.
    High,
    /// Highest priority — urgent / critical alert.
    Urgent,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" | "0" | "min" => Ok(Self::Low),
            "normal" | "1" | "default" => Ok(Self::Normal),
            "high" | "2" => Ok(Self::High),
            "urgent" | "critical" | "3" | "max" | "emergency" => Ok(Self::Urgent),
            _ => Err(format!("unknown priority: {s}")),
        }
    }
}

impl Priority {
    /// Convert to a numeric value (0 = low, 3 = urgent).
    pub fn as_numeric(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Urgent);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Low.to_string(), "low");
        assert_eq!(Priority::Normal.to_string(), "normal");
        assert_eq!(Priority::High.to_string(), "high");
        assert_eq!(Priority::Urgent.to_string(), "urgent");
    }

    #[test]
    fn test_priority_parse() {
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("normal".parse::<Priority>().unwrap(), Priority::Normal);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("urgent".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("0".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("3".parse::<Priority>().unwrap(), Priority::Urgent);
        assert!("invalid".parse::<Priority>().is_err());
    }

    #[test]
    fn test_priority_numeric() {
        assert_eq!(Priority::Low.as_numeric(), 0);
        assert_eq!(Priority::Normal.as_numeric(), 1);
        assert_eq!(Priority::High.as_numeric(), 2);
        assert_eq!(Priority::Urgent.as_numeric(), 3);
    }

    #[test]
    fn test_priority_serde() {
        let json = serde_json::to_string(&Priority::High).unwrap();
        assert_eq!(json, "\"high\"");
        let parsed: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Priority::High);
    }
}
