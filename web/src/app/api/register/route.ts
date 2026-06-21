import { BADGE_SERVER_URL, SCAN_TIMEOUT_MS } from "@/lib/server-config";

export const maxDuration = 600;

export async function POST(request: Request) {
  const body = await request.text();

  try {
    const resp = await fetch(`${BADGE_SERVER_URL}/api/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
      signal: AbortSignal.timeout(SCAN_TIMEOUT_MS),
    });

    const data = await resp.json();
    return Response.json(data, { status: resp.status });
  } catch (err) {
    const message =
      err instanceof Error ? err.message : "Could not reach badge server";
    return Response.json(
      {
        success: false,
        message: `Registration failed: ${message}. Is ./start_server.sh running?`,
        badges: [],
      },
      { status: 503 }
    );
  }
}
