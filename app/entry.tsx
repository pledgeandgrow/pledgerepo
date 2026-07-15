import React from "react";
import { createRoot } from "react-dom/client";
import { render } from "/__pledge_router";

const root = createRoot(document.getElementById("root")!);

function renderApp() {
  const pathname = window.location.pathname;
  const element = render(pathname);
  root.render(element);
}

// Initial render
renderApp();

// Handle navigation
window.addEventListener("popstate", renderApp);

// Intercept link clicks for SPA navigation
document.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;
  const anchor = target.closest("a");
  if (anchor && anchor.href.startsWith(window.location.origin) && !anchor.target) {
    e.preventDefault();
    const url = new URL(anchor.href);
    window.history.pushState({}, "", url.pathname);
    renderApp();
  }
});
