import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState } from "react";

const appWindow = getCurrentWindow();

type AirPodsStatus = {
  configured: boolean;
  address: string | null;
  name: string | null;
  connected: boolean;
  batteryPercentage: number | null;
  micGainDb: number;
  limiterDbfs: number;
};

type ServiceStatus = {
  activeState: string;
  subState: string;
  enabled: string;
  mainPid: number | null;
};

const emptyService: ServiceStatus = {
  activeState: "unknown",
  subState: "unknown",
  enabled: "unknown",
  mainPid: null,
};

function App() {
  const [airpods, setAirpods] = useState<AirPodsStatus | null>(null);
  const [service, setService] = useState<ServiceStatus>(emptyService);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const [airpodsResult, serviceResult] = await Promise.allSettled([
      invoke<AirPodsStatus>("get_airpods_status"),
      invoke<ServiceStatus>("get_service_status"),
    ]);
    const errors: string[] = [];
    if (airpodsResult.status === "fulfilled") {
      setAirpods(airpodsResult.value);
    } else {
      errors.push(String(airpodsResult.reason));
    }
    if (serviceResult.status === "fulfilled") {
      setService(serviceResult.value);
    } else {
      errors.push(String(serviceResult.reason));
    }
    setError(errors.length > 0 ? errors.join(" · ") : null);
    setLoading(false);
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 15_000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const controlService = async (command: "start_service" | "stop_service" | "restart_service") => {
    setBusy(true);
    try {
      await invoke(command);
      await new Promise((resolve) => window.setTimeout(resolve, 500));
      await refresh();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const serviceActive = service.activeState === "active";
  const battery = airpods?.batteryPercentage;

  return (
    <div className="window-frame">
      <div className="titlebar">
        <div
          className="titlebar-drag"
          data-tauri-drag-region
          onDoubleClick={() => void appWindow.toggleMaximize()}
        >
          <span className="titlebar-mark" data-tauri-drag-region aria-hidden="true" />
          <span data-tauri-drag-region>AirPods Hi-Res Mic</span>
        </div>
        <div className="window-controls">
          <button aria-label="Minimize" title="Minimize" onClick={() => void appWindow.minimize()}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 11.5h10" />
            </svg>
          </button>
          <button aria-label="Maximize" title="Maximize" onClick={() => void appWindow.toggleMaximize()}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <rect x="3.5" y="3.5" width="9" height="9" rx="1" />
            </svg>
          </button>
          <button className="close" aria-label="Close" title="Hide to tray" onClick={() => void appWindow.close()}>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="m4 4 8 8m0-8-8 8" />
            </svg>
          </button>
        </div>
      </div>

      <main className="app-shell">
      <header className="hero">
        <div className="hero-icon" aria-hidden="true">
          <span />
        </div>
        <div>
          <p className="eyebrow">AIRPODS FOR LINUX</p>
          <h1>Hi-Res Mic</h1>
          <p className="subtitle">AACP microphone with A2DP/AAC playback preserved</p>
        </div>
        <button className="refresh" onClick={() => void refresh()} disabled={loading} title="Refresh status">
          ↻
        </button>
      </header>

      {error && <div className="error-banner">{error}</div>}

      <section className="status-grid" aria-busy={loading}>
        <article className="card device-card">
          <div className="card-heading">
            <div>
              <p className="label">AIRPODS</p>
              <h2>{airpods?.name ?? "Configured device"}</h2>
            </div>
            <span className={`status-dot ${airpods?.connected ? "online" : "offline"}`}>
              {airpods?.connected ? "Connected" : "Disconnected"}
            </span>
          </div>
          <p className="device-address">{airpods?.address ?? "No device configured"}</p>

          <div className="battery-row">
            <div className="battery-icon" aria-hidden="true">
              <div style={{ width: battery == null ? "0%" : `${battery}%` }} />
            </div>
            <div>
              <strong>{battery == null ? "Unavailable" : `${battery}%`}</strong>
              <span>BlueZ Battery1</span>
            </div>
          </div>
          {battery == null && (
            <p className="hint">This AirPods connection does not currently expose battery data through BlueZ.</p>
          )}
        </article>

        <article className="card">
          <div className="card-heading">
            <div>
              <p className="label">MICROPHONE SERVICE</p>
              <h2>AirPodsHiRes</h2>
            </div>
            <span className={`status-dot ${serviceActive ? "online" : "offline"}`}>
              {service.activeState}
            </span>
          </div>

          <dl className="metrics">
            <div>
              <dt>Gain</dt>
              <dd>{airpods ? `+${airpods.micGainDb.toFixed(0)} dB` : "—"}</dd>
            </div>
            <div>
              <dt>Limiter</dt>
              <dd>{airpods ? `${airpods.limiterDbfs.toFixed(0)} dBFS` : "—"}</dd>
            </div>
            <div>
              <dt>Startup</dt>
              <dd>{service.enabled}</dd>
            </div>
          </dl>

          <div className="actions">
            <button
              className="primary"
              disabled={busy || serviceActive || !airpods?.configured}
              onClick={() => void controlService("start_service")}
            >
              Start
            </button>
            <button disabled={busy || !serviceActive} onClick={() => void controlService("restart_service")}>
              Restart
            </button>
            <button disabled={busy || !serviceActive} onClick={() => void controlService("stop_service")}>
              Stop
            </button>
          </div>
        </article>
      </section>

        <footer>
          <span className="safety-mark">✓</span>
          Bluetooth profile and A2DP/AAC codec are never changed by this UI.
        </footer>
      </main>
    </div>
  );
}

export default App;
