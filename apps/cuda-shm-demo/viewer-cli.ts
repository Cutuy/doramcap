import { main } from './viewer.ts';
main().catch(error => { console.error(error); process.exitCode = 1; });
