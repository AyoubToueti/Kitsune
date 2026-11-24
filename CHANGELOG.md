# Changelog

All notable changes to Kitsune, generated from the git history. v1.0.0 is the first public release, so it covers both the original Tauri/SvelteKit era and the Qt6/QML rewrite; the backend is the same throughout.

## 1.0.0 (2026-11-24)

### Features

**qt**

- Windows single-instance backend over loopback TCP ([9c6c504](https://github.com/AyoubToueti/Kitsune/commit/9c6c504493d518e430eb519accb48d31f4e208e2), 2026-10-31)
- disk-cache cover art and episode thumbnails ([099d60f](https://github.com/AyoubToueti/Kitsune/commit/099d60f939f3b9b8ddbd3f8e1e7064493d472bb9), 2026-10-28)
- sync the list once per launch, not once per read ([7dcf59a](https://github.com/AyoubToueti/Kitsune/commit/7dcf59ab8027ff482029bf8aaa147f44ee783918), 2026-10-23)
- local-first list and provider-aware navigation ([d8831b6](https://github.com/AyoubToueti/Kitsune/commit/d8831b60bdd890d4ffd9a7a49e7a812c3f426ccb), 2026-10-22)
- reusable ErrorState block with a loading fox ([a772d81](https://github.com/AyoubToueti/Kitsune/commit/a772d81a243fed9e6bb18e4c49505912adf529b3), 2026-10-20)
- show every error through ErrorState ([f19d3f6](https://github.com/AyoubToueti/Kitsune/commit/f19d3f609467879f717be781b2acdc3e3a4b4ca5), 2026-10-20)
- use the thinking fox for every wait ([f6041e3](https://github.com/AyoubToueti/Kitsune/commit/f6041e32d1cb11aa1c5492860a810cb4ced50df4), 2026-10-20)
- enhance HeroCarousel and HeroSkeleton with improved styling and layout adjustments ([0736641](https://github.com/AyoubToueti/Kitsune/commit/07366413eedfd9d266b091439a54b1c1a640f63c), 2026-10-19)
- pickable title language and a light/dark theme ([deb20b9](https://github.com/AyoubToueti/Kitsune/commit/deb20b908e699e660bd79bd62626aed91335c992), 2026-10-19)
- drag a poster onto a list to change its status ([0600e35](https://github.com/AyoubToueti/Kitsune/commit/0600e35191ddd8b86fcb0e78c761967253272af8), 2026-10-18)
- add inline retry and loading rows for paged lists ([08462f7](https://github.com/AyoubToueti/Kitsune/commit/08462f718d05046eb9d97be87743196d12b1cd99), 2026-10-18)
- retry a failed page in place instead of restarting paging ([6ad4097](https://github.com/AyoubToueti/Kitsune/commit/6ad40972974deb2d8a6c657b8532b45f8d2397ed), 2026-10-18)
- paint detail pages instantly and humanise the retry overlay ([a01dff3](https://github.com/AyoubToueti/Kitsune/commit/a01dff37a814c09d9f5ee3f94ae6db5f4ad7d061), 2026-10-18)
- rework the continue-watching card ([cbfd91a](https://github.com/AyoubToueti/Kitsune/commit/cbfd91a530d1cc91655c1ff014ce46d23b2abf70), 2026-10-18)
- search and sort the reader's own list ([ce9f516](https://github.com/AyoubToueti/Kitsune/commit/ce9f516b959e2d7bf6e1a6333394ca16b98ce50a), 2026-10-18)
- remove a work straight from the continue-watching row ([e834057](https://github.com/AyoubToueti/Kitsune/commit/e8340571817163476b7f27f7ce67ae119910a88b), 2026-10-18)
- wire the internet speed test ([2a8f4ea](https://github.com/AyoubToueti/Kitsune/commit/2a8f4eaa1e4cf047ec560d2ebc04aed415c8d4c8), 2026-10-17)
- add back/forward navigation with mouse, gesture and keyboard ([3f45a21](https://github.com/AyoubToueti/Kitsune/commit/3f45a2110371741885e67bf246b97702cd4d1723), 2026-10-17)
- sort the release list and choose speed units ([67b4f7b](https://github.com/AyoubToueti/Kitsune/commit/67b4f7b4548e79548cdeac79cbf227d10dbfe7d9), 2026-10-17)
- show skeleton placeholders while data loads ([6a0056d](https://github.com/AyoubToueti/Kitsune/commit/6a0056d4d526af3235ce1a6f2939f3007bd15e21), 2026-10-17)
- add a retry overlay with rate-limit backoff ([93e3a16](https://github.com/AyoubToueti/Kitsune/commit/93e3a16e49883fbcd2bddc581007000c897fabbc), 2026-10-17)
- load the Top 100 a page at a time ([19be52e](https://github.com/AyoubToueti/Kitsune/commit/19be52ecedcb4e3d2435a5587b2661a25327e787), 2026-10-16)
- mark watched and current episodes on the detail page ([48508fb](https://github.com/AyoubToueti/Kitsune/commit/48508fb40fa2e20f88b3dfa0235839ce8024d2a1), 2026-10-16)
- enhance carousel and card components with hover effects and improved layouts ([617cabb](https://github.com/AyoubToueti/Kitsune/commit/617cabb21ff46d5e7e5e6048429dd7fe457ad6e1), 2026-10-16)
- recommendations page, voting, and quick-add on cards ([6c5e5f3](https://github.com/AyoubToueti/Kitsune/commit/6c5e5f3ba2ca2d4571f7289f60e9a8ca2e680755), 2026-10-16)
- in-place filter search, shelf presets, real Continue Watching sort ([6ed0af7](https://github.com/AyoubToueti/Kitsune/commit/6ed0af7bdf1dddb06abad06aef7ad3ab25a8d604), 2026-10-16)
- continue-watching row and spinning resume disc ([8a49d0b](https://github.com/AyoubToueti/Kitsune/commit/8a49d0b0e783a334a2fb109a0d5e7b703bb375e3), 2026-10-16)
- backend bridge for last-played, watch positions, releases ([a5f2115](https://github.com/AyoubToueti/Kitsune/commit/a5f211524655e5c6363f3ff190f042b713a5bb74), 2026-10-16)
- resume-on-open, save-on-close, and resume settings UI ([a74ad5c](https://github.com/AyoubToueti/Kitsune/commit/a74ad5c9fa7340002ddbef2276a1954562f4f213), 2026-10-16)
- use the SVG crest for the navbar brand mark ([dfe1c63](https://github.com/AyoubToueti/Kitsune/commit/dfe1c63105c5db705682c151b0dd1a167fe733b4), 2026-10-16)
- glass player theme -- tokens, GlassPanel, settings card ([48dd026](https://github.com/AyoubToueti/Kitsune/commit/48dd026966050adc4455e1c34cf2ab4a49212b82), 2026-10-15)
- glass OSD + chapter UI on the player controls ([61b5bfa](https://github.com/AyoubToueti/Kitsune/commit/61b5bfabeaab67d6e99bcaa9c4066d108fc7cd0b), 2026-10-15)
- skip the OP/ED music, never the cold-open intro ([6cee312](https://github.com/AyoubToueti/Kitsune/commit/6cee31261cd0adadfebcd54619f6fb7c55bdab5f), 2026-10-15)
- drive episode chapters from mpv events in the player item ([8ef854a](https://github.com/AyoubToueti/Kitsune/commit/8ef854ad605e81b5956ff2478c3d32f72cf44f29), 2026-10-15)
- configurable player shortcuts ([b2555a3](https://github.com/AyoubToueti/Kitsune/commit/b2555a312660f283b870db794aea553ebdc36934), 2026-10-15)
- player shortcuts editable from the in-player settings card ([fed8de6](https://github.com/AyoubToueti/Kitsune/commit/fed8de6dcfd68e9c94fbe67ab04aba99a511197f), 2026-10-15)
- player controls stage 1 -- transport, timeline, speed, volume ([037308f](https://github.com/AyoubToueti/Kitsune/commit/037308ffe265fa06ae232425cd96f1dc8bfbe28d), 2026-10-14)
- embedded libmpv player (render API) -- the migration's riskiest piece ([0506fd0](https://github.com/AyoubToueti/Kitsune/commit/0506fd03691b13a607577876a6250fc30c3ae479), 2026-10-14)
- global navbar + full filter page parity ([0e4136d](https://github.com/AyoubToueti/Kitsune/commit/0e4136d7d8867bb6be542534df55131d04c28de4), 2026-10-14)
- probeReleases + downloadTorrent on the Backend ([23c9551](https://github.com/AyoubToueti/Kitsune/commit/23c95516eb88b30128636d058d7f117cadbcd849), 2026-10-14)
- home page parity -- hero carousel, trending rail, list blocks ([35f4e6b](https://github.com/AyoubToueti/Kitsune/commit/35f4e6b2dc2121bacc7b987b9bf9c98bc2847ffa), 2026-10-14)
- restructure the filter page to match the reference layout ([4fdc983](https://github.com/AyoubToueti/Kitsune/commit/4fdc983283aa8c60164084648b272401ea394981), 2026-10-14)
- vector player icons, working settings popup, fullscreen ([59261c2](https://github.com/AyoubToueti/Kitsune/commit/59261c2470ea6cd890881ca7f377f2bcee9a0dc2), 2026-10-14)
- My List parity -- status tabs, poster grid, per-card menu ([621ae65](https://github.com/AyoubToueti/Kitsune/commit/621ae65f460998805139c24e7eeb5b4cd22f87f2), 2026-10-14)
- clean up the filter panel -- no search box, scrollable catalogue ([73509ac](https://github.com/AyoubToueti/Kitsune/commit/73509acb2a4b224f22d70c3b69c4690d8a246d5d), 2026-10-14)
- release search + torrent/stream commands on the Backend ([95c0247](https://github.com/AyoubToueti/Kitsune/commit/95c0247537620960aea6f4f5221beaccedf97c25), 2026-10-14)
- episode grid on detail page + release/file picking on watch page ([9d0760b](https://github.com/AyoubToueti/Kitsune/commit/9d0760b4b53d9ab721524a2c6806374687222783), 2026-10-14)
- settings commands + Settings screen ([9e00e9d](https://github.com/AyoubToueti/Kitsune/commit/9e00e9d3e344be992421b61d56bcbb5b1606c5b3), 2026-10-14)
- make the filter panel a popup card, not a full-width banner ([a3f5780](https://github.com/AyoubToueti/Kitsune/commit/a3f5780f1929817c371133100a4f2377be59c6d5), 2026-10-14)
- embed schedule + Top 100 sections on the home page ([b030b04](https://github.com/AyoubToueti/Kitsune/commit/b030b040b2d4c5d3a625f25d4a5da50321f2225f), 2026-10-14)
- buffering overlay over the player; free the player on stop ([d67afc8](https://github.com/AyoubToueti/Kitsune/commit/d67afc810b09a704880c59e7dac1161f1feb3ad8), 2026-10-14)
- detail page parity -- metadata strip, list menu, sidebar ([d8322f9](https://github.com/AyoubToueti/Kitsune/commit/d8322f906b1f10181d7fe8d333e4276a7c96d359), 2026-10-14)
- player controls stage 2 + 3 -- video, audio, subtitles tabs ([d84ee63](https://github.com/AyoubToueti/Kitsune/commit/d84ee637cef0a577c7c507c5dc351c4849843da9), 2026-10-14)
- episode watch modal -- release picker + embedded player ([ed90255](https://github.com/AyoubToueti/Kitsune/commit/ed90255234d686f8e490ba628508bbb0a9e80110), 2026-10-14)
- schedule screen ([444e20c](https://github.com/AyoubToueti/Kitsune/commit/444e20c57b9584d624b227a3122de9b8a4bf6e74), 2026-10-13)
- port the remaining anime metadata commands ([6453f0b](https://github.com/AyoubToueti/Kitsune/commit/6453f0ba80821a62349dc03355a38cfe7573249f), 2026-10-13)
- browse/filter screen ([7e51cba](https://github.com/AyoubToueti/Kitsune/commit/7e51cba0796b069511642423ac50cf3e473e92ef), 2026-10-13)
- top-rated screen ([a76259a](https://github.com/AyoubToueti/Kitsune/commit/a76259ab00f3b6f879fc39dd3fc3d8bb8ba7f89f), 2026-10-13)
- poster-grid home screen with cover art ([ad7f3fa](https://github.com/AyoubToueti/Kitsune/commit/ad7f3fa75fa9a6508e187e685940137c097b001c), 2026-10-13)
- account commands + My List screen ([b6d9259](https://github.com/AyoubToueti/Kitsune/commit/b6d925955048e6a719dc700dcbf991ba3766ba7c), 2026-10-13)
- detail page + StackView navigation ([c30267a](https://github.com/AyoubToueti/Kitsune/commit/c30267a4131cfa79a890fc852bd405ee02b6c3d9), 2026-10-13)
- add the Qt6/CXX-Qt frontend crate and fix the NixOS Qt build ([27d868c](https://github.com/AyoubToueti/Kitsune/commit/27d868cb8b13dad8eab5d859a2dcfe17e23eb810), 2026-10-12)
- async command pattern (Rust tokio -> Qt signal) via `getTrending` ([f08d031](https://github.com/AyoubToueti/Kitsune/commit/f08d0312fbbfe6e1a9ec2d200441edbafdb68d0f), 2026-10-12)

**ui**

- show the saved list on card quick-action buttons ([2d724ce](https://github.com/AyoubToueti/Kitsune/commit/2d724ce9ea9995f7e59d7556b76fa93afc8e8538), 2026-10-23)
- borrow a hero banner from the other source, then blur the fallback ([fcbc148](https://github.com/AyoubToueti/Kitsune/commit/fcbc1488d98578078345accea880d066a4c7d65d), 2026-10-23)
- replace anime card menus with reusable radial quick status menu ([8c17c9a](https://github.com/AyoubToueti/Kitsune/commit/8c17c9a5f5fcf0ee5d05b0f244e04dc2988046cf), 2026-10-22)
- themed Select replaces native dropdowns ([d2eb970](https://github.com/AyoubToueti/Kitsune/commit/d2eb970b481423b8d3e8a6323fa1510a0d363352), 2026-10-01)
- paginate search and genre, and search by relevance ([0ea0f5b](https://github.com/AyoubToueti/Kitsune/commit/0ea0f5b479c349e06294f55a7d4169eba4cf1d94), 2026-09-14)
- trigger the preview from the artwork and ease it in ([573bf1b](https://github.com/AyoubToueti/Kitsune/commit/573bf1b40c105281d66c222fc0323ea9bcc4bc29), 2026-09-14)
- add paging types, an anime grid and a pagination control ([721ed57](https://github.com/AyoubToueti/Kitsune/commit/721ed57961fe7c19bb611048be0dd780b096a0c8), 2026-09-14)
- add a filter page with genre and attribute filters ([abc315f](https://github.com/AyoubToueti/Kitsune/commit/abc315fc89b82a425753a8df85559549b1cf5409), 2026-09-14)
- add a hover preview to anime cards ([05315ef](https://github.com/AyoubToueti/Kitsune/commit/05315efccf63b40616dd53040852fe197ecdec13), 2026-09-13)
- add genre browse route ([0a2d2c2](https://github.com/AyoubToueti/Kitsune/commit/0a2d2c23fb3aa4cd827b1016053cd8887544e2ad), 2026-09-13)
- rework the home page to the reference arrangement ([0feb27c](https://github.com/AyoubToueti/Kitsune/commit/0feb27cd02bbc2dcb3ed4e2f64892000a1283303), 2026-09-13)
- add GenreGrid and ScheduleWidget ([55a690c](https://github.com/AyoubToueti/Kitsune/commit/55a690cb354b99191b95de3e1e3ab2ead0d81c72), 2026-09-13)
- rebuild home page as a shelf stack ([5d8a32c](https://github.com/AyoubToueti/Kitsune/commit/5d8a32c713517cf1a554c6e70178d5b1f88678e9), 2026-09-13)
- rebuild HeroCarousel as a full-bleed slider ([62676d9](https://github.com/AyoubToueti/Kitsune/commit/62676d923e6f8aa4cfa006bf20752edcb600ab58), 2026-09-13)
- give the schedule day tabs ([66fc424](https://github.com/AyoubToueti/Kitsune/commit/66fc4247d0c26e09430a61036c13bd8707231e86), 2026-09-13)
- anchor the hover preview to its card and follow it on scroll ([67fa86f](https://github.com/AyoubToueti/Kitsune/commit/67fa86faddc48e2fcd8b4512f813e5f29e1e18fe), 2026-09-13)
- overlap the preview by a quarter of the card's height ([7f7625f](https://github.com/AyoubToueti/Kitsune/commit/7f7625f2c2a7294e087a465a2f9e8be2410cca45), 2026-09-13)
- show a 29-day schedule strip, fetched per day ([a51fc2a](https://github.com/AyoubToueti/Kitsune/commit/a51fc2acaf007b9cf91ad9ce2f5ee1fcd9dd71f5), 2026-09-13)
- add numbered TrendingRail ([ab30b1d](https://github.com/AyoubToueti/Kitsune/commit/ab30b1d10074821fb35fb8f69f305458500c4450), 2026-09-13)
- add compact list blocks and a browse route ([c2c0e9a](https://github.com/AyoubToueti/Kitsune/commit/c2c0e9a257362b6d8c540fad5d31291e0882276a), 2026-09-13)
- add HeroCarousel and numbered PosterRow ([d071c8d](https://github.com/AyoubToueti/Kitsune/commit/d071c8d9157e5a8dc6e5ac3c874cb398d8414650), 2026-09-13)

**watch**

- add a speed-test button to the episode modal footer ([ea9483f](https://github.com/AyoubToueti/Kitsune/commit/ea9483f178171154a9ffc13328f865256e26eaef), 2026-10-04)
- in-place episode modal, buffering panel, and now-playing disc ([26dda96](https://github.com/AyoubToueti/Kitsune/commit/26dda96f696be592fbdac510bd07ad85cda76583), 2026-10-02)
- record the work last opened ([8b3d810](https://github.com/AyoubToueti/Kitsune/commit/8b3d81010df59a0388c8f1dee5301400137d0203), 2026-09-24)
- record the episode as watched when it starts playing ([1f8e118](https://github.com/AyoubToueti/Kitsune/commit/1f8e118b0d847190b83708373b9c8e691fd70606), 2026-09-23)
- save a release's torrent file from the row ([21a5f7f](https://github.com/AyoubToueti/Kitsune/commit/21a5f7f5ae7e7b7edf991767ec22ae98790be655), 2026-09-20)
- open a release's magnet in the reader's torrent client ([5cfcf1d](https://github.com/AyoubToueti/Kitsune/commit/5cfcf1d6412caa41a8e37469d5c32e26b051d354), 2026-09-20)
- grade how explicitly a release names its episode ([52a0054](https://github.com/AyoubToueti/Kitsune/commit/52a005475e10e65f617c72daabf517c835773796), 2026-09-19)
- search the indexers with every title form ([564042e](https://github.com/AyoubToueti/Kitsune/commit/564042e1165ae1bfe8a52828fc12d2aa148585cd), 2026-09-19)
- filter the release list by a typed query ([7de43f9](https://github.com/AyoubToueti/Kitsune/commit/7de43f94b8bbbc244d2c780c8f95a6acdd0b2a6c), 2026-09-19)
- mark the release rows that name their season and episode ([ad765d3](https://github.com/AyoubToueti/Kitsune/commit/ad765d38c965e1f703341ea6d2e3a050d68d6f03), 2026-09-19)
- bound the releases list height so it scrolls ([26e0098](https://github.com/AyoubToueti/Kitsune/commit/26e00987ae666255697797569d131d60d8db7c6e), 2026-09-18)
- search releases automatically and download torrents ([2aee991](https://github.com/AyoubToueti/Kitsune/commit/2aee9910e6404b70c71ae6a77e74354ff247254c), 2026-09-18)
- re-rank releases as swarm-health probes come back ([a7aa034](https://github.com/AyoubToueti/Kitsune/commit/a7aa034d4968d7fe0b10807594fbb61e27c8d8a3), 2026-09-18)
- filter releases by resolution, and scroll the file list ([a9659e8](https://github.com/AyoubToueti/Kitsune/commit/a9659e8f7304604e7b936b8bf06af417c1a4916a), 2026-09-18)
- open the watch page from an episode and honour ?ep= ([d8a6a0d](https://github.com/AyoubToueti/Kitsune/commit/d8a6a0db85c88d0d61563fdd12fc288c50512240), 2026-09-14)
- add a watch page that plays the reader's own torrent ([e142357](https://github.com/AyoubToueti/Kitsune/commit/e14235773985a0203eac39fc443f0672de87e4b5), 2026-09-14)

**anime**

- add a work to an AniList list from the detail page ([73e2574](https://github.com/AyoubToueti/Kitsune/commit/73e2574aaf18f0ae36f7c80d7330e272cb2c333b), 2026-09-23)
- make the detail-page genre pills clickable ([eeab787](https://github.com/AyoubToueti/Kitsune/commit/eeab78704d083244f3e09557adb266ec4248424d), 2026-09-22)
- carry the release season alongside the year ([086339e](https://github.com/AyoubToueti/Kitsune/commit/086339e3b2794b7a68f47fb07e7169206b9aef0a), 2026-09-14)
- add trending, favorites and date-added sort options ([14a0202](https://github.com/AyoubToueti/Kitsune/commit/14a0202f7c4fe46e7e750f77c041f511539bc8f2), 2026-09-14)
- show the trailer in the detail sidebar ([2d8c094](https://github.com/AyoubToueti/Kitsune/commit/2d8c094f2dccc890618228aaa7bd7aba01ac684c), 2026-09-14)
- lay episodes and related anime side by side ([2f3095f](https://github.com/AyoubToueti/Kitsune/commit/2f3095fa69458c733ce6374f8afbc7d98662fe20), 2026-09-14)
- add a related-anime sidebar list ([3c1fe21](https://github.com/AyoubToueti/Kitsune/commit/3c1fe21a2c12d7ac232b756b7b98635500b2769a), 2026-09-14)
- add a recommendations row to the detail page ([4d200ff](https://github.com/AyoubToueti/Kitsune/commit/4d200ffb3b6d786d46f27f809786f93072ee0314), 2026-09-14)
- carry relations, recommendations and a trailer on the detail ([8b77ec7](https://github.com/AyoubToueti/Kitsune/commit/8b77ec738035c84cab53b34a129475f8adc45769), 2026-09-14)
- filter browse results by tag ([93ccbb0](https://github.com/AyoubToueti/Kitsune/commit/93ccbb074142a54b0b2391a58ce319eea91a74f7), 2026-09-14)
- exclude tags from browse results ([cb3ce5a](https://github.com/AyoubToueti/Kitsune/commit/cb3ce5a0e2e267e08181150c68aafd820301899e), 2026-09-14)
- show episodes as a thumbnail grid ([e63decf](https://github.com/AyoubToueti/Kitsune/commit/e63decf2653eeaed8a99da5359992b7e65ec55ac), 2026-09-14)
- expose AniList's tag catalogue ([ec47b47](https://github.com/AyoubToueti/Kitsune/commit/ec47b474c666286cc1d4d079c5dfaec5a2895d3d), 2026-09-14)
- mirror the detail-only types and update the fixtures ([feb879f](https://github.com/AyoubToueti/Kitsune/commit/feb879ff20a6a32743d82eb23b02ab7861ac0d81), 2026-09-14)
- add detail page with episodes, metadata and streaming links ([affba7c](https://github.com/AyoubToueti/Kitsune/commit/affba7c8df73244846afbe2fde9645eabaa5910b), 2026-09-13)

**general**

- add remaining time toggle and improve time display in PlayerControls ([1e91c88](https://github.com/AyoubToueti/Kitsune/commit/1e91c8823dadbb2ea09f2563bef4cce65f42ddee), 2026-10-16)
- add CI and release workflows, version sync script, and update README ([1a114ef](https://github.com/AyoubToueti/Kitsune/commit/1a114ef873b483abe21fe26ad168956a73f27245), 2026-10-03)
- add torrent progress tracking and status panel ([bf5650b](https://github.com/AyoubToueti/Kitsune/commit/bf5650b0f5c75b7b986a1dd83dc2fc2660c903ee), 2026-09-30)
- add search page ([b98e900](https://github.com/AyoubToueti/Kitsune/commit/b98e9002e4f5e76f0732b102e177753ff8a5d57c), 2026-09-13)
- add app shell with navbar and routing ([5591d32](https://github.com/AyoubToueti/Kitsune/commit/5591d323be4f6946570921853b23baf3fd5b4b6b), 2026-09-12)
- add home page with hero banner and poster row ([7ba0a27](https://github.com/AyoubToueti/Kitsune/commit/7ba0a27da359ec7500c07014f7afe89098eff46d), 2026-09-12)
- add richer anime metadata ([8081104](https://github.com/AyoubToueti/Kitsune/commit/8081104e4b949e60ce1102b7da113362d69c0329), 2026-09-12)
- expose anime metadata over tauri ipc ([d1c4e67](https://github.com/AyoubToueti/Kitsune/commit/d1c4e676186f4ea2ac38c3d2db43dcfdc7af0819), 2026-09-12)
- add frontend api layer for anime metadata ([d7ad7d2](https://github.com/AyoubToueti/Kitsune/commit/d7ad7d2011d76849617fd3afc63c9c42acd42d38), 2026-09-12)
- add torrent engine wrapper over librqbit ([3420b48](https://github.com/AyoubToueti/Kitsune/commit/3420b485b8e8274f9e2559a67a07a8cab1bf6c94), 2026-09-11)
- add local HTTP stream bridge over librqbit's http-api ([585cd2f](https://github.com/AyoubToueti/Kitsune/commit/585cd2f72c7a631d53ed5aa6db4aa5f4b0972329), 2026-09-11)
- add provider trait and AniList provider ([a4a572e](https://github.com/AyoubToueti/Kitsune/commit/a4a572e5ccfabb4062471cc3924540f3e1f9c04c), 2026-09-11)
- add shared domain types ([fc935fb](https://github.com/AyoubToueti/Kitsune/commit/fc935fb341e1f75eb8b781335decb5ce634aa349), 2026-09-11)

**indexer**

- expose each release's publish date ([96fa76c](https://github.com/AyoubToueti/Kitsune/commit/96fa76c3bf4c4b65cad7f2667f89f8f7632b3ea7), 2026-10-17)
- add a packs search mode ([0827263](https://github.com/AyoubToueti/Kitsune/commit/0827263c1d63df5f224cb47f9ec70246f0898fd3), 2026-10-04)
- recognise a bare Sxx season pack ([7b53d98](https://github.com/AyoubToueti/Kitsune/commit/7b53d98427b02889dd902aa9033eb2bed474d87a), 2026-10-04)
- check a release's season against the one requested ([4dc7262](https://github.com/AyoubToueti/Kitsune/commit/4dc7262389d6d4051068dcc6fc9fa90c995dbe89), 2026-09-19)
- build season-aware, multi-title search queries ([6d817cb](https://github.com/AyoubToueti/Kitsune/commit/6d817cb53407511e3d77e10e351207432b7e2bca), 2026-09-19)
- search every title form and de-duplicate the merge ([9948c2e](https://github.com/AyoubToueti/Kitsune/commit/9948c2e918cb3f579b5ec7b964e98d3bc8a7a311), 2026-09-19)
- scrape trackers over HTTP and UDP for live peer counts ([2a59f2f](https://github.com/AyoubToueti/Kitsune/commit/2a59f2f41f75eb03b99f3a21b6842e65ae25e79e), 2026-09-18)
- re-rank releases by confirmed liveness, not just claims ([8ce3afc](https://github.com/AyoubToueti/Kitsune/commit/8ce3afcc1f0b5798e3c10df6a0cd47de4b8f3b50), 2026-09-18)
- probe releases from the frontend and report progress ([9b52c68](https://github.com/AyoubToueti/Kitsune/commit/9b52c681328621bf0b15dd60a62daad3e1d016e4), 2026-09-18)
- add pure tracker-scrape foundation for release probes ([af51ad1](https://github.com/AyoubToueti/Kitsune/commit/af51ad195fe0067676e42c8e6b34e5d3d0a89c71), 2026-09-18)
- probe a batch of releases concurrently, capped per tracker ([f5912ea](https://github.com/AyoubToueti/Kitsune/commit/f5912eacd7cc47505fb09e3baf54f5592a7ef426), 2026-09-18)
- probe a torrent's metadata without downloading pieces ([f7a5ef5](https://github.com/AyoubToueti/Kitsune/commit/f7a5ef59ad171bb52bef744045e99712a270b67c), 2026-09-18)

**filter**

- parse tag params and group the catalogue by category ([0aa62d4](https://github.com/AyoubToueti/Kitsune/commit/0aa62d4c5cd89ebd687ac6da2115448df2fb5fce), 2026-09-14)
- show the full tag catalogue grouped by category ([1f46a18](https://github.com/AyoubToueti/Kitsune/commit/1f46a18ae26a0ae4ea39b72210333d81cab5b38b), 2026-09-14)
- move sort into its own menu, opposite the filter button ([5512f26](https://github.com/AyoubToueti/Kitsune/commit/5512f2689fb3f29c1c4b3222857f4e2da92a4ed4), 2026-09-14)
- parse excluded tags and parameterise the page-link base ([77022ec](https://github.com/AyoubToueti/Kitsune/commit/77022ecf414eb1b26702efc95e1822a4b98d81ea), 2026-09-14)
- collapse the tag catalogue behind a disclosure ([8defb02](https://github.com/AyoubToueti/Kitsune/commit/8defb022fdeaf59f0ab4c4a73e50a08f1ba91916), 2026-09-14)
- search the tag catalogue instead of collapsing it ([9b96eb7](https://github.com/AyoubToueti/Kitsune/commit/9b96eb7276714f30baa2c4b2b48317633a5e03fa), 2026-09-14)
- grow the results list as the reader scrolls ([ba8f46d](https://github.com/AyoubToueti/Kitsune/commit/ba8f46ddc2fcf07e5637c8f2c2933c22ad212dea), 2026-09-14)
- cycle a tag chip through include, exclude and off ([bdae64c](https://github.com/AyoubToueti/Kitsune/commit/bdae64c6e8c2ea11300a881eecb363eef93c5f71), 2026-09-14)
- put the panel behind a Filters dropdown ([d9d478d](https://github.com/AyoubToueti/Kitsune/commit/d9d478dac7ffaee6340f95d410ac2743d62c7e15), 2026-09-14)
- add an active-filters bar with per-filter removal ([e777703](https://github.com/AyoubToueti/Kitsune/commit/e7777030df19c1e192af0aa0a405514129d4cfd1), 2026-09-14)
- show a tag's description as a chip tooltip ([f17fb8d](https://github.com/AyoubToueti/Kitsune/commit/f17fb8d87d43b0f97fd1119dfaf30928826f3387), 2026-09-14)

**diagnostics**

- measure internet latency and download speed ([37ec66f](https://github.com/AyoubToueti/Kitsune/commit/37ec66f8ba72253a0c85a67705e34f232f92171e), 2026-10-04)
- add the speed test to the diagnostics panel ([dec882e](https://github.com/AyoubToueti/Kitsune/commit/dec882e211dcd8add965fbf27521abaafd26934f), 2026-10-04)
- measure process CPU and memory ([1906e89](https://github.com/AyoubToueti/Kitsune/commit/1906e892c2993e639857694e4db98d6c7968ef86), 2026-10-03)
- show resource usage in the settings drawer ([26df100](https://github.com/AyoubToueti/Kitsune/commit/26df1001563be015d0c81a8928ba925a0c1ccba3), 2026-10-03)
- install a tracing subscriber with a rotating log file ([297ed13](https://github.com/AyoubToueti/Kitsune/commit/297ed13daa6acef9a150bc2e6fe0f4f8504abc8b), 2026-10-03)
- record panics and frontend errors in the log ([9503ce6](https://github.com/AyoubToueti/Kitsune/commit/9503ce6601e5ea9785e1014a8cd648347c8949c5), 2026-10-03)
- capture uncaught frontend errors into the log ([9f7feea](https://github.com/AyoubToueti/Kitsune/commit/9f7feeaedae616dde1225109efd7030d4a4d359a), 2026-10-03)
- show machine-wide CPU and process count in the panel ([d7ea8d9](https://github.com/AyoubToueti/Kitsune/commit/d7ea8d996ef8b002ef0e8bc2a7621e09ebf58b41), 2026-10-03)

**player**

- wheel adjusts volume, and raise the cap to 200% ([aa0bbde](https://github.com/AyoubToueti/Kitsune/commit/aa0bbdef7df0d94203821fd6a8c2cd7b30120eb8), 2026-10-23)
- hold a torrent while an external player reads it ([f1e2338](https://github.com/AyoubToueti/Kitsune/commit/f1e23388e29bbff0c9b822a87b030c668dc16787), 2026-09-23)
- remove a torrent from the session ([5018963](https://github.com/AyoubToueti/Kitsune/commit/5018963fb9c475527d8ab43efd0572798b8a50e9), 2026-09-22)
- add the frontend wrappers for the player commands ([c006d46](https://github.com/AyoubToueti/Kitsune/commit/c006d46554e62233408b646c5368bb99dd47e0a4), 2026-09-14)
- add the in-app player and the external-player control ([d38c2ab](https://github.com/AyoubToueti/Kitsune/commit/d38c2abc5631f4f5b19cc341f1d9dca9669efe28), 2026-09-14)
- give the torrent engine a lifecycle the app can drive ([dd3a555](https://github.com/AyoubToueti/Kitsune/commit/dd3a5555a1541f5c262a86a43de3c735fd5efc63), 2026-09-14)
- allow picking a torrent file and streaming it in-app ([fcda971](https://github.com/AyoubToueti/Kitsune/commit/fcda971fb4c51f6484b3ccfcc9445db113af75e9), 2026-09-14)

**home**

- fetch every home shelf in one GraphQL query ([fc0c3ba](https://github.com/AyoubToueti/Kitsune/commit/fc0c3ba879b73e302c7349c04f6488688b93fa73), 2026-10-17)
- show only started works in Continue Watching ([a066603](https://github.com/AyoubToueti/Kitsune/commit/a06660333a735dc3603aaeb524dddb78ca33da39), 2026-09-24)
- resume a work from its Continue Watching card ([03ef616](https://github.com/AyoubToueti/Kitsune/commit/03ef616ac1124844390ade5cbc0a4d6a68027f48), 2026-09-23)
- add a Continue Watching section ([4e786be](https://github.com/AyoubToueti/Kitsune/commit/4e786be0d1faba7c1879cbfcd61b0586d1cccd5f), 2026-09-23)
- point each list section's View more at its own ordering ([3c732b1](https://github.com/AyoubToueti/Kitsune/commit/3c732b166c5162b5c342de210eded57a8d6f5f48), 2026-09-20)
- add upcoming next season and Top 100 sections ([1328e75](https://github.com/AyoubToueti/Kitsune/commit/1328e755bcb25cc0e51ac1c824bea5dbf128bf3f), 2026-09-14)

**anilist**

- paged recommendations and voting ([eefc352](https://github.com/AyoubToueti/Kitsune/commit/eefc352313605459b03ed851ec4df005114f3feb), 2026-10-02)
- expose the reader's list entry to the frontend ([70e627f](https://github.com/AyoubToueti/Kitsune/commit/70e627fae26fef59edcb264ec5ed506b5da565ad), 2026-09-23)
- read the whole user list and delete an entry ([af82384](https://github.com/AyoubToueti/Kitsune/commit/af82384df9d5219bcaa749f5be664c3c2da56210), 2026-09-23)
- send a bearer token and support mutations ([b69d46f](https://github.com/AyoubToueti/Kitsune/commit/b69d46f6eb0c8098b0c5f750ddf959e19ecb3c06), 2026-09-23)
- read and write the reader's list status ([c499103](https://github.com/AyoubToueti/Kitsune/commit/c499103a60b5c39e4d7c0ebc06e89f91f7b8faa6), 2026-09-23)

**auth**

- announce when the reader's list changes ([7679122](https://github.com/AyoubToueti/Kitsune/commit/7679122f6c7bd2992705b69bb3901b8abbfbb806), 2026-09-24)
- remember the last work the reader opened ([dfef715](https://github.com/AyoubToueti/Kitsune/commit/dfef7153914e566d0c6f1687e508e4d138d85891), 2026-09-24)
- wrap the sign-in commands for the frontend ([2189c68](https://github.com/AyoubToueti/Kitsune/commit/2189c68b6b6ecc7c458b1acd97c6d07d4338c3a9), 2026-09-23)
- sign in to AniList via a custom URI scheme ([3809164](https://github.com/AyoubToueti/Kitsune/commit/38091640cb530bdef15bd0dd3c94f93c1f553828), 2026-09-23)
- store the AniList token on disk ([fad1481](https://github.com/AyoubToueti/Kitsune/commit/fad148181115a30cf38294a30ed3cd29bfae8f2c), 2026-09-23)

**api**

- wrap the user list and delete commands ([111774e](https://github.com/AyoubToueti/Kitsune/commit/111774e8503ebbd3dbe0d2e033b49abad0f6d817), 2026-09-24)
- subscribe to list changes ([a6b896c](https://github.com/AyoubToueti/Kitsune/commit/a6b896c68f12ad691d576dd246203bfa8c5ca530), 2026-09-24)
- cache provider results behind the wrappers ([658f2b2](https://github.com/AyoubToueti/Kitsune/commit/658f2b27f5bf8ed47cb5eb1ca73178700326e8c4), 2026-09-14)
- expose list, genre and schedule commands to the frontend ([97862c4](https://github.com/AyoubToueti/Kitsune/commit/97862c4ddbd3640b1e8dd90a704cf01cf5dd7ed8), 2026-09-13)

**backend**

- add a paged, filtered browse query ([1d16352](https://github.com/AyoubToueti/Kitsune/commit/1d163522b9c9b72b366bc06c76155f6e36582ef9), 2026-09-14)
- add ListFilter and parameterized list query ([887e823](https://github.com/AyoubToueti/Kitsune/commit/887e82375d8a8b8a472a43b835bd3634e3f1e783), 2026-09-13)
- add airing schedule query ([b087512](https://github.com/AyoubToueti/Kitsune/commit/b0875122741babd983d72b4f4109fd6a879532c5), 2026-09-13)
- add genre browse queries ([f580a01](https://github.com/AyoubToueti/Kitsune/commit/f580a0133f92da6d394d8dfbe767645671a7114f), 2026-09-13)

**resume**

- mount the disc globally in the root layout ([3366d6b](https://github.com/AyoubToueti/Kitsune/commit/3366d6b979609a455c10dd97519f524239076823), 2026-09-24)
- share the progress->?ep index and add record-spin tokens ([7ae0a26](https://github.com/AyoubToueti/Kitsune/commit/7ae0a265f848fbab2153151979bd1364cad9a811), 2026-09-24)
- the disc follows the work last opened ([bb2f499](https://github.com/AyoubToueti/Kitsune/commit/bb2f499521707f216df44b8b777c2b40f1722881), 2026-09-24)
- add the spinning resume disc ([c4d7d87](https://github.com/AyoubToueti/Kitsune/commit/c4d7d878c24400a52f6dec6642d3766c736a862d), 2026-09-24)

**settings**

- drop the dead player controls, auto-scroll to test results ([da92ad0](https://github.com/AyoubToueti/Kitsune/commit/da92ad0125f98bf30b908664509d674eade6a9c7), 2026-10-24)
- save each change as it happens ([3cbda1d](https://github.com/AyoubToueti/Kitsune/commit/3cbda1da951d0f0a06e301cefb0e50ef5d2f1db3), 2026-10-19)
- add a title language preference ([c944e69](https://github.com/AyoubToueti/Kitsune/commit/c944e6989a7880a95b4bc1052b307e4aebea7a31), 2026-10-19)
- persisted settings store, commands, and live player resolution ([bae2011](https://github.com/AyoubToueti/Kitsune/commit/bae2011a2be0acecee454515f4544af808bdfbc8), 2026-10-02)

**core**

- remember the last release per episode, not per work ([966b7d3](https://github.com/AyoubToueti/Kitsune/commit/966b7d322345a3ec89d4a5844232c273c0c6bc82), 2026-10-30)
- watch-position + last-release stores and resume settings ([4cad493](https://github.com/AyoubToueti/Kitsune/commit/4cad493a7cd38b82411a8556fd36f73f358e607e), 2026-10-16)
- page curated lists for the Top 100 view ([e48a7ce](https://github.com/AyoubToueti/Kitsune/commit/e48a7cee600170d95e79ccfa40fb8fb22bc8267e), 2026-10-16)

**episodes**

- prefix each numbered episode with its number ([b4cef09](https://github.com/AyoubToueti/Kitsune/commit/b4cef092a15b2012fc2030e30228fe4790443c71), 2026-09-22)
- enrich episode lists from Jikan ([350af44](https://github.com/AyoubToueti/Kitsune/commit/350af44ef02617eebe4ef7ffef02e9832f821481), 2026-09-19)
- hide unaired episodes and show how many have aired ([d85aaa4](https://github.com/AyoubToueti/Kitsune/commit/d85aaa413940b992f35b6bda5ea6510b185ce90d), 2026-09-19)

**providers**

- expose when a list entry was added ([58d8fa7](https://github.com/AyoubToueti/Kitsune/commit/58d8fa7379f8e7a22c17ee4fc53cba4c3b5194d5), 2026-10-18)
- seed detail pages from listed works and fold list entries into by_id ([b4c1b3b](https://github.com/AyoubToueti/Kitsune/commit/b4c1b3b8284ac7ef776eb86e8b7a1dffc9da4dda), 2026-10-18)
- cache metadata and the reader's list, invalidated on writes ([dfaf6e4](https://github.com/AyoubToueti/Kitsune/commit/dfaf6e4286558b068bdc50d88e7e3ff78ca7a314), 2026-10-17)

**search**

- match a later cour's absolute episode numbers ([d31cb8c](https://github.com/AyoubToueti/Kitsune/commit/d31cb8cc652f2da267896293465d77be19c13bd0), 2026-09-19)
- grow the results list as the reader scrolls ([032fdf6](https://github.com/AyoubToueti/Kitsune/commit/032fdf6e9833e3a90debd39ec3da7836cfa9d74c), 2026-09-14)
- put the filter panel on the search page ([6ef2024](https://github.com/AyoubToueti/Kitsune/commit/6ef2024fb8ee9cc403116c16db8a6139dab9e24f), 2026-09-14)

**cards**

- Watch resumes, and plays the trailer ([a21985f](https://github.com/AyoubToueti/Kitsune/commit/a21985ffe3dc1ce1e4596cb2d59665d238c37c35), 2026-10-02)
- hover trigger in the quick bar, neon Continue Watching ([dea6c2b](https://github.com/AyoubToueti/Kitsune/commit/dea6c2b76d89e35249d75421c4f2d8250bfcb375), 2026-10-01)

**genre**

- link genre pills to the filter page ([28190d9](https://github.com/AyoubToueti/Kitsune/commit/28190d9aa10a57f2b0dd3df6a1a11e12ed117929), 2026-09-20)
- grow the results list as the reader scrolls ([10b4553](https://github.com/AyoubToueti/Kitsune/commit/10b45536a8fad9d0f8e54d7c560c41124e68401d), 2026-09-14)

**list**

- add a per-card status menu ([1188d51](https://github.com/AyoubToueti/Kitsune/commit/1188d517e463a5ee16adf5e76f994319260cd1d7), 2026-09-24)
- add the My List page ([2d9923d](https://github.com/AyoubToueti/Kitsune/commit/2d9923d6a563ad5bafdb1e731096364b61adbb8a), 2026-09-24)

**qml**

- replay a remembered release on Resume and on watched cards ([3043657](https://github.com/AyoubToueti/Kitsune/commit/30436577c799799d6462cfbd606690e55798e393), 2026-10-30)
- redesign AvailabilityPopup with fox-themed compact layout and skeleton loading placeholders ([caf8efe](https://github.com/AyoubToueti/Kitsune/commit/caf8efe6a1ebc8c5ae3a481f844dac99c01180c4), 2026-10-28)

**trailer**

- serve YouTube embed over loopback origin and portal modals ([d7e5b57](https://github.com/AyoubToueti/Kitsune/commit/d7e5b57dcde1debb5e49a84e81d24734d689f0f7), 2026-10-04)
- implement modal for trailer playback and enhance URL handling ([164e190](https://github.com/AyoubToueti/Kitsune/commit/164e1905943ecb66dec81ea01446384876064d70), 2026-09-14)

**assets**

- update logo assets and enhance the NavBar design ([90067f9](https://github.com/AyoubToueti/Kitsune/commit/90067f9693f6afff29f4757569b404a36c02a39f), 2026-10-21)

**branding**

- update app icons, add Linux desktop integration, and UI branding ([255e79f](https://github.com/AyoubToueti/Kitsune/commit/255e79fda894f881a73947ffadfc55559cbdb4a0), 2026-10-04)

**browse**

- add an infinite-scroll composable ([7f98ec9](https://github.com/AyoubToueti/Kitsune/commit/7f98ec94ad66c2983a34a61315d9121aa0c4d77c), 2026-09-14)

**episode-list**

- add jump-to-episode functionality with highlighting and scroll behavior feat(related-anime-list): implement scrollable container for related anime list ([438e435](https://github.com/AyoubToueti/Kitsune/commit/438e4354f750c8c4c405c68d19f90228bba74eaa), 2026-09-14)

**kitsune-core, kitsune-qt**

- add fullscreen timecode overlay preference ([8918b9e](https://github.com/AyoubToueti/Kitsune/commit/8918b9ee128b30785cd32cc47b2931eea666fac9), 2026-10-31)

**lib**

- shared release-search, torrent-session, and status helpers ([312ed5e](https://github.com/AyoubToueti/Kitsune/commit/312ed5e4cf56f3ba0cfe608c5af38e298e9c5f44), 2026-10-02)

**ListStatusMenu**

- add entry removal functionality and update tests ([8ca8611](https://github.com/AyoubToueti/Kitsune/commit/8ca8611fc661645fc49c773b1e66d7ccecb07d3a), 2026-10-02)

**nav**

- reach the My List page from the navbar and Continue Watching ([dd680f7](https://github.com/AyoubToueti/Kitsune/commit/dd680f72cf52d2d91eeb58bce387b4a0f3f647d5), 2026-09-24)

**NavBar**

- clear search input on navigation away from search page feat(SearchBox): bind search input value and add keyboard shortcut for focus ([c4b0930](https://github.com/AyoubToueti/Kitsune/commit/c4b093002d3c0316bb523254b02aa7258568057c), 2026-10-05)

**README**

- update project description and add features section; add new screenshots ([3da4802](https://github.com/AyoubToueti/Kitsune/commit/3da48022c70a984b156b8ff1dcebfc77a44bb021), 2026-10-02)

**recommendations**

- full page, paging, and voting ([3a01ba5](https://github.com/AyoubToueti/Kitsune/commit/3a01ba59eb2d128b75069bb66f44764377771e86), 2026-10-02)

**release-mode-toggle**

- implement mode toggle for episodes and packs in EpisodeWatchModal ([ae283f5](https://github.com/AyoubToueti/Kitsune/commit/ae283f5c60aa929be33d562939f1529767f5fa0d), 2026-10-05)

**releases**

- thread a packs search mode through the frontend ([7ffd030](https://github.com/AyoubToueti/Kitsune/commit/7ffd030af84edcd9663c0ab50c20091fe95a25e0), 2026-10-04)

**schedule**

- snap the day strip to whole tabs ([bb82eca](https://github.com/AyoubToueti/Kitsune/commit/bb82eca88b17df855345011f33bdf4003165ad9c), 2026-10-01)

**scroll**

- reusable snap-strip and drag composable ([68c86e0](https://github.com/AyoubToueti/Kitsune/commit/68c86e0c28cf859b0f7e6d93081d3d2e8925f99f), 2026-10-01)

**sort**

- add reverse sorting option and update related functionality ([e056816](https://github.com/AyoubToueti/Kitsune/commit/e0568165ccca5496ab5bbc9cab55887b9dd2190b), 2026-10-01)

**speed-test**

- add the shared frontend logic for the speed test ([3482376](https://github.com/AyoubToueti/Kitsune/commit/3482376f7baa201c30dd9a786fa136ce9d48f971), 2026-10-04)

**tests**

- add Big Buck Bunny torrent for CI testing ([ab42468](https://github.com/AyoubToueti/Kitsune/commit/ab42468977ed6b007b177d6b6fb036ef3609ceea), 2026-10-03)

**trending**

- HiAnime-style rail with stacked nav ([6fa9bd4](https://github.com/AyoubToueti/Kitsune/commit/6fa9bd4169bd95e6ed3f4c39c0ed04f44e9999e6), 2026-10-01)

### Fixes

**qt**

- ensure UTF-8 encoding for MSVC and update episode display in DetailPage ([c218128](https://github.com/AyoubToueti/Kitsune/commit/c21812845c641aca0f27f872910804d727d87c9a), 2026-11-04)
- launch as a GUI app on Windows, not a console one ([c554076](https://github.com/AyoubToueti/Kitsune/commit/c55407661de01d59d90a09fc1835242bdb4d00ba), 2026-11-03)
- make the Qt frontend compile on MSVC ([7e8be7e](https://github.com/AyoubToueti/Kitsune/commit/7e8be7ea604c977d1202a3a9836af9bdd53cc599), 2026-10-31)
- drop Qt5Compat from NavBar; narrow the SVG plugin check ([b5a4996](https://github.com/AyoubToueti/Kitsune/commit/b5a4996c9e3c3b9af1371dfaa60af22ad65570da), 2026-10-31)
- key every list write on (provider, id), not the bare id ([f0fc845](https://github.com/AyoubToueti/Kitsune/commit/f0fc845f7b6e793d242b0d078ef1b51870fe2227), 2026-10-30)
- finish a list write even when the work was never cached ([730e7c2](https://github.com/AyoubToueti/Kitsune/commit/730e7c26aa6c60e1c16cc6e94156565973361da4), 2026-10-29)
- make DetailPage survive being rebuilt ([4683fe4](https://github.com/AyoubToueti/Kitsune/commit/4683fe4d188b91d8a8eff329561b457749211e59), 2026-10-28)
- handle sign-in errors and improve error messaging in MyListPage ([6f2d7ae](https://github.com/AyoubToueti/Kitsune/commit/6f2d7ae24764944770d0ec9eecc6c850f7238710), 2026-10-21)
- stop gating playback on a warm-up, and make the player observable ([8e4d80f](https://github.com/AyoubToueti/Kitsune/commit/8e4d80fb2347fc1635d48618116f80b258e899c8), 2026-10-21)
- coerce an optional field to bool in AnimeListRow ([4a92bdf](https://github.com/AyoubToueti/Kitsune/commit/4a92bdf57e0bd27f4b4d1f909ba82ac3701596c7), 2026-10-20)
- refetch releases per episode and separate season packs ([ed2ace4](https://github.com/AyoubToueti/Kitsune/commit/ed2ace4e59e0532a14f3adee2b6e4ac8873a51f4), 2026-10-17)
- number the trending rail from 01, not 06 ([043ebfb](https://github.com/AyoubToueti/Kitsune/commit/043ebfb2396bc91a0d081b3ba6a23d57e187291f), 2026-10-16)
- make the player load on every open, not just the first ([24235df](https://github.com/AyoubToueti/Kitsune/commit/24235dfae08aad3caf6b1943617b0cd9ff274737), 2026-10-16)
- skip XDG desktop portal registration ([327c1c7](https://github.com/AyoubToueti/Kitsune/commit/327c1c7a3ee18af4d990b5d0404e13a9c36d901c), 2026-10-16)
- keep scroll position when the recommendations page loads more ([505fad1](https://github.com/AyoubToueti/Kitsune/commit/505fad1ab944ca2a9217fc29449dffb198dae8cf), 2026-10-16)
- make the seek preview track the cursor while scrubbing ([72127df](https://github.com/AyoubToueti/Kitsune/commit/72127df6a0f1c64c331760878867d7abb8459efc), 2026-10-16)
- show the app's own icon in the OS taskbar ([88e8074](https://github.com/AyoubToueti/Kitsune/commit/88e8074295dde7958377e4e37fd94d5b5cc1bd3e), 2026-10-16)
- stop intermittent black video (mpv render-context race) ([a3288d9](https://github.com/AyoubToueti/Kitsune/commit/a3288d90a091ad89e6fb606419ae54170732d7fb), 2026-10-16)
- let the torrent pause button stop only the torrent ([ae72362](https://github.com/AyoubToueti/Kitsune/commit/ae723626bbf7501463c6130bc77a5dc63476416c), 2026-10-16)
- cache releases and stop the stuck searching spinner ([0cc66cf](https://github.com/AyoubToueti/Kitsune/commit/0cc66cfcd1669b17955962640fc13ce978f6d502), 2026-10-15)
- chapter menu toggles, no dimming, keeps chrome, fits the bar ([3c65a96](https://github.com/AyoubToueti/Kitsune/commit/3c65a9677e58514dfac689df93b9d2b8e4fb0ff7), 2026-10-15)
- stop the OSD row overflowing; move video details to the viewport ([806a231](https://github.com/AyoubToueti/Kitsune/commit/806a23114890bc78bf19072af6c14c57826a219a), 2026-10-15)
- restore player shortcuts; add a video-details panel ([9f0b299](https://github.com/AyoubToueti/Kitsune/commit/9f0b299316afb34c95745a60ac038a1ee0560b38), 2026-10-15)
- drop the hardcoded chapter timing fallback ([b6df8f0](https://github.com/AyoubToueti/Kitsune/commit/b6df8f08cc278d847ddd700504439917720125bd), 2026-10-15)
- make the whole Advanced Genre & Tag row clickable ([87ea411](https://github.com/AyoubToueti/Kitsune/commit/87ea411907729cf39d9b1e7a5519a0970e0e71c4), 2026-10-14)
- stop replaying the slide on the current route; settings as a popup ([b3db8b3](https://github.com/AyoubToueti/Kitsune/commit/b3db8b3ba6088afeea65119cd89e582e0da3e050), 2026-10-14)

**core**

- merge an AniList pull into the Kitsu record it belongs to ([d42648a](https://github.com/AyoubToueti/Kitsune/commit/d42648acbaf10d7ca28be9f9bf2379fb7917dcbd), 2026-10-30)
- cache recommended works so list writes can resolve them ([5b747c0](https://github.com/AyoubToueti/Kitsune/commit/5b747c02f688a17210093df75b7f220ff5b839a8), 2026-10-23)
- stop an airing cour drawing episodes that have not aired ([c2055ef](https://github.com/AyoubToueti/Kitsune/commit/c2055effbec6f0cfa3f9d8248f6f1d2068696caa), 2026-10-23)
- wait for a torrent to go live before handing out its stream URL ([1640095](https://github.com/AyoubToueti/Kitsune/commit/16400959985836e84ba83580490bc615ac50801e), 2026-10-15)
- keep downloaded files when removing a torrent ([4389dde](https://github.com/AyoubToueti/Kitsune/commit/4389dde753d96554a34eb99a8bd3e1c575a01005), 2026-10-15)

**general**

- improve episode count accuracy for long-running series ([48bb08f](https://github.com/AyoubToueti/Kitsune/commit/48bb08f48c37544fbe0a91a043efca939d18bd8f), 2026-10-25)
- register D-Bus service before name request, add auto-raise on deep link ([4b00835](https://github.com/AyoubToueti/Kitsune/commit/4b008359303cae51c4f97a459159f04e4ec3cde6), 2026-10-25)
- add packageManager field to package.json ([b973a8d](https://github.com/AyoubToueti/Kitsune/commit/b973a8d05c2f9b412c824308fb2f9ceb53bebc37), 2026-10-03)
- correct aspect ratio class syntax in AnimeCard and ContinueWatchingCard components ([df45662](https://github.com/AyoubToueti/Kitsune/commit/df4566251b9a5887729ffa4682ec29ac3868697d), 2026-10-02)
- update .gitignore to exclude all hidden files ([20f5183](https://github.com/AyoubToueti/Kitsune/commit/20f5183cd2d4035537b583b79470c0b2e708fc6c), 2026-09-13)

**release**

- resolve libmpv deps against System32, not a hardcoded list ([2947b38](https://github.com/AyoubToueti/Kitsune/commit/2947b38e98892c77fcd7fe8f3078e50da839674a), 2026-11-03)
- treat api-ms-win-* as OS-supplied, not missing files ([5b1a929](https://github.com/AyoubToueti/Kitsune/commit/5b1a929555f0b9d01058a6b5c0f6b74778da4503), 2026-11-03)
- correct the linuxdeploy plugin var; take mpv DLLs from both archives ([7f3765d](https://github.com/AyoubToueti/Kitsune/commit/7f3765deceb0f6983851c0e9ebe4e45fa5232d48), 2026-11-03)
- real AppStream metadata; derive ffmpeg deps from libmpv ([aee2de8](https://github.com/AyoubToueti/Kitsune/commit/aee2de880730ef3ab4f3dfe5689b46f663fc8872), 2026-11-03)
- make the Windows installer payload actually self-contained ([f3e86dd](https://github.com/AyoubToueti/Kitsune/commit/f3e86ddf73607d2550fef15b6256565863ace6a9), 2026-11-03)

**anime**

- stop the hero clipping the list-status menu ([68562fa](https://github.com/AyoubToueti/Kitsune/commit/68562faeb186cb427fb1be4f7c49d391905ba871), 2026-09-24)
- accept a negative recommendation rating ([2bfdc7a](https://github.com/AyoubToueti/Kitsune/commit/2bfdc7af6f1163d1365d8e131961c2e753cdde5f), 2026-09-14)
- repair the trailer url and restore streaming links ([4ff4755](https://github.com/AyoubToueti/Kitsune/commit/4ff47550fae37859149b63f172532f08d78258d7), 2026-09-14)
- request the detail fields from AniList ([51b2e9f](https://github.com/AyoubToueti/Kitsune/commit/51b2e9fe7d2ef4df43dcbb7a304eb27d7c1c9853), 2026-09-14)

**ui**

- move the hover preview animation out of a <style> block ([855cd75](https://github.com/AyoubToueti/Kitsune/commit/855cd75313d7d8ea5412b4c7e1214ffdae81bca9), 2026-09-19)
- share one trending fetch between hero and row ([37105b2](https://github.com/AyoubToueti/Kitsune/commit/37105b2cb39cad66a99df6158a1db15ebd05985f), 2026-09-13)
- measure the hover preview instead of estimating its height ([9530a41](https://github.com/AyoubToueti/Kitsune/commit/9530a419e1c2f85f5502634dd771a661aa5d2e5e), 2026-09-13)
- anchor the preview to the card's centre, not a measured overlap ([bfcd751](https://github.com/AyoubToueti/Kitsune/commit/bfcd7510902c0b9eed93896837ff62a1a44bc997), 2026-09-13)

**anilist**

- pass an explicit userId to MediaListCollection ([1163a62](https://github.com/AyoubToueti/Kitsune/commit/1163a62f9ac56bde447d6923a3ab74bfccd8c571), 2026-09-23)
- send mutations under the query key ([4f120fc](https://github.com/AyoubToueti/Kitsune/commit/4f120fc2cd3075d230bfe5b2bce378e4bc8da2ae), 2026-09-23)
- omit the progress variable when it is unset ([d16aa04](https://github.com/AyoubToueti/Kitsune/commit/d16aa0466fe628e34bccbda23126cb443026d1cf), 2026-09-23)

**flake**

- point GStreamer at the output that holds the core plugins ([58fb2c4](https://github.com/AyoubToueti/Kitsune/commit/58fb2c4423169e45a142e9e6b4239a5d259f559e), 2026-09-14)
- add glib-networking so WebKitGTK can load remote images ([3d3525b](https://github.com/AyoubToueti/Kitsune/commit/3d3525b85322adee77e44ee72758cf272f82a687), 2026-09-13)
- add gstreamer plugins for webkitgtk runtime ([c433965](https://github.com/AyoubToueti/Kitsune/commit/c433965fcc391d52f78043096041cdf0db6e6480), 2026-09-11)

**indexer**

- take the season from any title form for the Sxx query ([7663776](https://github.com/AyoubToueti/Kitsune/commit/7663776ed1da32081d5e0232b87cc73d256bc647), 2026-10-04)
- stop rejecting episode releases that name a season ([c38fe5c](https://github.com/AyoubToueti/Kitsune/commit/c38fe5c6fadde124dd089b38c7582369c1abc157), 2026-09-19)
- probe torrents with no files selected, so probing never downloads ([db0166d](https://github.com/AyoubToueti/Kitsune/commit/db0166da4b06da611e6627091de5d66c7c3a2145), 2026-09-18)

**qml**

- carry the provider when resuming from Continue Watching ([8afbd78](https://github.com/AyoubToueti/Kitsune/commit/8afbd783f7a0ffae96d02c853a774286e484663c), 2026-10-30)
- stop QuickStatusMenu throwing when a status is clicked ([34e32da](https://github.com/AyoubToueti/Kitsune/commit/34e32dac126046b41a7403bf9c03549eaa8eec51), 2026-10-29)
- let the local list work without an AniList account ([9d43497](https://github.com/AyoubToueti/Kitsune/commit/9d43497e8e5e02ec01e459950d29ba93d3fb5d52), 2026-10-29)

**watch**

- record the work even without an episode number ([23d2e24](https://github.com/AyoubToueti/Kitsune/commit/23d2e24bdbbfbcd1c5c4bcbb5a1c0912ff9b781d), 2026-09-24)
- hold the torrent while an external player is open ([b1e61e6](https://github.com/AyoubToueti/Kitsune/commit/b1e61e68499b5efadb34acdef1481ce3f291c314), 2026-09-23)
- release the torrent when the page is left ([7438449](https://github.com/AyoubToueti/Kitsune/commit/74384496d53acb70cb9b0b5f6d9c35474085e05b), 2026-09-22)

**windows**

- match the real subsystem line; add a desktop shortcut ([2ca962b](https://github.com/AyoubToueti/Kitsune/commit/2ca962b13c13cc43a7d21a692f12ec848021300f), 2026-11-03)
- embed the icon and version info in kitsune.exe ([b9c9f79](https://github.com/AyoubToueti/Kitsune/commit/b9c9f79b768f000cfd74fef1f701969e560d5252), 2026-11-03)
- give the installer a real .ico instead of the exe ([c5bfca5](https://github.com/AyoubToueti/Kitsune/commit/c5bfca53db6ca8a0ac7c6739d607bfcbd6116e52), 2026-11-03)

**diagnostics**

- measure the whole process tree, not just the main process ([016295a](https://github.com/AyoubToueti/Kitsune/commit/016295ab476dbc0082aa567b72f707a07b0d21c5), 2026-10-03)
- exclude thread entries from the process-tree sum ([7d98359](https://github.com/AyoubToueti/Kitsune/commit/7d983593d48c05c6e89010b2f4ce5574b6a2cbe4), 2026-10-03)

**episodes**

- reach every episode of a short catalogue ([a21f9c3](https://github.com/AyoubToueti/Kitsune/commit/a21f9c31c009f3264b2651e7ecc76c093af7eb49), 2026-09-22)
- source the episode list from Jikan, not streaming links ([731ddc0](https://github.com/AyoubToueti/Kitsune/commit/731ddc0ae788fa185b047f86da55a00ab416300f), 2026-09-19)

**ci**

- remove redundant comments and clean up workflow structure ([26cbe68](https://github.com/AyoubToueti/Kitsune/commit/26cbe68d0234aae9f8195c36924b96a1df5e8624), 2026-10-03)

**ContinueWatchingCard**

- update resume link to remove episode query parameter ([ae2866a](https://github.com/AyoubToueti/Kitsune/commit/ae2866aacbfa07b08839909726300dfe21dd4da4), 2026-10-02)

**hover**

- close preview when its card scrolls off screen ([5634a89](https://github.com/AyoubToueti/Kitsune/commit/5634a89f66a4f47c66b7801659d22780f9387a2c), 2026-10-01)

**infinite-scroll**

- keep the effect's dependency on the query key ([b51963f](https://github.com/AyoubToueti/Kitsune/commit/b51963f8b9949b54cc4da665c4e68c42850306fc), 2026-10-03)

**kitsune-qt**

- pass MPV_DIR include path to C++ compiler on Windows ([4ebd987](https://github.com/AyoubToueti/Kitsune/commit/4ebd987220a5670d8be8cde9673d5cdbedcf8d2c), 2026-10-31)

**list**

- keep the card menu on screen ([487c6e0](https://github.com/AyoubToueti/Kitsune/commit/487c6e022e58a89b9db15f89180d76d6b0db99df), 2026-09-24)

**Modal**

- adjust max-width for modal dialog; improve layout feat(NowPlayingDisc): enhance hover state and add detailed playback info refactor(InfiniteScroll): remove unused variable in createInfiniteScroll function style(page): update gradient classes for better readability ([f916d6d](https://github.com/AyoubToueti/Kitsune/commit/f916d6d04eefbdf9315c6b77159bb97af73beb81), 2026-10-03)

**player**

- stop the in-app video when the watch page is left ([5984453](https://github.com/AyoubToueti/Kitsune/commit/5984453073fd46b0a8afea56130e5a904cd0f6a2), 2026-09-23)

**progress**

- report a failed progress write instead of swallowing it ([b01b243](https://github.com/AyoubToueti/Kitsune/commit/b01b243fd12f950ce65dd3f7521a0e919d08e70a), 2026-09-24)

**providers**

- retry a 429 after Retry-After instead of failing ([4531aa1](https://github.com/AyoubToueti/Kitsune/commit/4531aa18968c516312fd64322edeb14656a2fd6b), 2026-10-17)

**recommendations**

- ask for the fields the hover preview renders ([23968dd](https://github.com/AyoubToueti/Kitsune/commit/23968ddc6ff3a38aa7ad8472c9c2dd504f93d4bd), 2026-09-22)

**resume**

- refresh the disc and row when the list changes ([a3fb77e](https://github.com/AyoubToueti/Kitsune/commit/a3fb77e92754076069f722913bfe32f5c694cebb), 2026-09-24)

**schedule**

- adjust button width and increase skeleton loading items ([5635d16](https://github.com/AyoubToueti/Kitsune/commit/5635d163452b6ebd9c1928db8a63071b24ac0279), 2026-09-24)

**tauri**

- pass the provider to record_last_played ([4e1fd73](https://github.com/AyoubToueti/Kitsune/commit/4e1fd7367cf8800c07980deea9231564df45b683), 2026-10-24)

### Performance

**qt**

- one persistent preview player + guard the render FBO ([2772f18](https://github.com/AyoubToueti/Kitsune/commit/2772f186d40870983407c40d37864d47f5d84763), 2026-10-15)

### Refactoring

**general**

- remove the Tauri app; kitsune-qt is now the only frontend ([1adfd13](https://github.com/AyoubToueti/Kitsune/commit/1adfd137e2ac3d3f861193a8ca72851755cef154), 2026-10-31) **BREAKING**
- move border overlays to z-indexed rectangle in card QML components ([8b42714](https://github.com/AyoubToueti/Kitsune/commit/8b42714d23163c58474d84673073b0e7baf2e0c5), 2026-10-27)
- move the external-player stack into src-tauri ([76f219e](https://github.com/AyoubToueti/Kitsune/commit/76f219e6ed03b26e8488fafe84ff85862f3d7823), 2026-10-24)
- extract pure `diagnostics` modules into kitsune-core ([109e3b4](https://github.com/AyoubToueti/Kitsune/commit/109e3b4cc8105b83d1fb204ec227de73ef3913ad), 2026-10-12)
- extract `settings::store` into kitsune-core ([4bf84aa](https://github.com/AyoubToueti/Kitsune/commit/4bf84aaa7ee1d2099f1f12fdd95557f9cba12b52), 2026-10-12)
- extract `torrent` into kitsune-core ([5c3c151](https://github.com/AyoubToueti/Kitsune/commit/5c3c1510a577f6149f1f9e57a804a7726c0afb72), 2026-10-12)
- extract `providers` into kitsune-core ([7cf1077](https://github.com/AyoubToueti/Kitsune/commit/7cf10776e8b6a6043f001cff1360bfce70f6ebfe), 2026-10-12)
- extract pure `player::{launch,icons}` into kitsune-core ([837ad75](https://github.com/AyoubToueti/Kitsune/commit/837ad7548e2bfea96356a36771ca318e19e2d1c6), 2026-10-12)
- extract `player::state` + player-choice types into kitsune-core ([c067904](https://github.com/AyoubToueti/Kitsune/commit/c067904fdaf40f33e25c8c8803e690e395996105), 2026-10-12)
- extract the `indexer` pipeline into kitsune-core ([c4669b0](https://github.com/AyoubToueti/Kitsune/commit/c4669b051acedb095224fce7cc58aaa468351200), 2026-10-12)
- extract `auth::{flow,store,last_played}` into kitsune-core ([c9f8163](https://github.com/AyoubToueti/Kitsune/commit/c9f816338c7494d45cf333b943b8a6875983b12e), 2026-10-12)
- extract `types` into kitsune-core ([f385873](https://github.com/AyoubToueti/Kitsune/commit/f385873cc51a5236246dce0f2817357d66c05ff1), 2026-10-12)
- enhance DetailPageSkeleton and ResumeDisc components with improved styles and structure ([612bb95](https://github.com/AyoubToueti/Kitsune/commit/612bb95220489f7664d62bc3d3009fb4ad6c43fd), 2026-10-02)
- enhance NavBar with active link states and improved styles ([fcefb3b](https://github.com/AyoubToueti/Kitsune/commit/fcefb3bec6831fc9e532c9dd17378234b7eee146), 2026-10-02)

**core**

- share episode rules, file matching, and indexer registry ([9206d8a](https://github.com/AyoubToueti/Kitsune/commit/9206d8a08442bf9520ed2b676cadf00574679283), 2026-10-14)
- share probe_batch and torrent download with the Qt frontend ([947d597](https://github.com/AyoubToueti/Kitsune/commit/947d59759c9b32cb3f88872796568d05902c7b4b), 2026-10-14)

**qt**

- name the shipped binary `kitsune` ([a2d0ff4](https://github.com/AyoubToueti/Kitsune/commit/a2d0ff4a5ba6d4cd1df0e9baf845ab0b7d98f52e), 2026-10-31)
- drop narration comments from the filter page ([08604f2](https://github.com/AyoubToueti/Kitsune/commit/08604f2d0f4364b9a1e151ffc2b8a4a127846bd0), 2026-10-18)

**routes**

- drop the /genre route ([46cd599](https://github.com/AyoubToueti/Kitsune/commit/46cd599dd1ecfac519530887b3449c90ad8b6ede), 2026-09-20)
- drop the /browse list route ([8562c3a](https://github.com/AyoubToueti/Kitsune/commit/8562c3aafabb0962ffcb24a134a49a04e7fda046), 2026-09-20)

**anime**

- delete search and by_genre, superseded by browse ([2e44f51](https://github.com/AyoubToueti/Kitsune/commit/2e44f51cc4b312e7d06efe2c584c6b480ec523b4), 2026-09-14)

**AnimeCard, ContinueWatchingCard**

- improve floating quick bar styles for better focus visibility ([c037e6d](https://github.com/AyoubToueti/Kitsune/commit/c037e6d0c5189e1878ac14353a2d74642e3a1a86), 2026-10-02)

**qml**

- move the trending card's action stack into QuickAddButton ([17b2174](https://github.com/AyoubToueti/Kitsune/commit/17b21742d76fcb693e730ae495703d44bc5868df), 2026-10-29)

**ResumeDisc**

- improve floating info card positioning and styles ([e6b0b78](https://github.com/AyoubToueti/Kitsune/commit/e6b0b78e911cebe61dc4ba475aa49111f0c8664f), 2026-10-02)

### Other changes

**general**

- Downloads page: grouped cards, local playback, and downloaded-episode context ([d13dfd8](https://github.com/AyoubToueti/Kitsune/commit/d13dfd8bd385578a97abe4e5f4b63959c541c80a), 2026-10-27)
- Always wait on a multi-file torrent, and fetch only the picked file ([0ee9567](https://github.com/AyoubToueti/Kitsune/commit/0ee9567726c0b416696f03b5d92193ba95060918), 2026-10-05)
- Add a file-name filter to the torrent file lists ([13bcdab](https://github.com/AyoubToueti/Kitsune/commit/13bcdab1d35fe6574985634c98a0f675b31d56f6), 2026-10-05)
- Add pause/resume to the torrent session ([2357a90](https://github.com/AyoubToueti/Kitsune/commit/2357a9064ddd56f959c91f39befead661ee53d46), 2026-10-05)
- Add pause/cancel controls to the buffering panel ([2bdbefb](https://github.com/AyoubToueti/Kitsune/commit/2bdbefb0ff1849f0a5798b01fae08e188280da67), 2026-10-05)
- Keep the status panel when a resume or stream resolve fails ([31b808f](https://github.com/AyoubToueti/Kitsune/commit/31b808f294f660bdeb1608b345238ad86bc52840), 2026-10-05)
- Show a pick-a-file hint when waiting on a multi-file torrent ([4aa0fd6](https://github.com/AyoubToueti/Kitsune/commit/4aa0fd62990ea99d6a6dcc5c4b4cea67c2050787), 2026-10-05)
- Wait for a file pick, resume on pick, pause on player exit ([5352cd6](https://github.com/AyoubToueti/Kitsune/commit/5352cd6ad49d0a2c8c53377f6ec35a369624b3d0), 2026-10-05)
- Add pause/resume torrent commands ([760b3f5](https://github.com/AyoubToueti/Kitsune/commit/760b3f5aa69a61d78de277081af3939326c3440e), 2026-10-05)
- Keep the status panel when setOnlyFiles fails ([970525b](https://github.com/AyoubToueti/Kitsune/commit/970525b7b4c23245e064a9ca9c99588db03be220), 2026-10-05)
- Show the chosen file's buffered bytes, not the whole torrent ([9802624](https://github.com/AyoubToueti/Kitsune/commit/98026243d8c1538ac3c2fc0dc84c765c8a363be6), 2026-10-05)
- Scroll the modal's file list like the releases list ([9e96880](https://github.com/AyoubToueti/Kitsune/commit/9e9688055bb25a8875994846e17d25a25c32addd), 2026-10-05)
- Add playableCount and firstTargetFile file-selection helpers ([d61b9e3](https://github.com/AyoubToueti/Kitsune/commit/d61b9e305fed225b466ea5dd6705310cec0d5d02), 2026-10-05)
- Fix detail-page crash and give every episode a clickable card ([c3476b5](https://github.com/AyoubToueti/Kitsune/commit/c3476b534045a6cb369cc0fb1169362877ca880e), 2026-09-19)
- UI fixes ([a7bb147](https://github.com/AyoubToueti/Kitsune/commit/a7bb147eff1b76f43999da74d7945c2c29c4c2fd), 2026-09-18)
- comment "Where to watch" section ([4654687](https://github.com/AyoubToueti/Kitsune/commit/4654687bfa425d853720ca08181afc4f22065d54), 2026-09-14)
- Add layout-faithful skeleton loaders across the app ([4d89e46](https://github.com/AyoubToueti/Kitsune/commit/4d89e468480b0595b0682c0e0dd7653a60df67ff), 2026-09-14)
- Skeleton loaders now replace every "Loading…" text in the app — all 10 spots across 4 shape families, built from one reusable primitive: ([dc42669](https://github.com/AyoubToueti/Kitsune/commit/dc42669e924aa2055c8824904cc65da344bc5b43), 2026-09-14)

**core**

- skip a list pull that cannot have changed anything ([6827f40](https://github.com/AyoubToueti/Kitsune/commit/6827f403bc96a51d56d11799a9442b8cec140785), 2026-10-23)
- add a local media list with AniList sync ([2f1cc5c](https://github.com/AyoubToueti/Kitsune/commit/2f1cc5c1355d6a9525034243eec799b68376a4c9), 2026-10-22)
- add the Kitsu metadata provider ([3a5b6fb](https://github.com/AyoubToueti/Kitsune/commit/3a5b6fb7ad258059fbaca87d26d00e42b01d6915), 2026-10-22)
- cap synthesised episodes at the aired count ([7859e8f](https://github.com/AyoubToueti/Kitsune/commit/7859e8fc031d92b9704f9d95c12b1c37b9558a85), 2026-10-22)
- let the reader choose the metadata source ([f997d8f](https://github.com/AyoubToueti/Kitsune/commit/f997d8f671202f3e8cf252c852af2bd803a76dbd), 2026-10-22)

**player**

- add the GTK app-chooser module ([417b00a](https://github.com/AyoubToueti/Kitsune/commit/417b00a11c9248f402621d49b1d9b83fca5c6561), 2026-10-05)
- let the reader name a player when none is found ([a0083a3](https://github.com/AyoubToueti/Kitsune/commit/a0083a3b14081abf87fdeac8bbf272d58bd160d3), 2026-10-05)
- enumerate installed players for an in-app picker ([a8d6abd](https://github.com/AyoubToueti/Kitsune/commit/a8d6abd9f1010ae459255255aa7c725eb4066ee2), 2026-10-05)
- launch through the chooser, holding the torrent ([b52d8d4](https://github.com/AyoubToueti/Kitsune/commit/b52d8d4892345f255dba3afa62b0ed370f47af49), 2026-10-05)
- find installed players on Windows ([e955a09](https://github.com/AyoubToueti/Kitsune/commit/e955a09185df6cc48af997f7363b87153f232356), 2026-10-05)

**watch**

- remember the chosen player with an Always action ([10de290](https://github.com/AyoubToueti/Kitsune/commit/10de2904214829446b7820f89b664bded8c7341c), 2026-10-05)
- prompt the player chooser at the buffer threshold ([3cd0645](https://github.com/AyoubToueti/Kitsune/commit/3cd064555a7cd01f51f19aa490c6e61e3d79a9bc), 2026-10-05)
- choose a player from an in-app picker ([7b1a3ec](https://github.com/AyoubToueti/Kitsune/commit/7b1a3ecc21737c649d3d62229fc9c1740619ff40), 2026-10-05)
- prompt the chooser on the watch page too ([b6db4ee](https://github.com/AyoubToueti/Kitsune/commit/b6db4ee21c3685f865c1cc651606c31815f68af3), 2026-10-05)
- add a Choose-player action to the modal ([d6f7303](https://github.com/AyoubToueti/Kitsune/commit/d6f7303c7cf9d47c5c1e70f4d097fc1c2dd78a44), 2026-10-05)

**api**

- wrap the chooser commands ([56b7ea3](https://github.com/AyoubToueti/Kitsune/commit/56b7ea39e613902921a022c619ca0e4940c3b415), 2026-10-05)

**iconsearch**

- add file drawer and natural sort ([457b3c7](https://github.com/AyoubToueti/Kitsune/commit/457b3c71f4d141804aaa3cb438554928aef2e4b3), 2026-10-30)

<details>
<summary>Maintenance (documentation, build, CI, tests, chores)</summary>

### Documentation

**general**

- rewrite the README for the Qt-only app ([1fc65bd](https://github.com/AyoubToueti/Kitsune/commit/1fc65bd9fe23288e4b8f5d7319849def38290f51), 2026-10-31)
- add project master plan ([c215a35](https://github.com/AyoubToueti/Kitsune/commit/c215a350dbac972cd6ac2e4bef859f9e1bf36340), 2026-09-11)

**qt**

- record the GUI subsystem, the exe icon, and two corrected facts ([75c5307](https://github.com/AyoubToueti/Kitsune/commit/75c530735be41b86c589ad5e9c064fd5b5e9f468), 2026-11-03)
- cross-platform build guide and version sync ([cc935a6](https://github.com/AyoubToueti/Kitsune/commit/cc935a6de95439b9734cfe7d3ba9b98922e41f05), 2026-10-31)

### Build

**qt**

- resolve libmpv per platform and gate Unix-only deps ([6f6e415](https://github.com/AyoubToueti/Kitsune/commit/6f6e415f1b98ae4c0bde8d78cbf2b2472a651b63), 2026-10-31)
- AppImage/deb/rpm and NSIS packaging with a release workflow ([7ef4207](https://github.com/AyoubToueti/Kitsune/commit/7ef420725622b21ffe723fecb864d7570d3f8ca3), 2026-10-31)

**general**

- make the workspace version the single source of truth ([041e4e4](https://github.com/AyoubToueti/Kitsune/commit/041e4e42a531d0b04e57dcf43a19edb655c3490e), 2026-10-31)

**nix**

- trim the dev shell to what the Qt app actually needs ([4df0397](https://github.com/AyoubToueti/Kitsune/commit/4df0397600d66c296a6faa20135ca818fff72d7f), 2026-10-31)

### CI

**general**

- authenticate the mpv release lookups ([b119d6e](https://github.com/AyoubToueti/Kitsune/commit/b119d6eb9b76c60efa37ba7451b60820f8754f1d), 2026-11-03)
- map GITHUB_TOKEN into the steps that call the API ([dbf8057](https://github.com/AyoubToueti/Kitsune/commit/dbf80575e3a1803a1e3d70da282f1111f770db26), 2026-11-03)
- stop requesting qtsvg as a Qt module ([1b52c03](https://github.com/AyoubToueti/Kitsune/commit/1b52c03891ec5adc7d84856e13191f248b8310a9), 2026-10-31)
- add manual build-only workflow ([eb8c399](https://github.com/AyoubToueti/Kitsune/commit/eb8c39930a05883cd6365b44e35cebe1e89ae873), 2026-10-04)

**qt**

- build and test the Qt frontend on Linux and Windows ([1845f31](https://github.com/AyoubToueti/Kitsune/commit/1845f312557ec43d3708ed8dfc188e3312c61f7b), 2026-10-31)

### Tests

**general**

- enhance diagnostics mocks for speed test handling ([fa7c116](https://github.com/AyoubToueti/Kitsune/commit/fa7c1160cc5c63386d661839823d414ba660e27d), 2026-10-05)
- align the redesigned-component specs with the current markup ([a7346af](https://github.com/AyoubToueti/Kitsune/commit/a7346afc600e3eae1181d8770b5da03b88555d92), 2026-10-03)
- launch a player from the smoke test ([50ad1d6](https://github.com/AyoubToueti/Kitsune/commit/50ad1d66fc93197e08bd75d71825ade83b1288c4), 2026-09-13)
- add rust test harness ([71ad554](https://github.com/AyoubToueti/Kitsune/commit/71ad554ae06f3b44c9ebeedbb28200fd61fa1377), 2026-09-11)
- prove end-to-end torrent streaming with a real .torrent ([a847bca](https://github.com/AyoubToueti/Kitsune/commit/a847bcaaed2a061aba9904c932785a6a03588147), 2026-09-11)
- add frontend test harness ([f948a15](https://github.com/AyoubToueti/Kitsune/commit/f948a155ea7d8afedcb4e5793aaa3ba5cbb900f1), 2026-09-11)

**AnimeCard**

- assert the shorthand focus-visible class ([80d7f2d](https://github.com/AyoubToueti/Kitsune/commit/80d7f2dd15dc3742c29997ed313c255d1ed72479), 2026-10-02)

**indexer**

- prove probing and playback cannot share one session ([d24ec46](https://github.com/AyoubToueti/Kitsune/commit/d24ec462b413346603edc9610ebfc65b44bdab38), 2026-09-18)

**player**

- assert the buffering hint's real gate ([5e5198d](https://github.com/AyoubToueti/Kitsune/commit/5e5198d1e665bd3f71915fdd1a84c11a352a5054), 2026-09-19)

### Style

**qt**

- shift the palette to a crimson accent ([282efd4](https://github.com/AyoubToueti/Kitsune/commit/282efd47b7057468b1ebd1e687360169dd4c72cb), 2026-10-22)

**watch**

- put the release actions inside their cell ([34a66cf](https://github.com/AyoubToueti/Kitsune/commit/34a66cf6f86e4fdc1532cf219315372e27b98b41), 2026-09-20)

### Chores

**general**

- drop the last Svelte-specific editor setting ([68558e8](https://github.com/AyoubToueti/Kitsune/commit/68558e86569809be34577adb5722925909ee8d5b), 2026-10-31)
- add cargo workspace and Qt6 toolchain spike ([4d5791f](https://github.com/AyoubToueti/Kitsune/commit/4d5791f914ec3143526f92606944afb8400eb2c0), 2026-10-12)
- add tailwind v4 for the frontend ([06b0734](https://github.com/AyoubToueti/Kitsune/commit/06b07345176bfdf7e4593d21828e75c3005953eb), 2026-09-11)
- replace blanket dotfile ignore with explicit rules ([8a91b77](https://github.com/AyoubToueti/Kitsune/commit/8a91b77463799971f38bb2c8dbbb5463517b3493), 2026-09-11)
- scaffold tauri + svelte app ([b5a4ab0](https://github.com/AyoubToueti/Kitsune/commit/b5a4ab0ddd017286b279952720b14bc578458825), 2026-09-11)
- point nix env selector at the project flake ([c80d3dd](https://github.com/AyoubToueti/Kitsune/commit/c80d3dd359dc88ac298bf1c6062e8cf95c1518a3), 2026-09-11)
- add nix dev shell for pinned toolchain ([f63db60](https://github.com/AyoubToueti/Kitsune/commit/f63db60a7e2ff0c797485ba30026bb70aef66c35), 2026-09-11)
- configure app metadata and window defaults ([fc5c596](https://github.com/AyoubToueti/Kitsune/commit/fc5c5967d374335fc1cbf3330f799dca77d12b90), 2026-09-11)

**qt**

- silence the harmless qmlRegisterType startup warning ([2ce7203](https://github.com/AyoubToueti/Kitsune/commit/2ce7203ae7128aae3f8f164386bda3a65b1824a7), 2026-10-16)
- KITSUNE_OPEN_EPISODE dev aid for the watch flow ([7526c61](https://github.com/AyoubToueti/Kitsune/commit/7526c61a2fcabe4436f19d59ec9cf0c2b98d7330), 2026-10-14)
- KITSUNE_AUTOPLAY dev aid; verify the play stage end to end ([f46a7b1](https://github.com/AyoubToueti/Kitsune/commit/f46a7b1c0a0a96030685d9e0bfd376b67d38b412), 2026-10-14)

**indexer**

- scope the test-only `unresolved` helper to tests ([90c0c97](https://github.com/AyoubToueti/Kitsune/commit/90c0c97094b1281b1f884da22f8f0a16b6d53e99), 2026-09-18)

**tauri**

- deduplicate the invoke_handler list ([dd97c34](https://github.com/AyoubToueti/Kitsune/commit/dd97c34e2379fc5a646fcbadbb14dc62e23d467a), 2026-09-22)

</details>
