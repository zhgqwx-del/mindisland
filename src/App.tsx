import { useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSessionStore } from "./stores/sessions";
import type { AgentSession } from "./stores/sessions";
import { SessionList } from "./components/SessionList";

function App() {
  const setSessions = useSessionStore((s) => s.setSessions);

  const refresh = useCallback(() => {
    invoke<AgentSession[]>("get_sessions").then((sessions) => {
      setSessions(sessions);
      if (sessions.length > 0) {
        console.log("[mindisland] sessions:", sessions.map(s => ({
          id: s.id.slice(0, 8),
          phase: s.phase,
          summary: s.summary,
          tool: s.currentTool,
        })));
      }
    });
  }, [setSessions]);

  useEffect(() => {
    // Initial load
    refresh();

    // Real-time updates from Rust backend
    const unlistenUpdate = listen<AgentSession[]>(
      "sessions-updated",
      (event) => setSessions(event.payload)
    );

    // Re-fetch when panel is opened (tray click)
    const unlistenOpen = listen("panel-opened", () => refresh());

    // Backup: poll every 2s to catch any missed events
    const interval = setInterval(refresh, 2000);

    return () => {
      unlistenUpdate.then((fn) => fn());
      unlistenOpen.then((fn) => fn());
      clearInterval(interval);
    };
  }, [setSessions, refresh]);

  return (
    <div className="flex flex-col h-screen rounded-xl overflow-hidden border border-zinc-700/50 bg-zinc-900/95 backdrop-blur-xl">
      <Header />
      <div className="flex-1 overflow-y-auto">
        <SessionList />
      </div>
      <Footer />
    </div>
  );
}

function Header() {
  const sessions = useSessionStore((s) => s.sessions);
  const running = sessions.filter((s) => s.phase === "running").length;
  const attention = sessions.filter(
    (s) =>
      s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer"
  ).length;

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between px-4 py-3 border-b border-zinc-800"
    >
      <div className="flex items-center gap-2">
        <span className="text-base">🏝️</span>
        <h1 className="text-sm font-semibold text-zinc-200">MindIsland</h1>
      </div>
      <div className="flex items-center gap-3 text-xs text-zinc-500">
        {attention > 0 && (
          <span className="text-red-400 font-medium animate-pulse">
            {attention} waiting
          </span>
        )}
        {running > 0 && (
          <span className="text-yellow-400">{running} running</span>
        )}
        <span>{sessions.length} sessions</span>
      </div>
    </div>
  );
}

function Footer() {
  const sessions = useSessionStore((s) => s.sessions);
  const agents = new Set(sessions.map((s) => s.agentId)).size;

  return (
    <div className="px-4 py-2 border-t border-zinc-800 flex items-center justify-between text-xs text-zinc-600">
      <span>
        {agents} agent{agents !== 1 ? "s" : ""} connected
      </span>
      <span className="text-zinc-700">v0.1.0</span>
    </div>
  );
}

export default App;
