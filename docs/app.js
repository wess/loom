/* ==========================================================================
   Loom — shared behaviour: theme toggle, clipboard copy, active nav.
   Vanilla JS, no dependencies. Loaded on every page.
   ========================================================================== */

/* ----- Theme ------------------------------------------------------------- */
// Resolve the initial theme: stored choice wins, else system preference.
// (An inline script in <head> sets the attribute early to avoid a flash;
//  this keeps the two in sync and wires up the toggle.)
const THEME_KEY = "loom-theme";

function systemTheme() {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
}

function initTheme() {
  const stored = localStorage.getItem(THEME_KEY);
  applyTheme(stored || systemTheme());

  // If the user never chose explicitly, follow the OS as it changes.
  window
    .matchMedia("(prefers-color-scheme: dark)")
    .addEventListener("change", (e) => {
      if (!localStorage.getItem(THEME_KEY)) {
        applyTheme(e.matches ? "dark" : "light");
      }
    });
}

function toggleTheme() {
  const current =
    document.documentElement.getAttribute("data-theme") || systemTheme();
  const next = current === "dark" ? "light" : "dark";
  applyTheme(next);
  localStorage.setItem(THEME_KEY, next);
}

/* ----- Clipboard --------------------------------------------------------- */
// Copy text and give brief visual + assistive feedback on the button.
async function copyText(text, button) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // Fallback for insecure contexts / older browsers.
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    try {
      document.execCommand("copy");
    } catch {
      /* give up silently */
    }
    ta.remove();
  }

  if (!button) return;
  const label = button.querySelector(".copy-btn__label");
  const original = label ? label.textContent : "";
  button.classList.add("is-copied");
  if (label) label.textContent = "Copied";
  button.setAttribute("aria-label", "Copied to clipboard");
  window.setTimeout(() => {
    button.classList.remove("is-copied");
    if (label) label.textContent = original;
    button.setAttribute("aria-label", "Copy to clipboard");
  }, 1400);
}

// Any element with [data-copy] copies its value (or the sibling command text).
function wireCopyButtons(root = document) {
  root.querySelectorAll("[data-copy]").forEach((btn) => {
    if (btn.dataset.wired) return;
    btn.dataset.wired = "1";
    btn.addEventListener("click", () => copyText(btn.dataset.copy, btn));
  });
}

/* ----- Active nav -------------------------------------------------------- */
function markActiveNav() {
  const here = location.pathname.split("/").pop() || "index.html";
  document.querySelectorAll(".nav__link[data-page]").forEach((link) => {
    if (link.dataset.page === here) {
      link.setAttribute("aria-current", "page");
    }
  });
}

/* ----- Table of contents ------------------------------------------------- */
// Highlight the section currently in view. The rootMargin collapses the
// viewport to a band around its middle, so exactly one section is "current".
function initToc() {
  const links = Array.from(document.querySelectorAll(".toc a"));
  if (!links.length || !("IntersectionObserver" in window)) return;

  const linkFor = new Map();
  const sections = [];
  for (const link of links) {
    const id = link.getAttribute("href").slice(1);
    const section = document.getElementById(id);
    if (!section) continue;
    linkFor.set(section, link);
    sections.push(section);
  }
  if (!sections.length) return;

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        links.forEach((l) => l.classList.remove("is-active"));
        linkFor.get(entry.target).classList.add("is-active");
      }
    },
    { rootMargin: "-40% 0px -55% 0px", threshold: 0 }
  );
  sections.forEach((s) => observer.observe(s));
}

/* ----- Boot -------------------------------------------------------------- */
initTheme();

document.addEventListener("DOMContentLoaded", () => {
  markActiveNav();
  wireCopyButtons();
  initToc();

  const toggle = document.querySelector(".theme-toggle");
  if (toggle) toggle.addEventListener("click", toggleTheme);
});

// Expose helpers for page-specific scripts (search.js).
window.Loom = { copyText, wireCopyButtons };
