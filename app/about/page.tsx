import React from "react";

export default function AboutPage() {
  return (
    <div className="page">
      <h1>About</h1>
      <p>Pledge is a next-generation bundler written in Rust.</p>
      <h2>Why Pledge?</h2>
      <ul>
        <li>10x faster than JavaScript bundlers</li>
        <li>Built-in file-based routing</li>
        <li>First-class React, Vue, Svelte, and Solid support</li>
        <li>Persistent caching for incremental builds</li>
        <li>Zero-config setup with sensible defaults</li>
      </ul>
    </div>
  );
}
