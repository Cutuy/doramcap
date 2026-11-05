import { useState } from 'react';
import { RotateCcw } from 'lucide-react';
import { App, ConsoleWindow } from 'doramcap-console-frontend/ui';
import { createDemoClient } from 'doramcap-console-frontend/demo';
import { CONSOLE_PATH } from './model';
import ConsoleUpcoming from './ConsoleUpcoming';

// One client per page, so a run continues while the visitor reads other sections.
let client = createDemoClient();

/** The production Console screens over the in-browser demo client, followed by planned features. */
export default function ConsoleDemo() {
  const [generation, setGeneration] = useState(0);
  const reset = () => {
    client.dispose();
    client = createDemoClient();
    setGeneration(current => current + 1);
    window.location.hash = `#${CONSOLE_PATH}`;
  };
  return <div className="console-demo">
    <div className="intro"><h1>Try Console.</h1></div>
    <App key={generation} client={client} basePath={CONSOLE_PATH} frame={({ views, phases, screen }) => <>
      <div className="console-tabs">
        <nav aria-label="Console views">{views.map(view => <a key={view.label} href={view.href} aria-current={view.current ? 'page' : undefined}>{view.label}</a>)}</nav>
        {phases.length > 0 && <nav className="console-phases" aria-label="Run phases">{phases.map(({ label, current, onSelect }) => onSelect ? <button key={label} type="button" aria-current={current ? 'step' : undefined} onClick={onSelect}>{label}</button> : <span key={label} aria-current={current ? 'step' : undefined}>{label}</span>)}</nav>}
        <button className="console-reset" type="button" onClick={reset}><RotateCcw size={14} aria-hidden="true" />Reset demo</button>
      </div>
      <ConsoleWindow>{screen}</ConsoleWindow>
    </>}/>
    <ConsoleUpcoming />
  </div>;
}
