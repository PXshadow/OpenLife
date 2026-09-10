# AI life stats

Live process writes `SaveFiles/ai_life_stats.md` on each NPC activity flush (about every 30 seconds). The Ops dashboard shows the same text at the bottom (`/ops`, from `/api/npc/stats`).

## What is tracked

- **Food eaten** — held object id when an NPC eat / seek-food step runs
- **Deaths** — player id, age, and reason string
- **Objects / actions created** — first token of each craft detail (USE / walk / profession action)

## Birth (Haxe parity)

- Eve/Adam pairing is **same-kind only** (human Eve↔human Adam, AI Eve↔AI Adam)
- **Children** may be born to the other kind, with Haxe mali:
  - AI mother + human child: `AiMotherBirthMaliForHumanChild` (3)
  - Human mother + AI child: `HumanMotherBirthMaliForAiChild` (1)
- Mother/father fitness is the Haxe additive formula (food/10, class boni, graves, exhaustion, parent-child little-kid factor, father score / 2)

## AI timing (Haxe `AiBase.doTimeStuff`)

- Wait `AiReactionTime` (0.5s Commoner / 0.7 Serf / 0.2 Noble; ×0.2 if angry)
- If moving and not just finished a tile, skip replanning
- Targeted tiles are reserved on `blockedByAI` (~5s) so other AIs skip them
- Sticky move checks the expected parent still matches before USE
- World scan is reused for the same think; a busy world lock does **not** block — last snapshot is reused and the AI re-checks the target

## Iteration log

Filled after live 30-second checks. See the dashboard footer after the server has been running.
