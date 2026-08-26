import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const hudRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(hudRoot, "..");

/** @param {string} name */
function rootEnv(name) {
  try {
    const raw = fs.readFileSync(path.join(repoRoot, ".env"), "utf8");
    for (const line of raw.split(/\r?\n/)) {
      if (!line || line.startsWith("#") || !line.includes("=")) continue;
      const i = line.indexOf("=");
      const key = line.slice(0, i).trim();
      if (key !== name) continue;
      return line.slice(i + 1).trim().replace(/^["']|["']$/g, "");
    }
  } catch {
    /* no repo-root .env */
  }
  return "";
}

const cloudWs =
  process.env.NEXT_PUBLIC_JARVIS_CLOUD_WS ||
  process.env.JARVIS_CLOUD_WS ||
  rootEnv("NEXT_PUBLIC_JARVIS_CLOUD_WS") ||
  rootEnv("JARVIS_CLOUD_WS") ||
  "";

const pairingToken =
  process.env.NEXT_PUBLIC_JARVIS_PAIRING_TOKEN ||
  process.env.JARVIS_PAIRING_TOKEN ||
  rootEnv("NEXT_PUBLIC_JARVIS_PAIRING_TOKEN") ||
  rootEnv("JARVIS_PAIRING_TOKEN") ||
  "uMrUM1mJIQFOmGPwMVekLpsjBTwV9QcO1lsX/im7l5I=";

/** @type {import('next').NextConfig} */
const nextConfig = {
  env: {
    NEXT_PUBLIC_JARVIS_CLOUD_WS: cloudWs,
    NEXT_PUBLIC_JARVIS_PAIRING_TOKEN: pairingToken,
  },
  output: "standalone",
  transpilePackages: ["three"],
  outputFileTracingRoot: repoRoot,
  turbopack: {
    root: repoRoot,
  },
};
export default nextConfig;
