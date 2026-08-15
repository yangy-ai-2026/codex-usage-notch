import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type UsageWindow = {
  key: string;
  windowDurationMins: number;
  remainingPercent: number;
  resetsAt: string | null;
};

type UsageStatus = "loading" | "fresh" | "stale" | "partial" | "unavailable" | "error";

type UsageSnapshot = {
  windows: UsageWindow[];
  status: UsageStatus;
  lastSuccessfulAt: number | null;
};

function formatDuration(minutes: number) {
  if (minutes % 10080 === 0) return `${minutes / 10080} week${minutes === 10080 ? "" : "s"}`;
  if (minutes % 1440 === 0) return `${minutes / 1440} day${minutes === 1440 ? "" : "s"}`;
  if (minutes % 60 === 0) return `${minutes / 60}h`;
  return `${minutes} min`;
}

function formatReset(value: string | null) {
  if (!value) return "Reset not provided";
  const timestamp = Number(value);
  if (!Number.isFinite(timestamp)) return "Reset not provided";
  return `Reset ${new Date(timestamp * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
}

function statusLabel(status: UsageStatus) {
  switch (status) {
    case "stale":
      return "Stale usage";
    case "partial":
      return "Partial usage";
    case "unavailable":
      return "Usage unavailable";
    case "error":
      return "Usage error";
    case "loading":
      return "Reading usage";
    default:
      return "Codex usage";
  }
}

export default function App() {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null);
  const [expanded, setExpanded] = useState(false);
  const hoverTimer = useRef<number | null>(null);

  useEffect(() => {
    let active = true;

    const readUsage = async () => {
      try {
        const result = await invoke<UsageSnapshot>("read_usage");
        if (active) setSnapshot(result);
      } catch {
        if (active) {
          setSnapshot({ windows: [], status: "error", lastSuccessfulAt: null });
        }
      }
    };

    void readUsage();
    const refreshTimer = window.setInterval(() => void readUsage(), 60_000);
    return () => {
      active = false;
      window.clearInterval(refreshTimer);
      if (hoverTimer.current !== null) window.clearTimeout(hoverTimer.current);
    };
  }, []);

  const currentStatus = snapshot?.status ?? "loading";
  const primaryWindow = snapshot?.windows[0];

  function scheduleHover(nextExpanded: boolean, delay: number) {
    if (hoverTimer.current !== null) window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(() => {
      setExpanded(nextExpanded);
      void invoke("set_notch_expanded", { expanded: nextExpanded });
    }, delay);
  }

  return (
    <main
      className={`notch ${expanded ? "notch--expanded" : ""}`}
      onMouseEnter={() => scheduleHover(true, 150)}
      onMouseLeave={() => scheduleHover(false, 250)}
      aria-label="Codex usage notch"
    >
      <div className="notch__collapsed">
        <span className="notch__mark" aria-hidden="true" />
        <span className="notch__title">Codex</span>
        {primaryWindow && currentStatus !== "unavailable" && currentStatus !== "error" ? (
          <strong>{primaryWindow.remainingPercent}%</strong>
        ) : (
          <strong className="notch__muted">—</strong>
        )}
        <span className={`notch__state notch__state--${currentStatus}`}>{statusLabel(currentStatus)}</span>
      </div>

      {expanded && (
        <section className="notch__details" aria-live="polite">
          <div className="notch__details-heading">
            <span>Codex Usage</span>
            <span className="notch__status">{statusLabel(currentStatus)}</span>
          </div>
          {snapshot?.windows.length ? (
            snapshot.windows.map((window) => (
              <article className="usage-window" key={window.key}>
                <div className="usage-window__label">
                  <span>{formatDuration(window.windowDurationMins)}</span>
                  <strong>{window.remainingPercent}%</strong>
                </div>
                <div className="usage-window__bar" aria-hidden="true">
                  <span style={{ width: `${window.remainingPercent}%` }} />
                </div>
                <small>{formatReset(window.resetsAt)}</small>
              </article>
            ))
          ) : (
            <p className="notch__empty">No allowance window is available from Codex.</p>
          )}
        </section>
      )}
    </main>
  );
}
