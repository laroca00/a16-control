import { useState } from "react";
import { KeyboardLightNotice, SetupNotice } from "../components/Notices";
import { useFanControl } from "../hooks/useFanControl";
import { ComingSoon } from "../pages/ComingSoon";
import { FanPage } from "../pages/FanPage";
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
      <Sidebar page={page} onSelect={setPage} />
      <main className="main__content">
        {(control.needsSetup || control.status?.keyboardLight) && (
          <div className="main__notices">
            {control.needsSetup && <SetupNotice />}
            {control.status?.keyboardLight && <KeyboardLightNotice />}
          </div>
        )}
        <div key={page} className="main__page">
          {page === "overview" && <Overview control={control} />}
          {page === "fan" && <FanPage control={control} />}
          {page === "power" && <ComingSoon title="navPower" body="soonPower" />}
          {page === "keyboard" && <ComingSoon title="navKeyboard" body="soonKeyboard" />}
          {page === "display" && <ComingSoon title="navDisplay" body="soonDisplay" />}
          {page === "settings" && <SettingsPage status={control.status} />}
        </div>
      </main>
    </div>
  );
}
