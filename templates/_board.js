/* rmap static HTML — shared board behaviour for the single-project and
   portfolio pages. Injected verbatim (not template-parsed). Vanilla, no deps.
   - lane / marker switches filter board and rack cards
   - clicking a card "pulls" it: the detail panel renders the task's full spec
     from the rmap-data island (the same data agents read); deep link #task=R:ID
   - portfolio: repo rails expand, repo search, cross-repo cords in the gutter */
(function () {
  "use strict";

  var island = JSON.parse(document.getElementById("rmap-data").textContent || "{}");
  var projects = island.projects || [island];
  var index = {};      // "repo:id" → task
  var dependents = {}; // "repo:id" → [id, …] (direct, in-repo)
  // Portfolio aliases are resolved by the renderer, including collisions.
  var repoByName = Object.assign(Object.create(null), island.repo_aliases);

  projects.forEach(function (proj, r) {
    if (!island.projects && proj.project) repoByName[String(proj.project).toLowerCase()] = r;
    (proj.task || []).forEach(function (t) {
      index[r + ":" + t.id] = t;
      (t.depends_on || []).forEach(function (d) {
        var k = r + ":" + d;
        (dependents[k] = dependents[k] || []).push(String(t.id));
      });
    });
  });

  // ── Filters ──────────────────────────────────────────────────────────────
  var lanesOn = { ready: true, active: true, hold: true, waiting: true, done: true };
  var markerOn = null;

  function applyFilters() {
    document.querySelectorAll(".task-card, .rack-card").forEach(function (card) {
      var markers = (card.dataset.markers || "").split(" ");
      var ok = lanesOn[card.dataset.lane] && (!markerOn || markers.indexOf(markerOn) !== -1);
      card.classList.toggle("hidden", !ok);
    });
    document.querySelectorAll("[data-lane-col]").forEach(function (col) {
      col.classList.toggle("lane-off", !lanesOn[col.dataset.laneCol]);
    });
  }

  document.querySelectorAll("[data-lane-toggle]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var lane = btn.dataset.laneToggle;
      lanesOn[lane] = !lanesOn[lane];
      btn.setAttribute("aria-pressed", String(lanesOn[lane]));
      applyFilters();
      drawCords();
    });
  });
  document.querySelectorAll("[data-marker]").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var m = btn.dataset.marker;
      markerOn = markerOn === m ? null : m;
      document.querySelectorAll("[data-marker]").forEach(function (b) {
        b.setAttribute("aria-pressed", String(b.dataset.marker === markerOn));
      });
      applyFilters();
    });
  });

  document.querySelectorAll(".rack-more").forEach(function (btn) {
    var rack = btn.closest(".rack");
    var label = btn.textContent;
    btn.addEventListener("click", function () {
      var open = rack.classList.toggle("rack-open");
      btn.setAttribute("aria-expanded", String(open));
      btn.textContent = open ? "Show top 8" : label;
    });
  });

  // ── Pulled card: detail panel ────────────────────────────────────────────
  var panel = document.getElementById("card-detail");
  var metaEl = panel.querySelector(".detail-meta");
  var titleEl = panel.querySelector(".detail-title");
  var backEl = panel.querySelector(".detail-back");
  var pulled = null;
  var returnFocus = null;

  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text !== undefined && text !== null) n.textContent = String(text);
    return n;
  }

  // Inline `code` spans; everything else stays text (no HTML injection).
  function richText(parent, text) {
    String(text).split("`").forEach(function (part, i) {
      if (!part) return;
      parent.appendChild(i % 2 ? el("code", null, part) : document.createTextNode(part));
    });
    return parent;
  }

  function laneOf(t, r) {
    if (t.status === "done" || t.status === "superseded") return "done";
    if (t.status === "in_progress") return "active";
    if (t.status === "blocked") return "hold";
    var met = (t.depends_on || []).every(function (d) {
      var dep = index[r + ":" + d];
      return dep && dep.status === "done";
    });
    return met ? "ready" : "waiting";
  }

  function taskRef(r, id, extraLabel) {
    var t = index[r + ":" + id];
    var b = el("button", "ref" + (t ? " lane-" + laneOf(t, r) : " ref-missing"));
    b.type = "button";
    b.appendChild(el("b", null, id));
    if (t) b.appendChild(el("span", null, t.title));
    if (extraLabel) b.appendChild(el("i", null, extraLabel));
    if (t) b.addEventListener("click", function () { openTask(r, id, true); });
    else b.disabled = true;
    return b;
  }

  function section(title, count) {
    var s = el("section", "spec");
    var h = el("h3", "spec-head", title);
    if (count !== undefined) h.appendChild(el("span", "lane-count", count));
    s.appendChild(h);
    backEl.appendChild(s);
    return s;
  }

  function field(grid, label, value, cls) {
    if (value === undefined || value === null || value === "") return;
    var f = el("div", "field" + (cls ? " " + cls : ""));
    f.appendChild(el("span", "field-lbl", label));
    var v = el("span", "field-val");
    if (value instanceof Node) v.appendChild(value); else v.textContent = String(value);
    f.appendChild(v);
    grid.appendChild(f);
  }

  function linkOrText(ref) {
    if (/^https?:\/\//.test(ref)) {
      var a = el("a", null, ref);
      a.href = ref; a.target = "_blank"; a.rel = "noopener";
      return a;
    }
    return el("code", null, ref);
  }

  function list(parent, items, ordered, cls) {
    var l = el(ordered ? "ol" : "ul", cls);
    items.forEach(function (it) { l.appendChild(richText(el("li"), it)); });
    parent.appendChild(l);
  }

  function render(r, t) {
    var proj = projects[r] || {};
    var lane = laneOf(t, r);
    panel.className = "detail is-open lane-" + lane;

    metaEl.textContent = "";
    if (projects.length > 1) metaEl.appendChild(el("span", "detail-repo", proj.project));
    metaEl.appendChild(el("span", "detail-id", t.id));
    metaEl.appendChild(el("span", "detail-lane", lane));
    if (t.status !== "pending") metaEl.appendChild(el("span", "detail-status", t.status.replace("_", " ")));
    titleEl.textContent = t.title;

    backEl.textContent = "";
    if (t.status === "blocked") backEl.appendChild(el("p", "stamp stamp-hold stamp-large", "Hold"));
    if (t.status === "superseded") backEl.appendChild(el("p", "stamp stamp-void stamp-large", "Void"));
    if (t.blocked_reason) richText(backEl.appendChild(el("p", "detail-reason")), t.blocked_reason);

    var grid = el("div", "fields");
    var s = t.scores || {};
    field(grid, "Eff", typeof t.eff === "number" ? t.eff.toFixed(2) : t.eff, "field-num");
    field(grid, "D / B / U", s.d !== undefined ? s.d + " / " + s.b + " / " + s.u : "", "field-num");
    field(grid, "Unlocks", t.unlocks, "field-num");
    field(grid, "Layer", t.dep_layer, "field-num");
    var phaseMeta = (proj.phases || {})[t.phase];
    field(grid, "Phase", t.phase + (phaseMeta && phaseMeta.name ? " · " + phaseMeta.name : ""));
    field(grid, "Bundle", t.bundle);
    field(grid, "Milestone", t.milestone);
    field(grid, "Module", t.module);
    field(grid, "Assignee", t.assignee);
    field(grid, "Model", t.model);
    field(grid, "Target repo", t.target_repo);
    field(grid, "Branch", t.branch);
    field(grid, "Linear", t.linear_id);
    if (t.landing_ref) field(grid, "Landing", linkOrText(t.landing_ref), "field-wide");
    if (t.markers && t.markers.length) field(grid, "Markers", t.markers.join(" · "));
    if (t.domains && t.domains.length) field(grid, "Domains", t.domains.join(" · "));
    var dates = [["created", t.created_at], ["scored", t.scored_at], ["started", t.started_at], ["done", t.done_at]]
      .filter(function (d) { return d[1]; })
      .map(function (d) { return d[0] + " " + d[1]; }).join(" · ");
    field(grid, "Dates", dates, "field-wide");
    backEl.appendChild(grid);

    // Same omission rule as `rmap delegate`: empty lists add no heading.
    // Reviewer checks are hints; this page never runs the commands.
    if (t.context_refs && t.context_refs.length) {
      var read = section("Read first");
      read.appendChild(el("p", "spec-text", "Read these before starting."));
      var refs = el("ul", "paths");
      t.context_refs.forEach(function (ref) {
        refs.appendChild(el("li")).appendChild(linkOrText(ref));
      });
      read.appendChild(refs);
    }
    if (t.body) {
      var body = section("Spec");
      String(t.body).trim().split(/\n{2,}/).forEach(function (para) {
        richText(body.appendChild(el("p", "spec-text")), para);
      });
    }
    if (t.acceptance_criteria && t.acceptance_criteria.length) {
      list(section("Acceptance criteria", t.acceptance_criteria.length), t.acceptance_criteria, true, "criteria");
    }
    if (t.checks && t.checks.length) {
      var checks = section("Reviewer checks");
      checks.appendChild(el(
        "p",
        "spec-text",
        "Hints for the reviewer, not an automated gate. rmap does not execute these commands; the reviewer runs and judges them."
      ));
      list(checks, t.checks, false, "checks");
    }
    if (t.out_of_scope && t.out_of_scope.length) {
      list(section("Out of scope"), t.out_of_scope, false, "oos");
    }
    var files = (t.files_to_modify || []);
    var touches = (t.touches || []).filter(function (f) { return files.indexOf(f) === -1; });
    if (files.length || touches.length) {
      var fs = section("Files");
      if (files.length) { fs.appendChild(el("p", "spec-sub", "Writes")); list(fs, files, false, "paths"); }
      if (touches.length) { fs.appendChild(el("p", "spec-sub", "Touches")); list(fs, touches, false, "paths"); }
    }
    var deps = t.depends_on || [];
    var down = dependents[r + ":" + t.id] || [];
    if (deps.length || down.length) {
      var ws = section("Wiring");
      if (deps.length) {
        ws.appendChild(el("p", "spec-sub", "Depends on"));
        var dl = el("div", "refs");
        deps.forEach(function (d) { dl.appendChild(taskRef(r, String(d))); });
        ws.appendChild(dl);
      }
      if (down.length) {
        ws.appendChild(el("p", "spec-sub", "Directly unblocks"));
        var ul = el("div", "refs");
        down.forEach(function (d) { ul.appendChild(taskRef(r, d)); });
        ws.appendChild(ul);
      }
    }
    if (t.cross_repo && t.cross_repo.length) {
      var cs = el("div", "refs");
      t.cross_repo.forEach(function (c) {
        var target = repoByName[String(c.repo).toLowerCase()];
        var label = c.relation.replace("_", " ") + " · " + c.repo;
        if (target !== undefined && index[target + ":" + c.task_id]) {
          cs.appendChild(taskRef(target, String(c.task_id), label));
        } else {
          var span = el("span", "ref ref-missing");
          span.appendChild(el("b", null, c.task_id));
          span.appendChild(el("span", null, label + (c.linear_id ? " · " + c.linear_id : "")));
          cs.appendChild(span);
        }
      });
      section("Cross-repo").appendChild(cs);
    }
    if (t.attempts && t.attempts.length) {
      var as = section("Prior attempts", t.attempts.length);
      t.attempts.forEach(function (a) {
        var item = el("div", "attempt");
        item.appendChild(el("p", "attempt-meta", a.at + (a.by ? " · " + a.by : "")));
        richText(item.appendChild(el("p", "spec-text")), a.report);
        as.appendChild(item);
      });
    }
    if (t.implemented || t.shipped_in || t.delivered_by || t.verified !== undefined) {
      var os = section("Outcome");
      var og = el("div", "fields");
      field(og, "Shipped in", t.shipped_in);
      field(og, "Delivered by", t.delivered_by);
      field(og, "Verified", t.verified === true ? "yes" + (t.verified_by ? " · " + t.verified_by : "") : t.verified === false ? "no" : "");
      if (t.verification_ref) field(og, "Evidence", linkOrText(t.verification_ref), "field-wide");
      os.appendChild(og);
      if (t.implemented) richText(os.appendChild(el("p", "spec-text")), t.implemented);
    }
    panel.scrollTop = 0;
  }

  function cardsFor(r, id) {
    var sel = '[data-repo="' + r + '"]';
    return document.querySelectorAll(
      '.task-card' + sel + '[data-id="' + CSS.escape(id) + '"], .rack-card' + sel + '[data-task="' + CSS.escape(id) + '"]'
    );
  }

  function openTask(r, id, push) {
    var t = index[r + ":" + id];
    if (!t) return;
    if (!panel.classList.contains("is-open")) returnFocus = document.activeElement;
    if (pulled) pulled.forEach(function (c) { c.classList.remove("is-pulled"); });
    pulled = Array.prototype.slice.call(cardsFor(r, String(id)));
    pulled.forEach(function (c) { c.classList.add("is-pulled"); });
    render(r, t);
    panel.setAttribute("aria-hidden", "false");
    document.body.classList.add("has-detail");
    panel.focus({ preventScroll: true });
    var hash = "#task=" + r + ":" + encodeURIComponent(id);
    if (push && location.hash !== hash) history.pushState(null, "", hash);
    else if (!push) history.replaceState(null, "", hash);
  }

  function closeTask() {
    if (!panel.classList.contains("is-open")) return;
    panel.classList.remove("is-open");
    panel.setAttribute("aria-hidden", "true");
    document.body.classList.remove("has-detail");
    if (pulled) pulled.forEach(function (c) { c.classList.remove("is-pulled"); });
    pulled = null;
    if (location.hash.indexOf("#task=") === 0) history.pushState(null, "", location.pathname + location.search);
    if (returnFocus && returnFocus.focus) returnFocus.focus({ preventScroll: true });
  }

  function fromHash() {
    var m = /^#task=(\d+):(.+)$/.exec(location.hash);
    if (m) openTask(Number(m[1]), decodeURIComponent(m[2]), false);
    else closeTask();
  }

  function activate(target) {
    var card = target.closest(".task-card, .rack-card, .dag-node");
    if (!card) return false;
    var id = card.dataset.id || card.dataset.task;
    openTask(Number(card.dataset.repo || 0), id, true);
    return true;
  }

  document.addEventListener("click", function (e) {
    if (activate(e.target)) e.preventDefault();
  });
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") { closeTask(); return; }
    if ((e.key === "Enter" || e.key === " ") && e.target.matches(".task-card, .rack-card, .dag-node")) {
      e.preventDefault();
      activate(e.target);
    }
  });
  panel.querySelector(".detail-close").addEventListener("click", closeTask);
  window.addEventListener("popstate", fromHash);

  // Print: every folded board opens so the paper copy is complete.
  window.addEventListener("beforeprint", function () {
    document.querySelectorAll("details").forEach(function (d) { d.open = true; });
  });

  // ── Portfolio rails ──────────────────────────────────────────────────────
  var repos = document.getElementById("repos");
  var overlay = document.getElementById("relations-overlay");
  var relNode = document.getElementById("rmap-relations");
  var edges = relNode ? JSON.parse(relNode.textContent || "[]") : [];

  function setOpen(row, open) {
    row.classList.toggle("open", open);
    var hdr = row.querySelector(".repo-row-header");
    if (hdr) hdr.setAttribute("aria-expanded", String(open));
  }

  document.querySelectorAll(".repo-row-header").forEach(function (hdr) {
    var row = hdr.closest(".repo-row");
    hdr.addEventListener("click", function () { setOpen(row, !row.classList.contains("open")); drawCords(); });
    hdr.addEventListener("keydown", function (e) {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); setOpen(row, !row.classList.contains("open")); drawCords(); }
    });
  });

  var search = document.getElementById("repo-search");
  if (search) {
    search.addEventListener("input", function () {
      var q = search.value.trim().toLowerCase();
      document.querySelectorAll(".repo-row").forEach(function (row) {
        row.hidden = !!q && (row.dataset.name || "").toLowerCase().indexOf(q) === -1;
      });
      drawCords();
    });
  }

  var expandBtn = document.getElementById("expand-all");
  if (expandBtn) {
    expandBtn.addEventListener("click", function () {
      var open = expandBtn.getAttribute("aria-pressed") !== "true";
      expandBtn.setAttribute("aria-pressed", String(open));
      document.querySelectorAll(".repo-row").forEach(function (row) { setOpen(row, open); });
      drawCords();
    });
  }

  // Opening a card inside a closed rail (deep link, cross-repo ref) opens it.
  function revealPulled() {
    if (!pulled) return;
    pulled.forEach(function (c) {
      var row = c.closest(".repo-row");
      if (row && !row.classList.contains("open")) { setOpen(row, true); drawCords(); }
      var det = c.closest("details");
      if (det && !det.open) det.open = true;
    });
  }
  var baseOpen = openTask;
  openTask = function (r, id, push) { baseOpen(r, id, push); revealPulled(); };

  function anchorPoint(slug) {
    var row = document.getElementById("repo-" + slug);
    if (!row || row.hidden) return null;
    var a = row.querySelector(".repo-anchor");
    var box = a.getBoundingClientRect();
    var origin = repos.getBoundingClientRect();
    return { x: box.left - origin.left, y: box.top - origin.top + box.height / 2 };
  }

  function drawCords() {
    if (!overlay) return;
    while (overlay.lastChild && overlay.lastChild.tagName !== "defs") overlay.removeChild(overlay.lastChild);
    overlay.setAttribute("width", repos.scrollWidth);
    overlay.setAttribute("height", repos.scrollHeight);
    edges.forEach(function (e) {
      var s = anchorPoint(e.source), t = anchorPoint(e.target);
      if (!s || !t) return;
      var bend = Math.max(14, Math.min(34, Math.abs(t.y - s.y) * 0.2));
      var cx = Math.min(s.x, t.x) - bend;
      var path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.setAttribute("d", "M" + s.x + "," + s.y + " C" + cx + "," + s.y + " " + cx + "," + t.y + " " + t.x + "," + t.y);
      path.setAttribute("class", "rel-edge " + e.relation);
      path.setAttribute("marker-end", "url(#rel-arrow)");
      overlay.appendChild(path);
    });
  }

  window.addEventListener("resize", drawCords);
  drawCords();
  fromHash();
})();
