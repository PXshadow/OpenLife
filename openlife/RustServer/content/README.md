# Content (external)

This directory is a **placeholder**. Do not commit the full `OneLifeData7` tree
into git (it is huge and already lives with the Haxe project).

## Recommended layout (local only)

```
content/
  OneLifeData7/     → copy or junction/symlink from C:\OhOl\OpenLife\OneLifeData7
  world_pack/       → baked chunk packs (generated, gitignored)
```

### Windows junction example

```powershell
cmd /c mklink /J "C:\OhOl\OpenLifeReborn\content\OneLifeData7" "C:\OhOl\OpenLife\OneLifeData7"
```

Server config will later point at `content/OneLifeData7` (or an absolute path).
