import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { LanguageProvider } from "./lib/language";
import { startQualityGovernor } from "./lib/motion";
import { WorldProvider } from "./lib/world";

// Follow the OS light/dark setting; the orb shader reads the `dark` class too.
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
const applyTheme = () => document.documentElement.classList.toggle("dark", darkQuery.matches);
applyTheme();
darkQuery.addEventListener("change", applyTheme);

// "lite" by default; the governor turns on richer ambients only if frames are cheap.
startQualityGovernor();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <WorldProvider>
      <LanguageProvider>
        <App />
      </LanguageProvider>
    </WorldProvider>
  </React.StrictMode>,
);
