//! Haxe `NamingHelper` subset: first + family names, `YOU ARE` / `I AM`.
//!
//! Full Haxe loads `maleNames.txt` / `femaleNames.txt` / `lastNames.txt` (not embedded).
//! Curated lists + 2-letter index / second-char mutate (**AI-LASTNAMES**).

use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Haxe `letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"`.
const NAME_LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// Haxe `for (i in 0...20)` — original probe + 19 second-char mutates.
const GET_NAME_MUTATE_TRIES: usize = 20;

/// Curated male first names (subset of OHOL `maleNames.txt`; uppercase).
pub const MALE_FIRST_NAMES: &[&str] = &[
    "AARON", "ADAM", "ADRIAN", "ALEX", "ANDREW", "ANTHONY", "BEN", "BRIAN", "CALEB", "CHARLES",
    "DANIEL", "DAVID", "ELIAS", "ETHAN", "FELIX", "GABRIEL", "HENRY", "ISAAC", "JACK", "JACOB",
    "JAMES", "JASON", "JOHN", "LEO", "LIAM", "LUCAS", "LUKE", "MARK", "MARTIN", "MASON", "MICHAEL",
    "NOAH", "OLIVER", "OSCAR", "OWEN", "PAUL", "PETER", "RYAN", "SAM", "SEAN", "THOMAS", "VICTOR",
    "WILLIAM",
];

/// Curated female first names (subset of OHOL `femaleNames.txt`; uppercase).
pub const FEMALE_FIRST_NAMES: &[&str] = &[
    "ABIGAIL", "ALICE", "AMELIA", "ANNA", "ARIA", "AURORA", "CARLA", "CHLOE", "CLARA", "DIANA",
    "ELENA", "ELLA", "EMILY", "EMMA", "EVA", "EVE", "FIONA", "GRACE", "HANNAH", "HARPER", "IRENE",
    "ISABEL", "JANE", "JULIA", "KAREN", "KATE", "LILA", "LILY", "LUCY", "MARIA", "MARY", "MIA",
    "NINA", "OLIVIA", "RACHEL", "REBECCA", "ROSE", "RUBY", "SARA", "SOFIA", "SOPHIA", "VIOLET",
    "ZARA", "ZOE",
];

/// Mixed first names (`MALE_FIRST_NAMES` + `FEMALE_FIRST_NAMES`; YOU ARE default).
pub const FIRST_NAMES: &[&str] = &[
    "AARON", "ABIGAIL", "ADAM", "ADRIAN", "ALEX", "ALICE", "AMELIA", "ANDREW", "ANNA", "ANTHONY",
    "ARIA", "AURORA", "BEN", "BRIAN", "CALEB", "CARLA", "CHARLES", "CHLOE", "CLARA", "DANIEL",
    "DAVID", "DIANA", "ELENA", "ELIAS", "ELLA", "EMILY", "EMMA", "ETHAN", "EVA", "EVE", "FELIX",
    "FIONA", "GABRIEL", "GRACE", "HANNAH", "HARPER", "HENRY", "IRENE", "ISAAC", "ISABEL", "JACK",
    "JACOB", "JAMES", "JANE", "JASON", "JOHN", "JULIA", "KAREN", "KATE", "LEO", "LIAM", "LILA",
    "LILY", "LUCAS", "LUCY", "LUKE", "MARIA", "MARK", "MARTIN", "MARY", "MASON", "MIA", "MICHAEL",
    "NINA", "NOAH", "OLIVER", "OLIVIA", "OSCAR", "OWEN", "PAUL", "PETER", "RACHEL", "REBECCA",
    "ROSE", "RUBY", "RYAN", "SAM", "SARA", "SEAN", "SOFIA", "SOPHIA", "THOMAS", "VICTOR", "VIOLET",
    "WILLIAM", "ZARA", "ZOE",
];

