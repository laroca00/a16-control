import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import type { Telemetry } from "../api";
import { FanModeSwitch } from "./FanModeSwitch";

const base: Telemetry = { rpm1: 2400, rpm2: 2700, cpuTemp: 60, gpuTemp: 50, cpuDuty: 25, gpuDuty: 25, fanMode: "quiet" };

test("fan mode switch shows the current mode and picks another", async () => {
  const onChange = vi.fn();
  render(
    <I18nProvider initial="en">
      <FanModeSwitch telemetry={base} busy={false} onChange={onChange} />
    </I18nProvider>,
  );
  expect(screen.getByRole("radio", { name: "Quiet" })).toHaveAttribute("aria-checked", "true");
  await userEvent.click(screen.getByRole("radio", { name: "Auto" }));
  expect(onChange).toHaveBeenCalledWith("auto");
  await userEvent.click(screen.getByRole("radio", { name: "Quiet" }));
  expect(onChange).toHaveBeenCalledTimes(1);
});
