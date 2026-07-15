import React from "react";

export default function NotFoundPage() {
  return (
    <div className="page not-found">
      <h1>404</h1>
      <p>Page not found.</p>
      <a href="/" className="back-link">← Go home</a>
    </div>
  );
}