/// Curated family names (subset of OHOL `lastNames.txt`; uppercase).
pub const FAMILY_NAMES: &[&str] = &[
    "AARHUS",
    "ABBOTT",
    "ADAMS",
    "ALLEN",
    "ANDERSON",
    "BAKER",
    "BARNES",
    "BELL",
    "BENNETT",
    "BROOKS",
    "BROWN",
    "CAMPBELL",
    "CARTER",
    "CLARK",
    "COLE",
    "COLLINS",
    "COOK",
    "COOPER",
    "COX",
    "DAVIS",
    "EDWARDS",
    "EVANS",
    "FISHER",
    "FOSTER",
    "GRAY",
    "GREEN",
    "HALL",
    "HARRIS",
    "HILL",
    "HUGHES",
    "JACKSON",
    "JAMES",
    "JENKINS",
    "JOHNSON",
    "JONES",
    "KELLY",
    "KING",
    "LEE",
    "LEWIS",
    "LONG",
    "MARTIN",
    "MILLER",
    "MITCHELL",
    "MOORE",
    "MORGAN",
    "MORRIS",
    "MURPHY",
    "NELSON",
    "PARKER",
    "PATTERSON",
    "PERRY",
    "PHILLIPS",
    "POWELL",
    "PRICE",
    "REED",
    "RICHARDSON",
    "ROBERTS",
    "ROBINSON",
    "ROGERS",
    "ROSS",
    "RUSSELL",
    "SCOTT",
    "SMITH",
    "SNOW",
    "STEWART",
    "TAYLOR",
    "THOMAS",
    "THOMPSON",
    "TURNER",
    "WALKER",
    "WARD",
    "WATSON",
    "WHITE",
    "WILLIAMS",
    "WILSON",
    "WOOD",
    "WRIGHT",
    "YOUNG",
];

/// Two-letter bucket index (Haxe `FamilyNames[index]` / `MaleNames` / `FemaleNames`).
// Haxe: NamingHelper.ReadNamesByGender `index = name.substr(0, 2)`
#[derive(Clone, Debug, Default)]
pub struct NameIndex {
    /// Uppercase 2-letter key → sorted unique names in that bucket.
    buckets: HashMap<[u8; 2], Vec<String>>,
}

fn name_index_key(s: &str) -> Option<[u8; 2]> {
    let b = s.as_bytes();
    if b.len() < 2 {
        return None;
    }
    Some([b[0].to_ascii_uppercase(), b[1].to_ascii_uppercase()])
}

/// Build a 2-letter index from any name list (Haxe `ReadNamesByGender` map fill).
// Haxe: NamingHelper.ReadNamesByGender map[name] = name
pub fn build_name_index<'a, I>(names: I) -> NameIndex
where
    I: IntoIterator<Item = &'a str>,
{
    let mut buckets: HashMap<[u8; 2], Vec<String>> = HashMap::new();
    for n in names {
        let u = n.trim().to_ascii_uppercase();
        if u.len() < 2 {
            continue;
        }
        if let Some(k) = name_index_key(&u) {
            buckets.entry(k).or_default().push(u);
        }
    }
    for v in buckets.values_mut() {
        v.sort();
        v.dedup();
    }
    NameIndex { buckets }
}

/// Parse `lastNames.txt` / `maleNames.txt` / `femaleNames.txt` body (one name per line).
// Haxe: NamingHelper.ReadNamesByGender File.readLine
pub fn load_name_index_from_text(text: &str) -> NameIndex {
    build_name_index(text.lines())
}

fn mutate_second_char(name: &str, ch: char) -> String {
    let mut it = name.chars();
    let first = it.next().unwrap_or('A');
    let _second = it.next();
    let rest: String = it.collect();
    format!("{first}{ch}{rest}")
}

fn used_name_set(used: &[&str]) -> HashSet<String> {
    used.iter().map(|s| s.trim().to_ascii_uppercase()).collect()
}

fn pick_from_bucket(
    bucket: &[String],
    probe: &str,
    is_used: &impl Fn(&str) -> bool,
) -> Option<String> {
    if bucket.iter().any(|n| n == probe) && !is_used(probe) {
        return Some(probe.to_string());
    }
    // Haxe `for (ii in 1...newName.length - 1)` exclusive end → ii = 1..=len-2.
    let len = probe.len();
    if len < 3 {
        return None;
    }
    for ii in 1..(len - 1) {
        let test = &probe[..len - ii];
        for n in bucket {
            if n.starts_with(test) || n.contains(test) {
                // Haxe bug: `isUsedName(name)` uses exact-lookup var, not candidate `n`.
                // Product + existing tests require unused **candidate**.
                if !is_used(n) {
                    return Some(n.clone());
                }
            }
        }
    }
    None
}

fn probe_bucket(probe: &str, index: &NameIndex, is_used: &impl Fn(&str) -> bool) -> Option<String> {
    let k = name_index_key(probe)?;
    let bucket = index.buckets.get(&k)?;
    pick_from_bucket(bucket, probe, is_used)
}

