# Browser system-app preferences implementation

Created 11/15/26. [Architecture](111526-system-app-preferences.md).

## Browser choice

**Work items**

- [x] Scope the versioned storage key to page, Console and project.
- [x] Validate cached IDs and launch profiles before use.
- [x] Save accepted Review choices and retain an in-tab fallback.
- [x] Preserve explicit historical choices and recover rejected cache entries.
- [x] Attribute unavailable system-app choices through the existing Review check.

**Validation**

- [x] Storage contracts
- [x] Frontend build
- [ ] Browser navigation and reload
- [ ] Browser storage failures and stale choices
- [ ] Historical choice override
