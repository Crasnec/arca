import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { Workbench } from "./components/workbench";
import { ShellOperationWindow } from "./components/shell-operation";
import { I18nProvider } from "./i18n";
import { AppSettingsProvider } from "./settings";
import type { StartupRequest } from "./shared/types";
import "./styles/index.css";

type AppMode =
  | { kind: "loading" }
  | { kind: "workbench" }
  | { kind: "shellOperation"; request: StartupRequest };

function shellRequestFromSearch(): StartupRequest | null {
  const params = new URLSearchParams(window.location.search);
  const action = params.get("shellAction");
  const archivePath = params.get("archivePath");
  if ((action === "extract" || action === "test") && archivePath) {
    return { action, archivePath };
  }
  return null;
}

function startupShellRequest(requests: StartupRequest[]) {
  if (requests.length !== 1) {
    return null;
  }
  const [request] = requests;
  return request.action === "extract" || request.action === "test" ? request : null;
}

function App() {
  const [mode, setMode] = React.useState<AppMode>({ kind: "loading" });

  React.useEffect(() => {
    const queryRequest = shellRequestFromSearch();
    if (queryRequest) {
      setMode({ kind: "shellOperation", request: queryRequest });
      return;
    }

    invoke<StartupRequest[]>("startup_requests")
      .then((requests) => {
        const shellRequest = startupShellRequest(requests);
        setMode(shellRequest ? { kind: "shellOperation", request: shellRequest } : { kind: "workbench" });
      })
      .catch(() => setMode({ kind: "workbench" }));
  }, []);

  if (mode.kind === "loading") {
    return null;
  }

  return (
    <I18nProvider nativeMenu={mode.kind !== "shellOperation"}>
      {mode.kind === "shellOperation" ? (
        <ShellOperationWindow request={mode.request} />
      ) : (
        <Workbench />
      )}
    </I18nProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppSettingsProvider>
      <App />
    </AppSettingsProvider>
  </React.StrictMode>
);
