// Rendering contract test with a small DOM fixture; no browser dependency.
import * as fs from 'node:fs';
import * as vm from 'node:vm';
import assert from 'node:assert/strict';
const html = fs.readFileSync(`${__dirname}/../frontend/embedded/index.html`, 'utf8');
assert.doesNotMatch(html, /CPU stream|CPU sample deviation|CPU arrival deviation/);
const elements = new Map([...html.matchAll(/id="([^"]+)"/g)].map(([, id]) => [id, {
  value: '', innerHTML: '', textContent: '', disabled: false,
  style: {} as Record<string, string>, classList: {toggle() {}}, querySelectorAll: () => [],
  clientWidth: 600, getContext: () => new Proxy({}, {get: () => () => {}}),
}]));
const context = vm.createContext({
  document: {baseURI:'http://localhost/', getElementById: (id: string) => {assert(elements.has(id), id); return elements.get(id)}, querySelectorAll: () => []},
  URL, devicePixelRatio: 1, WebSocket: class {}, location:{protocol:'http:',host:'localhost'}, window:{addEventListener(){}},
});
vm.runInContext(html.split('<script type="module">')[1].split('</script>')[0].replace(/^import .*;$/m,''), context);
const state = {
  status: 'live', session: 'one', ring: {writer:'9007199254740993',file:'2',outstanding:1,awaitingSubmission:0,awaitingCqe:1,completions:2,accepted:8192,completed:4096,synced:0,pendingBytes:4096},
  combining: {used:0,capacity:65536,ageMs:null,messages:0}, pool:{free:7,total:8},
  observation:{mode:'Capture',missed:3,contentionDrops:4,timelineEvicted:5},
  activeWrites:[{write:'9007199254740993',region:'44',state:'AwaitingCqe',completed:0,len:4096,ageUs:null,attempts:1,retries:0}],
  events:[{id:'1',kind:'CqeObserved',write:'9007199254740993',file:'2',operation:'3',region:'44',slabUse:'0',offset:0,len:4096,result:4096,latencyUs:null}], messages:[], error:'',
  exercises:{heldReaders:1,pool:{available:3,capacity:4,slabs:[{region:'44',use:'55',phase:'Sealed',readers:1,ioLeases:0,retainedAfterIo:true,ageUs:1000}]},receipts:[{action:'sealed_slab',status:'accepted',messages:2,bytes:524288,write:'77',region:'44',registrationUpdates:0}]},
};
context.input = state;
vm.runInContext('render(input)', context);
assert(elements.get('events')!.innerHTML.includes('9007199254740993'));
assert(elements.get('activeWrites')!.innerHTML.includes('AwaitingCqe'));
assert(elements.get('slabRows')!.innerHTML.includes('Reader held after I/O'));
assert(elements.get('pathReceipts')!.innerHTML.includes('sealed_slab'));
assert(elements.get('slabSummary')!.textContent.includes('3 / 4 slabs free'));
assert(elements.get('eventLoss')!.textContent.includes('contention drops 4'));
state.exercises.pool.available=0; state.exercises.heldReaders=4;
vm.runInContext('render(input)',context);
assert(elements.get('slabSummary')!.textContent.includes('Release readers'));
assert(!/id="holdReader"[^>]*checked/.test(html));
elements.get('eventFilter')!.value='45';
vm.runInContext('renderObservation(input)',context);
assert.equal(elements.get('events')!.innerHTML,'');
state.status='error';state.error='disk failure';
vm.runInContext('render(input)',context);
assert.equal(elements.get('notice')!.textContent,'disk failure');
assert.equal(elements.get('applyObservation')!.disabled,true);
console.log('Frontend: rendering, exact IDs, null ages, filtering and writer errors passed');
