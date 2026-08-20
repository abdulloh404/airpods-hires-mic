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
};

type AirPodsBattery = {
  left: number | null;
  right: number | null;
  case: number | null;
  leftCharging: boolean;
  rightCharging: boolean;
  caseCharging: boolean;
  modelId: string;
  bleAddress: string;
  rssi: number;
};

type ServiceStatus = {
  activeState: string;
  subState: string;
  enabled: string;
  mainPid: number | null;
};

type MicSettings = { micGainDb: number; limiterDbfs: number };
type BluetoothDevice = { address: string; name: string };

const emptyService: ServiceStatus = {
  activeState: "unknown",
  subState: "unknown",
  enabled: "unknown",
  mainPid: null,
};
const defaultSettings: MicSettings = { micGainDb: 18, limiterDbfs: -3 };

function BatteryItem({ kind, label, value, charging }: {
  kind: "left" | "right" | "case";
  label: string;
  value: number | null;
  charging: boolean;
}) {
  const level = value ?? 0;
  const low = value !== null && value <= 20;
  return (
    <div className={`battery-item ${kind}`}>
      <div className="device-glyph" aria-hidden="true">
        {kind === "case" ? (
          <svg viewBox="0 0 64 64"><rect x="8" y="18" width="48" height="34" rx="14"/><path d="M9 31h46"/><circle cx="32" cy="38" r="1.5"/></svg>
        ) : (
          <svg viewBox="0 0 64 64"><path d={kind === "left" ? "M38 11c-11 0-17 7-17 16 0 6 3 10 8 12v13c0 4 2 7 6 7s6-3 6-7V31c5-1 8-5 8-10 0-6-4-10-11-10Z" : "M26 11c11 0 17 7 17 16 0 6-3 10-8 12v13c0 4-2 7-6 7s-6-3-6-7V31c-5-1-8-5-8-10 0-6 4-10 11-10Z"}/></svg>
        )}
      </div>
      <div className="battery-copy"><span>{label}</span><strong className={low ? "low" : ""}>{value === null ? "—" : `${value}%`}</strong></div>
      <div className="level-track" aria-hidden="true"><div className={low ? "low" : ""} style={{ width: `${level}%` }} /></div>
      {charging && <span className="charging" title="Charging">ϟ</span>}
    </div>
  );
}

function Slider({ label, description, value, min, max, unit, onChange }: {
  label: string;
  description: string;
  value: number;
  min: number;
  max: number;
  unit: string;
  onChange: (value: number) => void;
}) {
  const progress = ((value - min) / (max - min)) * 100;
  return (
    <label className="slider-setting">
      <span className="slider-copy">
        <span><strong>{label}</strong><small>{description}</small></span>
        <output>{value > 0 ? "+" : ""}{value.toFixed(0)} {unit}</output>
      </span>
      <input type="range" min={min} max={max} step="1" value={value}
        style={{ "--range-progress": `${progress}%` } as React.CSSProperties}
        onChange={(event) => onChange(Number(event.target.value))} />
      <span className="range-labels"><span>{min}</span><span>{max}</span></span>
    </label>
  );
}

