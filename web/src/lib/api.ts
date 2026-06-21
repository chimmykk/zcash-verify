export type Badge = {
  platform: string;
  username: string;
  badge_tier: number;
  badge_name: string;
  badge_image: string;
  verified: boolean;
  expires_at: string;
};

export type RegisterRequest = {
  seed: string;
  account?: number;
  start_height?: number;
  network?: string;
  x?: string;
  zcashforum?: string;
  bluesky?: string;
};

export type RegisterResponse = {
  success: boolean;
  message: string;
  badges: Badge[];
  balance_zat?: number;
  badge_tier?: string;
};

export type ScanResponse = {
  success: boolean;
  message: string;
  balance_zat?: number;
  balance_zec?: string;
  badge_tier?: string;
  badge_name?: string;
  block_height?: number;
  address?: string;
};

async function postJson<T>(path: string, body: unknown): Promise<T> {
  const resp = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  const data = (await resp.json()) as T;
  if (!resp.ok) {
    const message =
      typeof data === "object" &&
      data !== null &&
      "message" in data &&
      typeof (data as { message: unknown }).message === "string"
        ? (data as { message: string }).message
        : `Request failed (${resp.status})`;
    throw new Error(message);
  }
  return data;
}

export async function scanBalance(body: {
  seed: string;
  account?: number;
  start_height?: number;
  network?: string;
}): Promise<ScanResponse> {
  return postJson<ScanResponse>("/api/scan", body);
}

export async function registerBadges(
  body: RegisterRequest
): Promise<RegisterResponse> {
  return postJson<RegisterResponse>("/api/register", body);
}

export async function lookupBadge(
  platform: string,
  username: string
): Promise<Badge | null> {
  const resp = await fetch(
    `/api/badge/${encodeURIComponent(platform)}/${encodeURIComponent(username)}`
  );
  if (resp.status === 404) return null;
  if (!resp.ok) throw new Error(`Lookup failed (${resp.status})`);
  return resp.json();
}

export async function checkHealth(): Promise<boolean> {
  try {
    const resp = await fetch("/api/health");
    return resp.ok;
  } catch {
    return false;
  }
}

export async function fetchChainHeight(): Promise<number | null> {
  try {
    const resp = await fetch("/api/chain-height");
    if (!resp.ok) return null;
    const data = (await resp.json()) as { height?: number };
    return typeof data.height === "number" ? data.height : null;
  } catch {
    return null;
  }
}

export function formatZec(zats: number): string {
  return `${(zats / 100_000_000).toFixed(8)} ZEC`;
}
