// ES2025 JSON source access, supported by the embedded QuickJS runtime.
interface JSON {
  rawJSON(source: string): { readonly rawJSON: string };
  isRawJSON(value: unknown): value is { readonly rawJSON: string };
  parse(
    text: string,
    reviver: (key: string, value: any, context: { source: string }) => any,
  ): any;
}
