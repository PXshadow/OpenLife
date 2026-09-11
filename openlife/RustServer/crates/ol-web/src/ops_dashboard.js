(function () {
  "use strict";

  var STORAGE_KEY = "openlife-ops-dashboard-v2";

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

  var CHARTS = [
    {
      id: "tickWork",
      title: "Tick Work",
      tip: "Time spent running one simulation tick, smoothed. Lower is healthier.",
      field: "tick_work_us",
      kind: "duration"
    },
    {
      id: "intent",
      title: "Intent Time",
      tip: "Time to apply one player or NPC action (smoothed average).",
      field: "intent_ema_us",
      kind: "duration"
    },
    {
      id: "lockWait",
      title: "Lock Wait",
      tip: "Time spent waiting for the world lock before a tick can run.",
      field: "lock_wait_ema_us",
      kind: "duration"
    },
    {
      id: "skipTicks",
      title: "Skip Ticks",
      tip: "Catch-up steps when the server fell behind real time. These are extra advances, not dropped frames.",
      field: "skip_ticks",
      kind: "count"
    },
    {
      id: "humanIntent",
      title: "Human Intent Time",
      tip: "Time to apply one human client action (smoothed).",
      field: "human_intent_avg_us",
      kind: "duration"
    },
    {
      id: "simCpu",
      title: "Sim CPU Total",
      tip: "Cumulative simulation tick work.",
      field: "sim_cpu_us",
      kind: "duration"
    }
  ];

  function loadSettings() {
    try {
      var raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return {};
      var parsed = JSON.parse(raw);
      return parsed && typeof parsed === "object" ? parsed : {};
    } catch (e) {
      return {};
    }
  }

  function saveSettings(s) {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
    } catch (e) {
      /* private mode */
    }
  }

  function formatDurationUs(us) {
    us = Number(us) || 0;
    if (us < 1000) return Math.round(us) + " µs";
    if (us < 1000000) {
      if (us % 1000 === 0) return us / 1000 + " ms";
      if (us % 100 === 0) return (us / 1000).toFixed(1) + " ms";
      return (us / 1000).toFixed(2) + " ms";
    }
    if (us % 1000000 === 0) return us / 1000000 + " s";
    return (us / 1000000).toFixed(2) + " s";
  }

  function formatDurationMs(ms) {
    return formatDurationUs((Number(ms) || 0) * 1000);
  }

  function formatUnixMs(ms) {
    ms = Number(ms) || 0;
    if (!ms) return "—";
    var d = new Date(ms);
    if (isNaN(d.getTime())) return "—";
    function pad(n) {
      return n < 10 ? "0" + n : String(n);
    }
    return (
      d.getUTCFullYear() +
      "-" +
      pad(d.getUTCMonth() + 1) +
      "-" +
      pad(d.getUTCDate()) +
      " " +
      pad(d.getUTCHours()) +
      ":" +
      pad(d.getUTCMinutes()) +
      ":" +
      pad(d.getUTCSeconds()) +
      " UTC"
    );
  }

  function formatCount(n) {
    n = Number(n) || 0;
    return String(Math.round(n));
  }

  function formatValue(kind, v) {
    if (kind === "duration") return formatDurationUs(v);
    if (kind === "percent") return Math.round(Number(v) || 0) + "%";
    return formatCount(v);
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

  function $(sel, root) {
    return (root || document).querySelector(sel);
  }

  function renderCards(metrics, samples) {
    var last = samples.length ? samples[samples.length - 1] : null;
    var lat = (metrics && metrics.latency) || {};
    var boot = (metrics && metrics.boot) || {};
    var set = function (name, text) {
      var el = document.querySelector('[data-field="' + name + '"]');
      if (el) el.textContent = text;
    };
    set("started", formatUnixMs(metrics.start_unix_ms));
    set("tick", formatCount(metrics.ticks));
    set("skipTicks", formatCount(metrics.skip_ticks));
    set(
      "tickWork",
      formatDurationUs(
        last ? last.tick_work_us : metrics.tick_work_ema_us
      )
    );
    set(
      "intent",
      formatDurationUs(last ? last.intent_ema_us : metrics.intent_ema_us)
    );
    set(
      "tickAvgP90",
      formatDurationUs(lat.tick_avg_us) + " / " + formatDurationUs(lat.tick_p90_us)
    );
    set(
      "tickOutliers",
      formatCount(lat.tick_outliers) + " / " + formatCount(lat.tick_normal)
    );
    set(
      "intentAvgP90",
      formatDurationUs(lat.intent_avg_us) +
        " / " +
        formatDurationUs(lat.intent_p90_us)
    );
    set(
      "intentOutliers",
      formatCount(lat.intent_outliers) + " / " + formatCount(lat.intent_normal)
    );
    set(
      "humanIntent",
      formatDurationUs(lat.human_intent_avg_us) +
        " / " +
        formatDurationUs(lat.human_intent_p90_us)
    );
    set("humanIntentN", "n = " + formatCount(lat.human_intent_count));
    set(
      "aiIntent",
      formatDurationUs(lat.ai_intent_avg_us) +
        " / " +
        formatDurationUs(lat.ai_intent_p90_us)
    );
    set("aiIntentN", "n = " + formatCount(lat.ai_intent_count));
    set(
      "lockWait",
      formatDurationUs(last ? last.lock_wait_ema_us : metrics.lock_wait_ema_us)
    );
    set("bootTotal", formatDurationMs(boot.total_ms));
    set(
      "bootParts",
      formatDurationMs(boot.objects_ms) +
        " / " +
        formatDurationMs(boot.transitions_ms) +
        " / " +
        formatDurationMs(boot.world_ms)
    );
    set("samples", formatCount(samples.length));
    set("aiThinks", formatCount(metrics.ai_thinks));
    set("aiThinkTime", formatDurationUs(metrics.ai_think_ema_us));
    set("aiThinkLast", "last " + formatDurationUs(metrics.ai_think_last_us));
    set("aiCpu", formatDurationUs(metrics.ai_cpu_us));
    var tickUs = last ? last.tick_work_us : metrics.tick_work_ema_us;
    var thinkUs = metrics.ai_think_ema_us || 0;
    set(
      "aiVsTick",
      tickUs
        ? Math.min(999, Math.round((thinkUs / tickUs) * 100)) + "%"
        : "—"
    );
    set("aiScanTime", formatDurationUs(metrics.ai_scan_ema_us));
    set("aiScanLast", "last " + formatDurationUs(metrics.ai_scan_last_us));
    set("aiOtherTime", formatDurationUs(metrics.ai_other_ema_us));
    set("aiScanCpu", formatDurationUs(metrics.ai_scan_cpu_us));
    var scanUs = metrics.ai_scan_ema_us || 0;
    set(
      "aiScanVsThink",
      thinkUs
        ? Math.min(100, Math.round((scanUs / thinkUs) * 100)) + "%"
        : "—"
    );
    set(
      "aiScanCache",
      formatCount(metrics.ai_scan_hits) + " / " + formatCount(metrics.ai_scan_calls)
    );
    set("simCpu", formatDurationUs(metrics.sim_cpu_us));
    var simUs = metrics.sim_cpu_us || 0;
    var aiCpu = metrics.ai_cpu_us || 0;
    set(
      "aiVsSim",
      simUs ? Math.min(999, Math.round((aiCpu / simUs) * 100)) + "%" : "—"
    );
  }

  function bindTimeRangeSelect() {
    var sel = document.getElementById("ops-time-range");
    if (!sel || sel.getAttribute("data-bound") === "1") return;
    sel.value = currentTimeRange();
    sel.addEventListener("change", function () {
      settings.timeRange = sel.value;
      saveSettings(settings);
      drawAll(window.OLR_OPS && window.OLR_OPS.samples ? window.OLR_OPS.samples : []);
    });
    sel.setAttribute("data-bound", "1");
  }

  function ensureChartShells() {
    bindTimeRangeSelect();
    var host = document.getElementById("ops-charts");
    if (!host || host.getAttribute("data-ready") === "1") return;
    var html = "";
    for (var i = 0; i < CHARTS.length; i++) {
      var c = CHARTS[i];
      html +=
        '<div class="chart" id="chart-' +
        c.id +
        '">' +
        '<div class="chart-head">' +
        "<h3 title=\"" +
        c.tip.replace(/"/g, "&quot;") +
        '">' +
        c.title +
        '<span class="tip">' +
        c.tip +
        "</span></h3>" +
        "</div>" +
        '<div class="chart-wrap">' +
        '<svg viewBox="0 0 640 180" preserveAspectRatio="none" role="img" aria-label="' +
        c.title +
        '"></svg>' +
        '<div class="hover-tip" hidden></div>' +
        "</div></div>";
    }
    host.innerHTML = html;
    host.setAttribute("data-ready", "1");
  }

  function drawAll(samples) {
    var shown = filterSamples(samples || []);
    for (var i = 0; i < CHARTS.length; i++) drawChart(CHARTS[i], shown);
  }

  function sampleValue(sample, field) {
    var v = sample[field];
    return typeof v === "number" ? v : 0;
  }

  function drawChart(chart, samples) {
    var root = document.getElementById("chart-" + chart.id);
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
    var values = [];
    for (var i = 0; i < samples.length; i++) {
      values.push(sampleValue(samples[i], chart.field));
    }
    var yMax = yMaxFor(values);
    if (yMax < 1) yMax = 1;

    var pts = [];
    var n = values.length;
    for (var j = 0; j < n; j++) {
      var x = padL + (n > 1 ? (j / (n - 1)) * innerW : 0);
      var y = padT + innerH - (Math.min(values[j], yMax) / yMax) * innerH;
      pts.push(x.toFixed(1) + "," + y.toFixed(1));
    }

    var y0 = formatValue(chart.kind, 0);
    var yMid = formatValue(chart.kind, yMax / 2);
    var yTop = formatValue(chart.kind, yMax);
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
    html +=
      '<text class="axis-label" x="8" y="' +
      (padT + 4) +
      '">' +
      yTop +
      "</text>";
    html +=
      '<text class="axis-label" x="8" y="' +
      (gridY1 + 4) +
      '">' +
      yMid +
      "</text>";
    html +=
      '<text class="axis-label" x="8" y="' +
      (axisY + 4) +
      '">' +
      y0 +
      "</text>";
    if (tFirst) {
      html +=
        '<text class="axis-label" x="' +
        padL +
        '" y="' +
        (H - 8) +
        '">' +
        tFirst +
        "</text>";
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
      html +=
        '<polyline class="line" fill="none" points="' + pts.join(" ") + '"/>';
    } else {
      html +=
        '<text class="axis-label" x="' +
        (padL + 12) +
        '" y="' +
        (padT + innerH / 2) +
        '">No samples yet</text>';
    }
    html +=
      '<line class="cross" x1="0" y1="' +
      padT +
      '" x2="0" y2="' +
      axisY +
      '" visibility="hidden"/>';
    html += '<circle class="dot" r="4" visibility="hidden"/>';
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
    svg.innerHTML = html;

    var hit = svg.querySelector(".hit");
    var cross = svg.querySelector(".cross");
    var dot = svg.querySelector(".dot");
    function hideHover() {
      cross.setAttribute("visibility", "hidden");
      dot.setAttribute("visibility", "hidden");
      tip.hidden = true;
    }
    function showAt(clientX) {
      if (!n) {
        hideHover();
        return;
      }
      var rect = svg.getBoundingClientRect();
      var xPix = ((clientX - rect.left) / rect.width) * W;
      var t = (xPix - padL) / innerW;
      if (t < 0) t = 0;
      if (t > 1) t = 1;
      var idx = n > 1 ? Math.round(t * (n - 1)) : 0;
      var sm = samples[idx];
      var val = values[idx];
      var x = padL + (n > 1 ? (idx / (n - 1)) * innerW : 0);
      var y = padT + innerH - (Math.min(val, yMax) / yMax) * innerH;
      cross.setAttribute("x1", x.toFixed(1));
      cross.setAttribute("x2", x.toFixed(1));
      cross.setAttribute("visibility", "visible");
      dot.setAttribute("cx", x.toFixed(1));
      dot.setAttribute("cy", y.toFixed(1));
      dot.setAttribute("visibility", "visible");
      tip.hidden = false;
      tip.innerHTML =
        "<strong>" +
        chart.title +
        "</strong><br/>" +
        formatValue(chart.kind, val) +
        "<br/>" +
        formatUnixMs(sm.wall_unix_ms) +
        "<br/>Tick " +
        formatCount(sm.tick);
      var wrap = root.querySelector(".chart-wrap").getBoundingClientRect();
      var left = clientX - wrap.left + 12;
      var top = y * (rect.height / H) - 8;
      if (left + 200 > wrap.width) left = clientX - wrap.left - 210;
      if (top < 0) top = 0;
      tip.style.left = left + "px";
      tip.style.top = top + "px";
    }
    hit.addEventListener("mousemove", function (ev) {
      showAt(ev.clientX);
    });
    hit.addEventListener("mouseleave", hideHover);
  }

  function apply(metrics, payload) {
    var samples = (payload && payload.samples) || [];
    window.OLR_OPS = payload || { samples: [], count: 0 };
    renderCards(metrics || {}, samples);
    ensureChartShells();
    drawAll(samples);
  }

  function poll() {
    Promise.all([
      fetch("/api/metrics").then(function (r) {
        return r.json();
      }),
      fetch("/api/ops/series").then(function (r) {
        return r.json();
      })
    ])
      .then(function (pair) {
        apply(pair[0], pair[1]);
      })
      .catch(function () {
        /* keep last view */
      });
  }

  apply(
    window.OLR_OPS_METRICS || {},
    window.OLR_OPS || { samples: [], count: 0 }
  );
  setInterval(poll, 5000);
})();
