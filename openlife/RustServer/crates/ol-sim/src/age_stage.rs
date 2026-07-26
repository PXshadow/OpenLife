//! Age stage labels (Haxe life stage subset).

/// Coarse life stage from age years.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeStage {
    Infant,
    Child,
    Adult,
    Elder,
}

impl AgeStage {
    pub fn from_age(age: f32) -> Self {
        if !age.is_finite() || age < 3.0 {
            Self::Infant
        } else if age < 14.0 {
            Self::Child
        } else if age < 60.0 {
            Self::Adult
        } else {
            Self::Elder
        }
    }

    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Infant => "infant",
            Self::Child => "child",
            Self::Adult => "adult",
            Self::Elder => "elder",
        }
    }
}

/// `STAGE infant|child|adult|elder age=N.NN`
pub fn format_stage_query(age: f32) -> String {
    let s = AgeStage::from_age(age);
    format!("STAGE {} age={age:.2}", s.wire_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages() {
        assert_eq!(AgeStage::from_age(0.0), AgeStage::Infant);
        assert_eq!(AgeStage::from_age(10.0), AgeStage::Child);
        assert_eq!(AgeStage::from_age(20.0), AgeStage::Adult);
        assert_eq!(AgeStage::from_age(70.0), AgeStage::Elder);
        assert!(format_stage_query(14.0).contains("adult"));
    }
}
