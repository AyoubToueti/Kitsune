# Changelog

All notable changes to Kitsune, generated from the git history. v1.0.0 is the first public release, so it covers both the original Tauri/SvelteKit era and the Qt6/QML rewrite; the backend is the same throughout.

## 1.1.0 (2026-11-25)

### Features

**qt**

- disk-coverage stripe on the classic-mode timeline ([33a808a](https://github.com/AyoubToueti/Kitsune/commit/33a808a4f820045a9cc2eff18322f29c0e17b6ed), 2026-11-14)
- wire the cached chip into the watch page player ([08a7471](https://github.com/AyoubToueti/Kitsune/commit/08a7471f590633cb23409228a5bb94b6fdc69c8a), 2026-11-13)
- RAM buffer readout chip and scrub-preview guard for RAM streams ([3ce3e8a](https://github.com/AyoubToueti/Kitsune/commit/3ce3e8a3e62cdf176330b4a72a8e2485ce48f051), 2026-11-13)
- 'cached X / Y' chip next to the download speed ([a41e136](https://github.com/AyoubToueti/Kitsune/commit/a41e13670afd14a6045bc1fdd925222707a84464), 2026-11-13)

**core**

- tiered RAM eviction policy ([c273486](https://github.com/AyoubToueti/Kitsune/commit/c273486b5e227e514252f3ba40a73cf9a913ed48), 2026-11-13)

**general**

- implement streaming mode settings and UI updates ([086b585](https://github.com/AyoubToueti/Kitsune/commit/086b5852a7a21f1345ff0a4c4cc9b986b54f6ebb), 2026-11-13)

### Fixes

**general**

- disk-shortage handling actually triggers ([625dcfb](https://github.com/AyoubToueti/Kitsune/commit/625dcfbdaab7ee1951e9a60611c7f4b8a544910e), 2026-11-13)

**librqbit**

- piece stealing prefers the piece a stream is waiting on ([0644fc2](https://github.com/AyoubToueti/Kitsune/commit/0644fc2d04633e4e9ad72724b67cbc8a70425e88), 2026-11-14)

**packaging**

- require glibc instead of libc6 for rpm ([bf65728](https://github.com/AyoubToueti/Kitsune/commit/bf65728d3df292bf5f1162744aedf75ba4a12ec6), 2026-11-25)

**qt**

- reload a dead stream at the last playhead instead of black-screening ([19c8467](https://github.com/AyoubToueti/Kitsune/commit/19c846712e7aa9e789026249275b036237c24b47), 2026-11-13)

**vendor**

- refresh patch and CI coverage ([420e90b](https://github.com/AyoubToueti/Kitsune/commit/420e90b610b79a3ae782ab81560e28ce7f838056), 2026-11-25)

### Other changes

**vendor**

- streaming fixes (ghost-read repair, dead-peer kick, watchdog) ([7fb5a58](https://github.com/AyoubToueti/Kitsune/commit/7fb5a586cb1f4e6ebdb6ffe671161366aa8c05c4), 2026-11-13)
- librqbit 9.0.1 (pristine) behind a path patch ([d7904db](https://github.com/AyoubToueti/Kitsune/commit/d7904db4af6f2bd79f4dada98004f5877a23054d), 2026-11-13)

**scripts**

- vendor update flow for the patched librqbit ([17bc0a2](https://github.com/AyoubToueti/Kitsune/commit/17bc0a230fcbcecaae7ae3937c4c1352e55e651f), 2026-11-13)

<details>
<summary>Maintenance (documentation, build, CI, tests, chores)</summary>

### Documentation

**release**

- expand the 1.0.0 notes and correct libmpv ([ca573d4](https://github.com/AyoubToueti/Kitsune/commit/ca573d43f224e4619775f1c840d5db46447d89c7), 2026-11-25)

### Tests

**core**

- classic-mode seek repro tools ([8531e37](https://github.com/AyoubToueti/Kitsune/commit/8531e370306ef02efbcbd9980f5fa1d78a7007e3), 2026-11-14)
- RAM streaming e2e -- ghost-seek and long-idle recovery ([2697a6d](https://github.com/AyoubToueti/Kitsune/commit/2697a6d29ed989071a7a2d13ebd4c248f5afef88), 2026-11-13)

</details>
