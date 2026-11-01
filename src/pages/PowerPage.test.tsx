import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { api, type PowerView } from "../api";
import { PowerPage } from "./PowerPage";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const view: PowerView = {
  mode: "performance",
  profile: "performance",
  reason: "userPerformance",
  limits: { fppt: 80000, sppt: 65000, spl: 40000 },
  options: { autoGame: false, acdcSwitch: true },
  lastError: null,
};

vi.mock("../api", async (orig) => {
  const real = await orig<typeof import("../api")>();
  return {
    ...real,
    api: {
      getPower: vi.fn(() => Promise.resolve(view)),
      setPowerMode: vi.fn(() => Promise.resolve(view)),
      setPowerOption: vi.fn(() => Promise.resolve({ ...view, options: { autoGame: true, acdcSwitch: true } })),
      getAfterburner: vi.fn(() => Promise.resolve({ installed: true, options: { enabled: false, ocProfile: 1, stockProfile: 5 } })),
      setAfterburner: vi.fn((options) => Promise.resolve({ installed: true, options })),
      getBattery: vi.fn(() =>
        Promise.resolve({
          watching: false,
          progress: 0,
          summary: { avgWatts: 12.5, avgCpu: 18, idlePct: 40, targets: ["chrome", "slack"], minutes: 10 },
          ecoEnabled: true,
          targets: ["chrome", "slack"],
        }),
      ),
      startWatch: vi.fn(() => Promise.reject("unplug from power first")),
      cancelWatch: vi.fn(),
      setEcoEnabled: vi.fn(),
      getGpu: vi.fn(() => Promise.resolve({ state: "d3", igpuOnBattery: true, autoReset: false })),
      setGpuOption: vi.fn(() => Promise.resolve({ state: "d3", igpuOnBattery: true, autoReset: true })),
      getConfig: vi.fn(() => Promise.resolve({ lang: null })),
      setLang: vi.fn(() => Promise.resolve()),
    },
  };
});

function renderPage() {
  render(
    <I18nProvider initial="en">
      <PowerPage />
    </I18nProvider>,
  );
}

test("power page shows the applied limits in watts", async () => {
  renderPage();
  expect(await screen.findByText("40 W")).toBeInTheDocument();
  expect(screen.getByText("80 W")).toBeInTheDocument();
});

test("power page toggles options", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("switch", { name: /game is detected/ }));
  expect(api.setPowerOption).toHaveBeenCalledWith("autoGame", true);
});

test("power page shows the dGPU state and toggles reset", async () => {
  renderPage();
  expect(await screen.findByText("Graphics card: asleep")).toBeInTheDocument();
  await userEvent.click(screen.getByRole("switch", { name: /stays awake/ }));
  expect(api.setGpuOption).toHaveBeenCalledWith("autoReset", true);
});

test("afterburner can be enabled and the overclock profile chosen", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("switch", { name: /Afterburner/ }));
  expect(api.setAfterburner).toHaveBeenCalledWith({ enabled: true, ocProfile: 1, stockProfile: 5 });
  await userEvent.selectOptions(screen.getByRole("combobox", { name: "Overclock profile" }), "3");
  expect(api.setAfterburner).toHaveBeenLastCalledWith({ enabled: true, ocProfile: 3, stockProfile: 5 });
});

test("summary lists targets", async () => {
  renderPage();
  expect(await screen.findByText("12.5 W")).toBeInTheDocument();
  expect(screen.getByText("chrome")).toBeInTheDocument();
  expect(screen.getByText("slack")).toBeInTheDocument();
});

test("watcher start reports why it cannot run on AC", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("button", { name: "Watch for 10 minutes" }));
  expect(await screen.findByText(/Unplug the charger/)).toBeInTheDocument();
});
