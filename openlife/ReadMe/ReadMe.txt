This will be the new manual for Open Life Reborn

---

**How to protect against attackers**
Updated: 2026/03/23

1. Exile them --> your followers will attack them
- If they attack they should be auto exiled (Needs testing)
- attacking them should auto exile them, but be careful that you are not seen as the agressor (Needs testing)
TODO: block doors / gates for the exiled

2. Tell leader --> Go close to your leader, if the attacker is not allied or if the leader trusts you more (double) than the attcker the leader will automatically exile the attacker

Important for auto exile is if your leader trusts you more see: Impact on Trust

- The more prestige you have the more likely you are born as a higher class and the more followers you have for defence
- Mark graves from attackers and make nice graves from people you like / protect the graves TODO: Marked graves are not tested yet

Hints:
- The more kids the better they eat / cloths / grandkids the more prestige and the more people that defend you
- If you are born a low class or have less prestige, ask one with better class prestige to exile and let him inform your leader

*Impact on Trust*:
- if allied
- your / their prestige: Base of the trust 
- your / their relative class (SERF, COMMONER +20%, NOBLE +50%)
- your combat reputation (good if attacking peple with bad combat reputation, bad if attacking innocents)
- if cursed: 100% Boni / Mali
- your family: 100% Boni / Mali
- similar color like leader  50% Boni / Mali 

TODO: consider their family reputation
TODO: consider their family relationship
TODO: consider if blessed (Your old graves nearby) 

TODO: 
*Allow only trusted new followers*
- TODO: consider their trust
- TODO?: consider how long they stayed close to their new leader they want to follow

*Protect Graves*
- TODO: count down to 5 sec before allowing to interact with protect graves
- TODO: Let players / NPCs get angry
- TODO: Let players / NPCs attack if on 2 sec countdown
- TODO: Change reputation if destroying graves
- TODO: Add exceptions for main leader except for eve graves
- TODO: Add exceptions for graves from players that have a lot of similar graves close by
- TODO: Add exceptions for nobles removing lower rank graves

*Add blessed*
- TODO: give some smal protection 10% if your old grave is nearby
- TODO: give some prestige boni 20% from followers if your old grave is nearby

*Add Protective Dogs*

*Protect Walls / Doors / Buildings / Wells*
- TODO: Let NPCs defend Buildings
- TODO: Add similar to combat cool down for doors / walls
- TODO: Allow to more easy remove walls / doors if old grave nearby
- TODO: implement ownership for doors. Check if close allied is close by give ownership to closeby ally if original owner is >50 tiles

*Protect Horses / Sheeps / Domestic Animals*

*Protect against bringing dangerous animals*

---

**Property System**
- cursed cant open / close doors
- TODO: dont allow cursed to bring down walls?
- TODO: dont allow not allied to pass fences / springy doors
- TODO: Owner for normal doors / gates: Check once in a while if current owner or ally of owner is too far away.
    If so choose a new close owner if there is while prefering allies to original owner 

---

**Prestige Class**
- TODO: allow to level up a class if prestige for noble, enough followers and too few nobles in your family (world)
- TODO: level down a class if attacking innocents and too less prestige left for being noble

---

**Kids**
- TODO: dont allow containers

---

**NPCS**
- TODO: allow to take food from containers (not on bear skin floor)
- TODO: allow to store food in containers (not on bear skin floor)
- TODO: allow to take tools from containers (not on bear skin floor)
- TODO: allow to store tools in containers (not on bear skin floor)

- TODO: Fix: I am command not working

- TODO: On disconnect wait little bit especially if on horse before taking over
- TODO: If on horse place on fence if nearby  

- TODO: Finish Omni crafter AI

---

**Living World**
- TODO: Biome can change according to temperature
- TODO: implemented water map
- TODO: Biome can change according to water

---

**Custom Client**
FIrst is to make a simple Fork that can connect by default to right server. Then we can add custom data.
- TODO: simple client with default Ip set
- TODO: add a proper biome for Mountains
- TODO: Display coins
- TODO: display score in the end plus link to score on Website once implemented

- TODO: Advanced: Change protocoll to alllow players in containers like a horse cart or backpack

**Custom Data*
All custom data should look Jason style better save than sorry
- TODO: Boat, Blood
- TODO: more cloths
- TODO: Decay items
- TODO: more people skins

---

**Stats**
**Player stats**
- TODO: Display stats in one line each
- TODO: Show current needed prestige for prestige classes
- TODO: Show born with prestige and account prestige for Animals
- TODO: Display Dynasty family name if there is
- TODO: Display number in their family
- TODO: Display number of allies
- TODO: Display from where they gained prestige (per player and per account)

**World Stats**
- TODO: Display number of items and original items (over time with graph)

---

**Animals** 
- TODO: make animals only give damage if they move over you
- TODO: Add animal interactions

---

**Combat / Damage**
- TODO: Allow only to loose max halve health per hit

*Movement*
- TODO: fix too slow movement Display (display jumps)
- TODO: fix that player is displayed at wrong position (standing and interacting) where he is not
- TODO: only send player messages when close (might encounter bug like naming not working if not seen while naming)

**Not implemented**
- TODO: container locked stuff like no water bowl in backpack
- TODO: cloth interactions while wearing (for example piling sheep skin)

*General*
- NPCS: Dont use container on bear skin floor
- NPCS: Dont use closed or locked chests
- NPCS: Drop in container. Add list which item in which container. Prefer containers 

- TODO: fix teleport to human
- TODO: make yum Display as not default setting
- TODO: fix !KILLOBJ

- TODO: Change grave teleport to use account coins if too few coins
- TODO: allow to teleport to different graves like with !TV
- TODO: reduce teleport cost to one coin if close to own grave
- TODO: Add teleport cool down for 5 sec 

- TODO: increase curse radius for better cursed graves

