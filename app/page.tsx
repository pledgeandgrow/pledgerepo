import React from "react";

export default function HomePage() {
  return (
    <div className="page">
      <h1>Welcome to Pledge</h1>
      <p>A blazing-fast bundler with file-based routing.</p>
      <div className="features">
        <div className="feature-card">
          <h3>⚡ Fast Builds</h3>
          <p>Rust-powered transforms with persistent caching.</p>
        </div>
        <div className="feature-card">
          <h3>📁 File-Based Routing</h3>
          <p>Next.js/Expo-style app directory convention.</p>
        </div>
        <div className="feature-card">
          <h3>🔥 HMR</h3>
          <p>Instant hot module replacement via WebSocket.</p>
        </div>
        <div className="feature-card">
          <h3>📦 Code Splitting</h3>
          <p>Automatic chunk splitting per route.</p>
        </div>
      </div>
    </div>
  );
}
