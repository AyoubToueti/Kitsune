// Adds jest-dom matchers (toBeInTheDocument, toHaveTextContent, ...) to
// Vitest's expect. The @testing-library/svelte Vite plugin handles
// component cleanup between tests.
import "@testing-library/jest-dom/vitest";