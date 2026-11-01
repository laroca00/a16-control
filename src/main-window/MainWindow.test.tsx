import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { api } from "../api";
import { MainWindow } from "./MainWindow";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock("../api", async (orig) => {
  const real = await orig<typeof import("../api")>();
  return {
    ...real,
    api: {
      getSetupStatus: vi.fn(() =>
        Promise.resolve({ elevated: true, task: true, wmiSet: true, wmiGet: true, getClass: true, keyboardLight: false }),
      ),
      getTelemetry: vi.fn(() =>
        Promise.resolve({ rpm1: 4400, rpm2: 4900, cpuTemp: 70, gpuTemp: 60, cpuDuty: 50, gpuDuty: 50, fanMode: "auto" }),
      ),
      setFan: vi.fn(),
      runInstall: vi.fn(),
      runUninstall: vi.fn(() => Promise.resolve()),
      openMain: vi.fn(),
      getConfig: vi.fn(() => Promise.resolve({ lang: null })),
      setLang: vi.fn(() => Promise.resolve()),
    },
  };
});

function renderMain() {
  render(
    <I18nProvider initial="tr">
      <MainWindow />
    </I18nProvider>,
  );
}

test("sidebar navigates between pages", async () => {
  renderMain();
  await userEvent.click(screen.getByRole("button", { name: "Fan" }));
  expect(screen.getByRole("heading", { level: 1, name: "Fan" })).toBeInTheDocument();
});

test.each(["Güç", "Klavye", "Ekran"])("%s shows coming soon", async (name) => {
  renderMain();
  await userEvent.click(screen.getByRole("button", { name }));
  expect(screen.getByText(/sonraki sürümde/)).toBeInTheDocument();
});

test("language switch updates labels", async () => {
  renderMain();
  await userEvent.click(screen.getByRole("button", { name: "Ayarlar" }));
  await userEvent.click(screen.getByRole("radio", { name: "English" }));
  expect(api.setLang).toHaveBeenCalledWith("en");
  expect(screen.getByRole("button", { name: "Overview" })).toBeInTheDocument();
});

test("settings lists setup checks", async () => {
  renderMain();
  await userEvent.click(screen.getByRole("button", { name: "Ayarlar" }));
  expect(await screen.findByText("Yönetici yetkisiyle çalışıyor")).toBeInTheDocument();
});
