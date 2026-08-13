import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type UsageWindow = {
  key: string;
  limitId: string;
  windowDurationMins: number;
  usedPercent: number;
  remainingPercent: number;
  resetsAt: string | null;
  sourceSlot: string;
};

type UsageSnapshot = {
  windows: UsageWindow[];
  status: "loading" | "fresh" | "stale" | "partial" | "unavailable" | "error";
  fetchedAt: number | null;
  lastSuccessfulAt: number | null;
  source: string;
  capability: string;
  diagnosticCode: string | null;
};

export default function App() {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function readUsage() {
    setError(null);
    try {
      const result = await invoke<UsageSnapshot>("read_usage");
      setSnapshot(result);
    } catch (reason) {
      setError(String(reason));
    }
  }

  return (
    <main className="shell">
      <header>
        <p className="eyebrow">CODEX USAGE NOTCH</p>
        <h1>Core Usage Engine</h1>
        <p className="lede">Development verification surface. The formal Notch UI is not implemented yet.</p>
      </header>
      <button type="button" onClick={readUsage}>Read Codex allowance</button>
      {error && <p className="error">{error}</p>}
      {snapshot && (
        <section className="snapshot" aria-live="polite">
          <div className="status-row"><span>Status</span><strong>{snapshot.status}</strong></div>
          {snapshot.windows.map((window) => (
            <article key={window.key}>
              <div><strong>{window.windowDurationMins} min window</strong><span>{window.remainingPercent}% remaining</span></div>
              <div className="bar"><span style={{ width: `${window.remainingPercent}%` }} /></div>
              <small>Reset: {window.resetsAt ?? "not provided"}</small>
            </article>
          ))}
          <small>Source: {snapshot.source} · Capability: {snapshot.capability}</small>
        </section>
      )}
    </main>
  );
}