function App() {
  const [airpods, setAirpods] = useState<AirPodsStatus | null>(null);
  const [battery, setBattery] = useState<AirPodsBattery | null>(null);
  const [service, setService] = useState<ServiceStatus>(emptyService);
  const [settings, setSettings] = useState<MicSettings>(defaultSettings);
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState("");
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanMessage, setScanMessage] = useState("กำลังค้นหาข้อมูลแบตเตอรี่…");
  const [saved, setSaved] = useState(false);

  const refreshStatus = useCallback(async () => {
    const [airpodsResult, serviceResult] = await Promise.allSettled([
      invoke<AirPodsStatus>("get_airpods_status"),
      invoke<ServiceStatus>("get_service_status"),
    ]);
    const errors: string[] = [];
    if (airpodsResult.status === "fulfilled") setAirpods(airpodsResult.value);
    else errors.push(String(airpodsResult.reason));
    if (serviceResult.status === "fulfilled") setService(serviceResult.value);
    else errors.push(String(serviceResult.reason));
    setError(errors.length ? errors.join(" · ") : null);
    setLoading(false);
  }, []);

  const loadSettings = useCallback(async () => {
    try {
      setSettings(await invoke<MicSettings>("get_mic_settings"));
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  const loadDevices = useCallback(async () => {
    try {
      const connected = await invoke<BluetoothDevice[]>("get_connected_bluetooth_devices");
      setDevices(connected);
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  const scanBattery = useCallback(async () => {
    setScanning(true);
    setScanMessage("กำลังสแกน BLE ใกล้เครื่อง…");
    try {
      const result = await invoke<AirPodsBattery>("scan_airpods_battery");
      setBattery(result);
      setScanMessage(`อัปเดตจาก BLE · ${result.rssi} dBm`);
    } catch (cause) {
      setBattery(null);
      setScanMessage(String(cause));
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    void loadSettings();
    void loadDevices();
    void scanBattery();
    const statusTimer = window.setInterval(() => void refreshStatus(), 15_000);
    return () => {
      window.clearInterval(statusTimer);
    };
  }, [loadDevices, loadSettings, refreshStatus, scanBattery]);

  const controlService = async (command: "start_service" | "stop_service" | "restart_service") => {
    setBusy(true);
    setError(null);
    try {
      await invoke(command);
      await new Promise((resolve) => window.setTimeout(resolve, 500));
      await refreshStatus();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const saveSettings = async () => {
    setBusy(true);
    setSaved(false);
    setError(null);
    try {
      const persisted = await invoke<MicSettings>("save_mic_settings", { settings });
      setSettings(persisted);
      if (service.activeState === "active") await invoke("restart_service");
      setSaved(true);
      await refreshStatus();
      window.setTimeout(() => setSaved(false), 2500);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const saveDevice = async () => {
    if (!selectedDevice) return;
    setBusy(true);
    setError(null);
    try {
      await invoke<string>("save_device_address", { address: selectedDevice });
      setSelectedDevice("");
      if (service.activeState === "active") await invoke("restart_service");
      await refreshStatus();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  const serviceActive = service.activeState === "active";

  return (
    <div className="window-frame">
      <div className="titlebar">
        <div className="titlebar-drag" data-tauri-drag-region onDoubleClick={() => void appWindow.toggleMaximize()}>
          <span className="titlebar-mark" data-tauri-drag-region aria-hidden="true" /><span data-tauri-drag-region>AirPods Hi-Res Mic</span>
        </div>
        <div className="window-controls">
          <button aria-label="Minimize" onClick={() => void appWindow.minimize()}><svg viewBox="0 0 16 16"><path d="M3 11.5h10" /></svg></button>
          <button aria-label="Maximize" onClick={() => void appWindow.toggleMaximize()}><svg viewBox="0 0 16 16"><rect x="3.5" y="3.5" width="9" height="9" rx="1" /></svg></button>
          <button className="close" aria-label="Hide to tray" onClick={() => void appWindow.close()}><svg viewBox="0 0 16 16"><path d="m4 4 8 8m0-8-8 8" /></svg></button>
        </div>
      </div>

      <main className="app-shell">
        <header className="topbar">
          <div className="brand"><div className="brand-icon"><span /><span /></div><div><p>AIRPODS FOR LINUX</p><h1>Hi-Res Mic</h1></div></div>
          <div className="header-actions">
            <span className={`service-pill ${serviceActive ? "online" : ""}`}><i />{serviceActive ? "Mic active" : "Mic stopped"}</span>
            <button className={`icon-button ${scanning ? "spin" : ""}`} aria-label="Refresh" disabled={loading || scanning} onClick={() => { void refreshStatus(); void loadDevices(); void scanBattery(); }}>↻</button>
          </div>
        </header>

        {error && <div className="error-banner">{error}</div>}

        <section className="battery-panel panel">
          <div className="panel-head">
            <div><div className="connected-line"><span className={airpods?.connected ? "connected-dot" : "connected-dot off"} />{airpods?.connected ? "CONNECTED" : "DISCONNECTED"}</div><h2>{airpods?.name ?? "AirPods"}</h2><p className="address">{airpods?.address ?? "ยังไม่ได้กำหนดอุปกรณ์"}</p></div>
            <button className="scan-button" disabled={scanning} onClick={() => void scanBattery()}>{scanning ? "Scanning…" : "Scan battery"}</button>
          </div>
          <div className="device-setup">
            <div><strong>{airpods?.configured ? "เปลี่ยน AirPods" : "เลือก AirPods ที่เชื่อมต่ออยู่"}</strong><small>เลือกอุปกรณ์ด้วยตัวเอง แล้วกด Refresh หากยังไม่พบ</small></div>
            <select value={selectedDevice} disabled={busy || devices.length === 0} onChange={(event) => setSelectedDevice(event.target.value)}>
              <option value="">{devices.length === 0 ? "ไม่พบอุปกรณ์ Bluetooth ที่เชื่อมต่อ" : "เลือกอุปกรณ์…"}</option>
              {devices.map((device) => <option key={device.address} value={device.address}>{device.name} · {device.address}</option>)}
            </select>
            <button disabled={busy || !selectedDevice} onClick={() => void saveDevice()}>{airpods?.configured ? "Change device" : "Use this device"}</button>
          </div>
          <div className="battery-grid">
            <BatteryItem kind="left" label="Left" value={battery?.left ?? null} charging={battery?.leftCharging ?? false} />
            <BatteryItem kind="right" label="Right" value={battery?.right ?? null} charging={battery?.rightCharging ?? false} />
            <BatteryItem kind="case" label="Case" value={battery?.case ?? null} charging={battery?.caseCharging ?? false} />
          </div>
          <div className="scan-note"><span className={battery ? "pulse" : ""} />{scanMessage}{battery && <span className="model-id">Model {battery.modelId}</span>}</div>
        </section>

        <div className="lower-grid">
          <section className="settings-panel panel">
            <div className="section-title"><div><p>MICROPHONE</p><h2>Sound tuning</h2></div><span>AirPodsHiRes</span></div>
            <Slider label="Input gain" description="เพิ่มความดังของเสียงไมค์" value={settings.micGainDb} min={0} max={30} unit="dB" onChange={(micGainDb) => { setSettings({ ...settings, micGainDb }); setSaved(false); }} />
            <Slider label="Limiter ceiling" description="ป้องกันเสียงแตกเมื่อดังเกินไป" value={settings.limiterDbfs} min={-12} max={0} unit="dBFS" onChange={(limiterDbfs) => { setSettings({ ...settings, limiterDbfs }); setSaved(false); }} />
            <div className="settings-actions"><button className="reset-button" disabled={busy} onClick={() => setSettings(defaultSettings)}>Default</button><button className="save-button" disabled={busy || !airpods?.configured} onClick={() => void saveSettings()}>{saved ? "Saved ✓" : serviceActive ? "Save & restart" : "Save settings"}</button></div>
          </section>

          <section className="service-panel panel">
            <div className="section-title"><div><p>SERVICE</p><h2>Microphone engine</h2></div><span className={`state-badge ${serviceActive ? "active" : ""}`}>{service.activeState}</span></div>
            <div className="service-visual"><div className={serviceActive ? "waves active" : "waves"}><i /><i /><i /><i /><i /></div><strong>{serviceActive ? "Listening" : "Ready when you are"}</strong><small>{service.mainPid ? `Process ${service.mainPid}` : `Startup: ${service.enabled}`}</small></div>
            <div className="service-actions"><button className="power-button" disabled={busy || !airpods?.configured} onClick={() => void controlService(serviceActive ? "stop_service" : "start_service")}>{serviceActive ? "Stop mic" : "Start mic"}</button><button className="restart-button" disabled={busy || !serviceActive} onClick={() => void controlService("restart_service")}>Restart</button></div>
          </section>
        </div>

        <footer><span>✓</span> UI นี้ไม่เปลี่ยน Bluetooth profile หรือ codec A2DP/AAC</footer>
      </main>
    </div>
  );
}

export default App;
