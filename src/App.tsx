import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

export function App() {
  const [line, setLine] = useState("waiting for telemetry");
  useEffect(() => {
    const un = listen("telemetry", (e) => setLine(JSON.stringify(e.payload)));
    return () => void un.then((f) => f());
  }, []);
  return (
    <p>
      {getCurrentWindow().label}: {line}
    </p>
  );
}
