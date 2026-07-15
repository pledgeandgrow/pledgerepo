import React from "react";

export default function BlogPostPage({ slug }: { slug?: string }) {
  return (
    <div className="page">
      <a href="/blog" className="back-link">← Back to Blog</a>
      <h1>Blog Post: {slug}</h1>
      <p>This is a dynamic route for a blog post with slug: <code>{slug}</code></p>
      <p>The route pattern <code>/blog/:slug</code> was matched from <code>app/blog/[slug]/page.tsx</code>.</p>
    </div>
  );
}
