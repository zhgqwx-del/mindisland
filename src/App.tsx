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
    <div className="flex flex-col h-screen rounded-2xl overflow-hidden border border-white/[0.07] bg-[#0d0d0f]/95 backdrop-blur-2xl shadow-2xl">
      <Header />
      <div className="flex-1 overflow-y-auto scrollbar-thin">
        <SessionList />
      </div>
      <Footer />
    </div>
  );
}

function Header() {
  const sessions = useSessionStore((s) => s.sessions);
  const active = sessions.filter(
    (s) =>
      s.phase === "running" ||
      s.phase === "waiting-for-approval" ||
      s.phase === "waiting-for-answer"
  );
  const attention = sessions.filter(
    (s) =>
      s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer"
  ).length;

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between px-4 py-2.5 border-b border-white/[0.06]"
    >
      <div className="flex items-center gap-2">
        <h1 className="text-[13px] font-semibold text-[#f1ead9]">
          MindIsland
        </h1>
      </div>
      <div className="flex items-center gap-2">
        {attention > 0 && (
          <span className="bg-rose-500/20 text-rose-400 text-[10px] px-2 py-0.5 rounded-full font-medium animate-pulse">
            {attention} needs action
          </span>
        )}
        {active.length > 0 && attention === 0 && (
          <span className="bg-blue-500/15 text-blue-400 text-[10px] px-2 py-0.5 rounded-full font-medium">
            {active.length} active
          </span>
        )}
        {active.length === 0 && (
          <span className="text-[10px] text-zinc-600 font-medium">idle</span>
        )}
      </div>
    </div>
  );
}

function Footer() {
  const sessions = useSessionStore((s) => s.sessions);
  const agents = new Set(sessions.map((s) => s.agentId)).size;

  return (
    <div className="px-4 py-2 border-t border-white/[0.06] flex items-center justify-between">
      <span className="text-[10px] text-zinc-600">
        {agents > 0
          ? `${agents} agent${agents !== 1 ? "s" : ""}`
          : "no agents"}
      </span>
      <span className="text-[10px] text-zinc-700">v0.1.0</span>
    </div>
  );
}

export default App;
