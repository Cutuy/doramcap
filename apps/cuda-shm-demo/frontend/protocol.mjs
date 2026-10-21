import {doramcap} from './generated/wire.js';
const wire = doramcap.cuda_viewer.v1;
const activity = doramcap.activity.v1;
const text = new TextDecoder('utf-8', {fatal:true});
const integer = value => {
  const n=BigInt(value.toString());
  return n <= BigInt(Number.MAX_SAFE_INTEGER) && n >= BigInt(Number.MIN_SAFE_INTEGER) ? Number(n) : n.toString();
};
function fieldValue(f, strings) {
  const bits=BigInt(f.bits.toString());
  switch (f.kind) {
    case 0: return integer(bits);
    case 1: return integer(BigInt.asIntN(64,bits));
    case 2: {
      const data=new DataView(new ArrayBuffer(8));
      data.setBigUint64(0,bits,true);
      return data.getFloat64(0,true);
    }
    case 3: return bits !== 0n;
    case 4: {
      if (f.offset+f.length>strings.length) throw Error('Invalid activity string range');
      return text.decode(strings.subarray(f.offset,f.offset+f.length));
    }
    default: throw Error('Invalid activity field kind');
  }
}
export function decodeSnapshot(bytes) {
  const s=wire.Snapshot.decode(bytes);
  if(s.version!==1) throw Error('Unsupported viewer version');
  const events=[];
  for(const envelope of s.batches) {
    const b=activity.Batch.decode(envelope.payload);
    if(b.version!==1 || b.session.toString()==='0') throw Error('Unsupported activity batch');
    const name=id=>{if(id>=b.dictionary.length)throw Error('Invalid activity dictionary');return b.dictionary[id];};
    b.records.forEach((r,index)=>{
      const fields=Object.fromEntries(r.fields.map(f=>[name(f.name),fieldValue(f,b.strings)]));
      const ns=BigInt(b.unixOriginNs.toString())+BigInt(r.monotonicNs.toString());
      events.push({id:`${envelope.id}/${index}`,pid:b.pid,role:envelope.role,
        session:b.session.toString(),batch:integer(b.sequence),
        time_ms:Number(ns/1000000n)+Number(ns%1000000n)/1e6,
        kind:r.kind,name:name(r.name),fields,
        losses:{records:integer(b.droppedRecords),fields:integer(b.droppedFields),
          spans:integer(b.rejectedSpans),unregistered:integer(b.unregisteredRecords)}});
    });
  }
  const result={run:s.run,state:s.state,config:wire.RunConfig.toObject(s.config ?? {},{defaults:true}),
    cursor:s.cursor.toString(),oldest:integer(s.oldest),events,artifacts:s.artifacts};
  if(Object.hasOwn(s,'log')) result.log=s.log;
  return result;
}
export function encodeConfig(config) { return wire.RunConfig.encode(config).finish(); }
export function decodeReply(bytes) { return wire.Reply.decode(bytes); }
