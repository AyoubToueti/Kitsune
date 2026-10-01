import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";
import { fileURLToPath } from "node:url";

// Kept separate from vite.config.js so SvelteKit's dev/build plugin chain
// is not involved when running unit tests.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  resolve: {
    alias: {
      // SvelteKit provides $lib via .svelte-kit/tsconfig.json, which Vitest
      // does not read, so map it explicitly.
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
      // $app/state is virtual, supplied by the sveltekit() plugin which is
      // not loaded here. Point it at a stub so specs resolve the import and
      // can set the URL themselves.
      "$app/state": fileURLToPath(
        new URL("./src/test/app-state-stub.ts", import.meta.url),
      ),
      // Same reasoning: `goto` is virtual too, and a spec asserting that an
      // episode click navigates needs a seam to observe it.
      "$app/navigation": fileURLToPath(
        new URL("./src/test/app-navigation-stub.ts", import.meta.url),
      ),
      // Same reasoning: `$app/environment` is virtual too, and the theme module
      // reads `browser` from it.
      "$app/environment": fileURLToPath(
        new URL("./src/test/app-environment-stub.ts", import.meta.url),
      ),
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.js"],
    include: ["src/**/*.{test,spec}.{js,ts}"],
  },
});