import { useState } from 'react';
import { RotateCcw } from 'lucide-react';
import { App } from 'doramcap-console-frontend/ui';
import { createDemoClient } from 'doramcap-console-frontend/demo';
import { CONSOLE_PATH } from './model';

const INSTALL_ROUTE = '#/apps/console';
// One client per page, so a run continues while the visitor reads other sections.
let client = createDemoClient();

/** The production Console screens over the in-browser demo client. */
export default function ConsoleDemo() {
  const [generation, setGeneration] = useState(0);
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
      <a href={INSTALL_ROUTE}>Install Console</a>
      <button type="button" onClick={reset}><RotateCcw size={14} aria-hidden="true" />Reset demo</button>
    </div>
    <App key={generation} client={client} basePath={CONSOLE_PATH}/>
  </div>;
}
