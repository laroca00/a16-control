import * as matchers from "@testing-library/jest-dom/matchers";
// Types only: brings the matcher declarations onto vitest's expect.
import type {} from "@testing-library/jest-dom/vitest";

// Extend the runner's own expect; importing jest-dom/vitest can bind to a second
// vitest instance once the Tailwind plugin pulls jiti into the module graph.
expect.extend(matchers);

// jsdom lacks these; the glass and gradient components only need them to exist.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver;
window.matchMedia ??= ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
  dispatchEvent: () => false,
})) as unknown as typeof window.matchMedia;
HTMLCanvasElement.prototype.getContext = (() => null) as unknown as HTMLCanvasElement["getContext"];
