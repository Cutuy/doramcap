# Service invocation comparison implementation

Created 11/15/26. [Architecture](111526-invocation-comparison.md).

The Compose snapshot projects the three native fields from the canonical model it
already hashes. Console reads the copy with the existing safe file capability and
shares invocation rows between History and Review. Stable JSON preserves argument
boundaries; masking happens before values enter comparison rows. A missing canonical
copy retains raw captured text and marks the invocation verdict unavailable.

## Milestone 1: Saved invocation facts

**Work items**
- [x] Publish the canonical three-field copy atomically.
- [x] Exclude the private copy from application changes.
- [x] Share History and Review extraction and masking.
- [x] Preserve older captured values and availability.

**Validation**
- [x] Generated and configured values
- [x] Native fields and private publication
- [x] Save failure and incomplete copies
- [x] Invocation comparison and masking
- [x] Browser comparison
