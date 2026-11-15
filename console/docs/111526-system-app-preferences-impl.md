# Browser system-app preferences implementation

Created 11/15/26. [Architecture](111526-system-app-preferences.md).

## Browser choice

**Work items**

- [x] Scope the versioned storage key to page, Console and project.
- [x] Validate cached choices against the complete request limit before use.
- [x] Save accepted Review choices and retain an in-tab fallback.
- [x] Preserve explicit historical choices and recover rejected cache entries.
- [x] Attribute unavailable system-app choices through the existing Review check.

**Validation**

- [x] Storage contracts
- [x] Frontend build
- [x] Browser navigation and reload
- [x] Browser storage failures and stale choices
- [x] Historical choice override
- [x] Selection rejection attribution
