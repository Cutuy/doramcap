import { test } from "node:test";
import assert from "node:assert/strict";
import { Batch, TraceStore, rows, rootOf } from "./model.js";
function packet(sequence: number, records: object[], extra: object = {}) {
  return Batch.encode(
    Batch.fromObject({
      version: 1,
      session: "18446744073709551000",
      sequence,
      dictionary: ["Aster", "publish", "Birch", "receive", "text"],
      records,
      ...extra,
    }),
  ).finish();
}
test("keeps uint64 identities exact and preserves cross-thread causal parent", () => {
  const s = new TraceStore();
  s.ingest(
    packet(1, [
      {
        kind: 0,
        spanId: "9007199254740993",
        traceRoot: "9007199254740993",
        name: 1,
        threadName: 0,
        monotonicNs: 1000,
      },
      {
        kind: 4,
        spanId: "9007199254740993",
        traceRoot: "9007199254740993",
        name: 1,
        threadName: 0,
        monotonicNs: 2000,
      },
    ]),
  );
  s.ingest(
    packet(2, [
      {
        kind: 0,
        spanId: "9007199254740994",
        traceRoot: "9007199254740993",
        parentId: "9007199254740993",
        name: 3,
        threadName: 2,
        monotonicNs: 3000,
      },
    ]),
  );
  const spans = [...s.spans.values()];
  assert.equal(spans[1].parent, spans[0].id);
  assert.equal(spans[1].root, spans[0].root);
  assert.equal(spans[0].end, 0.000002);
  assert.match(spans[0].id, /9007199254740993$/);
});
test("deduplicates replay, exposes gaps, resets on process restart", () => {
  const s = new TraceStore();
  s.ingest(packet(5, []));
  assert.equal(s.ingest(packet(5, [])), false);
  s.ingest(packet(8, []));
  assert.equal(s.gaps, 6);
  s.ingest(packet(1, [], { session: "2" }));
  assert.equal(s.gaps, 0);
  assert.equal(s.lastSequence, 1n);
});
test("marks missing starts and bounds retained history", () => {
  const s = new TraceStore();
  s.ingest(packet(1, [{ kind: 4, spanId: 9, name: 1, threadName: 0 }]));
  assert.equal([...s.spans.values()][0].partial, true);
  for (let n = 2; n < 2200; n++)
    s.ingest(packet(n, [{ kind: 0, spanId: n, name: 1, threadName: 0 }]));
  assert.equal(s.spans.size, 2000);
});
test("rejects out-of-range string payloads", () => {
  const s = new TraceStore();
  assert.throws(
    () =>
      s.ingest(
        packet(1, [
          {
            kind: 1,
            name: 1,
            threadName: 0,
            fields: [{ name: 4, kind: 4, offset: 0, length: 5 }],
          },
        ]),
      ),
    /Invalid string/,
  );
});

test("independent writes retain overlapping lifetimes and I/O parentage", () => {
 const s=new TraceStore();
 s.ingest(packet(1,[
  {kind:0,spanId:1,name:1,threadName:0,monotonicNs:1000000},
  {kind:0,spanId:2,name:1,threadName:0,monotonicNs:2000000},
  {kind:0,spanId:3,parentId:1,name:3,threadName:0,monotonicNs:3000000},
  {kind:4,spanId:2,name:1,threadName:0,monotonicNs:4000000},
  {kind:3,spanId:1,linkedId:2,name:1,threadName:0,monotonicNs:4500000},
  {kind:4,spanId:3,parentId:1,name:3,threadName:0,monotonicNs:5000000},
  {kind:4,spanId:1,name:1,threadName:0,monotonicNs:6000000},
 ]));
 const [a,b,c]=[...s.spans.values()];
 assert.equal(rootOf(c,s.spans),a);
 assert.deepEqual(rows(s,0,.01,""),[b,a]);
 assert.deepEqual(a.links,[b.id]);
 assert.ok(b.end! < a.end!);
});
test("missing batches mark open spans partial and eviction is visible",()=>{
 const s=new TraceStore();
 s.ingest(packet(1,[{kind:0,spanId:1,name:1,threadName:0}]));
 s.ingest(packet(3,[]));
 assert.equal([...s.spans.values()][0].partial,true);
 s.ingest(packet(4,[{kind:0,spanId:2,name:1,threadName:0,monotonicNs:70000000000}]));
 assert.equal(s.evicted,1);
});

test("buffer lanes stay fixed and pack overlapping instances without hiding writes", async () => {
 const {bufferLanes}=await import('./model.js');
 const s=new TraceStore();
 assert.deepEqual(bufferLanes(s,0,10,'').map(l=>l.name),['combining','combined','large','slab']);
 const span=(id:string,start:number,end:number,path:string)=>({id,root:id,parent:'0',name:'storage.write',worker:'writer',tid:1,start,end,partial:false,fields:{path},links:[]});
 s.spans.set('a',span('a',1,4,'large'));
 s.spans.set('b',span('b',2,3,'large'));
 s.spans.set('c',span('c',5,6,'large'));
 s.spans.set('d',span('d',2,3,'mapped'));
 const lanes=bufferLanes(s,0,10,'');
 assert.equal(lanes.length,5);
 assert.equal(lanes[2].tracks,2);
 assert.deepEqual(lanes[2].entries.map(e=>[e.span.id,e.track]),[['a',0],['b',1],['c',0]]);
 assert.equal(lanes[4].name,'mapped');
 assert.equal(bufferLanes(s,0,10,'slab').length,4);
 assert.equal(bufferLanes(s,0,10,'slab').flatMap(l=>l.entries).length,0);
});

test("lost END never implies continued I/O after a CQE or parent close", async () => {
 const {spanExtent}=await import('./model.js');
 const s=new TraceStore();
 const dictionary=['writer','storage.write','storage.io','message'];
 s.ingest(packet(1,[
  {kind:0,spanId:1,name:1,threadName:0,monotonicNs:1000000},
  {kind:0,spanId:2,parentId:1,name:2,threadName:0,monotonicNs:2000000},
 ],{dictionary}));
 s.ingest(packet(3,[{kind:1,spanId:2,parentId:1,name:2,threadName:0,monotonicNs:3000000,fields:[{name:3,kind:4,offset:0,length:12}]}],{dictionary,strings:Buffer.from('cqe_observed')}));
 const [parent,io]=[...s.spans.values()];
 assert.equal(io.end,undefined);
 assert.equal(spanExtent(io,s.spans,50).time,.003);
 assert.match(spanExtent(io,s.spans,50).status,/CQE observed/);
 assert.equal(spanExtent(parent,s.spans,50).time,.001);
 assert.match(spanExtent(parent,s.spans,50).status,/Unknown/);
 io.cqeAt=undefined;
 parent.end=.004;
 assert.equal(spanExtent(io,s.spans,50).time,.004);
 assert.match(spanExtent(io,s.spans,50).status,/upper bound/);
 io.end=.0035;
 assert.equal(spanExtent(io,s.spans,50).time,.0035);
 assert.equal(spanExtent(io,s.spans,50).status,'Closed');
});
