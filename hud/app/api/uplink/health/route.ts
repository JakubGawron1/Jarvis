import { healthUrl, isCloudFallbackHost, normalizeWs } from "../../../lib/uplink";

export const runtime = "nodejs";
export const maxDuration = 60;

/** Server-side GET /health — browser CORS blocks Render's sleeping 404 page. */
export async function GET(req: Request) {
  const ws = new URL(req.url).searchParams.get("ws") ?? "";
  const n = normalizeWs(ws);
  if (!n || !isCloudFallbackHost(n)) {
    return Response.json({ ok: false, error: "forbidden" }, { status: 400 });
  }
  try {
    const r = await fetch(healthUrl(n), {
      cache: "no-store",
      signal: AbortSignal.timeout(55_000),
    });
    const body = (await r.json().catch(() => null)) as { ok?: boolean } | null;
    return Response.json({ ok: Boolean(r.ok && (body?.ok === true || r.ok)), status: r.status });
  } catch {
    return Response.json({ ok: false, status: 0 });
  }
}
