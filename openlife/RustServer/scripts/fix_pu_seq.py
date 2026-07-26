from pathlib import Path
import re

path = Path("crates/ol-sim/src/lib.rs")
text = path.read_text(encoding="utf-8")


def fix_block(block: str) -> str:
    # Already patched?
    if re.search(r"spd,\s*\n\s*.*done_moving_seq", block):
        return block
    if re.search(r"spd,\s*\n\s*[^\n]+,\s*\n\s*\);", block):
        return block

    seq = "1"
    if re.search(r"\bap\.x\b", block):
        seq = "ap.done_moving_seq.max(1)"
    elif re.search(r"\btp\.x\b", block):
        seq = "tp.done_moving_seq.max(1)"
    elif re.search(r"\bfeeder\.x\b", block):
        seq = "feeder.done_moving_seq.max(1)"
    elif re.search(r"\bnp\.x\b", block):
        seq = "np.done_moving_seq.max(1)"
    elif re.search(r"\bp\.x\b|\bp\.age\b|\bpx,|\brx,", block):
        seq = "p.done_moving_seq.max(1)"

    def repl(m: re.Match) -> str:
        indent = m.group(1)
        return f"spd,\n{indent}{seq},\n{indent});"

    return re.sub(r"spd,\n(\s*)\);", repl, block, count=1)


def fix_eat(block: str) -> str:
    if "done_moving_seq" in block:
        return block

    def repl(m: re.Match) -> str:
        indent = m.group(1)
        return f"p.yum.just_ate_id,\n{indent}p.done_moving_seq.max(1),\n{indent});"

    return re.sub(r"p\.yum\.just_ate_id,\n(\s*)\);", repl, block, count=1)


pat = re.compile(r"format_player_update_line\(\n(?:.*?\n)*?.*?\);", re.M)
pat_eat = re.compile(r"format_player_update_line_eat\(\n(?:.*?\n)*?.*?\);", re.M)

new_text, n1 = pat.subn(lambda m: fix_block(m.group(0)), text)
new_text, n2 = pat_eat.subn(lambda m: fix_eat(m.group(0)), new_text)
path.write_text(new_text, encoding="utf-8")
print(f"updated format_player_update_line={n1}, eat={n2}")
