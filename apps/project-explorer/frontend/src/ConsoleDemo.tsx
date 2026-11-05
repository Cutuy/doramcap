import { useState } from 'react';
import { RotateCcw } from 'lucide-react';
import { App } from 'doramcap-console-frontend/ui';
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
    <div className="console-demo-heading">
      <div className="intro"><h1>Try Console.</h1></div>
      <button type="button" onClick={reset}><RotateCcw size={14} aria-hidden="true" />Reset demo</button>
    </div>
    <App key={generation} client={client} basePath={CONSOLE_PATH}/>
    <ConsoleUpcoming />
  </div>;
}
