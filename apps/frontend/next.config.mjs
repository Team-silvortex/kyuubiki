import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.dirname(fileURLToPath(import.meta.url));

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  outputFileTracingRoot: appRoot,
  eslint: {
    // TypeScript and repository-specific guards run independently in CI.
    ignoreDuringBuilds: true,
  },
  async rewrites() {
    return [
      {
        source: "/api/v1/:path*",
        destination: "http://127.0.0.1:4000/api/v1/:path*",
      },
      {
        source: "/api/playground/:path*",
        destination: "http://127.0.0.1:4000/api/playground/:path*",
      },
      {
        source: "/api/health",
        destination: "http://127.0.0.1:4000/api/health",
      },
    ];
  },
};

export default nextConfig;
