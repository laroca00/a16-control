import { getCurrentWindow } from "@tauri-apps/api/window";
import { I18nProvider } from "./i18n";
import { QuickPanel } from "./panel/QuickPanel";
import "./styles.css";

export function App() {
  const label = getCurrentWindow().label;
  return <I18nProvider>{label === "panel" ? <QuickPanel /> : <p>main</p>}</I18nProvider>;
}
