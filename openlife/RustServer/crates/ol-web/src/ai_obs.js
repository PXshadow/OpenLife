(function () {
  "use strict";

  var STORAGE_KEY = "openlife-ops-dashboard-v2";
  var GRAPH_TOP = 10;

  var TIME_RANGES = {
    all: 0,
    "30s": 30 * 1000,
    "1m": 60 * 1000,
    "5m": 5 * 60 * 1000,
    "15m": 15 * 60 * 1000,
    "1h": 60 * 60 * 1000,
    "1d": 24 * 60 * 60 * 1000,
    "1w": 7 * 24 * 60 * 60 * 1000,
    "30d": 30 * 24 * 60 * 60 * 1000
  };

  var AGE_LABELS = [
    "0-5",
    "5-10",
    "10-15",
    "15-20",
    "20-25",
    "25-30",
    "30-40",
    "40-50",
    "50-60",
    "60+"
  ];

  var AI_TIMING_CHARTS = [
    { id: "aiThink", title: "AI Think Time", field: "ai_think_ema_us", kind: "duration" },
    { id: "aiCpu", title: "AI CPU Total", field: "ai_cpu_us", kind: "duration" },
    { id: "aiThinks", title: "AI Thinks", field: "ai_thinks", kind: "count" },
    { id: "aiIntent", title: "AI Intent Time", field: "ai_intent_avg_us", kind: "duration" },
    { id: "aiScan", title: "AI Scan Time", field: "ai_scan_ema_us", kind: "duration" },
    { id: "aiOther", title: "AI Other Time", field: "ai_other_ema_us", kind: "duration" },
    { id: "aiVsSim", title: "AI vs Sim", field: "ai_vs_sim_pct", kind: "percent" }
  ];

  var SPEND_LABELS = {
    walk_target: "Walking to target",
    walk_use: "Walking to use item",
    walk_baby: "Walking to baby",
    walk_drop: "Walking to drop",
    walk_food: "Walking to food",
    walk_follow: "Following",
    walk_home: "Walking home",
    escape_animal: "Escaping animals",
    escape_player: "Escaping hostile players",
    attack_player: "Attacking hostile players",
    eat: "Eating",
    craft_use: "Using / crafting",
    nurse: "Nursing",
    pickup_baby: "Picking up baby",
    drop_baby: "Dropping baby",
    name_baby: "Naming baby",
    wait_baby: "Baby waiting on mother",
    explore: "Exploring",
    stuck: "Stuck",
    think: "Thinking / other"
  };

  function loadSettings() {
    try {
      return JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}") || {};
    } catch (e) {
      return {};
    }
  }

  function saveSettings(s) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
    } catch (e) {}
  }

  var settings = loadSettings();

  function currentTimeRange() {
    var id = settings.timeRange || "5m";
    return TIME_RANGES.hasOwnProperty(id) ? id : "5m";
  }

  function filterSamples(samples) {
    samples = samples || [];
    var id = currentTimeRange();
    var windowMs = TIME_RANGES[id] || 0;
    if (!windowMs || samples.length === 0) return samples;
    var latest = Number(samples[samples.length - 1].wall_unix_ms) || Date.now();
    var cut = latest - windowMs;
    var out = [];
    for (var i = 0; i < samples.length; i++) {
      if (Number(samples[i].wall_unix_ms) >= cut) out.push(samples[i]);
    }
    return out;
  }

  function yMaxFor(values) {
    var m = 1;
    for (var i = 0; i < values.length; i++) {
      if (values[i] > m) m = values[i];
    }
    return m;
  }

  function formatUnixMs(ms) {
    if (!ms) return "—";
    var d = new Date(Number(ms));
    if (isNaN(d.getTime())) return "—";
    return d.toISOString().replace("T", " ").slice(0, 19) + " UTC";
  }

  function formatCount(n) {
    n = Number(n) || 0;
    if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
    if (n >= 10000) return Math.round(n / 1000) + "k";
    return String(Math.round(n));
  }

  function formatDurationUs(us) {
    us = Number(us) || 0;
    if (us < 1000) return Math.round(us) + " µs";
    if (us < 1000000) return (us / 1000).toFixed(us % 1000 === 0 ? 0 : 1) + " ms";
    return (us / 1000000).toFixed(2) + " s";
  }

  function formatMs(ms) {
    ms = Number(ms) || 0;
    if (ms < 1000) return Math.round(ms) + " ms";
    if (ms < 60000) return (ms / 1000).toFixed(1) + " s";
    return (ms / 60000).toFixed(1) + " min";
  }

  function htmlEscape(s) {
    return String(s || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function chartShell(hostPrefix, id, title) {
    return (
      '<div class="chart" id="chart-' +
      hostPrefix +
      "-" +
      id +
      '"><div class="chart-head"><h3>' +
      htmlEscape(title) +
      "</h3></div>" +
      '<div class="chart-wrap">' +
      '<svg viewBox="0 0 640 180" preserveAspectRatio="none" role="img" aria-label="' +
      htmlEscape(title) +
      '"></svg>' +
      '<div class="hover-tip" hidden></div>' +
      "</div></div>"
    );
  }

  function drawChart(rootId, samples, values, formatY) {
    var root = document.getElementById("chart-" + rootId);
    if (!root) return;
    var svg = root.querySelector("svg");
    var tip = root.querySelector(".hover-tip");
    var W = 640;
    var H = 180;
    var padL = 72;
    var padR = 16;
    var padT = 12;
    var padB = 28;
    var innerW = W - padL - padR;
    var innerH = H - padT - padB;
    var n = values.length;
    var yMax = yMaxFor(values);
    if (yMax < 1) yMax = 1;
    var pts = [];
    for (var j = 0; j < n; j++) {
      var x = padL + (n > 1 ? (j / (n - 1)) * innerW : 0);
      var y = padT + innerH - (Math.min(values[j], yMax) / yMax) * innerH;
      pts.push(x.toFixed(1) + "," + y.toFixed(1));
    }
    var y0 = formatY(0);
    var yMid = formatY(yMax / 2);
    var yTop = formatY(yMax);
    var tFirst = samples.length ? formatUnixMs(samples[0].wall_unix_ms) : "";
    var tLast = samples.length ? formatUnixMs(samples[samples.length - 1].wall_unix_ms) : "";
    var gridY1 = padT + innerH / 2;
    var axisY = padT + innerH;
    var html = "";
    html +=
      '<line class="grid" x1="' +
      padL +
      '" y1="' +
      padT +
      '" x2="' +
      (W - padR) +
      '" y2="' +
      padT +
      '"/>';
    html +=
      '<line class="grid" x1="' +
      padL +
      '" y1="' +
      gridY1 +
      '" x2="' +
      (W - padR) +
      '" y2="' +
      gridY1 +
      '"/>';
    html +=
      '<line class="axis" x1="' +
      padL +
      '" y1="' +
      padT +
      '" x2="' +
      padL +
      '" y2="' +
      axisY +
      '"/>';
    html +=
      '<line class="axis" x1="' +
      padL +
      '" y1="' +
      axisY +
      '" x2="' +
      (W - padR) +
      '" y2="' +
      axisY +
      '"/>';
    html += '<text class="axis-label" x="8" y="' + (padT + 4) + '">' + yTop + "</text>";
    html += '<text class="axis-label" x="8" y="' + (gridY1 + 4) + '">' + yMid + "</text>";
    html += '<text class="axis-label" x="8" y="' + (axisY + 4) + '">' + y0 + "</text>";
    if (tFirst) {
      html +=
        '<text class="axis-label" x="' + padL + '" y="' + (H - 8) + '">' + tFirst + "</text>";
    }
    if (tLast) {
      html +=
        '<text class="axis-label" text-anchor="end" x="' +
        (W - padR) +
        '" y="' +
        (H - 8) +
        '">' +
        tLast +
        "</text>";
    }
    if (pts.length) {
      html += '<polyline class="line" points="' + pts.join(" ") + '"/>';
    }
    html +=
      '<rect class="hit" x="' +
      padL +
      '" y="' +
      padT +
      '" width="' +
      innerW +
      '" height="' +
      innerH +
      '" fill="transparent"/>';
    html += '<line class="cross" hidden x1="0" y1="' + padT + '" x2="0" y2="' + axisY + '"/>';
    html += '<circle class="dot" hidden r="4" cx="0" cy="0"/>';
    svg.innerHTML = html;
    bindHover(root, tip, samples, values, formatY, padL, padT, innerW, innerH, yMax);
  }

  function bindHover(root, tip, samples, values, formatY, padL, padT, innerW, innerH, yMax) {
    var hit = root.querySelector(".hit");
    var cross = root.querySelector(".cross");
    var dot = root.querySelector(".dot");
    if (!hit || !samples.length) return;
    function hide() {
      if (tip) tip.hidden = true;
      if (cross) cross.setAttribute("hidden", "");
      if (dot) dot.setAttribute("hidden", "");
    }
    function showAt(clientX) {
      var rect = hit.getBoundingClientRect();
      var t = (clientX - rect.left) / Math.max(1, rect.width);
      t = Math.max(0, Math.min(1, t));
      var idx = Math.round(t * (values.length - 1));
      var x = padL + t * innerW;
      var y = padT + innerH - (Math.min(values[idx], yMax) / yMax) * innerH;
      if (cross) {
        cross.removeAttribute("hidden");
        cross.setAttribute("x1", x.toFixed(1));
        cross.setAttribute("x2", x.toFixed(1));
      }
      if (dot) {
        dot.removeAttribute("hidden");
        dot.setAttribute("cx", x.toFixed(1));
        dot.setAttribute("cy", y.toFixed(1));
      }
      if (tip) {
        tip.hidden = false;
        tip.textContent =
          formatY(values[idx]) + " · " + formatUnixMs(samples[idx].wall_unix_ms);
        var wrap = root.querySelector(".chart-wrap");
        var wr = wrap.getBoundingClientRect();
        tip.style.left = Math.min(wr.width - 160, Math.max(8, clientX - wr.left + 12)) + "px";
        tip.style.top = "12px";
      }
    }
    hit.addEventListener("mousemove", function (ev) {
      showAt(ev.clientX);
    });
    hit.addEventListener("mouseleave", hide);
  }

  function set(field, text) {
    var el = document.querySelector("[data-field=\"" + field + "\"]");
    if (el) el.textContent = text;
  }

  function spendMsOf(sample, key) {
    if (!sample || !sample.spend_ms) return 0;
    return Number(sample.spend_ms[key]) || 0;
  }

  function spendMsFallback(stats, key) {
    var arr = stats && stats.spend_ms;
    if (Array.isArray(arr)) {
      for (var i = 0; i < arr.length; i++) {
        if (arr[i].key === key) return Number(arr[i].ms) || 0;
      }
    } else if (arr && typeof arr === "object") {
      return Number(arr[key]) || 0;
    }
    return 0;
  }

  function craftedCount(sample, id) {
    var rows = (sample && sample.crafted) || [];
    for (var i = 0; i < rows.length; i++) {
      if (Number(rows[i][0]) === id) return Number(rows[i][1]) || 0;
    }
    return 0;
  }

  function ageCount(sample, idx) {
    var a = (sample && sample.death_ages) || [];
    return Number(a[idx]) || 0;
  }

  function topSpendKeys(last) {
    var keys = Object.keys(SPEND_LABELS);
    keys.sort(function (a, b) {
      return spendMsOf(last, b) - spendMsOf(last, a);
    });
    return keys.filter(function (k) {
      return spendMsOf(last, k) > 0;
    });
  }

  function topCrafted(last) {
    var rows = (last && last.crafted) || [];
    var out = [];
    for (var i = 0; i < rows.length; i++) {
      out.push({ id: Number(rows[i][0]), count: Number(rows[i][1]) || 0 });
    }
    out.sort(function (a, b) {
      return b.count - a.count;
    });
    return out;
  }

  function redraw(stats) {
    stats = stats || window.OLR_AI || {};
    var samples = filterSamples(stats.samples || []);
    var last = samples.length ? samples[samples.length - 1] : null;
    set("named", formatCount(stats.baby_named));
    set("namedUnique", "unique " + formatCount(stats.baby_named_unique));
    set("pickup", formatCount(stats.baby_pickup));
    set("pickupUnique", "unique " + formatCount(stats.baby_pickup_unique));
    set("drop", formatCount(stats.baby_drop));
    set("deaths", formatCount(stats.deaths));
    set("craft", formatCount(stats.craft_attempts));
    set("eat", formatCount(stats.eat_attempts));
    set("stuck", formatCount(stats.stuck_events));

    var babyCharts = document.getElementById("baby-charts");
    if (babyCharts) {
      var babyKeys = [
        { id: "named", title: "Babies named", field: "named" },
        { id: "namedUnique", title: "Babies named (unique)", field: "named_unique" },
        { id: "pickup", title: "Babies picked up", field: "pickup" },
        { id: "pickupUnique", title: "Babies picked up (unique)", field: "pickup_unique" },
        { id: "drop", title: "Babies dropped", field: "drop_baby" }
      ];
      var bh = "";
      for (var bi = 0; bi < babyKeys.length; bi++) {
        bh += chartShell("baby", babyKeys[bi].id, babyKeys[bi].title);
      }
      babyCharts.innerHTML = bh;
      for (var bj = 0; bj < babyKeys.length; bj++) {
        var bv = [];
        for (var bs = 0; bs < samples.length; bs++) {
          bv.push(Number(samples[bs][babyKeys[bj].field]) || 0);
        }
        drawChart("baby-" + babyKeys[bj].id, samples, bv, formatCount);
      }
    }

    var ageHost = document.getElementById("age-cards");
    if (ageHost) {
      var ageHtml = "";
      var ages = stats.death_ages || [];
      for (var i = 0; i < AGE_LABELS.length; i++) {
        var n = 0;
        if (ages[i] && typeof ages[i] === "object") n = Number(ages[i].count) || 0;
        else if (last) n = ageCount(last, i);
        ageHtml +=
          '<div class="card"><strong>' +
          AGE_LABELS[i] +
          "</strong><br/><span>" +
          formatCount(n) +
          "</span></div>";
      }
      ageHost.innerHTML = ageHtml;
    }

    var ageCharts = document.getElementById("age-charts");
    if (ageCharts) {
      var ah = "";
      for (var a = 0; a < AGE_LABELS.length; a++) {
        ah += chartShell("age", String(a), "Deaths age " + AGE_LABELS[a]);
      }
      ageCharts.innerHTML = ah;
      for (var b = 0; b < AGE_LABELS.length; b++) {
        var av = [];
        for (var s = 0; s < samples.length; s++) av.push(ageCount(samples[s], b));
        drawChart("age-" + b, samples, av, formatCount);
      }
    }

    var spendKeys = topSpendKeys(last);
    if (!spendKeys.length) {
      spendKeys = Object.keys(SPEND_LABELS).filter(function (k) {
        return spendMsFallback(stats, k) > 0;
      });
      spendKeys.sort(function (a, b) {
        return spendMsFallback(stats, b) - spendMsFallback(stats, a);
      });
    }
    var spendCharts = document.getElementById("spend-charts");
    if (spendCharts) {
      var sh = "";
      var shown = spendKeys.slice(0, GRAPH_TOP);
      if (!shown.length) shown = ["walk_use", "walk_baby", "escape_animal", "attack_player", "walk_drop", "eat", "craft_use"];
      for (var k = 0; k < shown.length; k++) {
        sh += chartShell("spend", shown[k], SPEND_LABELS[shown[k]] || shown[k]);
      }
      spendCharts.innerHTML = sh;
      for (var k2 = 0; k2 < shown.length; k2++) {
        var sv = [];
        for (var s2 = 0; s2 < samples.length; s2++) sv.push(spendMsOf(samples[s2], shown[k2]));
        drawChart("spend-" + shown[k2], samples, sv, formatMs);
      }
    }
    var spendTable = document.getElementById("spend-table");
    if (spendTable) {
      var st = "";
      var allKeys = Object.keys(SPEND_LABELS);
      allKeys.sort(function (a, b) {
        var av = last ? spendMsOf(last, a) : spendMsFallback(stats, a);
        var bv = last ? spendMsOf(last, b) : spendMsFallback(stats, b);
        return bv - av;
      });
      for (var t = 0; t < allKeys.length; t++) {
        var sms = last ? spendMsOf(last, allKeys[t]) : spendMsFallback(stats, allKeys[t]);
        st +=
          "<tr><td>" +
          htmlEscape(SPEND_LABELS[allKeys[t]]) +
          "</td><td>" +
          formatMs(sms) +
          "</td></tr>";
      }
      spendTable.innerHTML = st;
    }

    var crafts = topCrafted(last);
    if (!crafts.length && stats.crafted_objects) {
      for (var c = 0; c < stats.crafted_objects.length; c++) {
        crafts.push({
          id: Number(stats.crafted_objects[c].id),
          count: Number(stats.crafted_objects[c].count) || 0
        });
      }
      crafts.sort(function (a, b) {
        return b.count - a.count;
      });
    }
    var names = stats.names || {};
    var craftCharts = document.getElementById("craft-charts");
    if (craftCharts) {
      var ch = "";
      var top = crafts.slice(0, GRAPH_TOP);
      for (var g = 0; g < top.length; g++) {
        var title = (names[String(top[g].id)] || "object") + " [" + top[g].id + "]";
        ch += chartShell("craft", String(top[g].id), title);
      }
      if (!top.length) ch = '<p class="muted">No craft samples yet.</p>';
      craftCharts.innerHTML = ch;
      for (var g2 = 0; g2 < top.length; g2++) {
        var cv = [];
        for (var s3 = 0; s3 < samples.length; s3++) cv.push(craftedCount(samples[s3], top[g2].id));
        drawChart("craft-" + top[g2].id, samples, cv, formatCount);
      }
    }
    var craftTable = document.getElementById("craft-table");
    if (craftTable) {
      var ct = "";
      for (var r = 0; r < crafts.length; r++) {
        var cname = names[String(crafts[r].id)] || ("object " + crafts[r].id);
        ct +=
          "<tr><td class=\"obj-name\">" +
          htmlEscape(cname) +
          "</td><td class=\"id\">" +
          crafts[r].id +
          "</td><td class=\"num\">" +
          formatCount(crafts[r].count) +
          "</td></tr>";
      }
      craftTable.innerHTML = ct || '<tr><td colspan="3" class="muted">none yet</td></tr>';
    }

    var actionTable = document.getElementById("action-table");
    if (actionTable) {
      var actions = stats.objects_created || [];
      actions = actions.slice().sort(function (a, b) {
        return (b.count || 0) - (a.count || 0);
      });
      var at = "";
      for (var u = 0; u < actions.length && u < 40; u++) {
        at +=
          "<tr><td class=\"obj-name\">" +
          htmlEscape(actions[u].action) +
          "</td><td class=\"num\">" +
          formatCount(actions[u].count) +
          "</td></tr>";
      }
      actionTable.innerHTML = at || '<tr><td colspan="2" class="muted">none yet</td></tr>';
    }

    var foodTable = document.getElementById("food-table");
    if (foodTable) {
      var food = stats.food_eaten || [];
      food = food.slice().sort(function (a, b) {
        return (b.count || 0) - (a.count || 0);
      });
      var ft = "";
      for (var f = 0; f < food.length; f++) {
        var fid = food[f].id;
        var fname = food[f].name || names[String(fid)] || ("object " + fid);
        ft +=
          "<tr><td class=\"obj-name\">" +
          htmlEscape(fname) +
          "</td><td class=\"id\">" +
          fid +
          "</td><td class=\"num\">" +
          formatCount(food[f].count) +
          "</td></tr>";
      }
      foodTable.innerHTML = ft || '<tr><td colspan="3" class="muted">none yet</td></tr>';
    }
  }

  function bindTimeRangeSelect() {
    var sel = document.getElementById("ops-time-range");
    if (!sel || sel.getAttribute("data-bound") === "1") return;
    sel.value = currentTimeRange();
    sel.addEventListener("change", function () {
      settings.timeRange = sel.value;
      saveSettings(settings);
      redraw(window.OLR_AI);
    });
    sel.setAttribute("data-bound", "1");
  }

  function sampleField(sample, field) {
    if (!sample) return 0;
    if (field === "ai_vs_sim_pct") {
      var sim = Number(sample.sim_cpu_us) || 0;
      var ai = Number(sample.ai_cpu_us) || 0;
      return sim ? Math.min(999, (ai / sim) * 100) : 0;
    }
    return Number(sample[field]) || 0;
  }

  function formatTimingY(kind, v) {
    if (kind === "duration") return formatDurationUs(v);
    if (kind === "percent") return Math.round(v) + "%";
    return formatCount(v);
  }

  function drawAiTimings(metrics, opsPayload) {
    var cards = document.getElementById("ai-timing-cards");
    var host = document.getElementById("ai-timing-charts");
    if (!cards || !host) return;
    metrics = metrics || {};
    var samples = filterSamples((opsPayload && opsPayload.samples) || []);
    var last = samples.length ? samples[samples.length - 1] : null;
    var html = "";
    html +=
      '<div class="card"><strong>AI thinks</strong><br/>' +
      formatCount(metrics.ai_thinks) +
      "</div>";
    html +=
      '<div class="card"><strong>Think time</strong><br/>' +
      formatDurationUs(metrics.ai_think_ema_us) +
      "</div>";
    html +=
      '<div class="card"><strong>AI CPU</strong><br/>' +
      formatDurationUs(metrics.ai_cpu_us) +
      "</div>";
    html +=
      '<div class="card"><strong>Scan time</strong><br/>' +
      formatDurationUs(metrics.ai_scan_ema_us) +
      "</div>";
    var sim = Number(metrics.sim_cpu_us) || 0;
    var ai = Number(metrics.ai_cpu_us) || 0;
    html +=
      '<div class="card"><strong>AI vs sim</strong><br/>' +
      (sim ? Math.min(999, Math.round((ai / sim) * 100)) + "%" : "—") +
      "</div>";
    cards.innerHTML = html;
    var ch = "";
    for (var i = 0; i < AI_TIMING_CHARTS.length; i++) {
      ch += chartShell("time", AI_TIMING_CHARTS[i].id, AI_TIMING_CHARTS[i].title);
    }
    host.innerHTML = ch;
    for (var j = 0; j < AI_TIMING_CHARTS.length; j++) {
      var spec = AI_TIMING_CHARTS[j];
      var vals = [];
      for (var s = 0; s < samples.length; s++) vals.push(sampleField(samples[s], spec.field));
      if (!vals.length && last) vals = [sampleField(last, spec.field)];
      (function (kind, chartId) {
        drawChart(chartId, samples, vals, function (v) {
          return formatTimingY(kind, v);
        });
      })(spec.kind, "time-" + spec.id);
    }
  }

  function poll() {
    Promise.all([
      fetch("/api/npc/stats").then(function (r) {
        return r.json();
      }),
      fetch("/api/metrics").then(function (r) {
        return r.json();
      }),
      fetch("/api/ops/series").then(function (r) {
        return r.json();
      })
    ])
      .then(function (parts) {
        var j = parts[0];
        drawAiTimings(parts[1], parts[2]);
        var names = {};
        if (window.OLR_AI && window.OLR_AI.names) {
          Object.assign(names, window.OLR_AI.names);
        }
        if (j && j.names) Object.assign(names, j.names);
        function addNames(rows) {
          if (!rows) return;
          for (var i = 0; i < rows.length; i++) {
            if (rows[i].id != null && rows[i].name) {
              names[String(rows[i].id)] = rows[i].name;
            }
          }
        }
        if (j) {
          addNames(j.crafted_objects);
          addNames(j.food_eaten);
          j.names = names;
        }
        window.OLR_AI = j;
        redraw(j);
      })
      .catch(function () {});
  }

  bindTimeRangeSelect();
  redraw(window.OLR_AI);
  poll();
  setInterval(poll, 5000);
})();
