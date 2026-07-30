# Split AI out of `ol-sim` (design)

**Status:** **Phase A–C done** (2026-07-30) —  
`PHASE_A_PLAYER_INTERFACES.md`, `PHASE_B_PLAYER_HELPER.md`, `PHASE_C_AI_HELPER_CRAFTING.md`.  

- **`ol-ai-api`**: write + read interfaces (best-food r=30)  
- **`ol-player-helper`**: shared pure food/geom/eat gates  
- **`ol-ai-crafting`**: craft graph/plan/value  
- **`ol-ai-helper`**: goals, ladder, path-reach, profession pure SMs  
- **`ol-ai`**: façade re-exports (stable import path)  
- `ol-sim` adapters + NPC write/food via interfaces  

**Goal:** faster incremental builds + same write path as human clients + fast AI read path

---

## 1. Target crate graph

```
ol-server
  ├─ ol-net          NetIntent (commands) + OutboundHub
  ├─ ol-sim          SimState, apply_intent, ticks, world mutation (no profession AI brains)
  ├─ ol-ai           NEW — pure decisions + NPC think using *interfaces only*
  ├─ ol-world / ol-content / …
  └─ ol-web
```

| Crate | Owns |
|-------|------|
| **ol-net** | `NetIntent` — **the** command interface humans and AI share |
| **ol-sim** | `apply_intent`, vitals, combat, transitions; implements **adapters** for AI reads |
| **ol-ai** | Profession SMs, craft plans, food seek policy, path-reach *policy* (marks still applied via intents/sim) |
| **ol-server** | Threads: TCP → intents; sim tick; NPC scheduler calls `ol_ai` then `intent_tx.send(NetIntent)` |

**Hard rule:** AI never mutates `SimState` / `World` directly. It only:

1. **Reads** via traits (`WorldView`, `PlayerView`, `FoodSearch`)  
2. **Acts** via `NetIntent` on the same channel as clients  

That matches Haxe’s “AI is a player” idea without sharing the mutex soup.

---

## 2. Shared command interface (player commands)

Already exists: `ol_net::NetIntent`

```text
Login | KeepAlive | Use | Drop | Move | Raw | Disconnected
```

AI maps decisions → the same variants:

| AI want | NetIntent |
|---------|-----------|
| Walk | `Move { conn_id, xs, ys, deltas, seq }` |
| Use tile / held | `Use { conn_id, x, y, id, index }` |
| Drop | `Drop { … }` |
| SAY / JUMP / etc. | `Raw { tag, payload }` |

Optional later: thin helpers in `ol-ai` or `ol-net`:

```rust
// Conceptual — same intents under the hood
trait PlayerCommands {
    fn use_at(&self, conn: u64, x: i32, y: i32, id: Option<i32>, index: Option<i32>);
    fn drop_at(&self, conn: u64, x: i32, y: i32, clothing_slot: Option<i32>);
    fn move_path(&self, conn: u64, xs: i32, ys: i32, deltas: &[(i32, i32)], seq: Option<i32>);
    fn say_raw(&self, conn: u64, tag: &str, payload: &str);
}
```

Implemented by `IntentTx` / a small `IntentPlayerCommands` wrapper in ol-server.

---

## 3. Read interfaces (AI → world)

Defined in **`ol-ai`** (or a tiny `ol-sim-api` if we want zero AI→sim type coupling later).  
**Implemented** in **`ol-sim`** (or ol-server with snapshot locks) so AI crate does not depend on full `SimState`.

### 3.1 Fast world read

```rust
/// Cheap, read-only map/player queries for AI think ticks.
pub trait WorldView {
    fn width_height(&self) -> (i32, i32);
    fn wrap(&self) -> bool;
    /// Object id at tile (0 = empty). Fast path: resident chunk only.
    fn object_at(&self, x: i32, y: i32) -> i32;
    fn biome_at(&self, x: i32, y: i32) -> u8;
    fn floor_at(&self, x: i32, y: i32) -> i32;
    /// Scan axis-aligned box (inclusive), max tiles capped for AI budgets.
    fn for_each_object_in_rect(
        &self,
        x0: i32, y0: i32, x1: i32, y1: i32,
        f: &mut dyn FnMut(i32, i32, i32), // x, y, object_id
    );
}
```

**Speed notes:**

- Prefer **snapshot / shared read** (`Arc<World>` or chunk map under `RwLock` read) so NPC thread doesn’t block sim writer long  
- Or **publish a compact AI map slice** every N ticks (coarser, even faster)  
- Chebyshev radius scans stay O(r²); r=30 → ~3.6k tiles — fine if `object_at` is O(1)

