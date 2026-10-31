import "./text-decoder.js";
import { analyze, complete } from "./plan.js";
import { root, encode, fromProto, textBytes } from "./model.js";
import type { doramcap } from "./generated/protocol.js";
const prefix = "doramcap.compose.planner.v1.";
// Each invocation receives a protobuf message and returns an owned byte buffer.
Object.assign(globalThis, {
  doramcapPlanner: {
    analyze(bytes: Uint8Array) {
      return encode(
        prefix + "AnalyzeOutput",
        analyze(
          root
            .lookupType(prefix + "AnalyzeInput")
            .decode(
              bytes,
            ) as doramcap.compose.planner.v1.AnalyzeInput.$Properties,
        ),
      );
    },
    complete(bytes: Uint8Array) {
      return encode(
        prefix + "CompleteOutput",
        complete(
          root
            .lookupType(prefix + "CompleteInput")
            .decode(
              bytes,
            ) as doramcap.compose.planner.v1.CompleteInput.$Properties,
        ),
      );
    },
    render(bytes: Uint8Array) {
      const type = root.lookupType("doramcap.compose.v1.ComposeModel");
      return textBytes(JSON.stringify(fromProto(type, type.decode(bytes))));
    },
  },
});
