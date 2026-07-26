//! Haxe `NamingHelper` subset: random first + family names for newborns.
//!
//! Full Haxe loads `maleNames.txt` / `femaleNames.txt` / `lastNames.txt` and
//! supports "YOU ARE" / "I AM" naming. This crate embeds a small curated list
//! and only exposes [`pick_random_name`] for Eve/spawn assignment.

use rand::Rng;

/// Curated first names (subset of OHOL male + female name lists; uppercase).
pub const FIRST_NAMES: &[&str] = &[
    "AARON", "ABIGAIL", "ADAM", "ADRIAN", "ALEX", "ALICE", "AMELIA", "ANDREW",
    "ANNA", "ANTHONY", "ARIA", "AURORA", "BEN", "BRIAN", "CALEB", "CARLA",
    "CHARLES", "CHLOE", "CLARA", "DANIEL", "DAVID", "DIANA", "ELENA", "ELIAS",
    "ELLA", "EMILY", "EMMA", "ETHAN", "EVA", "EVE", "FELIX", "FIONA", "GABRIEL",
    "GRACE", "HANNAH", "HARPER", "HENRY", "IRENE", "ISAAC", "ISABEL", "JACK",
    "JACOB", "JAMES", "JANE", "JASON", "JOHN", "JULIA", "KAREN", "KATE", "LEO",
    "LIAM", "LILA", "LILY", "LUCAS", "LUCY", "LUKE", "MARIA", "MARK", "MARTIN",
    "MARY", "MASON", "MIA", "MICHAEL", "NINA", "NOAH", "OLIVER", "OLIVIA",
    "OSCAR", "OWEN", "PAUL", "PETER", "RACHEL", "REBECCA", "ROSE", "RUBY",
    "RYAN", "SAM", "SARA", "SEAN", "SOFIA", "SOPHIA", "THOMAS", "VICTOR",
    "VIOLET", "WILLIAM", "ZARA", "ZOE",
];

/// Curated family names (subset of OHOL `lastNames.txt`; uppercase).
pub const FAMILY_NAMES: &[&str] = &[
    "AARHUS", "ABBOTT", "ADAMS", "ALLEN", "ANDERSON", "BAKER", "BARNES",
    "BELL", "BENNETT", "BROOKS", "BROWN", "CAMPBELL", "CARTER", "CLARK",
    "COLE", "COLLINS", "COOK", "COOPER", "COX", "DAVIS", "EDWARDS", "EVANS",
    "FISHER", "FOSTER", "GRAY", "GREEN", "HALL", "HARRIS", "HILL", "HUGHES",
    "JACKSON", "JAMES", "JENKINS", "JOHNSON", "JONES", "KELLY", "KING",
    "LEE", "LEWIS", "LONG", "MARTIN", "MILLER", "MITCHELL", "MOORE", "MORGAN",
    "MORRIS", "MURPHY", "NELSON", "PARKER", "PATTERSON", "PERRY", "PHILLIPS",
    "POWELL", "PRICE", "REED", "RICHARDSON", "ROBERTS", "ROBINSON", "ROGERS",
    "ROSS", "RUSSELL", "SCOTT", "SMITH", "SNOW", "STEWART", "TAYLOR",
    "THOMAS", "THOMPSON", "TURNER", "WALKER", "WARD", "WATSON", "WHITE",
    "WILLIAMS", "WILSON", "WOOD", "WRIGHT", "YOUNG",
];

/// Pick a random first name and family name (Haxe `GetRandomName` + family list).
///
/// Returns owned uppercase strings suitable for NM packets / [`Player`] fields.
pub fn pick_random_name(rng: &mut impl Rng) -> (String, String) {
    let first = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())];
    let family = FAMILY_NAMES[rng.gen_range(0..FAMILY_NAMES.len())];
    (first.to_string(), family.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn name_lists_non_empty() {
        assert!(!FIRST_NAMES.is_empty());
        assert!(!FAMILY_NAMES.is_empty());
        for n in FIRST_NAMES {
            assert!(!n.is_empty(), "empty first name entry");
        }
        for n in FAMILY_NAMES {
            assert!(!n.is_empty(), "empty family name entry");
        }
    }

    #[test]
    fn pick_random_name_non_empty() {
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..64 {
            let (first, family) = pick_random_name(&mut rng);
            assert!(!first.is_empty(), "first name empty");
            assert!(!family.is_empty(), "family name empty");
            assert!(
                FIRST_NAMES.contains(&first.as_str()),
                "unexpected first: {first}"
            );
            assert!(
                FAMILY_NAMES.contains(&family.as_str()),
                "unexpected family: {family}"
            );
        }
    }

    #[test]
    fn pick_random_name_varies_with_seed() {
        let mut a = StdRng::seed_from_u64(1);
        let mut b = StdRng::seed_from_u64(2);
        let names_a: Vec<_> = (0..8).map(|_| pick_random_name(&mut a)).collect();
        let names_b: Vec<_> = (0..8).map(|_| pick_random_name(&mut b)).collect();
        assert_ne!(
            names_a, names_b,
            "different seeds should usually yield different sequences"
        );
    }
}
