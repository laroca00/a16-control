import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { api, type SetupStatus } from "../api";
import { QuickPanel } from "./QuickPanel";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock("../api", async (orig) => {
  const real = await orig<typeof import("../api")>();
  return {
    ...real,
    api: {
      getSetupStatus: vi.fn(),
      getPower: vi.fn(() =>
        Promise.resolve({
          mode: "auto",
          profile: "balancedAc",
          reason: "autoAc",
          limits: { fppt: 80000, sppt: 65000, spl: 25000 },
          options: { autoGame: false, acdcSwitch: true },
          lastError: null,
        }),
      ),
      setPowerMode: vi.fn(),
      setPowerOption: vi.fn(),
      getTelemetry: vi.fn(() => Promise.reject("setup")),
      setFan: vi.fn(),
      runInstall: vi.fn(() => Promise.resolve()),
      openMain: vi.fn(() => Promise.resolve()),
      getConfig: vi.fn(() => Promise.resolve({ lang: null })),
      setLang: vi.fn(() => Promise.resolve()),
    },
  };
});

const ready: SetupStatus = { elevated: true, task: true, wmiSet: true, wmiGet: true, getClass: true, keyboardLight: false };

function renderPanel(status: SetupStatus) {
  vi.mocked(api.getSetupStatus).mockResolvedValue(status);
  render(
    <I18nProvider initial="en">
      <QuickPanel />
    </I18nProvider>,
  );
}

test("shows setup notice when setup needed", async () => {
  renderPanel({ ...ready, wmiSet: false });
  await userEvent.click(await screen.findByRole("button", { name: "Set up" }));
  expect(api.runInstall).toHaveBeenCalled();
});

test("shows keyboard light warning", async () => {
  renderPanel({ ...ready, keyboardLight: true });
  expect(await screen.findByText(/KeyboardLight is still installed/)).toBeInTheDocument();
});

test("no notices when everything is in place", async () => {
  renderPanel(ready);
  await screen.findByRole("button", { name: "All settings" });
  expect(screen.queryByRole("button", { name: "Set up" })).not.toBeInTheDocument();
});

test("explains an unsupported sensor interface instead of offering setup", async () => {
  renderPanel({ ...ready, wmiGet: false, getClass: true });
  expect(await screen.findByText("This laptop's sensor interface is not supported")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Set up" })).not.toBeInTheDocument();
});

test("shows why the fans cannot be reached", async () => {
  vi.mocked(api.getTelemetry).mockRejectedValueOnce("firmware interface unavailable: no instance");
  renderPanel(ready);
  expect(await screen.findByText(/no instance/)).toBeInTheDocument();
});

test("panel shows current reason", async () => {
  renderPanel(ready);
  expect(await screen.findByText("Plugged in, balanced")).toBeInTheDocument();
});
