# librqbit

> **PATCHED COPY — do not pull upstream changes over this blindly.**
>
> Kitsune vendors librqbit 9.0.1 (`[patch.crates-io]` in the workspace root)
> with two behavioural fixes, kept as `patches/librqbit-streaming-fixes.patch`
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