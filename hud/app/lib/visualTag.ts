import type { VisualSpec } from "../components/VisualStage";

/** Pull `[[visual:{...}]]` out of a reply so the log never dumps raw JSON. */
export function takeVisual(raw: string): { text: string; spec: VisualSpec | null } {
  const marker = "[[visual:";
  const start = raw.indexOf(marker);
  if (start < 0) return { text: raw.trim(), spec: null };

  const jsonStart = raw.indexOf("{", start);
  if (jsonStart < 0) {
    return { text: (raw.slice(0, start) + raw.slice(start + marker.length)).trim(), spec: null };
  }

  let depth = 0;
  let inStr = false;
  let escape = false;
  let jsonEnd = -1;
  for (let i = jsonStart; i < raw.length; i++) {
    const c = raw[i];
    if (inStr) {
      if (escape) {
        escape = false;
        continue;
      }
      if (c === "\\") {
        escape = true;
        continue;
      }
      if (c === '"') inStr = false;
      continue;
    }
    if (c === '"') {
      inStr = true;
      continue;
    }
    if (c === "{") depth++;
    if (c === "}") {
      depth--;
      if (depth === 0) {
        jsonEnd = i;
        break;
      }
    }
  }

  let spec: VisualSpec | null = null;
  if (jsonEnd >= 0) {
    try {
      spec = JSON.parse(raw.slice(jsonStart, jsonEnd + 1)) as VisualSpec;
    } catch {
      spec = null;
    }
  }

  let end = jsonEnd >= 0 ? jsonEnd + 1 : raw.length;
  const rest = raw.slice(end).trimStart();
  if (rest.startsWith("]]")) {
    end += raw.slice(end).length - rest.length + 2;
  }
  return { text: (raw.slice(0, start) + raw.slice(end)).trim(), spec };
}
