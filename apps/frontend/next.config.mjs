/** @type {import('next').NextConfig} */
const nextConfig = {
  async rewrites() {
    return [
      {
        source: "/api/v1/fem/axial-bar/jobs",
        destination: "http://127.0.0.1:4000/api/v1/fem/axial-bar/jobs",
      },
      {
        source: "/api/v1/fem/truss-2d/jobs",
        destination: "http://127.0.0.1:4000/api/v1/fem/truss-2d/jobs",
      },
      {
        source: "/api/v1/jobs/:path*",
        destination: "http://127.0.0.1:4000/api/v1/jobs/:path*",
      },
      {
        source: "/api/v1/jobs",
        destination: "http://127.0.0.1:4000/api/v1/jobs",
      },
      {
        source: "/api/playground/run",
        destination: "http://127.0.0.1:4000/api/playground/run",
      },
      {
        source: "/api/health",
        destination: "http://127.0.0.1:4000/api/health",
      },
    ];
  },
};

export default nextConfig;
