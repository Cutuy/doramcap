// The embedded engine supplies this pure UTF-8 codec; Node already has TextDecoder.
declare const __plannerUtf8: (bytes: Uint8Array, fatal: boolean) => string;
if (typeof globalThis.TextDecoder === "undefined")
  Object.assign(globalThis, {
    TextDecoder: class {
      private fatal: boolean;
      constructor(_encoding: string, options: { fatal?: boolean } = {}) {
        this.fatal = options.fatal ?? false;
      }
      decode(bytes: Uint8Array) {
        return __plannerUtf8(bytes, this.fatal);
      }
    },
  });
