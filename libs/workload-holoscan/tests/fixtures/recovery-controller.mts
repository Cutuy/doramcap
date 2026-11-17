/** Qualification process: one real native owner, killed by the parent without graceful close. */
import { capabilities, execution, holoscan } from '../../../../console/server/src/native.js';

const owner = await capabilities.open(process.argv[2]!, 1);
process.send?.({ ready: true });
process.on('message', async (message: { id: number; operation: string; input: unknown }) => {
  try {
    let value: unknown;
    switch (message.operation) {
      case 'build': value = (await owner.build(holoscan.BuildRequest.fromObject(message.input as object))).toJSON(); break;
      case 'lookup': value = (await owner.buildLookup(holoscan.BuildStoreRequest.fromObject(message.input as object))).toJSON(); break;
      case 'inspect': value = (await owner.inspect(execution.RunRequest.fromObject(message.input as object))).toJSON(); break;
      case 'start': await owner.start(execution.StartRequest.fromObject(message.input as object)); value = null; break;
      case 'stop': await owner.stop(String(message.input)); value = null; break;
      case 'poll': value = owner.poll().toJSON(); break;
      case 'close': await owner.close(); process.send?.({ id: message.id, value: null }, () => process.disconnect?.()); return;
      default: throw new Error(`Unknown qualification operation ${message.operation}`);
    }
    process.send?.({ id: message.id, value });
  } catch (error) { process.send?.({ id: message.id, error: String(error) }); }
});
