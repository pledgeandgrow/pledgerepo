import { greet } from "./utils";

function App() {
  const message = greet("Pledge");
  return <div>{message}</div>;
}

const root = document.getElementById("root");
if (root) {
  root.innerHTML = "";
  const container = document.createElement("div");
  container.style.fontFamily = "system-ui, -apple-system, sans-serif";
  container.style.minHeight = "100vh";
  container.style.display = "flex";
  container.style.flexDirection = "column";
  container.style.alignItems = "center";
  container.style.justifyContent = "center";
  container.style.background = "#0a0a0a";
  container.style.color = "#fff";
  container.style.margin = "0";

  const title = document.createElement("h1");
  title.textContent = ".pledge";
  title.style.fontSize = "3rem";
  title.style.fontWeight = "700";
  title.style.letterSpacing = "-0.02em";
  title.style.margin = "0 0 0.5rem 0";
  title.style.background = "linear-gradient(135deg, #6366f1, #a855f7)";
  title.style.webkitBackgroundClip = "text";
  title.style.webkitTextFillColor = "transparent";

  const subtitle = document.createElement("p");
  subtitle.textContent = greet("Pledge");
  subtitle.style.fontSize = "1.25rem";
  subtitle.style.color = "#888";
  subtitle.style.margin = "0";

  container.appendChild(title);
  container.appendChild(subtitle);
  root.appendChild(container);
}

export default App;
