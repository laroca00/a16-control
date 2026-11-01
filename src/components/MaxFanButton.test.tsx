import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import type { Telemetry } from "../api";
import { MaxFanButton } from "./MaxFanButton";

const base: Telemetry = { rpm1: 4400, rpm2: 4900, cpuTemp: 70, gpuTemp: 60, cpuDuty: 50, gpuDuty: 50, fanMode: "auto" };

function renderButton(telemetry: Telemetry | null, onToggle = vi.fn()) {
  render(
    <I18nProvider initial="en">
      <MaxFanButton telemetry={telemetry} busy={false} onToggle={onToggle} />
    </I18nProvider>,
  );
  return onToggle;
}

test("shows live rpm when max", () => {
  renderButton({ ...base, fanMode: "max", rpm1: 5820, rpm2: 6040 });
  const button = screen.getByRole("button");
  expect(button).toHaveTextContent("5820");
  expect(button).toHaveTextContent("6040");
  expect(button).toHaveAttribute("aria-pressed", "true");
});

test("button_reflects_telemetry_not_click", async () => {
  const onToggle = renderButton(base);
  await userEvent.click(screen.getByRole("button"));
  expect(onToggle).toHaveBeenCalledWith("max");
  expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "false");
});

test("asks for auto when currently max", async () => {
  const onToggle = renderButton({ ...base, fanMode: "max" });
  await userEvent.click(screen.getByRole("button"));
  expect(onToggle).toHaveBeenCalledWith("auto");
});

test("is disabled without telemetry", () => {
  renderButton(null);
  expect(screen.getByRole("button")).toBeDisabled();
});
