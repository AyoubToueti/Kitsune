// Adds jest-dom matchers (toBeInTheDocument, toHaveTextContent, ...) to
// Vitest's expect. The @testing-library/svelte Vite plugin handles
// component cleanup between tests.
import "@testing-library/jest-dom/vitest";

// jsdom implements neither of these, and any test that renders a <video> and
// then unmounts reaches them: teardown releases the media element, which is
// what stops WebKitGTK's pipeline in the real app. Left unimplemented, every
// such test prints "Not implemented" to the console.
//
// Plain no-ops rather than spies, because they run in the setup file before any
// test module. A test that needs to ASSERT on them (VideoPlayer.test.ts)
// replaces these with its own mock when its module loads.
HTMLMediaElement.prototype.load = () => {};
HTMLMediaElement.prototype.pause = () => {};