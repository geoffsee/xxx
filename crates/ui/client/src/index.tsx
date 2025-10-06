import { serve } from "bun";
import index from "./index.html";

const REPL_API_URL = process.env.REPL_API_URL || "http://localhost:3001";

function upstreamFrom(req: Request, fallback: string) {
  const url = new URL(req.url);
  const override = url.searchParams.get("upstream");
  try {
    return override && override.trim().length > 0 ? override : fallback;
  } catch {
    return fallback;
  }
}

const server = serve({
  routes: {
    // Serve index.html for all unmatched routes.
    "/*": index,

    "/api/hello": {
      async GET(req) {
        return Response.json({
          message: "Hello, world!",
          method: "GET",
        });
      },
      async PUT(req) {
        return Response.json({
          message: "Hello, world!",
          method: "PUT",
        });
      },
    },

    // Proxy to the REPL API to avoid CORS in the browser
    "/api/repl/languages": {
      async GET(req) {
        const base = upstreamFrom(req, REPL_API_URL);
        const upstream = `${base}/api/repl/languages`;
        const res = await fetch(upstream);
        return new Response(await res.text(), {
          status: res.status,
          headers: { "content-type": res.headers.get("content-type") ?? "application/json" },
        });
      },
    },
    
    "/api/repl/execute": {
      async POST(req) {
        const base = upstreamFrom(req, REPL_API_URL);
        const upstream = `${base}/api/repl/execute`;
        let body: unknown = undefined;
        try {
          body = await req.json();
        } catch (_) {
          body = undefined;
        }
        const res = await fetch(upstream, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: body ? JSON.stringify(body) : undefined,
        });
        return new Response(await res.text(), {
          status: res.status,
          headers: { "content-type": res.headers.get("content-type") ?? "application/json" },
        });
      },
    },

    "/api/hello/:name": async (req) => {
      const name = req.params.name;
      return Response.json({
        message: `Hello, ${name}!`,
      });
    },
  },

  development: process.env.NODE_ENV !== "production",
});

console.log(`🚀 Server running at ${server.url}`);
