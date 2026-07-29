DOC SNIP — applied by _apply_th_alt_outcome.py

### TODO_PORT insert after TH-HORSE block:
- [x] **TH-ALT-OUTCOME alt_transition_outcome** — pure `evaluate_alternative_outcome` + ContentDb `alt_outcomes_*` / fortification tables + live `apply_use_at` TryAgain/Proceed; tests `alt_outcome::*` + `use_transition::alt_outcome_*`; residual LiveSettings / PropertyGate bulk / coinCost

### FILE_MATRIX S-TH residual:
**TH-ALT-OUTCOME** DONE (core): alt outcomes + fort drop pure+live

### CALL_INDEX:
| `evaluate_alternative_outcome` / `alt_outcome_gate_applies` | `ol-sim/src/alt_outcome.rs` | TH-ALT-OUTCOME pure L1260–1306 |
| `apply_default_alternative_outcome_patches` | `ol-content` | ServerSettings alt/fort tables |
| `ContentDb::alternative_outcomes_for` | `ol-content` | transition list > new-target object list |
| `apply_use_at` alt TryAgain/Proceed | `use_transition.rs` | live hits stamp + place_object_by_id + keep/transform |
