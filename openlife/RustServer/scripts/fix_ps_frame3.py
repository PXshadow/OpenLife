from pathlib import Path
import re

p = Path("crates/ol-sim/src/lib.rs")
t = p.read_text(encoding="utf-8")

pat = re.compile(
    r"send_nearby\(\s*\n\s*outbound,\s*\n\s*&near,\s*\n\s*format_server_message\(\"PS\",\s*&\[&line\]\)\.into_bytes\(\),\s*\n\s*\);",
    re.M,
)
t, n1 = pat.subn("send_nearby_ps_lines(outbound, &near, &line);", t)

# RATE (various indent)
t, n2 = re.subn(
    r"outbound\.send\(\s*\n\s*conn_id,\s*\n\s*format_server_message\(\"PS\",\s*&\[\"RATE\"\]\)\.into_bytes\(\),\s*\n\s*\);",
    'send_ps_reply(outbound, conn_id, "RATE");',
    t,
)

# EMOTE RATE as argument to send(
t = t.replace(
    'format_server_message("PS", &["EMOTE RATE"]).into_bytes()',
    'format_player_says(0, false, "EMOTE RATE").into_bytes()',
)

# shutdown nearby
old = """send_nearby(
                outbound,
                &near,
                format_server_message("PS", &[&format!("{} {}", p.p_id, msg)]).into_bytes(),
            );"""
new = """{
                let _ps = format!("{} {}", p.p_id, msg);
                send_nearby_ps_lines(outbound, &near, &_ps);
            }"""
n3 = t.count(old)
t = t.replace(old, new)

# any remaining send_nearby with PS &line single-line
t, n4 = re.subn(
    r'send_nearby\(\s*outbound,\s*&near,\s*format_server_message\("PS",\s*&\[&line\]\)\.into_bytes\(\)\s*\);',
    "send_nearby_ps_lines(outbound, &near, &line);",
    t,
)

p.write_text(t, encoding="utf-8")
rem = len(re.findall(r'format_server_message\("PS"', t))
print(f"n1={n1} n2={n2} n3={n3} n4={n4} remaining={rem}")
for m in re.finditer(r".{0,30}format_server_message\(\"PS\".{0,70}", t):
    s = m.group(0).replace("\n", " ")
    # skip helpers inside send_ps_reply
    print(s[:120])
