export const BADGE_SERVER_URL =
  process.env.BADGE_SERVER_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "http://localhost:3000";

export const SCAN_TIMEOUT_MS = 10 * 60 * 1000;