/// Haxe `GetNameFromList` over a 2-letter index (20 tries, random second char).
///
/// `rng` picks A–Z (`WorldMap.calculateRandomInt(25)` → `0..26`). Live wrappers
/// use `thread_rng`.
// Haxe: NamingHelper.GetNameFromList L278–321
pub fn get_name_from_index(
    current: &str,
    used: &[&str],
    index: &NameIndex,
    rng: &mut impl Rng,
) -> Option<String> {
    let mut probe = current.trim().to_ascii_uppercase();
    if probe.len() < 2 {
        return None;
    }
    let used_up = used_name_set(used);
    let is_used = |n: &str| used_up.contains(n);

    for i in 0..GET_NAME_MUTATE_TRIES {
        if i > 0 {
            let ch = NAME_LETTERS[rng.gen_range(0..NAME_LETTERS.len())] as char;
            probe = mutate_second_char(&probe, ch);
        }
        if let Some(n) = probe_bucket(&probe, index, &is_used) {
            return Some(n);
        }
    }
    None
}

/// Sparse curated buckets vs dense Haxe `lastNames.txt`: after 19 random second
/// chars miss, scan A–Z so unused same-initial names still resolve in tests.
fn get_name_from_index_az(current: &str, used: &[&str], index: &NameIndex) -> Option<String> {
    let original = current.trim().to_ascii_uppercase();
    if original.len() < 2 {
        return None;
    }
    let used_up = used_name_set(used);
    let is_used = |n: &str| used_up.contains(n);
    for &b in NAME_LETTERS {
        let probe = mutate_second_char(&original, b as char);
        if let Some(n) = probe_bucket(&probe, index, &is_used) {
            return Some(n);
        }
    }
    None
}

fn lookup_name(current: &str, used: &[&str], index: &NameIndex) -> Option<String> {
    let mut rng = rand::thread_rng();
    get_name_from_index(current, used, index, &mut rng)
        .or_else(|| get_name_from_index_az(current, used, index))
}

/// Unused name from `pool` via Haxe 2-letter map + second-char mutate.
// Haxe: NamingHelper.GetNameFromList
pub fn get_name_from_list(current: &str, used: &[&str], pool: &[&str]) -> Option<String> {
    let idx = build_name_index(pool.iter().copied());
    lookup_name(current, used, &idx)
}

/// Cwd `lastNames.txt` / `maleNames.txt` / `femaleNames.txt` (Haxe `./`).
/// Tests never call this (`cfg(test)` uses curated lists).
// Haxe: NamingHelper.ReadNamesByGender File.read `./lastNames.txt`
fn try_load_cwd_name_index(filename: &str) -> Option<NameIndex> {
    let text = std::fs::read_to_string(filename).ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(load_name_index_from_text(&text))
}

fn load_or_curated(curated: &'static [&'static str], cwd_file: &str) -> NameIndex {
    if !cfg!(test) {
        if let Some(idx) = try_load_cwd_name_index(cwd_file) {
            return idx;
        }
    }
    build_name_index(curated.iter().copied())
}

fn family_name_index() -> &'static NameIndex {
    static IDX: OnceLock<NameIndex> = OnceLock::new();
    IDX.get_or_init(|| load_or_curated(FAMILY_NAMES, "lastNames.txt"))
}

fn first_name_index() -> &'static NameIndex {
    static IDX: OnceLock<NameIndex> = OnceLock::new();
    IDX.get_or_init(|| build_name_index(FIRST_NAMES.iter().copied()))
}

/// Haxe `NamingHelper.GetFamilyNameFromList` — unused family from curated list
/// (or cwd `lastNames.txt` on live boot).
// Haxe: NamingHelper.GetFamilyNameFromList / GetNameFromList family=true
pub fn get_family_name_from_list(current: &str, used: &[&str]) -> Option<String> {
    lookup_name(current, used, family_name_index())
}

/// Haxe `GetNameFromList` first-name (mixed curated list).
// Haxe: NamingHelper.GetNameFromList (YOU ARE planner has no sex flag → mixed)
pub fn get_first_name_from_list(current: &str, used: &[&str]) -> Option<String> {
    lookup_name(current, used, first_name_index())
}

fn male_first_name_index() -> &'static NameIndex {
    static IDX: OnceLock<NameIndex> = OnceLock::new();
    IDX.get_or_init(|| load_or_curated(MALE_FIRST_NAMES, "maleNames.txt"))
}

fn female_first_name_index() -> &'static NameIndex {
    static IDX: OnceLock<NameIndex> = OnceLock::new();
    IDX.get_or_init(|| load_or_curated(FEMALE_FIRST_NAMES, "femaleNames.txt"))
}

/// Haxe `GetNameFromList(newName, female)` gender-split curated lists.
// Haxe: NamingHelper.GetNameFromList female=true/false
pub fn get_first_name_from_list_gender(
    current: &str,
    used: &[&str],
    female: bool,
) -> Option<String> {
    let idx = if female {
        female_first_name_index()
    } else {
        male_first_name_index()
    };
    lookup_name(current, used, idx)
}

