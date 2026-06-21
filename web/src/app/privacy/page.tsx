import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Privacy Policy — ZcashVerify",
  description: "Privacy policy for the ZcashVerify registration app and ZcashBadge Chrome extension.",
};

export default function PrivacyPage() {
  return (
    <div className="min-h-full bg-[#0b0f0d]">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_top,rgba(16,185,129,0.12),transparent_45%)]" />

      <header className="relative border-b border-zinc-800/80 bg-black/20 backdrop-blur">
        <div className="mx-auto flex max-w-3xl items-center justify-between px-6 py-5">
          <Link href="/" className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-500 text-lg font-bold text-black">
              Z
            </div>
            <div>
              <h1 className="text-lg font-semibold text-white">ZcashVerify</h1>
              <p className="text-sm text-zinc-400">Privacy Policy</p>
            </div>
          </Link>
          <Link
            href="/"
            className="text-sm text-zinc-400 transition hover:text-emerald-300"
          >
            Back to app
          </Link>
        </div>
      </header>

      <main className="relative mx-auto max-w-3xl px-6 py-10">
        <article className="prose-zcash space-y-8 text-sm leading-7 text-zinc-300">
          <div>
            <h2 className="text-3xl font-semibold tracking-tight text-white">
              Privacy Policy
            </h2>
            <p className="mt-2 text-zinc-500">Last updated: June 21, 2025</p>
          </div>

          <section className="space-y-3">
            <p>
              ZcashVerify and the ZcashBadge Chrome extension (&ldquo;we&rdquo;,
              &ldquo;our&rdquo;, &ldquo;the service&rdquo;) help you prove Zcash
              Orchard wallet ownership and display verified balance badges on supported
              social platforms (X, Bluesky, and Zcash Forum).
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">
              Registration web app
            </h3>
            <p>
              When you use this site to register a badge, you may enter a BIP39 seed
              phrase, scan start height, network, and social handles. That information
              is sent to the badge server configured for this deployment to scan your
              Orchard balance, generate proofs, verify them, and store badge records.
            </p>
            <p>
              Seed phrases are highly sensitive. Only use a badge server you control or
              fully trust. Do not register on a public server operated by someone else.
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">
              Chrome extension
            </h3>
            <p>The ZcashBadge extension may store locally on your device:</p>
            <ul className="list-disc space-y-1 pl-5 text-zinc-400">
              <li>Social platform usernames you linked during registration</li>
              <li>Your configured badge server URL</li>
              <li>Cached badge lookup results</li>
            </ul>
            <p>
              To display badges, the extension sends public usernames and platform names
              to your badge server. It does not send wallet seed phrases, private keys,
              or your full browsing history.
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">
              Badge server and third parties
            </h3>
            <p>
              Proof generation and balance scanning use lightwalletd-compatible
              infrastructure to read Zcash chain data. Verified badge metadata (platform,
              username, tier, expiry, and related proof fields) is stored in the badge
              server database.
            </p>
            <p>
              The extension injects badge UI only on x.com, twitter.com, bsky.app, and
              forum.zcashcommunity.com. We do not sell user data or use it for
              advertising.
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">Data retention</h3>
            <p>
              Data stored by the extension remains on your device until you remove the
              extension or clear its storage. Badge records on the badge server are
              retained according to that server&apos;s configuration and operator.
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">Your choices</h3>
            <p>
              You can stop using the service at any time by uninstalling the extension,
              clearing extension storage, and avoiding further registration. If you
              operate your own badge server, you control deletion of stored badge
              records.
            </p>
          </section>

          <section className="space-y-3">
            <h3 className="text-lg font-semibold text-white">Contact</h3>
            <p>
              For privacy questions, open an issue on{" "}
              <a
                href="https://github.com/chimmykk/zcash-verify/issues"
                className="text-emerald-400 underline-offset-2 hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                github.com/chimmykk/zcash-verify
              </a>
              .
            </p>
          </section>
        </article>
      </main>
    </div>
  );
}
