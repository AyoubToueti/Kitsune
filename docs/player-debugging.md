# Debugging the embedded player

The Qt player has two independent halves, and a "stuck at buffering" report is
almost always one specific one. This is how to tell which.

## Turning logging on

    RUST_LOG=debug KITSUNE_MPV_LOG=1 cargo run -p kitsune-qt

| Variable | What it adds |
| --- | --- |
| *(none)* | Startup line, `prepare_stream` stage timings, `END_FILE` failures |
| `RUST_LOG=debug` | Our code, including the per-poll torrent sampler |
| `KITSUNE_MPV_LOG=1` | mpv's own log messages, per-event lines, the 1 Hz sampler |

Load failures (`END_FILE`) are **always** logged. They are never noise.

Logs go to stdout and to a rotating file:

    ~/.local/share/kitsune/logs/kitsune.log.YYYY-MM-DD

Note the date suffix: the appender rotates daily, so there is no plain
`kitsune.log`.

## The timeline of one play

Grep for these in order. Each line is stamped, so a slow stage is visible as a
gap.

    prepare_stream: torrent=N file=M starting
    set_only_files: torrent N is initializing|live after Xms
    set_only_files: torrent N VERIFIED only_files=Some([M]) on attempt 1 after Xms
    warm head ok in Xms, fetched N bytes
    warm length probe ok in Xms: len=N bytes
    prepare_stream: torrent=N file=M ready in Xms -> http://...

`ready in` is how long the user waits **before mpv is even asked to open
anything**. If that number is small and playback is still slow, the problem is
in mpv or the swarm, not in the engine setup.

## The three failures this instrumented for

### 1. Wrong file being downloaded

    prepare_stream: set_only_files(M) NOT accepted after 30000ms, engine keeps
    its own file order

The engine refused the selection and is downloading files in *its* order, so
the byte counter climbs while the episode being watched stays at zero.

### 2. mpv blocked on the end of the file

    [mpv/ffmpeg] stream level seek from 732442 to 1428012632
    (Buffering) AV: 00:00:07 / 00:23:20 (1%) Cache: 0.0s/1KB
    [mpv/cplayer] Enter buffering (buffer went from 100% -> 0%)

mpv decoded a few seconds, then seeked to the **end** of a 1.4 GB file for the
Matroska `Cues` index and blocked. This is why a partially downloaded episode
can look 40% complete and still not play: the missing part is the tail.

The fix is the background tail warm-up:

    warm tail ok in 116ms, fetched 4194304 bytes (range=bytes=1423844764-)

A tail line that says `delivered 0 bytes` means the request was answered but
nothing was fetched -- the pieces were never pulled from peers.

### 3. A torrent that is simply slow

    [mpv] t=+30s pos=25.27s dur=1400.62s cache-end=52.01s seekable=1
          last-end=50.01s fw-bytes=0.2MiB idle=1
    torrent 0: state=live peers=3 dl=0.8MiB/s total=120MB/1428MB finished=false

`cache-end` tracking ahead of `pos` with a healthy `dl` is normal. `cache-end`
stuck while `dl` is fine means mpv is not asking for the bytes it needs.

## What "good" looks like

From a cold download directory, on a healthy swarm:

- `prepare_stream ... ready in` under about 1.5 s
- `warm tail ok ... fetched 4194304 bytes`
- **no** `Enter buffering` line at all
- the 1 Hz sampler showing `pos` advancing every second