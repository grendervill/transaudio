import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import { tauriBackend } from "./lib/backend";
import "./styles.css";

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <App backend={tauriBackend} />
  </StrictMode>,
);
