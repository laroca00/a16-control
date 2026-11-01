import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nProvider } from "../i18n";
import { TitleBar } from "./TitleBar";

const win = { minimize: vi.fn(), toggleMaximize: vi.fn(), hide: vi.fn() };
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => win }));

function renderBar() {
  render(
    <I18nProvider initial="en">
      <TitleBar />
    </I18nProvider>,
  );
}

test("window buttons drive the window", async () => {
  renderBar();
  await userEvent.click(screen.getByRole("button", { name: "Minimize" }));
  await userEvent.click(screen.getByRole("button", { name: "Maximize" }));
  await userEvent.click(screen.getByRole("button", { name: "Close to tray" }));
  expect(win.minimize).toHaveBeenCalled();
  expect(win.toggleMaximize).toHaveBeenCalled();
  expect(win.hide).toHaveBeenCalled();
});

test("the bar is a drag region", () => {
  renderBar();
  expect(screen.getByRole("banner")).toHaveAttribute("data-tauri-drag-region");
});
