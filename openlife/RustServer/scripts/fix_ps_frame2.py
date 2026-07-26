"""Catch remaining PS format_server_message sends (incl. multiline)."""
from pathlib import Path
import re

path = Path("crates/ol-sim/src/lib.rs")
text = path.read_text(encoding="utf-8")

# Multiline: outbound.send(\n  conn_id,\n  format_server_message("PS", &[&foo]).into_bytes(),\n);
pat = re.compile(
    r"outbound\.send\(\s*\n\s*conn_id,\s*\n\s*format_server_message\(\"PS\",\s*&\[&([A-Za-z_][A-Za-z0-9_]*)\]\)\.into_bytes\(\),\s*\n\s*\);",
    re.M,
)
text, n1 = pat.subn(r"send_ps_reply(outbound, conn_id, &\1);", text)

# Single line remaining
pat2 = re.compile(
    r"outbound\.send\(\s*conn_id,\s*format_server_message\(\"PS\",\s*&\[&([A-Za-z_][A-Za-z0-9_]*)\]\)\.into_bytes\(\)\s*\);"
)
text, n2 = pat2.subn(r"send_ps_reply(outbound, conn_id, &\1);", text)

# format_server_message("PS", &[&format!(...)])  hard — leave for now
# outbound.send(conn_id, format_server_message("PS", &[&format!(...)]).into_bytes());
pat3 = re.compile(
    r"outbound\.send\(\s*conn_id,\s*format_server_message\(\"PS\",\s*&\[&format!\(([^)]*)\)\]\)\.into_bytes\(\)\s*\);"
)
# too hard

# Literals
for lit in ["RATE", "EMOTE RATE", "MUTE OK", "UNMUTE OK", "OK", "FAIL", "NO", "YES"]:
    old = f'outbound.send(conn_id, format_server_message("PS", &["{lit}"]).into_bytes());'
    new = f'send_ps_reply(outbound, conn_id, "{lit}");'
    c = text.count(old)
    text = text.replace(old, new)
    n2 += c

path.write_text(text, encoding="utf-8")
rem = len(re.findall(r'format_server_message\("PS"', text))
print(f"n1={n1} n2={n2} remaining={rem}")
# show a few remaining lines
for m in re.finditer(r".{0,40}format_server_message\(\"PS\".{0,80}", text):
    print(m.group(0)[:120])
    if sum(1 for _ in re.finditer(r'format_server_message\("PS"', text[: m.end()])) > 15:
        break
