import React, { useState } from "react";

export default function Layout({ children }: { children: React.ReactNode }) {
  const [navOpen, setNavOpen] = useState(false);

  return (
    <div className="app-layout">
      <nav className="app-nav">
        <a href="/" className="app-nav-brand">⚓ Pledge</a>
        <button className="app-nav-toggle" onClick={() => setNavOpen(!navOpen)}>
          ☰
        </button>
        <div className={`app-nav-links ${navOpen ? "open" : ""}`}>
          <a href="/">Home</a>
          <a href="/about">About</a>
          <a href="/blog">Blog</a>
          <a href="/tasks">Tasks</a>
        </div>
      </nav>
      <main className="app-content">{children}</main>
      <footer className="app-footer">
        <p>Built with Pledge — file-based routing</p>
      </footer>
    </div>
  );
}
