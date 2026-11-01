import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { api, type KeyboardView } from "../api";
import { KeyboardPage } from "./KeyboardPage";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

const view: KeyboardView = {
  rgb: [255, 255, 255],
  brightness: 70,
  rainbow: false,
  presets: [
    { name: "white", rgb: [255, 255, 255] },
    { name: "red", rgb: [255, 0, 0] },
  ],
};

vi.mock("../api", async (orig) => {
  const real = await orig<typeof import("../api")>();
  return {
    ...real,
    api: {
      getKeyboard: vi.fn(() => Promise.resolve(view)),
      setKeyboardColor: vi.fn((rgb) => Promise.resolve({ ...view, rgb })),
      setKeyboardBrightness: vi.fn((brightness) => Promise.resolve({ ...view, brightness })),
      setKeyboardRainbow: vi.fn((rainbow) => Promise.resolve({ ...view, rainbow })),
      getConfig: vi.fn(() => Promise.resolve({ lang: null })),
      setLang: vi.fn(() => Promise.resolve()),
    },
  };
});

function renderPage() {
  render(
    <I18nProvider initial="en">
      <KeyboardPage />
    </I18nProvider>,
  );
}

test("preset click sets colour", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("button", { name: "Red" }));
  expect(api.setKeyboardColor).toHaveBeenCalledWith([255, 0, 0]);
});

test("brightness slider sends value", async () => {
  renderPage();
  fireEvent.change(await screen.findByRole("slider", { name: "Brightness" }), { target: { value: "50" } });
  expect(api.setKeyboardBrightness).toHaveBeenCalledWith(50);
});

test("rainbow toggle", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("switch", { name: "Rainbow effect" }));
  expect(api.setKeyboardRainbow).toHaveBeenCalledWith(true);
});

test("lights off sets brightness to zero", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("button", { name: "Turn lights off" }));
  expect(api.setKeyboardBrightness).toHaveBeenCalledWith(0);
});
