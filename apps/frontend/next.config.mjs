/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  allowedDevOrigins: ["127.0.0.1"],
  eslint: {
    // TypeScript and repository-specific guards run independently in CI.
    ignoreDuringBuilds: true,
  },
};

export default nextConfig;
