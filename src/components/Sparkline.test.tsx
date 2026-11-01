import { render } from "@testing-library/react";
import { Sparkline } from "./Sparkline";

function pathOf(values: number[]) {
  const { container } = render(<Sparkline values={values} max={10} label="rpm" />);
  return container.querySelector("path")!.getAttribute("d")!;
}

test("sparkline path has one point per value", () => {
  expect(pathOf([1, 2, 3]).match(/[ML]/g)).toHaveLength(3);
});

test("sparkline clamps values above max to the top", () => {
  const d = pathOf([20]);
  expect(d).toMatch(/^M0(\.0+)?,0(\.0+)?$/);
});

test("sparkline renders nothing to draw for no values", () => {
  expect(pathOf([])).toBe("");
});
