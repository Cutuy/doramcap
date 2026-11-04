import { useRef, useState } from 'react';
import { ArrowDownToLine, ArrowUpRight, Check, Copy, Terminal } from 'lucide-react';
import { routeHref } from './model';

const INSTALLER_NAME = 'install.sh';
const INSTALL_GUIDE = 'console/docs/install.md';

export default function ConsoleInstall({ repository }: { repository: string }) {
  const releaseUrl = `${repository}/releases/latest`;
  const installerUrl = `${releaseUrl}/download/${INSTALLER_NAME}`;
  const installCommand = `curl -fLO ${installerUrl} && sh ${INSTALLER_NAME}`;
  const command = useRef<HTMLElement>(null);
  const [copyStatus, setCopyStatus] = useState('');

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(installCommand);
      setCopyStatus('Copied');
    } catch {
      const selection = window.getSelection();
      if (command.current && selection) {
        const range = document.createRange();
        range.selectNodeContents(command.current);
        selection.removeAllRanges();
        selection.addRange(range);
        setCopyStatus('Command selected. Press Ctrl+C or ⌘C to copy.');
      } else {
        setCopyStatus('Select the command to copy it.');
      }
    }
  }

  return <section className="console-install" aria-labelledby="console-install-heading">
    <div className="console-install-intro">
      <div className="eyebrow"><Terminal size={14} aria-hidden="true" />Console</div>
      <h2 id="console-install-heading">Bring your Compose apps to life.</h2>
      <p>Run experiments, watch live metrics, and capture MCAP recordings.</p>
      <div className="console-install-actions">
        <a className="launch-link" href={releaseUrl} target="_blank" rel="noreferrer">Download Console <ArrowUpRight size={14} aria-hidden="true" /></a>
        <a href={routeHref('/docs', new URLSearchParams({ path: INSTALL_GUIDE }))}>Installation guide <ArrowUpRight size={14} aria-hidden="true" /></a>
      </div>
      <p className="console-requirements">Ubuntu 24.04+ · x86_64 / ARM64 · Docker Engine</p>
    </div>
    <figure className="console-preview">
      <figcaption>Console preview</figcaption>
      <div className="console-preview-body">
        <div className="console-preview-apps">
          <h3>Compose project</h3><div className="console-preview-project">cpu-burn</div>
          <h3>System apps</h3>
          <div><Check size={12} aria-hidden="true" />CPU monitoring</div>
          <div><Check size={12} aria-hidden="true" />Data recorder</div>
        </div>
        <div className="console-preview-results">
          <h3>Live metrics</h3>
          <svg viewBox="0 0 268 48" role="img" aria-label="Example CPU activity chart" preserveAspectRatio="none"><path className="console-chart-axis" d="M1 1v46h266" /><path className="console-chart-line" d="M2 38l21 -9 20 5 21 -20 20 10 20 -5 21 15 20 -14 20 4 21 -12 20 11 21 -9 21 6" /></svg>
          <h3>Recordings</h3><div className="console-preview-recording"><span>experiment.mcap</span><ArrowDownToLine size={14} aria-hidden="true" /></div>
        </div>
      </div>
    </figure>
    <div className="console-install-command">
      <div className="console-command-row"><code ref={command}>{installCommand}</code><button type="button" onClick={() => void copyCommand()} aria-label="Copy Console installation command"><Copy size={13} aria-hidden="true" />Copy</button></div>
      <span className="console-copy-status" role="status">{copyStatus}</span>
    </div>
  </section>;
}
