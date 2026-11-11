import { useRef, useState } from 'react';
import { Activity, ArrowUpRight, Box, Cog, Copy, Cpu, Laptop, Layers, Terminal, Workflow } from 'lucide-react';
import { CONSOLE_PATH } from './model';

const INSTALLER_NAME = 'install.sh';
const START_COMMAND = 'cd your-compose-project && doramcap-console start';
const SOC_GROUPS = [
  { label: 'Holoscan apps', Icon: Workflow, items: ['Pose', 'Camera'] },
  { label: 'Containers', Icon: Box, items: ['camera', 'planner'] },
  { label: 'systemd services', Icon: Cog, items: ['can-bus', 'gps'] },
];

/** One shell command with a copy button; falls back to selecting the text when the clipboard is blocked. */
function Command({ text, label }: { text: string; label: string }) {
  const command = useRef<HTMLElement>(null);
  const [copyStatus, setCopyStatus] = useState('');

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(text);
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

  return <>
    <div className="console-command-row"><code ref={command}>{text}</code><button type="button" onClick={() => void copyCommand()} aria-label={label}><Copy size={13} aria-hidden="true" />Copy</button></div>
    <span className="console-copy-status" role="status">{copyStatus}</span>
  </>;
}

export default function ConsoleInstall({ repository }: { repository: string }) {
  const releaseUrl = `${repository}/releases/latest`;
  const installCommand = `curl -fLO ${releaseUrl}/download/${INSTALLER_NAME} && sh ${INSTALLER_NAME}`;

  return <section className="console-install" aria-labelledby="console-install-heading">
    <div className="console-install-intro">
      <div className="eyebrow"><Terminal size={14} aria-hidden="true" />Console</div>
      <h2 id="console-install-heading">Adopt today. No code changes required.</h2>
      <p>Run experiments and hardware-in-the-loop simulations on your robotic systems with containers, systemd services, and NVIDIA Holoscan apps. Watch runs live and compare results afterward.</p>
      <div className="console-install-actions">
        <a className="launch-link" href={releaseUrl} target="_blank" rel="noreferrer">Download Console <ArrowUpRight size={14} aria-hidden="true" /></a>
        <a className="console-try" href={`#${CONSOLE_PATH}`}>Try Console</a>
      </div>
      <p className="console-requirements">Ubuntu 24.04+ · x86_64 / ARM64 · Docker Engine</p>
    </div>
    <div className="console-topology" role="img" aria-label="A browser connects to doramcap Console over an SSH tunnel. Console runs beside Holoscan apps, containers, and systemd services on your SoC or workstation, above the optional doramcap runtime.">
      <div className="console-node"><Laptop size={18} strokeWidth={1.6} aria-hidden="true" /><strong>Your device</strong><span>Browser</span></div>
      <span className="console-tunnel">SSH tunnel</span>
      <div className="console-soc">
        <div className="console-soc-title"><Cpu size={16} strokeWidth={1.6} aria-hidden="true" /><strong>SoC or workstation</strong></div>
        <div className="console-soc-body">
          <div className="console-soc-console"><div><Activity size={13} aria-hidden="true" />Console</div></div>
          {SOC_GROUPS.map(({ label, Icon, items }) => <div key={label}>
            <span>{label}</span>
            {items.map((name) => <div key={name}><Icon size={13} aria-hidden="true" />{name}</div>)}
          </div>)}
        </div>
        <div className="console-soc-runtime"><Layers size={13} aria-hidden="true" /><strong>doramcap runtime</strong><span>optional</span></div>
      </div>
    </div>
    <ol className="console-install-steps">
      <li><h3>Install on your SoC or workstation</h3><Command text={installCommand} label="Copy Console installation command" /></li>
      <li><h3>Start it there, from your Compose project</h3><Command text={START_COMMAND} label="Copy Console start command" /></li>
    </ol>
  </section>;
}
