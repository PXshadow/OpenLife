(function () {
  "use strict";

  var STORAGE_KEY = "openlife-ops-dashboard-v2";
  var LIST_KEY = "openlife-object-counts-v1";

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

  var MS_DAY = 24 * 60 * 60 * 1000;
  var MS_WEEK = 7 * MS_DAY;
  var MS_MONTH = 30 * MS_DAY;
  var GRAPH_TOP = 10;
  var LIST_TOP = 100;

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

  function loadListSettings() {
    try {
      return JSON.parse(localStorage.getItem(LIST_KEY) || "{}") || {};
    } catch (e) {
      return {};
    }
  }

  function saveListSettings(s) {
    try {
      localStorage.setItem(LIST_KEY, JSON.stringify(s));
    } catch (e) {}
  }

  var settings = loadSettings();
  var listSettings = loadListSettings();

  function currentTimeRange() {
    var id = settings.timeRange || "5m";
    return TIME_RANGES.hasOwnProperty(id) ? id : "5m";
  }

  function currentSort() {
    return listSettings.sort === "count" ? "count" : "change";
  }

  function selectAllOn() {
    return listSettings.selectAll === true;
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
    return String(Math.round(n));
  }

  function formatPct(p) {
    if (p === null || typeof p !== "number" || !isFinite(p)) return "—";
    var sign = p > 0 ? "+" : "";
    return sign + p.toFixed(1) + "%";
  }

  function pctChange(now, then) {
    now = Number(now) || 0;
    then = Number(then) || 0;
    if (then === 0) return now === 0 ? 0 : 100;
    return ((now - then) / Math.abs(then)) * 100;
  }

  function isComplete(sample) {
    if (!sample) return false;
    var rows = sampleObjects(sample);
    var unique = Number(sample.unique) || 0;
    return Number(sample.total) > 0 && unique > 0 && rows.length === unique;
  }

  function nearestAtOrBefore(samples, targetMs) {
    if (!samples || !samples.length) return null;
    var best = null;
    var oldest = null;
    for (var i = 0; i < samples.length; i++) {
      if (!isComplete(samples[i])) continue;
      if (!oldest) oldest = samples[i];
      if (Number(samples[i].wall_unix_ms) <= targetMs) best = samples[i];
      else if (best) break;
    }
    return best || oldest;
  }

  function sampleObjects(sample) {
    if (!sample) return [];
    if (sample.objects && sample.objects.length) return sample.objects;
    var top = sample.top || [];
    var out = [];
    for (var i = 0; i < top.length; i++) {
      out.push([top[i].id, top[i].current, top[i].original]);
    }
    return out;
  }

  function countOf(sample, id) {
    var rows = sampleObjects(sample);
    for (var i = 0; i < rows.length; i++) {
      if (Number(rows[i][0]) === id) return Number(rows[i][1]) || 0;
    }
    return 0;
  }

  function countOpt(sample, id) {
    if (!sample || !isComplete(sample)) return null;
    var rows = sampleObjects(sample);
    for (var i = 0; i < rows.length; i++) {
      if (Number(rows[i][0]) === id) return Number(rows[i][1]) || 0;
    }
    return 0;
  }

  function htmlEscape(s) {
    return String(s || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function nameOf(id, names, sample) {
    var key = String(id);
    if (names && names[key]) return names[key];
    var top = (sample && sample.top) || [];
    for (var i = 0; i < top.length; i++) {
      if (Number(top[i].id) === id && top[i].name) return top[i].name;
    }
    return "object";
  }

  function buildRows(allSamples, last, names) {
    var latestMs = last ? Number(last.wall_unix_ms) : Date.now();
    var s24 = nearestAtOrBefore(allSamples, latestMs - MS_DAY);
    var sW = nearestAtOrBefore(allSamples, latestMs - MS_WEEK);
    var sM = nearestAtOrBefore(allSamples, latestMs - MS_MONTH);
    var rows = sampleObjects(last);
    var out = [];
    for (var i = 0; i < rows.length; i++) {
      var id = Number(rows[i][0]);
      var cur = Number(rows[i][1]) || 0;
      var orig = Number(rows[i][2]) || 0;
      var c24 = s24 ? countOpt(s24, id) : null;
      var cW = sW ? countOpt(sW, id) : null;
      var cM = sM ? countOpt(sM, id) : null;
      var p24 = c24 === null ? null : pctChange(cur, c24);
      var pW = cW === null ? null : pctChange(cur, cW);
      var pM = cM === null ? null : pctChange(cur, cM);
      out.push({
        id: id,
        name: nameOf(id, names, last),
        current: cur,
        original: orig,
        pct24: p24,
        pctWeek: pW,
        pctMonth: pM,
        abs24: p24 === null ? -1 : Math.abs(p24)
      });
    }
    return out;
  }

  function bindTimeRangeSelect() {
    var sel = document.getElementById("ops-time-range");
    if (!sel || sel.getAttribute("data-bound") === "1") return;
    sel.value = currentTimeRange();
    sel.addEventListener("change", function () {
      settings.timeRange = sel.value;
      saveSettings(settings);
      redraw();
    });
    sel.setAttribute("data-bound", "1");
  }

  function bindListControls() {
    var search = document.getElementById("obj-search");
    var sort = document.getElementById("obj-sort");
    var allBtn = document.getElementById("obj-select-all");
    if (search && search.getAttribute("data-bound") !== "1") {
      search.addEventListener("input", redraw);
      search.setAttribute("data-bound", "1");
    }
    if (sort && sort.getAttribute("data-bound") !== "1") {
      sort.value = currentSort();
      sort.addEventListener("change", function () {
        listSettings.sort = sort.value;
        saveListSettings(listSettings);
        redraw();
      });
      sort.setAttribute("data-bound", "1");
    }
    if (allBtn && allBtn.getAttribute("data-bound") !== "1") {
      allBtn.addEventListener("click", function () {
        listSettings.selectAll = !selectAllOn();
        saveListSettings(listSettings);
        redraw();
      });
      allBtn.setAttribute("data-bound", "1");
    }
  }

  function chartShell(id, title) {
    return (
      '<div class="chart" id="chart-' +
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

  function ensureChartShells(graphRows) {
    bindTimeRangeSelect();
    bindListControls();
    var host = document.getElementById("ops-charts");
    if (!host) return;
    var html = chartShell("total", "Total objects") + chartShell("unique", "Unique types");
    for (var i = 0; i < graphRows.length && i < GRAPH_TOP; i++) {
      var t = graphRows[i];
      html += chartShell("id-" + t.id, t.name + " [" + t.id + "]");
    }
    host.innerHTML = html;
  }

  function drawChart(rootId, title, samples, values, kind) {
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
    var y0 = formatCount(0);
    var yMid = formatCount(yMax / 2);
    var yTop = formatCount(yMax);
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
      '<text class="axis-label" x="8" y="' + (padT + 4) + '">' + yTop + "</text>";
    html +=
      '<text class="axis-label" x="8" y="' + (gridY1 + 4) + '">' + yMid + "</text>";
    html +=
      '<text class="axis-label" x="8" y="' + (axisY + 4) + '">' + y0 + "</text>";
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
      var px = padL + (n > 1 ? (idx / (n - 1)) * innerW : 0);
      var py = padT + innerH - (Math.min(val, yMax) / yMax) * innerH;
      cross.setAttribute("x1", px.toFixed(1));
      cross.setAttribute("x2", px.toFixed(1));
      cross.setAttribute("visibility", "visible");
      dot.setAttribute("cx", px.toFixed(1));
      dot.setAttribute("cy", py.toFixed(1));
      dot.setAttribute("visibility", "visible");
      tip.hidden = false;
      tip.innerHTML =
        "<strong>" +
        htmlEscape(title) +
        "</strong><br/>" +
        formatCount(val) +
        "<br/>" +
        formatUnixMs(sm.wall_unix_ms);
      var wrap = root.querySelector(".chart-wrap").getBoundingClientRect();
      var left = clientX - wrap.left + 12;
      var top = py * (rect.height / H) - 8;
      if (left + 200 > wrap.width) left = clientX - wrap.left - 210;
      if (top < 0) top = 0;
      tip.style.left = left + "px";
      tip.style.top = top + "px";
    }
    hit.addEventListener("mousemove", function (ev) {
      showAt(ev.clientX);
    });
    hit.addEventListener("mouseleave", hideHover);
    void kind;
  }

  function triangle(p) {
    if (p === null || typeof p !== "number" || !isFinite(p)) {
      return '<span class="tri-flat">–</span>';
    }
    if (p < -0.05) return '<span class="tri-down" title="down last 24h">▼</span>';
    if (p > 0.05) return '<span class="tri-up" title="up last 24h">▲</span>';
    return '<span class="tri-flat">–</span>';
  }

  function searchQuery() {
    var el = document.getElementById("obj-search");
    return el ? String(el.value || "").trim().toLowerCase() : "";
  }

  function matchesSearch(row, q) {
    if (!q) return true;
    return (
      String(row.id).indexOf(q) !== -1 ||
      String(row.name || "").toLowerCase().indexOf(q) !== -1
    );
  }

  function sortRows(rows) {
    var copy = rows.slice();
    if (currentSort() === "count") {
      copy.sort(function (a, b) {
        return b.current - a.current || a.id - b.id;
      });
    } else {
      copy.sort(function (a, b) {
        return b.abs24 - a.abs24 || b.current - a.current || a.id - b.id;
      });
    }
    return copy;
  }

  function drawAll(payload) {
    var samples = (payload && payload.samples) || [];
    var names = (payload && payload.names) || {};
    var shown = filterSamples(samples);
    var last = shown.length ? shown[shown.length - 1] : samples.length ? samples[samples.length - 1] : null;
    var rows = last ? buildRows(samples, last, names) : [];
    var sorted = sortRows(rows);
    var graphRows = sorted.slice(0, GRAPH_TOP);
    ensureChartShells(graphRows);

    var totals = [];
    var uniques = [];
    for (var i = 0; i < shown.length; i++) {
      totals.push(Number(shown[i].total) || 0);
      uniques.push(Number(shown[i].unique) || 0);
    }
    drawChart("total", "Total objects", shown, totals, "count");
    drawChart("unique", "Unique types", shown, uniques, "count");
    for (var g = 0; g < graphRows.length; g++) {
      var id = graphRows[g].id;
      var vals = [];
      for (var k = 0; k < shown.length; k++) vals.push(countOf(shown[k], id));
      drawChart(
        "id-" + id,
        graphRows[g].name + " [" + id + "]",
        shown,
        vals,
        "count"
      );
    }

    var q = searchQuery();
    var matched = [];
    for (var r = 0; r < sorted.length; r++) {
      if (matchesSearch(sorted[r], q)) matched.push(sorted[r]);
    }
    var excluded = sorted.length - matched.length;
    var visible = selectAllOn() ? matched : matched.slice(0, LIST_TOP);
    var tbody = document.getElementById("top-table");
    if (tbody) {
      var html = "";
      for (var v = 0; v < visible.length; v++) {
        var row = visible[v];
        html +=
          "<tr><td>" +
          triangle(row.pct24) +
          "</td><td>" +
          row.id +
          "</td><td>" +
          htmlEscape(row.name) +
          "</td><td>" +
          row.current +
          "</td><td>" +
          row.original +
          "</td><td>" +
          formatPct(row.pct24) +
          "</td><td>" +
          formatPct(row.pctWeek) +
          "</td><td>" +
          formatPct(row.pctMonth) +
          "</td></tr>";
      }
      tbody.innerHTML = html;
    }
    var set = function (name, text) {
      var el = document.querySelector('[data-field="' + name + '"]');
      if (el) el.textContent = text;
    };
    if (last) {
      set("total", String(last.total));
      set("unique", String(last.unique));
    }
    set("samples", String(shown.length));
    set("excluded", String(excluded));
    var allBtn = document.getElementById("obj-select-all");
    if (allBtn) {
      allBtn.textContent = selectAllOn() ? "Top 100" : "Select all";
    }
    var vis = document.getElementById("obj-visible");
    if (vis) {
      vis.textContent =
        visible.length +
        " shown of " +
        matched.length +
        " matches (" +
        excluded +
        " excluded by search)";
    }
  }

  function redraw() {
    drawAll(window.OLR_OBJ || { samples: [], count: 0, names: {} });
  }

  function poll() {
    fetch("/api/object-counts/series")
      .then(function (r) {
        return r.json();
      })
      .then(function (payload) {
        window.OLR_OBJ = payload || { samples: [], count: 0, names: {} };
        drawAll(window.OLR_OBJ);
      })
      .catch(function () {});
  }

  drawAll(window.OLR_OBJ || { samples: [], count: 0, names: {} });
  setInterval(poll, 15000);
})();
