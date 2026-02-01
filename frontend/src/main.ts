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

function renderMarkdown(text: string) {
  app.innerHTML = md.render(text);
}

function renderJsonl(text: string) {
  const pre = document.createElement("pre");
  pre.className = "jsonl";
  const lines = text.split("\n");
  pre.innerHTML = lines
    .map((line, i) => {
      const safe = escapeHtml(line);
      return `<span class="line"><span class="ln">${i + 1}</span>${safe}</span>`;
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
