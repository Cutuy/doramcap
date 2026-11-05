import { useState } from 'react';
import { RotateCcw } from 'lucide-react';
import { App } from 'doramcap-console-frontend/ui';
import { createDemoClient } from 'doramcap-console-frontend/demo';
import { CONSOLE_PATH, CONSOLE_UPCOMING_PATH } from './model';
import ConsoleUpcoming from './ConsoleUpcoming';

const INSTALL_ROUTE = '#/apps/console';
// One client per page, so a run continues while the visitor reads other sections.
let client = createDemoClient();

/** The production Console screens over the in-browser demo client, beside planned views. */
export default function ConsoleDemo({ path }: { path: string }) {
  const [generation, setGeneration] = useState(0);
  const upcoming = path === CONSOLE_UPCOMING_PATH;
  const reset = () => {
    client.dispose();
    client = createDemoClient();
    setGeneration(current => current + 1);
    window.location.hash = `#${CONSOLE_PATH}`;
  };
  return <div className="console-demo">
    <div className="console-demo-bar">
      <strong>Interactive demo</strong>
      <span>Runs in your browser with sample data.</span>
      <a href={`#${upcoming ? CONSOLE_PATH : CONSOLE_UPCOMING_PATH}`}>{upcoming ? 'Back to demo' : 'Upcoming views'}</a>
      <a href={INSTALL_ROUTE}>Install Console</a>
      <button type="button" onClick={reset}><RotateCcw size={14} aria-hidden="true" />Reset demo</button>
    </div>
    {upcoming ? <ConsoleUpcoming /> : <App key={generation} client={client} basePath={CONSOLE_PATH}/>}
  </div>;
}
