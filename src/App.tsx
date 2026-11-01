import { getCurrentWindow } from "@tauri-apps/api/window";
import { I18nProvider } from "./i18n";
import { MainWindow } from "./main-window/MainWindow";
import { QuickPanel } from "./panel/QuickPanel";
import { Blackout } from "./components/Blackout";
import "./styles.css";

export function App() {
  const label = getCurrentWindow().label;
  if (label === "blackout") return <Blackout />;
  return <I18nProvider>{label === "panel" ? <QuickPanel /> : <MainWindow />}</I18nProvider>;
}
