import { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSessionStore } from "./stores/sessions";
import type { AgentSession } from "./stores/sessions";
import { SessionList } from "./components/SessionList";

function App() {
  const setSessions = useSessionStore((s) => s.setSessions);

  const refresh = useCallback(() => {
    invoke<AgentSession[]>("get_sessions").then(setSessions);
  }, [setSessions]);

  useEffect(() => {
    refresh();
    const unlistenUpdate = listen<AgentSession[]>(
      "sessions-updated",
      (event) => setSessions(event.payload)
    );
    const unlistenOpen = listen("panel-opened", () => refresh());
    const interval = setInterval(refresh, 2000);
    return () => {
      unlistenUpdate.then((fn) => fn());
      unlistenOpen.then((fn) => fn());
      clearInterval(interval);
    };
  }, [setSessions, refresh]);

  return (
    <div className="flex flex-col h-screen rounded-2xl overflow-hidden border border-white/[0.07] bg-[#0d0d0f]/[0.97] backdrop-blur-2xl shadow-2xl">
      <Header />
      <div className="flex-1 overflow-y-auto">
        <SessionList />
      </div>
    </div>
  );
}

function Header() {
  const sessions = useSessionStore((s) => s.sessions);
  const attention = sessions.filter(
    (s) => s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer"
  ).length;
  const running = sessions.filter((s) => s.phase === "running").length;

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between px-4 py-2 border-b border-white/[0.05]"
    >
      <span className="text-[12px] font-semibold text-[#f1ead9]/70 tracking-wide">
        MindIsland
      </span>
      <div className="flex items-center gap-2">
        {attention > 0 && (
          <span className="flex items-center gap-1 bg-[#f4a4a4]/15 text-[#f4a4a4] text-[10px] px-2 py-0.5 rounded-full font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#f4a4a4] animate-ping" />
            {attention}
          </span>
        )}
        {running > 0 && (
          <span className="flex items-center gap-1 bg-[#6ea7ff]/10 text-[#6ea7ff] text-[10px] px-2 py-0.5 rounded-full font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#6ea7ff] animate-pulse" />
            {running}
          </span>
        )}
        {attention === 0 && running === 0 && (
          <span className="text-[10px] text-[#f1ead9]/20">idle</span>
        )}
      </div>
    </div>
  );
}

export default App;
