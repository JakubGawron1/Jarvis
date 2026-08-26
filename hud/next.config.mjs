import path from "node:path";
import { fileURLToPath } from "node:url";

const hudRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(hudRoot, "..");

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  transpilePackages: ["three"],
  outputFileTracingRoot: repoRoot,
  turbopack: {
    root: repoRoot,
  },
};
export default nextConfig;
