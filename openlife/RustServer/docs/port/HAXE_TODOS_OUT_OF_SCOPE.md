# Haxe TODOs — out of scope for the leftover picker

These are **`TODO` / `FIXME` comments in Haxe** for work Haxe itself never shipped.  
The leftover **picker must not pick them**. Port **working** Haxe only (functions that run today).

When a live Haxe chunk is ported and it contains one of these comments: port-as-is and leave the TODO, or note the choice in the changelog. Do **not** invent the feature.

Canonical comment dump: [`HAXE_OPEN_TODOS.md`](HAXE_OPEN_TODOS.md).

Also parked: PHOTO, VOG, multi-server twins, SQL, mutex/debug/secret, unused ModuleConst / SETTINGS-LONG-TAIL.

---

## Not picker (Haxe never implemented)

| Id | Haxe note |
|----|-----------|
| KNOCKOUT-PICKUP | GPI ~4956 `TODO allow pickup of knocked out players` |
| HIDDEN-CONTAINERS | TH ~130 / ~1408 `TODO implement hidden containers` |
| ROAD-QUAD-CLIENT (comment) | MoveHelper ~596 `TODO fix road movement` (live `/10` **is** ported: `jump_quad_with_floor`) |
| DEEP-RIVER-BLOCK | Biome ~19 `TODO deep river which is not walkable` |
| PASSABLE-OCEAN-COLOR | Biome ~35 `TODO use also for passable ocean` |
| SLOW-SERVER-SKIP | TimeHelper ~85 `TODO what to do if server is too slow` (skip_ticks catch-up **is** live) |
| MAP-TIME-PARTS-AUTO | Settings ~338 `TODO better auto calculate` WorldTimeParts |
| FOLLOW-RESEND-TICK (comment) | TimeHelper ~265 `TODO check why following is not updated` (the **call** is live DisplayStuff) |
| TRUST-CALC extras | TimeHelper ~536–539 extract fn / manual leader / blessed graves / protect nobles |
| WINTER-REGROW-PATH (comment) | TimeHelper ~1259 warned winter never runs — **incorrect for current objects** (L1120). **SEASON-REGROW-DECAY** ports the live winter/spring plant path. Hidden/original extra-only TODOs stay out of scope. |
| DECAY-NEAR-PLAYER | TimeHelper ~1479 `TODO dont decay if some on is close` |
| DECAY-CONTAINER-USES | TimeHelper ~1480–1483 containers / multi-use / trash pit 618 |
| COLORED-WALL-DECAY | TimeHelper ~1619 / ~1752 |
| FLOOR-DECAY-INSIDE | TimeHelper ~1689 |
| CUSTOM-OBJ-DECAY-TO | TimeHelper ~1482 |
| STONE-PILE-RESPAWN | TimeHelper ~1933 |
| GOOSE-POND-EGG | `TODO let egg come back` |
| ANIMAL-FLEEING | TimeHelper ~2268 `TODO fleeing` |
| ANIMAL-OFFSPRING-CHILD | TimeHelper ~2269 |
| EAT-MEAT-KILL-SHEEP | TimeHelper ~2270 |
| DOMESTIC-MULTIPLY-GREEN | TimeHelper ~2467 |
| DOMESTIC-EAT-GREEN | TimeHelper ~2468 |
| HORSE-WAGON-DIE-CARGO | TimeHelper ~2488 |
| ANIMAL-BLOODY-KNIFE | TimeHelper ~2643 |
| MOVE-NONEMPTY-GROUND | TimeHelper ~2768 |
| RABBIT-TREE-WALK | TimeHelper ~2739 |
| SAME-ACCOUNT-CHEST | TH ~287 coins without key |
| INTENTIONAL-USE-INDEX | TH ~736 |
| EMPTY-COLD-BOWL extra | TH ~1565 `TODO should ignore EMPTY + Cold Bowl` |
| TOOL-REDUCE | TH ~1268 |
| USE-PILES | TH ~1294 |
| WILD-GARLIC-CARROT | TH ~1668 |
| HIRE-FOLLOWER-PRICE extra | GPI ~2373 `TODO consider follower count` (hired-count cost **is** live) |
| SUB-LEADER-REDEEM | GPI ~2176 |
| OTHER-LEADER-FOLLOW | GPI ~2245 / ~2399 |
| HIRE-OWN-LEADER | GPI ~2400 |
| PROPERTY-POINTER | GPI ~2550 / ~4096 |
| INHERIT-ALLY-CLOSE | GPI ~4031 |
| FOLLOWER-COLOR-RESEND | GPI ~2205 |
| RANDOM-WOUND-CHANCE | GPI ~4719 |
| ANGER-TIMEHELPER | GPI ~4893 |
| SWITCH-CMD | GPI ~5279 commented `!SWITCH` needs client |
| OVEN-REGISTRY-CLEAR | GPI ~5519 |
| EVE-NOT-TOO-FAR | GPI ~977 |
| CLASS-FAMILY-PRESTIGE | GPI ~1066 |
| SMALL-FAMILY-FERTILITY | GPI ~1275 |
| PAST-FAMILIES-SPAWN | GPI ~1277 |
| GLOBAL-SPAWN | GPI ~1173 |
| DISEASE-REDPOINTS | GPI dehydration / spicyFood / love fields |
| NAME-SAVE-UNUSED | NamingHelper ~306 |
| FAMILY-NAME-DIST-PRESTIGE | Lineage ~629 |
| SCORE-FATHER-SIBLINGS | ScoreEntry ~63 |
| LINEAGE-ALIVE-LIVE | Lineage ~96 |
| AI-LOWER-SCORE | PlayerAccount ~242 |
| SOUL-FROM-PLAYER-ONLY | PlayerSoul ~209 |
| WORLD-WRAP-COUNT | WorldMap ~1615 |
| HIDDEN-OBJECT-HELPER | WorldMap ~416 |
| TEMP-VECTOR-SAVE | WorldMap ~992 |
| STARTING-POS-CLEAR | Server.hx ~130 |
| BIOME-EAT-XP | Biome ~127 `TODO not used yet` |
| BIRTH-DESIGN-CURSES | GPI ~6834 design notes |

---

## Also not picker

- Mutex / thread / socket / `TODO test` / `TODO AI` one-liners  
- Needle/thread known Haxe bugs (document / port-as-is already)  
- `OL-AI-SPLIT-P3` (Rust crate split, not a Haxe feature)  
- Container-aware AI beyond what Haxe already does (`AI-REMOVE-CONTAINER` is the live subset)
