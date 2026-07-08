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

/* ----- Boot -------------------------------------------------------------- */
initTheme();

document.addEventListener("DOMContentLoaded", () => {
  markActiveNav();
  wireCopyButtons();

  const toggle = document.querySelector(".theme-toggle");
  if (toggle) toggle.addEventListener("click", toggleTheme);
});

// Expose helpers for page-specific scripts (search.js).
window.Loom = { copyText, wireCopyButtons };
