import RegisterWizard from "@/components/RegisterWizard";
import ServerStatus from "@/components/ServerStatus";
import Link from "next/link";

export default function Home() {
  return (
    <div className="min-h-full bg-[#0b0f0d]">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_top,rgba(16,185,129,0.12),transparent_45%)]" />

      <header className="relative border-b border-zinc-800/80 bg-black/20 backdrop-blur">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-5">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500 text-lg font-bold text-black">
              Z
            </div>
            <div>
              <h1 className="text-lg font-semibold text-white">ZcashVerify</h1>
              <p className="text-sm text-zinc-400">
                Prove your balance. Get your badge.
              </p>
            </div>
          </div>
          <ServerStatus />
        </div>
      </header>

      <main className="relative mx-auto max-w-5xl px-6 py-10">
        <section className="mb-10 max-w-2xl">
          <h2 className="text-3xl font-semibold tracking-tight text-white sm:text-4xl">
            Verify your Zcash identity in minutes
          </h2>
          <p className="mt-4 text-base leading-7 text-zinc-400">
            No CLI. No JSON uploads. Generate your cryptographic proof here, register
            your social handles, and let the Chrome extension display your badge on X,
            Bluesky, and Zcash Forum.
          </p>
        </section>

        <RegisterWizard />

        <section className="mt-16 grid gap-4 sm:grid-cols-3">
          {[
            {
              title: "1. Generate proof",
              body: "We scan your Orchard wallet and create a signed proof bound to your social handles.",
            },
            {
              title: "2. Save to database",
              body: "Verified badges are stored in SQLite. No manual upload to the extension.",
            },
            {
              title: "3. Show everywhere",
              body: "The extension reads badges by username and renders them on supported platforms.",
            },
          ].map((item) => (
            <div
              key={item.title}
              className="rounded-2xl border border-zinc-800 bg-zinc-900/40 p-5"
            >
              <h3 className="font-medium text-white">{item.title}</h3>
              <p className="mt-2 text-sm leading-6 text-zinc-400">{item.body}</p>
            </div>
          ))}
        </section>
      </main>

      <footer className="relative border-t border-zinc-800/80 px-6 py-6 text-center text-sm text-zinc-500">
        <Link href="/privacy" className="transition hover:text-emerald-300">
          Privacy Policy
        </Link>
      </footer>
    </div>
  );
}
