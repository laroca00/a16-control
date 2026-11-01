import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { api, type DisplayView } from "../api";
import { DisplayPage } from "./DisplayPage";
import { Blackout } from "../components/Blackout";

const view: DisplayView = { brightness: 70, refresh: 165, rates: [165, 120, 60], screenOff: false };

vi.mock("../api", async (orig) => {
  const real = await orig<typeof import("../api")>();
  return {
    ...real,
    api: {
      getDisplay: vi.fn(() => Promise.resolve(view)),
      setBrightness: vi.fn((brightness) => Promise.resolve({ ...view, brightness })),
      setRefreshRate: vi.fn((refresh) => Promise.resolve({ ...view, refresh })),
      screenOff: vi.fn(() => Promise.resolve()),
      screenOn: vi.fn(() => Promise.resolve()),
      getConfig: vi.fn(() => Promise.resolve({ lang: null })),
      setLang: vi.fn(() => Promise.resolve()),
    },
  };
});

function renderPage() {
  render(
    <I18nProvider initial="en">
      <DisplayPage />
    </I18nProvider>,
  );
}

test("display page sends brightness and refresh", async () => {
  renderPage();
  const slider = await screen.findByRole("slider", { name: "Screen brightness" });
  fireEvent.change(slider, { target: { value: "40" } });
  fireEvent.change(slider, { target: { value: "55" } });
  // Dragging only moves the slider; the panel changes once, on release.
  expect(api.setBrightness).not.toHaveBeenCalled();
  fireEvent.pointerUp(slider);
  expect(api.setBrightness).toHaveBeenCalledTimes(1);
  expect(api.setBrightness).toHaveBeenCalledWith(55);
  await userEvent.click(screen.getByRole("radio", { name: "60 Hz" }));
  expect(api.setRefreshRate).toHaveBeenCalledWith(60);
});

test("screen off button turns the screen off", async () => {
  renderPage();
  await userEvent.click(await screen.findByRole("button", { name: "Turn screen off" }));
  expect(api.screenOff).toHaveBeenCalled();
});

test("blackout dismisses on key", () => {
  vi.useFakeTimers();
  render(<Blackout />);
  vi.advanceTimersByTime(1600);
  fireEvent.keyDown(window, { key: "a" });
  expect(api.screenOn).toHaveBeenCalled();
  vi.useRealTimers();
});
