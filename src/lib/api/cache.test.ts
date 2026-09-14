import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

import { API_TTL, cached, clearApiCache } from "./cache";

beforeEach(() => {
  clearApiCache();
  vi.useRealTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("cached", () => {
  it("runs the loader on a miss and returns its value", async () => {
    const load = vi.fn().mockResolvedValue("value");

    const result = await cached("key", 1000, load);

    expect(result).toBe("value");
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("serves a second call from the cache within the TTL", async () => {
    const load = vi.fn().mockResolvedValue("value");

    await cached("key", 1000, load);
    const second = await cached("key", 1000, load);

    // The loader must not run again: that is the whole point.
    expect(load).toHaveBeenCalledTimes(1);
    expect(second).toBe("value");
  });

  it("keys entries separately", async () => {
    const load = vi.fn((tag: string) => Promise.resolve(tag));

    const a = await cached("a", 1000, () => load("a"));
    const b = await cached("b", 1000, () => load("b"));

    expect(a).toBe("a");
    expect(b).toBe("b");
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("refetches once the TTL has elapsed", async () => {
    vi.useFakeTimers();
    const load = vi.fn().mockResolvedValue("value");

    await cached("key", 1000, load);
    // One millisecond past expiry: the entry is stale.
    vi.advanceTimersByTime(1001);
    await cached("key", 1000, load);

    expect(load).toHaveBeenCalledTimes(2);
  });

  it("still serves an entry that has not expired", async () => {
    vi.useFakeTimers();
    const load = vi.fn().mockResolvedValue("value");

    await cached("key", 1000, load);
    // Just inside the window.
    vi.advanceTimersByTime(999);
    await cached("key", 1000, load);

    expect(load).toHaveBeenCalledTimes(1);
  });

  it("shares one request between concurrent callers", async () => {
    let release: (value: string) => void = () => {};
    const load = vi.fn(
      () => new Promise<string>((resolve) => (release = resolve)),
    );

    // Two callers ask for the same key before either resolves.
    const first = cached("key", 1000, load);
    const second = cached("key", 1000, load);

    release("value");
    const [a, b] = await Promise.all([first, second]);

    expect(load).toHaveBeenCalledTimes(1);
    expect(a).toBe("value");
    expect(b).toBe("value");
  });

  it("does not cache a rejection", async () => {
    const load = vi
      .fn()
      .mockRejectedValueOnce("boom")
      .mockResolvedValueOnce("value");

    await expect(cached("key", 1000, load)).rejects.toBe("boom");
    // The failure must be retried, not held for the TTL.
    const second = await cached("key", 1000, load);

    expect(second).toBe("value");
    expect(load).toHaveBeenCalledTimes(2);
  });

  it("allows a retry after a concurrent failure", async () => {
    let reject: (reason: string) => void = () => {};
    const load = vi.fn(
      () => new Promise<string>((_resolve, rej) => (reject = rej)),
    );

    const first = cached("key", 1000, load);
    const second = cached("key", 1000, load);

    reject("boom");
    await expect(first).rejects.toBe("boom");
    await expect(second).rejects.toBe("boom");

    // The in-flight entry must be gone, so a later call runs the loader again.
    await cached("key", 1000, vi.fn().mockResolvedValue("value"));
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("evicts the least recently used entry past the cap", async () => {
    // 200 is the cap; 201 entries forces exactly one eviction.
    for (let i = 0; i < 201; i += 1) {
      await cached(`key-${i}`, 60_000, () => Promise.resolve(i));
    }

    const reloaded = vi.fn().mockResolvedValue("again");
    await cached("key-0", 60_000, reloaded);

    // key-0 was the oldest and should have been evicted.
    expect(reloaded).toHaveBeenCalledTimes(1);
  });

  it("keeps a recently read entry when the cap is reached", async () => {
    // Fill exactly to the cap.
    for (let i = 0; i < 200; i += 1) {
      await cached(`key-${i}`, 60_000, () => Promise.resolve(i));
    }

    // Read key-0, which marks it recently used and moves it to the back.
    await cached("key-0", 60_000, () => Promise.resolve("reloaded"));

    // One more entry overflows the cap, evicting the least recently used --
    // which is now key-1, not key-0.
    await cached("key-200", 60_000, () => Promise.resolve(200));

    const survivor = vi.fn().mockResolvedValue("reloaded");
    expect(await cached("key-0", 60_000, survivor)).toBe(0);
    expect(survivor).not.toHaveBeenCalled();

    const evicted = vi.fn().mockResolvedValue("fresh");
    await cached("key-1", 60_000, evicted);
    expect(evicted).toHaveBeenCalledTimes(1);
  });
});

describe("clearApiCache", () => {
  it("forgets a cached value", async () => {
    const load = vi.fn().mockResolvedValue("value");

    await cached("key", 1000, load);
    clearApiCache();
    await cached("key", 1000, load);

    expect(load).toHaveBeenCalledTimes(2);
  });

  it("forgets in-flight state", async () => {
    const load = vi.fn().mockResolvedValue("value");

    await cached("key", 1000, load);
    clearApiCache();

    const reloaded = vi.fn().mockResolvedValue("fresh");
    expect(await cached("key", 1000, reloaded)).toBe("fresh");
  });
});

describe("API_TTL", () => {
  it("orders the catalogue as the longest lived", () => {
    // The catalogue barely changes; the schedule moves by the minute.
    expect(API_TTL.catalogue).toBeGreaterThan(API_TTL.list);
    expect(API_TTL.list).toBeGreaterThan(API_TTL.schedule);
  });
});