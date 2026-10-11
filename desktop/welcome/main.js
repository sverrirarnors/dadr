// The welcome window: open a folder, reopen a recent project, or drop a folder.
// Everything that touches the file system happens in Rust (src/main.rs).

const { invoke } = window.__TAURI__.core;
const { getVersion } = window.__TAURI__.app;
const { getCurrentWebview } = window.__TAURI__.webview;

const openButton = document.getElementById("open");
const openLabel = openButton.querySelector(".label");
const errorBox = document.getElementById("error");
const recentSection = document.querySelector(".recent");
const recentList = document.getElementById("recent");

let busy = false;

async function open(path, name) {
  if (busy) return;
  busy = true;
  document.body.classList.add("busy");
  openButton.disabled = true;
  showError(null);
  if (name) openLabel.textContent = `Opening ${name}…`;
  try {
    // On success the app closes this window.
    await invoke("open_project", { path: path ?? null });
  } catch (error) {
    showError(String(error));
    await renderRecent();
  } finally {
    busy = false;
    document.body.classList.remove("busy");
    openButton.disabled = false;
    openLabel.textContent = "Open project folder…";
  }
}

function showError(message) {
  errorBox.hidden = !message;
  errorBox.textContent = message ?? "";
}

async function renderRecent() {
  const projects = await invoke("recent_projects");
  recentSection.hidden = projects.length === 0;
  recentList.replaceChildren(
    ...projects.map((project) => {
      const item = document.createElement("li");
      item.classList.toggle("missing", project.missing);

      const button = document.createElement("button");
      button.type = "button";
      button.className = "recent-item";
      button.title = project.path;
      const name = document.createElement("span");
      name.className = "name";
      name.textContent = project.name;
      const path = document.createElement("span");
      path.className = "path";
      // Isolated so the right-to-left ellipsis trims the start of the path
      // without reordering its characters.
      path.textContent = `⁨${project.path}⁩`;
      button.append(name, path);
      button.addEventListener("click", () => open(project.path, project.name));

      const forget = document.createElement("button");
      forget.type = "button";
      forget.className = "forget";
      forget.title = "Remove from Recent";
      forget.setAttribute("aria-label", `Remove ${project.name} from Recent`);
      forget.textContent = "×";
      forget.addEventListener("click", async () => {
        await invoke("forget_project", { path: project.path });
        await renderRecent();
      });

      item.append(button, forget);
      return item;
    }),
  );
}

const isMac = navigator.userAgent.includes("Mac");
document.getElementById("shortcut").textContent = isMac ? "⌘O" : "Ctrl+O";
openButton.addEventListener("click", () => open(null));

getCurrentWebview().onDragDropEvent(({ payload }) => {
  if (payload.type === "enter" || payload.type === "over") {
    document.body.classList.add("dragging");
    return;
  }
  document.body.classList.remove("dragging");
  if (payload.type === "drop" && payload.paths.length > 0) {
    const path = payload.paths[0];
    open(path, path.split(/[\\/]/).filter(Boolean).pop());
  }
});

getVersion().then((version) => {
  document.getElementById("version").textContent = `Dadrock ${version}`;
});

renderRecent();
