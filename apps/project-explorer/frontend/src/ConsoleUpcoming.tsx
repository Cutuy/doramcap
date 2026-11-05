const MOCKUPS_URL = 'https://github.com/Cutuy/doramcap/releases/download/console-mockups-2026-10-29';
const MOCKUP_WIDTH = 1536, MOCKUP_HEIGHT = 1024;
const VIEWS = [
  { file: 'run-investigation.png', title: 'Run investigation', summary: 'Workflow, services and CPU usage on one timeline.' },
  { file: 'execution-provenance.png', title: 'Execution provenance', summary: 'Trace a run from source and launch settings to its recordings.' },
  { file: 'evidence-bundle.png', title: 'Report and evidence bundle', summary: 'Export a run as portable, reopenable evidence.' },
  { file: 'system-mcap-viewer.png', title: 'System MCAP viewer', summary: 'Inspect CPU and process telemetry from a recording.' },
  { file: 'robotics-mcap-viewer.png', title: 'Robotics MCAP viewer', summary: 'Review robot topics alongside system activity.' },
  { file: 'local-projects.png', title: 'Local projects', summary: 'Find and open the Compose projects on this machine.' },
  { file: 'workflow-catalog.png', title: 'Workflow catalog', summary: 'Choose a workflow suited to your project.' },
];

/** Planned Console features, shown as the concept pictures from the mockups release. */
export default function ConsoleUpcoming() {
  return <section className="console-upcoming" aria-labelledby="console-upcoming-heading">
    <div className="intro"><h2 id="console-upcoming-heading">Upcoming features.</h2><p>Concept designs for planned Console features, with illustrative data.</p></div>
    <div className="upcoming-grid">{VIEWS.map(view => <article className="upcoming-card" key={view.file}>
      <a href={`${MOCKUPS_URL}/${view.file}`} target="_blank" rel="noreferrer"><img src={`${MOCKUPS_URL}/${view.file}`} alt={`Concept design of ${view.title}`} width={MOCKUP_WIDTH} height={MOCKUP_HEIGHT} loading="lazy"/></a>
      <div><span className="upcoming-badge">Upcoming</span><h3>{view.title}</h3><p>{view.summary}</p></div>
    </article>)}</div>
  </section>;
}
