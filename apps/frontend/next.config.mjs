/** @type {import('next').NextConfig} */
const nextConfig = {
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
