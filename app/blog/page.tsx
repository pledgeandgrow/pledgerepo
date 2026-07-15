import React from "react";

export default function BlogPage() {
  const posts = [
    { slug: "hello-world", title: "Hello World", date: "2025-01-01" },
    { slug: "rust-bundler", title: "Why a Rust Bundler?", date: "2025-02-01" },
    { slug: "file-routing", title: "File-Based Routing Done Right", date: "2025-03-01" },
  ];

  return (
    <div className="page">
      <h1>Blog</h1>
      <div className="blog-list">
        {posts.map((post) => (
          <a key={post.slug} href={`/blog/${post.slug}`} className="blog-item">
            <h3>{post.title}</h3>
            <span className="blog-date">{post.date}</span>
          </a>
        ))}
      </div>
    </div>
  );
}