/// Haxe `ServerSettings.StartingFamilyName`.
// Haxe: ServerSettings.StartingFamilyName = "SNOW"
pub const STARTING_FAMILY_NAME: &str = "SNOW";
/// Haxe `ServerSettings.StartingName`.
// Haxe: ServerSettings.StartingName = "SPOON"
pub const STARTING_NAME: &str = "SPOON";
/// Haxe `getClosestPlayer(5)` for YOU ARE when not holding.
// Haxe: NamingHelper.DoNaming L49
pub const YOU_ARE_CLOSE_TILES: i32 = 5;
/// Haxe `ServerSettings.FoundFamilyNeededPrestige` (DoNaming I AM gates).
pub const NAMING_FOUND_FAMILY_PRESTIGE: f32 = 50.0;
/// Haxe `ServerSettings.FoundFamilyNeededFollowers`.
pub const NAMING_FOUND_FAMILY_FOLLOWERS: i32 = 4;
/// Haxe `ServerSettings.FoundFamilyCost`.
pub const NAMING_FOUND_FAMILY_COST: f32 = 10.0;

/// Haxe `NamingHelper.GetName` (`nameFirst=false` → third whitespace token).
// Haxe: NamingHelper.GetName L225–231
pub fn get_name_token(text: &str) -> String {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 3 {
        String::new()
    } else {
        parts[2].to_ascii_uppercase()
    }
}

/// `I AM` family form: starts with I AM and has no `!`.
// Haxe: DoNaming `startsWith('I AM') && contains("!") == false`
pub fn is_iam_family_say(text: &str) -> bool {
    let up = text.trim().to_ascii_uppercase();
    up.starts_with("I AM") && !up.contains('!')
}

/// `YOU ARE` first-name form.
// Haxe: NamingHelper.DoNaming L45 `startsWith('YOU ARE')`
pub fn is_you_are_say(text: &str) -> bool {
    text.trim().to_ascii_uppercase().starts_with("YOU ARE")
}

/// Held baby else closest living other within `max_cheby` (Chebyshev).
// Haxe: DoNaming heldPlayer else getClosestPlayer(5)
pub fn pick_you_are_target(
    speaker_p_id: i32,
    held_p_id: i32,
    sx: i32,
    sy: i32,
    others: &[(i32, i32, i32)],
    max_cheby: i32,
) -> Option<i32> {
    if held_p_id > 0 && held_p_id != speaker_p_id {
        return Some(held_p_id);
    }
    let mut best: Option<(i32, i32)> = None; // cheby, p_id
    for &(pid, x, y) in others {
        if pid == speaker_p_id {
            continue;
        }
        let cheby = (x - sx).abs().max((y - sy).abs());
        if cheby > max_cheby {
            continue;
        }
        match best {
            None => best = Some((cheby, pid)),
            Some((bc, bp)) if cheby < bc || (cheby == bc && pid < bp) => {
                best = Some((cheby, pid));
            }
            _ => {}
        }
    }
    best.map(|(_, pid)| pid)
}

/// Haxe DoNaming YOU ARE first-name outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoNamingYouAre {
    NotNaming,
    Apply { first_name: String },
}

/// Haxe `DoNaming` YOU ARE planner (target already chosen).
// Haxe: NamingHelper.DoNaming L94–198
pub fn plan_do_naming_you_are(
    text: &str,
    target_first_name: &str,
    used_first_names: &[&str],
) -> DoNamingYouAre {
    plan_do_naming_you_are_ex(text, target_first_name, used_first_names, STARTING_NAME)
}

/// Live `StartingName` gate; empty/`trim` falls back to compiled `STARTING_NAME`.
// Haxe: NamingHelper.DoNaming `player.name == ServerSettings.StartingName`
pub fn plan_do_naming_you_are_ex(
    text: &str,
    target_first_name: &str,
    used_first_names: &[&str],
    starting_name: &str,
) -> DoNamingYouAre {
    if !is_you_are_say(text) {
        return DoNamingYouAre::NotNaming;
    }
    if target_first_name.trim().to_ascii_uppercase()
        != live_starting_upper(starting_name, STARTING_NAME)
    {
        return DoNamingYouAre::NotNaming;
    }
    let token = get_name_token(text);
    if !iam_name_token_ok(&token) {
        return DoNamingYouAre::NotNaming;
    }
    let Some(first) = get_first_name_from_list(&token, used_first_names) else {
        return DoNamingYouAre::NotNaming;
    };
    DoNamingYouAre::Apply { first_name: first }
}

