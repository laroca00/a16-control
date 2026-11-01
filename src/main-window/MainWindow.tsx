import { useState } from "react";
import { TitleBar } from "../components/TitleBar";
import { KeyboardLightNotice, SetupNotice, UnsupportedNotice } from "../components/Notices";
import { useFanControl } from "../hooks/useFanControl";
import { ComingSoon } from "../pages/ComingSoon";
import { FanPage } from "../pages/FanPage";
import { PowerPage } from "../pages/PowerPage";
import { KeyboardPage } from "../pages/KeyboardPage";
import { Overview } from "../pages/Overview";
import { SettingsPage } from "../pages/SettingsPage";
import { ShaderGradient } from "../ui/ShaderGradient";
import { Sidebar, type Page } from "./Sidebar";
import "./main.css";

export function MainWindow() {
  const [page, setPage] = useState<Page>("overview");
  const control = useFanControl();

  return (
    <div className="main">
      <ShaderGradient className="main__backdrop" colors={["#0b1846", "#2f55c8", "#1c9c92"]} brightness={0.85} />
      <TitleBar />
      <Sidebar page={page} onSelect={setPage} />
      <main className="main__content">
        {(control.needsSetup || control.unsupported || control.status?.keyboardLight) && (
          <div className="main__notices">
            {control.needsSetup && <SetupNotice />}
            {control.unsupported && <UnsupportedNotice />}
            {control.status?.keyboardLight && <KeyboardLightNotice />}
          </div>
        )}
        <div key={page} className="main__page">
          {page === "overview" && <Overview control={control} />}
          {page === "fan" && <FanPage control={control} />}
          {page === "power" && <PowerPage />}
          {page === "keyboard" && <KeyboardPage />}
          {page === "display" && <ComingSoon title="navDisplay" body="soonDisplay" />}
          {page === "settings" && <SettingsPage status={control.status} />}
        </div>
      </main>
    </div>
  );
}
