/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  eslint: {
    // TypeScript and repository-specific guards run independently in CI.
    ignoreDuringBuilds: true,
  },
};

export default nextConfig;
