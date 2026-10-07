import { HistoryWorkspace } from "./history/HistoryWorkspace";
import { useHistory } from "./history/useHistory";
import "./App.css";

export default function App() {
  return <HistoryWorkspace {...useHistory()} />;
}
