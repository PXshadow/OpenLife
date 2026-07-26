"""Rewrite private PS sends to use send_ps_reply (p_id/0 + FM)."""
from pathlib import Path
import re

path = Path("crates/ol-sim/src/lib.rs")
text = path.read_text(encoding="utf-8")

# outbound.send(conn_id, format_server_message("PS", &[&line]).into_bytes());
# outbound.send(conn_id, format_server_message("PS", &["RATE"]).into_bytes());
# outbound.send(conn_id, format_server_message("PS", &[&something]).into_bytes());

patterns = [
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&reply\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &reply);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&msg\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &msg);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&s\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &s);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&body\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &body);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&text\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &text);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&out\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &out);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&note\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &note);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&result\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &result);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&chat\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &chat);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&message\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &message);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&payload\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &payload);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&status\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &status);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&info\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &info);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&hint\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &hint);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&err\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &err);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&resp\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &resp);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&answer\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &answer);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&query\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &query);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&report\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &report);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&fmt\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &fmt);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&out_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &out_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&say_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &say_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&wire\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &wire);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps_text\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps_text);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&chat_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &chat_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&response\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &response);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&msg_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &msg_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&say\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &say);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&txt\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &txt);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&str_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &str_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&content\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &content);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&full\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &full);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&private_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &private_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&whisper\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &whisper);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&wline\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &wline);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&help\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &help);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&help_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &help_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&rate_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &rate_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&r\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &r);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&l\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &l);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps_msg\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps_msg);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps_body\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps_body);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&notify\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &notify);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&nline\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &nline);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ok\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ok);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ok_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ok_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&fail\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &fail);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&fail_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &fail_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&warn\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &warn);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&warning\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &warning);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&tip\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &tip);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&tip_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &tip_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&order\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &order);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&order_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &order_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&leader\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &leader);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&leader_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &leader_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&exile\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &exile);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&exile_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &exile_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&follow\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &follow);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&follow_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &follow_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&die_line\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &die_line);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&death\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &death);",
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&ps_line_str\]\)\.into_bytes\(\)\s*\);',
        "send_ps_reply(outbound, conn_id, &ps_line_str);",
    ),
    # string literals
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&?"RATE"\]\)\.into_bytes\(\)\s*\);',
        'send_ps_reply(outbound, conn_id, "RATE");',
    ),
    (
        r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&?"EMOTE RATE"\]\)\.into_bytes\(\)\s*\);',
        'send_ps_reply(outbound, conn_id, "EMOTE RATE");',
    ),
]

n_total = 0
for pat, repl in patterns:
    text, n = re.subn(pat, repl, text)
    n_total += n

# Generic catch-all for remaining format_server_message("PS", &[&IDENT])
pat_generic = re.compile(
    r'outbound\.send\(\s*conn_id,\s*format_server_message\("PS",\s*&\[&([A-Za-z_][A-Za-z0-9_]*)\]\)\.into_bytes\(\)\s*\);'
)
text, n = pat_generic.subn(r"send_ps_reply(outbound, conn_id, &\1);", text)
n_total += n

path.write_text(text, encoding="utf-8")
print(f"replaced {n_total} PS send sites")
# remaining
rem = len(re.findall(r'format_server_message\("PS"', text))
print(f"remaining format_server_message(PS): {rem}")
