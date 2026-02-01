import MarkdownIt from "markdown-it";
import "./style.css";

const app = document.getElementById("app") as HTMLElement;
const meta = document.getElementById("meta") as HTMLElement;

const md = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true
});

md.validateLink = (url: string) => {
  try {
    const u = new URL(url, window.location.origin);
    return ["http:", "https:", "mailto:"].includes(u.protocol);
  } catch {
    return false;
  }
};

function escapeHtml(input: string): string {
  return input
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/**
 * Syntax highlight JSON content using regex-based tokenization.
 * Returns HTML with span-wrapped tokens for CSS styling.
 */
function highlightJson(line: string): string {
  // First escape HTML entities
  const escaped = escapeHtml(line);

  // Tokenize and wrap with spans
  // Order matters: more specific patterns first

  return escaped
    // Keys (quoted strings followed by colon)
    .replace(
      /(&quot;[^&]*?&quot;)(\s*:)/g,
      '<span class="json-key">$1</span>$2'
    )
    // String values (quoted strings not followed by colon)
    .replace(
      /(&quot;[^&]*?&quot;)(?!\s*:)/g,
      '<span class="json-string">$1</span>'
    )
    // Numbers (integers and floats, including negative and exponential)
    .replace(
      /\b(-?\d+\.?\d*(?:[eE][+-]?\d+)?)\b/g,
      '<span class="json-number">$1</span>'
    )
    // Booleans
    .replace(
      /\b(true|false)\b/g,
      '<span class="json-bool">$1</span>'
    )
    // Null
    .replace(
      /\b(null)\b/g,
      '<span class="json-null">$1</span>'
    )
    // Brackets and braces
    .replace(
      /([{}\[\]])/g,
      '<span class="json-bracket">$1</span>'
    );
}

function renderMarkdown(text: string) {
  const wrapper = document.createElement("div");
  wrapper.className = "prose";
  wrapper.innerHTML = md.render(text);
  app.innerHTML = "";
  app.appendChild(wrapper);
}

function renderJsonl(text: string) {
  const pre = document.createElement("pre");
  pre.className = "jsonl";
  const lines = text.split("\n");

  // Calculate line number width based on total lines
  const lineNumWidth = Math.max(3, String(lines.length).length);

  pre.innerHTML = lines
    .map((line, i) => {
      const highlighted = highlightJson(line);
      const lineNum = String(i + 1).padStart(lineNumWidth, " ");
      return `<span class="line"><span class="ln">${lineNum}</span>${highlighted}</span>`;
    })
    .join("\n");
  app.innerHTML = "";
  app.appendChild(pre);
}

function parseTarget() {
  const path = window.location.pathname.replace(/^\/h\/?/, "");
  const isFront = path === "";
  const uuid = isFront ? null : path.split("/")[0];

  const params = new URLSearchParams(window.location.search);
  const rev = params.get("rev");

  return { isFront, uuid, rev };
}

async function load() {
  const { isFront, uuid, rev } = parseTarget();
  const apiBase = (import.meta as any).env.VITE_API_BASE ?? "";
  const target = isFront ? "/" : `/${uuid}`;
  const url = rev ? `${target}?rev=${encodeURIComponent(rev)}` : target;

  meta.textContent = isFront
    ? "Front page"
    : `UUID: ${uuid}${rev ? ` (rev ${rev})` : ""}`;

  try {
    const res = await fetch(`${apiBase}${url}`);
    if (!res.ok) {
      app.innerHTML = `<div class="error">Failed to load: ${res.status}</div>`;
      return;
    }

    const contentType = res.headers.get("content-type") ?? "";
    const text = await res.text();

    if (contentType.includes("markdown")) {
      renderMarkdown(text);
    } else {
      renderJsonl(text);
    }
  } finally {
    document.body.classList.add("loaded");
  }
}

load().catch((err) => {
  app.innerHTML = `<div class="error">Error: ${String(err)}</div>`;
});
