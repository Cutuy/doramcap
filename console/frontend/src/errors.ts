import { ApiError } from './api';

const messages: Partial<Record<ApiError['code'], string>> = {
  InvalidInput: 'The project could not be reviewed. Check its Compose files and the selected profiles.',
  StaleReview: 'The project changed after review. Review it again.',
  Conflict: 'Another run or request is already active.',
  Busy: 'The Console is busy. Try again.',
  ImageDownload: 'An image could not be downloaded. Check network and registry access, then review again. Details: sudo journalctl -u "doramcap-console-*"',
  Stopped: 'Review cancelled.',
};
const rejections: Record<string, (subject: string) => string> = {
  EmptySelection: () => 'Select a profile that enables at least one service.',
  NoComposeFile: () => 'Add a Compose file to the project before reviewing it.',
  UnresolvedEnvironment: () => 'Define the environment values required by the Compose files, then review again.',
  ImageNotPrepared: service => `Prepare a named local image for ${service}, then review again.`,
  AnonymousVolume: service => `Give each volume in ${service} a name and retention settings.`,
  ImageVolume: subject => `Add an explicit mount for the image volume ${subject}.`,
  ZeroReplicas: service => `Set a positive replica count for ${service}, or deselect it through profiles.`,
  NameOccupied: name => `The container name ${name} is already in use. Remove the conflicting container or choose another name.`,
  ReservedLabel: label => `Remove the label ${label} from the project; Console assigns it.`,
  UnscopedResource: resource => `Declare ${resource} external, or let Compose assign its name.`,
  ForeignResource: resource => `The resource ${resource} belongs to another run or controller. Resolve its ownership before reviewing again.`,
  NoJournal: services => `Journal logging is unavailable for ${services}. Enable Docker's journald driver and the host journal, or configure project logging.`,
};

/** Known Review rejections use Console wording; unknown reasons retain the raw detail. */
export function errorMessage(error: unknown): string {
  if (!(error instanceof ApiError)) return 'Connection lost; run may still be active.';
  if (error.code === 'InvalidInput' && Object.hasOwn(rejections, error.reason))
    return rejections[error.reason](error.subject);
  return [messages[error.code] ?? 'The request could not be completed.', error.detail].filter(Boolean).join('\n');
}
