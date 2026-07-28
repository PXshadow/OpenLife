# Client dependency graphs

**Narrative overview:** [README.md](README.md) · **modules:** [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md)  
Mermaid only — no status tables here.

---

## 1. C++ client runtime

```mermaid
flowchart TB
  GAME[game.cpp] --> LOAD[LoadingPage]
  GAME --> ACC[ExistingAccountPage]
  GAME --> LIVE[LivingLifePage]
  LOAD --> SB[spriteBank]
  LOAD --> OB[objectBank]
  LOAD --> AB[animationBank]
  LOAD --> TB[transitionBank]
  LOAD --> SND[soundBank]
  LOAD --> FC[folderCache / binFolderCache]
  LIVE --> NET[TCP protocol.txt]
  LIVE --> LO[LiveObject table]
  LIVE --> MAP[map chunks MC/MX]
  LIVE --> PF[pathFind]
  LIVE --> DRAW[draw ground/objects/HUD]
  LIVE --> IN[input clicks/keys]
  IN --> PF
  IN --> NET
  NET --> LO
  NET --> MAP
  OB --> DRAW
  AB --> DRAW
  SB --> DRAW
```

---

## 2. Haxe client (assets focus)

```mermaid
flowchart LR
  DATA[OneLifeData7] --> RES[Resource.hx]
  RES --> BAKE[ObjectBake]
  RES --> TGA[TgaData]
  BAKE --> OD[ObjectData]
  TGA --> PACK[BinPack]
  PACK --> BATCH[SpriteBatch]
  OD --> RENDER[Render.hx]
  BATCH --> RENDER
  CLIENT[Client.hx] --> TAGS[ClientTag]
  CLIENT --> GAME[Game.hx]
  GAME --> RENDER
```

---

## 3. Target Rust client

```mermaid
flowchart TB
  CLI[ohol-headless / ohol-client] --> NET[ol-client-net]
  CLI --> WORLD[ol-client-world]
  NET --> PROTO[ol-client-proto]
  WORLD --> CONTENT[ol-client-content]
  CONTENT --> CACHE[OLC1/OLT1/OLS1 cache]
  CONTENT --> RAW[OneLifeData7 text fallback]
  WORLD --> LIVE[LiveObject + ChunkMap]
  NET --> LIVE
  CLI --> INPUT[scripts / input]
  INPUT --> NET
  CLI -.->|feature gpu| GPU[ol-client-render]
  LIVE -.-> GPU
  CONTENT -.-> GPU
```

---

## 4. Shared with Rust server

```mermaid
flowchart TB
  SRC[OneLifeData7 text] --> BAKE[bake-content]
  BAKE --> OLC[olc1_objects.bin]
  BAKE --> OLT[olt1_transitions.bin]
  OLC --> SRV[ol-server / ol-content]
  OLT --> SRV
  OLC --> CL[RustClient content]
  OLT --> CL
  PROTO[protocol tags] --> SRV
  PROTO --> CL
```

---

## 5. Headless test flow

```mermaid
sequenceDiagram
  participant T as test/CLI
  participant S as Session
  participant W as LiveWorld
  participant G as game server
  T->>S: connect_and_login
  S->>G: TCP SN/LOGIN
  G->>S: ACCEPTED + PU/MC/...
  S->>W: apply events
  T->>S: MOVE/USE/...
  S->>G: commands
  G->>S: PM/PU/MX
  S->>W: apply
  T->>W: assert state
```

---

## 6. Port data flow (chunk work)

```mermaid
flowchart LR
  CPP[C++ gameSource] --> AUDIT[audit chunk]
  HX[Haxe client/resources] --> AUDIT
  AUDIT --> GAPS[gap list]
  GAPS --> RUST[Rust modules]
  RUST --> TEST[unit + headless]
  TEST --> DOCS[FILE_MATRIX + TODO]
```