/// Haxe `GetNameFromList(newName, female)` — gender-split first-name files.
// Haxe: NamingHelper.GetNameFromList female=true/false (DoNaming YOU ARE)
pub fn plan_do_naming_you_are_ex_gender(
    text: &str,
    target_first_name: &str,
    used_first_names: &[&str],
    starting_name: &str,
    female: bool,
) -> DoNamingYouAre {
    if !is_you_are_say(text) {
        return DoNamingYouAre::NotNaming;
    }
    if target_first_name.trim().to_ascii_uppercase()
        != live_starting_upper(starting_name, STARTING_NAME)
    {
        return DoNamingYouAre::NotNaming;
    }
    let token = get_name_token(text);
    if !iam_name_token_ok(&token) {
        return DoNamingYouAre::NotNaming;
    }
    let Some(first) = get_first_name_from_list_gender(&token, used_first_names, female) else {
        return DoNamingYouAre::NotNaming;
    };
    DoNamingYouAre::Apply { first_name: first }
}

fn live_starting_upper(live: &str, compiled: &str) -> String {
    let t = live.trim();
    if t.is_empty() {
        compiled.to_ascii_uppercase()
    } else {
        t.to_ascii_uppercase()
    }
}

fn live_nonneg_or(v: f32, compiled: f32) -> f32 {
    if v.is_finite() && v >= 0.0 {
        v
    } else {
        compiled
    }
}

fn live_i32_nonneg(v: i32, compiled: i32) -> i32 {
    if v < 0 {
        compiled
    } else {
        v
    }
}

fn iam_name_token_ok(name: &str) -> bool {
    if matches!(name, "NOT" | "HIRED" | "KING") {
        return false;
    }
    if name.len() < 2 {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphabetic())
}

/// Haxe DoNaming I AM family outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoNamingIam {
    /// Not an I AM family line (leave SAY unchanged).
    NotNaming,
    /// Private reject (Haxe `say(..., true)` then return null).
    Reject { private_say: String },
    /// Apply family name; `found_new` → coins + myEveId + follower migrate.
    Apply {
        family_name: String,
        found_new: bool,
    },
}

/// Haxe `NamingHelper.DoNaming` family-I-AM planner.
// Haxe: NamingHelper.DoNaming L40–173
pub fn plan_do_naming_iam(
    text: &str,
    current_family: &str,
    is_family_founder: bool,
    same_account_as_eve: bool,
    prestige: f32,
    same_family_followers: i32,
    coins: f32,
    used_family_names: &[&str],
) -> DoNamingIam {
    plan_do_naming_iam_ex(
        text,
        current_family,
        is_family_founder,
        same_account_as_eve,
        prestige,
        same_family_followers,
        coins,
        used_family_names,
        STARTING_FAMILY_NAME,
        NAMING_FOUND_FAMILY_PRESTIGE,
        NAMING_FOUND_FAMILY_COST,
        NAMING_FOUND_FAMILY_FOLLOWERS,
    )
}

/// Live `StartingFamilyName` found-new gate; empty/`trim` falls back to compiled.
/// Live `FoundFamilyNeededPrestige` / `FoundFamilyCost` (finite `>= 0` else compiled).
/// Live `FoundFamilyNeededFollowers` (`< 0` else compiled 4).
// Haxe: NamingHelper.DoNaming `familyName != ServerSettings.StartingFamilyName`
pub fn plan_do_naming_iam_ex(
    text: &str,
    current_family: &str,
    is_family_founder: bool,
    same_account_as_eve: bool,
    prestige: f32,
    same_family_followers: i32,
    coins: f32,
    used_family_names: &[&str],
    starting_family_name: &str,
    needed_prestige: f32,
    found_family_cost: f32,
    needed_followers: i32,
) -> DoNamingIam {
    if !is_iam_family_say(text) {
        return DoNamingIam::NotNaming;
    }
    let name = get_name_token(text);
    if !iam_name_token_ok(&name) {
        return DoNamingIam::NotNaming;
    }
    let cur = current_family.trim().to_ascii_uppercase();
    let found_new = cur != live_starting_upper(starting_family_name, STARTING_FAMILY_NAME);
    if found_new {
        if is_family_founder {
            return DoNamingIam::Reject {
                private_say: "I am the founder already!".into(),
            };
        }
        if same_account_as_eve {
            return DoNamingIam::Reject {
                private_say: "My spirit is the founder already!".into(),
            };
        }
        let needed_prestige = live_nonneg_or(needed_prestige, NAMING_FOUND_FAMILY_PRESTIGE);
        let missing = (needed_prestige - prestige).ceil() as i32;
        if missing > 0 {
            return DoNamingIam::Reject {
                private_say: format!("I need {missing} more prestige!"),
            };
        }
        let needed_followers = live_i32_nonneg(needed_followers, NAMING_FOUND_FAMILY_FOLLOWERS);
        let missing = needed_followers - same_family_followers;
        if missing > 0 {
            return DoNamingIam::Reject {
                private_say: format!("I need {missing} more followers!"),
            };
        }
        let found_family_cost = live_nonneg_or(found_family_cost, NAMING_FOUND_FAMILY_COST);
        let missing = (found_family_cost - coins).ceil() as i32;
        if missing > 0 {
            return DoNamingIam::Reject {
                private_say: format!("I need {missing} more coins!"),
            };
        }
    }
    let used_up: Vec<String> = used_family_names
        .iter()
        .map(|s| s.trim().to_ascii_uppercase())
        .collect();
    if used_up.iter().any(|u| u == &name) {
        return DoNamingIam::NotNaming;
    }
    DoNamingIam::Apply {
        family_name: name,
        found_new,
    }
}

