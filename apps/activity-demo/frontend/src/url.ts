/** Resolve an app endpoint against the document's deployment directory. */
export function appUrl(path: string): URL {
  return new URL(path, document.baseURI);
}
