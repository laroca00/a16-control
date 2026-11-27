import { compareVersions } from "./update";

test("versions compare number by number", () => {
  expect(compareVersions("0.3.0", "0.2.0")).toBe(1);
  expect(compareVersions("v0.2.0", "0.2.0")).toBe(0);
  expect(compareVersions("0.10.0", "0.9.9")).toBe(1);
  expect(compareVersions("0.2", "0.2.1")).toBe(-1);
});