/// Haxe follower-migrate rand gate (non-close relatives).
///
/// Close relatives always migrate. Else `rand + count*0.1 - 0.5 if same home
/// + 0.5 if different color`; skip when result `< 1`.
// Haxe: NamingHelper.DoNaming foundNewFamily follower loop L152–157
pub fn should_migrate_found_family_follower(
    close_relative: bool,
    same_home: bool,
    different_color: bool,
    already_migrated: i32,
    rand: f32,
) -> bool {
    if close_relative {
        return true;
    }
    let mut r = rand + already_migrated as f32 * 0.1;
    if same_home {
        r -= 0.5;
    }
    if different_color {
        r += 0.5;
    }
    r >= 1.0
}

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
        assert!(!MALE_FIRST_NAMES.is_empty());
        assert!(!FEMALE_FIRST_NAMES.is_empty());
        assert_eq!(
            FIRST_NAMES.len(),
            MALE_FIRST_NAMES.len() + FEMALE_FIRST_NAMES.len()
        );
        for n in FIRST_NAMES {
            assert!(!n.is_empty(), "empty first name entry");
        }
        for n in FAMILY_NAMES {
            assert!(!n.is_empty(), "empty family name entry");
        }
        for n in MALE_FIRST_NAMES {
            assert!(FIRST_NAMES.contains(n), "male {n} missing from FIRST_NAMES");
        }
        for n in FEMALE_FIRST_NAMES {
            assert!(
                FIRST_NAMES.contains(n),
                "female {n} missing from FIRST_NAMES"
            );
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

    #[test]
    fn get_family_name_from_list_skips_used_and_prefers_prefix() {
        assert_eq!(
            get_family_name_from_list("SNOW", &[]).as_deref(),
            Some("SNOW")
        );
        let used = ["SNOW"];
        let next = get_family_name_from_list("SNOW", &used).expect("unused family");
        assert_ne!(next, "SNOW");
        assert!(FAMILY_NAMES.contains(&next.as_str()));
        assert!(next.starts_with('S') || next.starts_with("SN"), "{next}");
        let all: Vec<&str> = FAMILY_NAMES.to_vec();
        assert_eq!(get_family_name_from_list("SNOW", &all), None);
    }

    #[test]
    fn get_name_from_index_two_letter_and_second_char_mutate() {
        let idx = load_name_index_from_text("SNOW\nSCOTT\nSMITH\nBAKER\n");
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            get_name_from_index("SNOW", &[], &idx, &mut rng).as_deref(),
            Some("SNOW")
        );
        // SN bucket exhausted → mutate second char; first char stays S.
        let next = (0u64..64)
            .find_map(|seed| {
                let mut r = StdRng::seed_from_u64(seed);
                get_name_from_index("SNOW", &["SNOW"], &idx, &mut r)
            })
            .expect("mutate");
        assert_eq!(next.chars().next(), Some('S'));
        assert!(matches!(next.as_str(), "SCOTT" | "SMITH"), "{next}");
        assert_ne!(next, "BAKER", "first char must stay S");
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(
            get_name_from_index("SNOW", &["SNOW", "SCOTT", "SMITH"], &idx, &mut rng),
            None
        );
        assert_eq!(
            get_name_from_index("BAKER", &[], &idx, &mut rng).as_deref(),
            Some("BAKER")
        );
        assert_eq!(
            get_first_name_from_list_gender("ALICE", &[], true).as_deref(),
            Some("ALICE")
        );
        assert_eq!(
            get_first_name_from_list_gender("ALICE", &[], false).as_deref(),
            Some("ALEX")
        );
        assert_eq!(
            get_first_name_from_list_gender("ADAM", &[], false).as_deref(),
            Some("ADAM")
        );
    }

    #[test]
    fn plan_do_naming_iam_gates() {
        assert_eq!(
            plan_do_naming_iam("I AM HOME!", "SNOW", false, false, 50.0, 4, 10.0, &[]),
            DoNamingIam::NotNaming
        );
        assert_eq!(
            plan_do_naming_iam("HELLO", "SNOW", false, false, 50.0, 4, 10.0, &[]),
            DoNamingIam::NotNaming
        );
        assert_eq!(get_name_token("I AM SMITH"), "SMITH");
        match plan_do_naming_iam("I AM SMITH", "SNOW", false, false, 0.0, 0, 0.0, &[]) {
            DoNamingIam::Apply {
                family_name,
                found_new,
            } => {
                assert_eq!(family_name, "SMITH");
                assert!(!found_new);
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam("I AM SMITH", "BAKER", true, false, 50.0, 4, 10.0, &[]) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("founder already"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam("I AM SMITH", "BAKER", false, false, 10.0, 4, 10.0, &[]) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("prestige"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam("I AM SMITH", "BAKER", false, false, 50.0, 4, 10.0, &[]) {
            DoNamingIam::Apply {
                family_name,
                found_new,
            } => {
                assert_eq!(family_name, "SMITH");
                assert!(found_new);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            plan_do_naming_iam(
                "I AM SMITH",
                "BAKER",
                false,
                false,
                50.0,
                4,
                10.0,
                &["SMITH"]
            ),
            DoNamingIam::NotNaming
        );
        match plan_do_naming_iam("I AM SMITH", "BAKER", false, false, 50.0, 1, 10.0, &[]) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("followers"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam("I AM SMITH", "BAKER", false, false, 50.0, 4, 1.0, &[]) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("coins"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            plan_do_naming_iam(
                "I AM NOT YOUR ALLY!",
                "SNOW",
                false,
                false,
                50.0,
                4,
                10.0,
                &[]
            ),
            DoNamingIam::NotNaming
        );
        assert_eq!(
            plan_do_naming_iam("I AM KING", "SNOW", false, false, 50.0, 4, 10.0, &[]),
            DoNamingIam::NotNaming
        );
        assert!(should_migrate_found_family_follower(
            true, false, false, 0, 0.0
        ));
        assert!(!should_migrate_found_family_follower(
            false, false, false, 0, 0.2
        ));
        // 1.6 - 0.5 same-home = 1.1 ≥ 1
        assert!(should_migrate_found_family_follower(
            false, true, false, 0, 1.6
        ));
        assert!(should_migrate_found_family_follower(
            false, true, false, 0, 1.5
        ));
        assert!(!should_migrate_found_family_follower(
            false, false, false, 0, 0.0
        ));
    }

    #[test]
    fn plan_do_naming_you_are_gates() {
        assert_eq!(
            plan_do_naming_you_are("YOU ARE ALICE", "BOB", &[]),
            DoNamingYouAre::NotNaming
        );
        assert_eq!(
            plan_do_naming_you_are("HELLO", STARTING_NAME, &[]),
            DoNamingYouAre::NotNaming
        );
        assert_eq!(
            plan_do_naming_you_are("YOU ARE NOT", STARTING_NAME, &[]),
            DoNamingYouAre::NotNaming
        );
        match plan_do_naming_you_are("YOU ARE ALICE", STARTING_NAME, &[]) {
            DoNamingYouAre::Apply { first_name } => assert_eq!(first_name, "ALICE"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            plan_do_naming_you_are("YOU ARE ALICE!", STARTING_NAME, &[]),
            DoNamingYouAre::NotNaming
        );
        assert_eq!(
            plan_do_naming_you_are("YOU ARE HIRED", STARTING_NAME, &[]),
            DoNamingYouAre::NotNaming
        );
        assert_eq!(
            plan_do_naming_you_are("YOU ARE KING", STARTING_NAME, &[]),
            DoNamingYouAre::NotNaming
        );
        match plan_do_naming_you_are("YOU ARE ALICE", STARTING_NAME, &["ALICE"]) {
            DoNamingYouAre::Apply { first_name } => {
                assert_ne!(first_name, "ALICE");
                assert!(FIRST_NAMES.contains(&first_name.as_str()));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            get_first_name_from_list("ALICE", &[]).as_deref(),
            Some("ALICE")
        );
        assert_eq!(
            pick_you_are_target(1, 9, 0, 0, &[(2, 1, 0), (3, 2, 0)], 5),
            Some(9)
        );
        assert_eq!(
            pick_you_are_target(1, 0, 0, 0, &[(2, 1, 0), (3, 2, 0)], 5),
            Some(2)
        );
        assert_eq!(pick_you_are_target(1, 0, 0, 0, &[(2, 8, 0)], 5), None);
    }

    #[test]
    fn plan_do_naming_iam_ex_live_starting_family() {
        // current SNOW vs live ICE ⇒ found_new; founder/prestige gates fire
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "SNOW",
            true,
            false,
            50.0,
            4,
            10.0,
            &[],
            "ICE",
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("founder already"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "SNOW",
            false,
            false,
            10.0,
            4,
            10.0,
            &[],
            "ICE",
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("prestige"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "SNOW",
            false,
            false,
            50.0,
            4,
            10.0,
            &[],
            "ICE",
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Apply {
                family_name,
                found_new,
            } => {
                assert_eq!(family_name, "SMITH");
                assert!(found_new);
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "SNOW",
            false,
            false,
            0.0,
            0,
            0.0,
            &[],
            "  ",
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Apply { found_new, .. } => assert!(!found_new),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plan_do_naming_iam_ex_live_prestige_and_cost() {
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "BAKER",
            false,
            false,
            50.0,
            4,
            10.0,
            &[],
            STARTING_FAMILY_NAME,
            100.0,
            10.0,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("prestige"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "BAKER",
            false,
            false,
            50.0,
            4,
            10.0,
            &[],
            STARTING_FAMILY_NAME,
            50.0,
            20.0,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("coins"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "BAKER",
            false,
            false,
            50.0,
            4,
            10.0,
            &[],
            STARTING_FAMILY_NAME,
            50.0,
            10.0,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Apply {
                family_name,
                found_new,
            } => {
                assert_eq!(family_name, "SMITH");
                assert!(found_new);
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            STARTING_FAMILY_NAME,
            false,
            false,
            0.0,
            0,
            0.0,
            &[],
            STARTING_FAMILY_NAME,
            100.0,
            20.0,
            NAMING_FOUND_FAMILY_FOLLOWERS,
        ) {
            DoNamingIam::Apply { found_new, .. } => assert!(!found_new),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plan_do_naming_iam_ex_live_needed_followers() {
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "BAKER",
            false,
            false,
            50.0,
            4,
            10.0,
            &[],
            STARTING_FAMILY_NAME,
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            8,
        ) {
            DoNamingIam::Reject { private_say } => {
                assert!(private_say.contains("followers"));
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            "BAKER",
            false,
            false,
            50.0,
            0,
            10.0,
            &[],
            STARTING_FAMILY_NAME,
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            0,
        ) {
            DoNamingIam::Apply {
                family_name,
                found_new,
            } => {
                assert_eq!(family_name, "SMITH");
                assert!(found_new);
            }
            other => panic!("{other:?}"),
        }
        match plan_do_naming_iam_ex(
            "I AM SMITH",
            STARTING_FAMILY_NAME,
            false,
            false,
            0.0,
            0,
            0.0,
            &[],
            STARTING_FAMILY_NAME,
            NAMING_FOUND_FAMILY_PRESTIGE,
            NAMING_FOUND_FAMILY_COST,
            8,
        ) {
            DoNamingIam::Apply { found_new, .. } => assert!(!found_new),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn plan_do_naming_you_are_ex_live_starting_name() {
        match plan_do_naming_you_are_ex("YOU ARE ALICE", "FORK", &[], "FORK") {
            DoNamingYouAre::Apply { first_name } => assert_eq!(first_name, "ALICE"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            plan_do_naming_you_are_ex("YOU ARE ALICE", "SPOON", &[], "FORK"),
            DoNamingYouAre::NotNaming
        );
        match plan_do_naming_you_are_ex("YOU ARE ALICE", "fork", &[], "FORK") {
            DoNamingYouAre::Apply { first_name } => assert_eq!(first_name, "ALICE"),
            other => panic!("{other:?}"),
        }
        assert_eq!(
            plan_do_naming_you_are_ex("YOU ARE NOT", "FORK", &[], "FORK"),
            DoNamingYouAre::NotNaming
        );
    }

    #[test]
    fn plan_do_naming_you_are_gender_split_lists() {
        match plan_do_naming_you_are_ex_gender("YOU ARE ALICE", "SPOON", &[], STARTING_NAME, true)
        {
            DoNamingYouAre::Apply { first_name } => assert_eq!(first_name, "ALICE"),
            other => panic!("{other:?}"),
        }
        match plan_do_naming_you_are_ex_gender("YOU ARE ALICE", "SPOON", &[], STARTING_NAME, false)
        {
            DoNamingYouAre::Apply { first_name } => {
                assert_ne!(first_name, "ALICE", "male list has no ALICE");
                assert!(MALE_FIRST_NAMES.contains(&first_name.as_str()));
            }
            other => panic!("male YOU ARE ALICE should still resolve a male name: {other:?}"),
        }
    }
}
