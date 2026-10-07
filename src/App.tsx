import { useState } from "react";
import { HistoryWorkspace } from "./history/HistoryWorkspace";
import { useHistory } from "./history/useHistory";
import { Settings } from "./settings/Settings";
import "./App.css";

export default function App() {
  const history = useHistory();
  const [settings, setSettings] = useState(false);
  return settings ? <Settings close={() => { setSettings(false); void history.refresh(); }} /> : <HistoryWorkspace {...history} openSettings={() => setSettings(true)} />;
}
