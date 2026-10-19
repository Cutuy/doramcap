import { useEffect, useRef, useState } from "react";
type Settings = { mu_ms: number; sigma_ms: number; frequency_hz: number };
export function SleepControls({ initial }: { initial: Settings }) {
  const [settings, setSettings] = useState(initial);
  const [revision, setRevision] = useState(0);
  const [state, setState] = useState("Applied");
  const queue = useRef(Promise.resolve());
  useEffect(() => {
    if (!revision) return;
    let obsolete = false;
    const timer = setTimeout(() => {
      queue.current = queue.current.then(async () => {
        if (obsolete) return;
        setState("Applying…");
        try {
          const response = await fetch("/api/playground", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(settings),
          });
          if (!response.ok)
            throw new Error(`Update failed (${response.status})`);
          if (!obsolete) setState("Applied");
        } catch (error) {
          if (!obsolete) setState(`${String(error)} · adjust to retry`);
        }
      });
    }, 300);
    return () => {
      obsolete = true;
      clearTimeout(timer);
    };
  }, [settings, revision]);
  function update(key: keyof Settings, value: number) {
    setSettings((s) => ({ ...s, [key]: value }));
    setRevision((r) => r + 1);
    setState("Waiting…");
  }
  // Preview the underlying Gaussian shape; the actual sampled duration is clamped.
  const points = Array.from({ length: 121 }, (_, i) => {
    const ms = (i * 3000) / 120;
    const z = (ms - settings.mu_ms) / Math.max(12, settings.sigma_ms);
    return `${10 + i * 2.5},${68 - 55 * Math.exp(-0.5 * z * z)}`;
  }).join(" ");
  return (
    <section className="panel sleep-controls" aria-label="Activity playground">
      <div>
        <h2>
          Activity playground <span>☁</span>
        </h2>
        <p>Sleep + publishing rate · 300 ms debounce</p>
        <small role="status">{state}</small>
      </div>
      <label>
        Mean <b>μ · {settings.mu_ms} ms</b>
        <input
          aria-label="Sleep mean"
          type="range"
          min="0"
          max="2000"
          step="10"
          value={settings.mu_ms}
          onChange={(e) => update("mu_ms", Number(e.target.value))}
        />
      </label>
      <label>
        Spread <b>σ · {settings.sigma_ms} ms</b>
        <input
          aria-label="Sleep standard deviation"
          type="range"
          min="0"
          max="1000"
          step="10"
          value={settings.sigma_ms}
          onChange={(e) => update("sigma_ms", Number(e.target.value))}
        />
      </label>
      <label>
        Frequency <b>{settings.frequency_hz.toFixed(1)} Hz</b>
        <input
          aria-label="Publishing frequency"
          type="range"
          min="0.1"
          max="20"
          step="0.1"
          value={settings.frequency_hz}
          onChange={(e) => update("frequency_hz", Number(e.target.value))}
        />
        <small>New conversations per second</small>
      </label>
      <div className="gaussian-preview">
        <svg
          viewBox="0 0 320 85"
          role="img"
          aria-label={`Gaussian preview: mean ${settings.mu_ms} milliseconds, standard deviation ${settings.sigma_ms} milliseconds`}
        >
          <path d={`M10,68 L${points} L310,68Z`} fill="#74a9e822" />
          <polyline
            points={points}
            fill="none"
            stroke="#74a9e8"
            strokeWidth="2"
          />
          <path d="M10 68H310" stroke="#365164" />
          <text x="10" y="82">
            0 ms
          </text>
          <text x="310" y="82" textAnchor="end">
            3,000 ms
          </text>
        </svg>
        <small>Samples clamped to 0–5,000 ms</small>
      </div>
    </section>
  );
}
