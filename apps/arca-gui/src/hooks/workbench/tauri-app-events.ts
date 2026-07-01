import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CLOSE_BLOCKED_EVENT, STARTUP_REQUESTS_EVENT } from "../../shared/constants";
import type { CloseBlockedPromptState, StartupRequest } from "../../shared/types";

function useLatestRef<T>(value: T) {
  const ref = React.useRef(value);
  React.useEffect(() => {
    ref.current = value;
  }, [value]);
  return ref;
}

export function useCloseBlockedPrompt({
  setCloseBlockedPrompt,
  setStatus
}: {
  setCloseBlockedPrompt: React.Dispatch<
    React.SetStateAction<CloseBlockedPromptState | null>
  >;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
}) {
  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    listen<CloseBlockedPromptState>(CLOSE_BLOCKED_EVENT, (event) => {
      if (!mounted) {
        return;
      }
      setCloseBlockedPrompt(event.payload);
      setStatus("Waiting for commit");
    })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => {
        if (mounted) {
          setStatus("Ready");
        }
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [setCloseBlockedPrompt, setStatus]);
}

export function useStartupRequests({
  onStartupRequest,
  setStatus
}: {
  onStartupRequest: (request: StartupRequest) => void | Promise<void>;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
}) {
  const onStartupRequestRef = useLatestRef(onStartupRequest);

  React.useEffect(() => {
    let mounted = true;
    invoke<StartupRequest[]>("startup_requests")
      .then((requests) => {
        const first = requests[0];
        if (mounted && first) {
          void onStartupRequestRef.current(first);
        }
      })
      .catch(() => {
        if (mounted) {
          setStatus("Ready");
        }
      });
    return () => {
      mounted = false;
    };
  }, [onStartupRequestRef, setStatus]);

  React.useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | null = null;

    listen<StartupRequest[]>(STARTUP_REQUESTS_EVENT, (event) => {
      if (!mounted) {
        return;
      }
      const first = event.payload[0];
      if (first) {
        void onStartupRequestRef.current(first);
      }
    })
      .then((value) => {
        unlisten = value;
      })
      .catch(() => {
        if (mounted) {
          setStatus("Ready");
        }
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [onStartupRequestRef, setStatus]);
}
