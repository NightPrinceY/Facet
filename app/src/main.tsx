import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/app.css";
import { App } from "./App";
import { inTauri, tauriApi } from "./lib/api";
import { createSampleApi } from "./lib/sample";

const api = inTauri() ? tauriApi : createSampleApi();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App api={api} />
  </StrictMode>,
);
