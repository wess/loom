/* ==========================================================================
   Loom — browse.html search + browse behaviour.
   Loads skills.json, does debounced client-side filtering, renders cards,
   and drives the detail modal. Depends on window.Loom from app.js.
   ========================================================================== */

(function () {
  "use strict";

  var els = {
    q: document.getElementById("q"),
    filters: document.getElementById("filters"),
    count: document.getElementById("count"),
    results: document.getElementById("results"),
    modal: document.getElementById("modal"),
    modalBody: document.getElementById("modalBody"),
    modalClose: document.getElementById("modalClose"),
  };

  var state = {
    all: [], // every skill
    query: "",
    agent: null, // active compatibility filter, or null for "all"
    lastFocused: null, // element to restore focus to on modal close
    open: null, // name of the skill whose detail modal is open, or null
  };

  /* ----- Helpers --------------------------------------------------------- */
  function esc(s) {
    return String(s == null ? "" : s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  function debounce(fn, ms) {
    var t;
    return function () {
      clearTimeout(t);
      t = setTimeout(fn, ms);
    };
  }

  function copyIcon() {
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>';
  }

  function linkIcon() {
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M10 13a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1.5 1.5"/><path d="M14 11a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1.5-1.5"/></svg>';
  }

  function fileIcon() {
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/></svg>';
  }

  /* Absolute, shareable URL for a skill, correct in any environment. */
  function shareUrl(name) {
    return (
      location.origin +
      location.pathname +
      "?skill=" +
      encodeURIComponent(name)
    );
  }

  /* ----- Manifest reconstruction ----------------------------------------- */
  // A scalar needs single-quoting when it could be misread as YAML: it
  // contains a colon+space, or opens with an indicator/whitespace char.
  function needsQuote(v) {
    return /: /.test(v) || /[:#]$/.test(v) || /^[\s&*!|>%@`"'#\-?:,\[\]{}]/.test(v);
  }
  function yamlStr(v) {
    v = String(v == null ? "" : v);
    return needsQuote(v) ? "'" + v.replace(/'/g, "''") + "'" : v;
  }
  function buildManifest(s) {
    var lines = [];
    lines.push("name: " + s.name);
    lines.push("version: " + s.version);
    lines.push("description: " + yamlStr(s.description || ""));
    if (s.homepage) lines.push("homepage: " + s.homepage);
    if (s.license) lines.push("license: " + s.license);
    if (s.authors && s.authors.length) {
      lines.push("authors:");
      s.authors.forEach(function (a) {
        lines.push("  - " + a);
      });
    }
    if (s.keywords && s.keywords.length) {
      lines.push("keywords: [" + s.keywords.join(", ") + "]");
    }
    if (s.compatibility && s.compatibility.length) {
      lines.push("compatibility: [" + s.compatibility.join(", ") + "]");
    }
    lines.push("source:");
    lines.push("  type: git");
    lines.push("  url: " + s.source);
    lines.push("install:");
    lines.push("  entry: SKILL.md");
    return lines.join("\n");
  }

  /* ----- Filtering ------------------------------------------------------- */
  function matches(skill) {
    if (state.agent && (skill.compatibility || []).indexOf(state.agent) === -1) {
      return false;
    }
    var q = state.query.trim().toLowerCase();
    if (!q) return true;
    var hay = [
      skill.name,
      skill.description,
      (skill.keywords || []).join(" "),
      (skill.authors || []).join(" "),
    ]
      .join(" ")
      .toLowerCase();
    // Every whitespace-separated term must appear somewhere.
    return q.split(/\s+/).every(function (term) {
      return hay.indexOf(term) !== -1;
    });
  }

  /* ----- Rendering ------------------------------------------------------- */
  function cardHTML(s) {
    var install = "loom install " + s.name;
    var badges = (s.compatibility || [])
      .map(function (a) {
        return '<span class="badge">' + esc(a) + "</span>";
      })
      .join("");
    return (
      '<button class="card" type="button" data-name="' + esc(s.name) + '">' +
      '<div class="card__top"><span class="card__name">' + esc(s.name) +
      '</span><span class="card__ver">v' + esc(s.version) + "</span></div>" +
      '<p class="card__desc">' + esc(s.description) + "</p>" +
      '<div class="card__badges">' + badges + "</div>" +
      '<div class="card__install"><span class="cmd__prompt" aria-hidden="true">$</span>' +
      '<span class="cmd__text">' + esc(install) + "</span>" +
      '<span class="copy-btn" role="button" tabindex="0" data-copy="' + esc(install) +
      '" aria-label="Copy install command">' + copyIcon() + "</span></div>" +
      "</button>"
    );
  }

  function render() {
    var list = state.all.filter(matches);

    els.count.textContent =
      list.length +
      (list.length === 1 ? " skill" : " skills") +
      (state.query || state.agent ? " match your filter" : " available");

    if (!list.length) {
      els.results.innerHTML =
        '<div class="state"><h3>No skills match</h3><p>Nothing found for ' +
        (state.query ? "<code>" + esc(state.query) + "</code>" : "this filter") +
        ". Try a broader term.</p></div>";
      return;
    }

    els.results.innerHTML = '<div class="grid">' + list.map(cardHTML).join("") + "</div>";
    window.Loom.wireCopyButtons(els.results);

    els.results.querySelectorAll(".card").forEach(function (card) {
      card.addEventListener("click", function (e) {
        // Ignore clicks that landed on the copy control.
        if (e.target.closest(".copy-btn")) return;
        openModal(card.dataset.name);
      });
    });

    // Support copy on the keyboard-focusable copy spans.
    els.results.querySelectorAll(".copy-btn[role='button']").forEach(function (b) {
      b.addEventListener("keydown", function (e) {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          b.click();
        }
      });
    });
  }

  /* ----- Filter chips ---------------------------------------------------- */
  function buildChips() {
    var seen = {};
    state.all.forEach(function (s) {
      (s.compatibility || []).forEach(function (a) {
        seen[a] = (seen[a] || 0) + 1;
      });
    });
    var agents = Object.keys(seen).sort();

    var html =
      '<button class="chip" type="button" data-agent="" aria-pressed="true">all</button>';
    html += agents
      .map(function (a) {
        return (
          '<button class="chip" type="button" data-agent="' + esc(a) +
          '" aria-pressed="false">' + esc(a) + "</button>"
        );
      })
      .join("");
    els.filters.insertAdjacentHTML("beforeend", html);

    els.filters.querySelectorAll(".chip").forEach(function (chip) {
      chip.addEventListener("click", function () {
        state.agent = chip.dataset.agent || null;
        els.filters.querySelectorAll(".chip").forEach(function (c) {
          c.setAttribute("aria-pressed", String(c === chip));
        });
        render();
      });
    });
  }

  /* ----- Modal ----------------------------------------------------------- */
  function modalHTML(s) {
    var install = "loom install " + s.name;
    var badges = (s.compatibility || [])
      .map(function (a) {
        return '<span class="badge">' + esc(a) + "</span>";
      })
      .join("");

    var rows = [];
    if (s.authors && s.authors.length)
      rows.push(["Authors", esc(s.authors.join(", "))]);
    rows.push(["License", esc(s.license || "—")]);
    if (s.homepage)
      rows.push([
        "Homepage",
        '<a href="' + esc(s.homepage) + '" rel="noopener">' + esc(s.homepage) + "</a>",
      ]);
    if (s.source)
      rows.push([
        "Source",
        '<a href="' + esc(s.source) + '" rel="noopener">' + esc(s.source) + "</a>",
      ]);
    if (s.keywords && s.keywords.length)
      rows.push(["Keywords", esc(s.keywords.join(", "))]);

    var deflist = rows
      .map(function (r) {
        return "<dt>" + r[0] + "</dt><dd>" + r[1] + "</dd>";
      })
      .join("");

    var manifest = buildManifest(s);
    var url = shareUrl(s.name);

    return (
      '<h2 class="modal__title" id="modalTitle">' + esc(s.name) + "</h2>" +
      '<p class="modal__meta">v' + esc(s.version) + "</p>" +
      '<p class="modal__desc">' + esc(s.description) + "</p>" +
      '<div class="modal__badges">' + badges + "</div>" +
      '<dl class="deflist">' + deflist + "</dl>" +
      '<div class="cmd">' +
      '<span class="cmd__prompt" aria-hidden="true">$</span>' +
      '<span class="cmd__text">' + esc(install) + "</span>" +
      '<button class="copy-btn" type="button" data-copy="' + esc(install) +
      '" aria-label="Copy install command">' + copyIcon() +
      '<span class="copy-btn__label">Copy</span></button></div>' +
      '<div class="modal__actions">' +
      '<button class="btn btn--ghost btn--sm" type="button" data-copy="' +
      esc(url) + '" aria-label="Copy shareable link">' + linkIcon() +
      '<span class="copy-btn__label">Copy link</span></button>' +
      '<button class="btn btn--ghost btn--sm" type="button" data-copy="' +
      esc(manifest) + '" aria-label="Copy ' + esc(s.name) +
      '.yml manifest">' + fileIcon() +
      '<span class="copy-btn__label">Copy manifest</span></button>' +
      "</div>"
    );
  }

  // push: add a history entry (default). Pass false when the navigation was
  // itself driven by history (deep-link load or a popstate event).
  function openModal(name, push) {
    var s = state.all.filter(function (x) {
      return x.name === name;
    })[0];
    if (!s) return;

    var wasOpen = els.modal.classList.contains("is-open");
    if (!wasOpen) state.lastFocused = document.activeElement;
    els.modalBody.innerHTML = modalHTML(s);
    window.Loom.wireCopyButtons(els.modalBody);
    els.modal.hidden = false;
    els.modal.classList.add("is-open");
    state.open = name;

    if (push !== false && history.pushState) {
      history.pushState(
        { skill: name },
        "",
        location.pathname + "?skill=" + encodeURIComponent(name)
      );
    }
    els.modalClose.focus();
    document.addEventListener("keydown", onKeydown);
  }

  function closeModal(push) {
    if (!els.modal.classList.contains("is-open")) return;
    els.modal.classList.remove("is-open");
    els.modal.hidden = true;
    state.open = null;
    document.removeEventListener("keydown", onKeydown);
    if (push !== false && history.pushState)
      history.pushState({}, "", location.pathname);
    if (state.lastFocused && state.lastFocused.focus) state.lastFocused.focus();
  }

  // Back/forward: reconcile the modal with the URL's ?skill= param.
  window.addEventListener("popstate", function () {
    var name = new URLSearchParams(location.search).get("skill");
    if (name) {
      openModal(name, false);
    } else {
      closeModal(false);
    }
  });

  function onKeydown(e) {
    if (e.key === "Escape") {
      closeModal();
      return;
    }
    // Trap focus inside the panel.
    if (e.key === "Tab") {
      var f = els.modal.querySelectorAll(
        'a[href], button, [tabindex]:not([tabindex="-1"])'
      );
      if (!f.length) return;
      var first = f[0];
      var last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  els.modalClose.addEventListener("click", closeModal);
  els.modal.addEventListener("click", function (e) {
    if (e.target === els.modal) closeModal(); // click the backdrop
  });

  /* ----- Search input ---------------------------------------------------- */
  var onInput = debounce(function () {
    state.query = els.q.value;
    render();
  }, 160);
  els.q.addEventListener("input", onInput);

  /* ----- Load ------------------------------------------------------------ */
  fetch("skills.json")
    .then(function (r) {
      if (!r.ok) throw new Error("http " + r.status);
      return r.json();
    })
    .then(function (data) {
      state.all = (data.skills || []).slice().sort(function (a, b) {
        return a.name.localeCompare(b.name);
      });
      buildChips();
      render();

      // Deep link: ?skill=<name> opens that skill's detail on load.
      // Fall back to a legacy #<name> hash if present.
      var name = new URLSearchParams(location.search).get("skill");
      if (!name && location.hash) {
        name = decodeURIComponent(location.hash.replace(/^#/, ""));
      }
      if (name) {
        openModal(name, false);
      } else {
        els.q.focus();
      }
    })
    .catch(function () {
      els.count.textContent = "";
      els.results.innerHTML =
        '<div class="state"><h3>Couldn\'t load the skill index</h3>' +
        "<p>The index (<code>skills.json</code>) is served over http on GitHub Pages. " +
        "If you opened this file directly from disk, the browser may block the request.</p></div>";
    });
})();
