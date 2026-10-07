import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AppQuery } from "./query";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppQuery><App /></AppQuery>
  </React.StrictMode>,
);
