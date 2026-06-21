"use client";

import { useEffect, useState } from "react";
import { checkHealth } from "@/lib/api";

export default function ServerStatus() {
  const [online, setOnline] = useState<boolean | null>(null);

  useEffect(() => {
    let active = true;
    checkHealth().then((ok) => {
      if (active) setOnline(ok);
    });
    return () => {
      active = false;
    };
  }, []);

  if (online === null) return null;

  return (
    <span
      className={`rounded-full px-3 py-1 text-xs font-medium ${
        online
          ? "bg-emerald-500/15 text-emerald-300"
          : "bg-red-500/15 text-red-300"
      }`}
    >
      {online
        ? "Badge server online"
        : "Badge server offline — run ./start_server.sh"}
    </span>
  );
}