### 3.2 Player view (for “this body”)

```rust
pub trait PlayerView {
    fn p_id(&self) -> i32;
    fn conn_id(&self) -> u64;
    fn pos(&self) -> (i32, i32);
    fn age(&self) -> f32;
    fn food(&self) -> (f32, f32); // store, max
    fn held_id(&self) -> i32;
    fn home(&self) -> (i32, i32);
    fn clothing(&self) -> [i32; 6];
    // path-reach / sticky craft as opaque handles or small DTOs
}
```

### 3.3 Best food (default 30 tiles)

```rust
pub struct BestFoodQuery {
    pub conn_id: u64,
    /// Chebyshev radius; default **30** (Haxe-ish close search band).
    pub max_dist: i32,
}

pub struct BestFoodHit {
    pub x: i32,
    pub y: i32,
    pub food_id: i32,
    pub score: f32,
    pub is_yum: bool,
}

pub trait FoodSearch {
    /// Best edible for this player within max_dist (default 30).
    fn best_food(&self, q: BestFoodQuery) -> Option<BestFoodHit>;
}

impl Default for BestFoodQuery {
    fn default() -> Self {
        Self { conn_id: 0, max_dist: 30 }
    }
}
```

**Implementation:** wrap existing pure + live helpers:

- `search_best_food` / `search_best_food_full` / `search_best_food_nearby` in ol-sim  
- Live adapter passes content, yum state, world food factors, not-reachable maps  

AI only calls `food.best_food(BestFoodQuery { conn_id, max_dist: 30 })` — no inlined map scan in profession code.

---

## 4. Move plan (phased)

### Phase 0 — freeze (while port workflows run)
- No large moves; finish TH-ALT / CURSED-GRAVE-TELEPORT etc.

### Phase 1 — traits + adapters (no file move)
- Add `crates/ol-ai` with traits + DTOs only  
- Implement `WorldView` / `PlayerView` / `FoodSearch` in ol-sim (or ol-server glue)  
- `npc_ai` uses `FoodSearch` for seek-food (default 30)  
- Commands stay `NetIntent` only  

### Phase 2 — move pure AI modules into `ol-ai`
Move first (low sim coupling):

- `ai_goals`, profession pure SMs (`*_profession.rs` pure parts)
- `craft_item` / `get_or_craft` pure graphs  
- `search_best_food` pure scoring (keep live wire in ol-sim adapter)

Keep in ol-sim:

- Live path-reach on `Player`, sticky fields, `apply_intent` side effects  

### Phase 3 — `npc_ai` → ol-ai (+ thin ol-server runner)
- Decision returns `Vec<NetIntent>` or sends via `PlayerCommands`  
- ol-server only: schedule, connect `IntentTx`, hold adapters  

### Phase 4 — optional `ol-sim-api` crate
If `ol-ai` still needs too many ol-sim types, extract DTOs only so **ol-ai does not depend on ol-sim** (ol-sim depends on ol-ai for nothing; ol-server depends on both and implements traits).

Ideal final deps:

```text
ol-ai  →  ol-net (NetIntent), ol-content? (optional), no ol-sim
ol-sim →  (implements traits, may depend on ol-ai only if sim embeds pure helpers — prefer not)
ol-server → ol-sim + ol-ai + ol-net
```

---

## 5. Compile model (why this helps)

| Before | After Phase 2+ |
|--------|----------------|
| Touch AI pure code → recompile huge `ol-sim` | Touch AI pure code → recompile **`ol-ai` only** (+ link server) |
| Touch combat → recompile AI together | Touch combat → **`ol-sim` only** if AI not inlined |

Human clients and AI share **one** intent pipeline → fewer “AI-only” code paths that diverge.

---

## 6. Non-goals (first pass)

- Multi-server AI  
- Full LLM inside ol-ai (keep `ai_handler` / provider on server side initially)  
- Rewriting all professions in one PR — pure modules move first  

---

## 7. Success criteria

- [ ] `ol-ai` crate builds with traits + at least food search + one profession pure path  
- [ ] NPC food seek uses `FoodSearch` default radius **30**  
- [ ] All NPC physical actions go through `NetIntent`  
- [ ] Changing a pure profession file does **not** force full `ol-sim` rustc of lib.rs if modules moved  
- [ ] Docs: ARCHITECTURE_RUST + BUILD_SPEED updated  
