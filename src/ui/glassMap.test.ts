import { displacementAt } from "./glassMap";

test("displacement is zero deep inside the shape", () => {
  expect(displacementAt(50, 30, 100, 60, 20)).toEqual([0, 0]);
});

test("displacement points inward near the left edge", () => {
  const [dx, dy] = displacementAt(1, 30, 100, 60, 20);
  expect(dx).toBeGreaterThan(0);
  expect(Math.abs(dy)).toBeLessThan(1e-6);
});

test("displacement points inward near the bottom edge", () => {
  const [, dy] = displacementAt(50, 59, 100, 60, 20);
  expect(dy).toBeLessThan(0);
});

test("displacement stays within -1..1", () => {
  for (const [x, y] of [[0, 0], [100, 60], [3, 57], [97, 2]]) {
    const [dx, dy] = displacementAt(x, y, 100, 60, 20);
    expect(Math.abs(dx)).toBeLessThanOrEqual(1);
    expect(Math.abs(dy)).toBeLessThanOrEqual(1);
  }
});
