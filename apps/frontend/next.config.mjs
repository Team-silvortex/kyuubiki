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
        source: "/api/v1/fem/truss-3d/jobs",
        destination: "http://127.0.0.1:4000/api/v1/fem/truss-3d/jobs",
      },
      {
        source: "/api/v1/fem/plane-triangle-2d/jobs",
        destination: "http://127.0.0.1:4000/api/v1/fem/plane-triangle-2d/jobs",
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
        source: "/api/v1/projects/:path*",
        destination: "http://127.0.0.1:4000/api/v1/projects/:path*",
      },
      {
        source: "/api/v1/projects",
        destination: "http://127.0.0.1:4000/api/v1/projects",
      },
      {
        source: "/api/v1/models/:path*",
        destination: "http://127.0.0.1:4000/api/v1/models/:path*",
      },
      {
        source: "/api/v1/model-versions/:path*",
        destination: "http://127.0.0.1:4000/api/v1/model-versions/:path*",
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
