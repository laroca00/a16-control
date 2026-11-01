import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import type { PowerView } from "../api";
import { ModeSwitch } from "./ModeSwitch";

const view: PowerView = {
  mode: "auto",
  profile: "balancedAc",
  reason: "autoAc",
  limits: { fppt: 80000, sppt: 65000, spl: 25000 },
  options: { autoGame: false, acdcSwitch: true },
  lastError: null,
};

test("mode switch selects and reports", async () => {
  const onChange = vi.fn();
  render(
    <I18nProvider initial="en">
      <ModeSwitch power={view} onChange={onChange} />
    </I18nProvider>,
  );
  expect(screen.getByRole("radio", { name: "Auto" })).toHaveAttribute("aria-checked", "true");
  expect(screen.getByText("Plugged in, balanced")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("radio", { name: "Performance" }));
  expect(onChange).toHaveBeenCalledWith("performance");
});
