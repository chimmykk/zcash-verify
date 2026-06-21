import { BADGE_SERVER_URL } from "@/lib/server-config";

export async function GET() {
  try {
    const resp = await fetch(`${BADGE_SERVER_URL}/api/health`, {
      signal: AbortSignal.timeout(5000),
    });
    return new Response(resp.ok ? "ok" : "error", { status: resp.status });
  } catch {
    return new Response("offline", { status: 503 });
  }
}
