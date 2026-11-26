# librqbit

> **PATCHED COPY — do not pull upstream changes over this blindly.**
>
> Kitsune vendors librqbit 9.0.1 (`[patch.crates-io]` in the workspace root)
> with four behavioural fixes, kept as `patches/librqbit-streaming-fixes.patch`
> and re-applied by `scripts/update-vendor.sh` on updates:
>
> - **Ghost-read repair** (the mid-episode black screen): a stream read that
>   hits a piece the have-bitfield claims is present but the storage cannot
>   provide (a RAM-cache eviction ghost) used to kill the HTTP stream
>   unrecoverably — the bitfield was never repaired, so every player retry
>   died on the same ghost piece and playback black-screened.
>   - `src/torrent_state/streaming.rs` — on a "not resident" read failure, the
>     stream waits (`Poll::Pending`) instead of erroring, and the piece is
>     re-queued for re-download (bounded by `GHOST_MISSES_LIMIT` consecutive
>     misses).
>   - `src/chunk_tracker.rs` / `src/piece_tracker.rs` /
>     `src/torrent_state/live/mod.rs` — `mark_piece_ghost_broken` clears the
>     stale have-bit, re-queues the piece and reconnects peers.
> - **Resume-after-idle stalls**: a stream waiting for a missing piece is
>   only woken when a piece completes, so a stalled swarm slept through the
>   peer death, the exponential backoff it accumulated while nobody was
>   reading (10s -> 60s -> 6min -> ...), and the reconnect — resuming a
>   paused episode could take minutes.
>   - `src/torrent_state/live/mod.rs` — `kick_dead_peers` resets dead peers'
>     backoffs and re-queues them (throttled), and a `stream_watchdog` task
>     wakes streams that have waited >4s for their current piece.
>   - `src/torrent_state/streaming.rs` — a `pending_since` stamp per stream
>     feeds the watchdog.
> - **Stream-aware piece stealing** (the long seek stall): when a stream
>   seeks to a not-yet-downloaded position, its target piece can sit
>   in-flight with a slow peer for tens of seconds (observed: 41s), because
>   `try_steal` always picked the globally-oldest in-flight piece — free
>   fast peers kept stealing unrelated sequential-frontier pieces instead of
>   the one the player was blocked on.
>   - `src/piece_tracker.rs` — `acquire_piece` collects the streaming
>     priority set once, and `try_steal` prefers pieces a stream is currently
>     waiting on (oldest-first within that set, same staleness thresholds,
>     falling back to the global-oldest behaviour when no stream piece is
>     eligible). Regression test:
>     `piece_tracker::tests::
>     test_steal_prefers_stream_wanted_piece_over_globally_oldest`.
> - **Per-file download order** (per-file priorities in a pack): upstream
>   rebuilds its piece queue from the filename-sorted file list every time a
>   torrent resumes, so a caller's file order only lasted until the next
>   pause/resume. The order is now kept on the shared torrent state and
>   reinstalled whenever the live state is rebuilt.
>   - `src/torrent_state/mod.rs` — `ManagedTorrent::update_file_priorities`
>     validates a supplied order (unknown or duplicate indices are ignored,
>     unlisted files follow in their default order), stores it on the shared
>     state and forwards it to the live state.
>   - `src/torrent_state/live/mod.rs` — `TorrentStateLive::new` consumes the
>     stored order when present, falling back to the filename sort;
>     `set_file_priority_order` is the live-state entry point.
>   - `src/session.rs` — `Session::update_file_priorities` wrapper for the
>     engine side.
>   Regression tests: `torrent_state::file_priority_order_tests::
>   {a_partial_order_is_completed_with_the_left_out_files,
>   duplicates_and_out_of_range_values_are_rejected,
>   an_empty_order_is_the_identity}` and
>   `chunk_tracker::tests::iter_queued_pieces_follows_the_file_priority_order`
>   (the tracker hands out pieces in the requested file order).
>
> One logging tweak: the initial-check line now reports free disk space on the
> download volume (`src/torrent_state/initializing.rs`), so a resumed torrent's
> "have 0" cannot be misread as "the disk has nothing free".
>
> Companion app-side hardening lives in
> `crates/kitsune-core/src/torrent/ram_storage.rs` (tiered eviction that
> keeps recently-read pieces resident). The regression tests are
> `crates/kitsune-core/tests/ram_stream_e2e.rs::
> seek_back_into_evicted_data_recovers_by_re_downloading` and
> `...::read_after_long_idle_recovers_without_waiting_out_peer_backoff`.

A fully featured, easy to use torrent downloading library used as a backbone of [rqbit](https://github.com/ikatson/rqbit).

## Basic example
See [examples on GitHub](https://github.com/ikatson/rqbit/tree/main/crates/librqbit/examples).

## Documentation
[librqbit at docs.rs](https://docs.rs/librqbit/latest/librqbit/)