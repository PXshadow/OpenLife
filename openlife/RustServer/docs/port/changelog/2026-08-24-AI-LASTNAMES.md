# AI-LASTNAMES — 2-letter name index + second-char mutate

## Haxe

- `NamingHelper.GetNameFromList` / `GetFamilyNameFromList` ~L274–321
- `ReadNamesByGender` ~L350–405 buckets by `substr(0, 2)` (`FamilyNames` / `MaleNames` / `FemaleNames`)
- Loop `i in 0...20`: exact `map[newName]`; else shorten `substr(0, len-ii)` (`ii in 1...len-1`) `startsWith` / `contains`
- `i>0`: mutate **second** char to random A–Z (`calculateRandomInt(25)`); first char stays
- Inner loop Haxe bug: `isUsedName(name)` uses exact-lookup var, not candidate `n`

## Rust

| Symbol | Role |
|--------|------|
| `NameIndex` / `build_name_index` / `load_name_index_from_text` | 2-letter buckets, sorted names |
| `get_name_from_index(current, used, index, rng)` | Haxe 20-try loop + random second-char |
| `get_family_name_from_list` | curated `FAMILY_NAMES` index (`thread_rng`; A–Z sparse net) |
| `get_first_name_from_list` | mixed `FIRST_NAMES` (YOU ARE planner has no sex flag) |
| `get_first_name_from_list_gender` | `MALE_FIRST_NAMES` / `FEMALE_FIRST_NAMES` |
| cwd `OnceLock` | live `lastNames.txt` / `maleNames.txt` / `femaleNames.txt` if present; **tests always curated** |

Unused check uses candidate `n` (Haxe inner-loop bug). Full 151k `lastNames.txt` is **not** embedded.

## Tests

```
cargo test -p ol-sim --lib -- get_family_name_from_list get_name_from_list plan_do_naming_iam plan_do_naming_you_are say_i_am say_you_are tick_found_family -- --test-threads=1
```

## Residual

1. Full OHOL name files not in repo (cwd load only)
2. YOU ARE planner still mixed `FIRST_NAMES` (no live `isFemale` flag on `plan_do_naming_you_are`)
3. `pick_random_name` still mixed first names
