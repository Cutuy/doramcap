import { wire } from '../../../native.js';
import { LIMITS } from '../../../generated/transport.js';
import { machineChanges } from './compare.js';

/** Keep the full Review available when its optional comparison exhausts the encoded response budget. */
export function boundReviewComparison(review: typeof wire.Review.prototype) {
  if (review.comparison && wire.Review.encode(review).finish().length > LIMITS.frameBytes) {
    review.comparison.rows = [];
    review.comparison.truncated = true;
  }
}
/** Machine changes are unavailable when their full live envelope exceeds the transport budget. */
export function boundMachineChanges(status: typeof wire.Status.prototype) {
  if (status.machineChanges.length && wire.LiveMessage.encode({ status: wire.Status.encode(status).finish() }).finish().length > LIMITS.frameBytes)
    status.machineChanges = machineChanges(null, null, '', 'Machine comparison is too large to display. Changes are unavailable.');
}
