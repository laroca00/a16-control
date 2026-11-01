import { en } from "./en";
import { tr } from "./tr";
import { format } from "./index";

test("tr and en define the same keys", () => {
  expect(Object.keys(tr).sort()).toEqual(Object.keys(en).sort());
});

test("t interpolates vars", () => {
  expect(format("{rpm} RPM", { rpm: 5820 })).toBe("5820 RPM");
});

test("unknown vars are left as written", () => {
  expect(format("{a} and {b}", { a: 1 })).toBe("1 and {b}");
});
