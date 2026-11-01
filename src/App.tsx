import { getCurrentWindow } from "@tauri-apps/api/window";

export function App() {
  return <p>{getCurrentWindow().label}</p>;
}
