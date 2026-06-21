export const revalidate = 60;

type BlockchairStats = {
  data?: {
    blocks?: number;
    best_block_height?: number;
  };
};

export async function GET() {
  try {
    const resp = await fetch("https://api.blockchair.com/zcash/stats", {
      signal: AbortSignal.timeout(10000),
      next: { revalidate: 60 },
    });

    if (!resp.ok) {
      return Response.json(
        { message: `Blockchair returned ${resp.status}` },
        { status: 502 }
      );
    }

    const data = (await resp.json()) as BlockchairStats;
    const height = data.data?.blocks ?? data.data?.best_block_height;

    if (!height || height <= 0) {
      return Response.json(
        { message: "Could not read chain height from Blockchair" },
        { status: 502 }
      );
    }

    return Response.json({ height });
  } catch (err) {
    const message = err instanceof Error ? err.message : "Chain height fetch failed";
    return Response.json({ message }, { status: 503 });
  }
}
