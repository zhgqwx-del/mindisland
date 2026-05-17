import { useEffect, useCallback, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSessionStore } from "./stores/sessions";
import type { AgentSession } from "./stores/sessions";
import { SessionList } from "./components/SessionList";

function App() {
  const setSessions = useSessionStore((s) => s.setSessions);
  const prevAttentionRef = useRef<boolean>(false);

  const refresh = useCallback(() => {
    invoke<AgentSession[]>("get_sessions").then(setSessions);
  }, [setSessions]);

  // Auto-hide panel 8s after attention clears (permission resolved)
  const sessions = useSessionStore((s) => s.sessions);
  const hasAttention = sessions.some(
    (s) => s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer"
  );
  const hasActive = sessions.some((s) => s.phase === "running");

  useEffect(() => {
    // If attention just cleared (was true, now false) and no active sessions, auto-hide
    if (prevAttentionRef.current && !hasAttention && !hasActive) {
      const timer = setTimeout(async () => {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        getCurrentWindow().hide();
      }, 8000);
      return () => clearTimeout(timer);
    }
    prevAttentionRef.current = hasAttention;
  }, [hasAttention, hasActive]);

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

  // Resize window to fit session count
  const visibleCount = sessions.filter(
    (s) =>
      s.phase === "running" ||
      s.phase === "waiting-for-approval" ||
      s.phase === "waiting-for-answer" ||
      (s.phase === "completed" && Date.now() - s.updatedAt < 120000)
  ).length;

  useEffect(() => {
    const headerHeight = 36;
    const emptyHeight = 120;
    const rowHeight = 90;
    const padding = 16;
    const height = visibleCount === 0
      ? headerHeight + emptyHeight
      : headerHeight + visibleCount * rowHeight + padding;
    invoke("resize_panel", { height });
  }, [visibleCount]);

  return (
    <div className="flex flex-col h-screen overflow-hidden bg-[#0d0d0f]">
      <Header />
      <div className="flex-1 overflow-y-auto">
        <SessionList />
      </div>
    </div>
  );
}

function Header() {
  const sessions = useSessionStore((s) => s.sessions);
  const [muted, setMuted] = useState(false);
  const attention = sessions.filter(
    (s) => s.phase === "waiting-for-approval" || s.phase === "waiting-for-answer"
  ).length;
  const running = sessions.filter((s) => s.phase === "running").length;

  useEffect(() => {
    invoke<boolean>("is_muted").then(setMuted);
  }, []);

  const handleToggleMute = async () => {
    const newMuted = await invoke<boolean>("toggle_mute");
    setMuted(newMuted);
  };

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between px-3 py-2 border-b border-white/[0.05]"
    >
      <span className="text-[12px] font-semibold text-[#f1ead9]/70 tracking-wide">
        MindIsland
      </span>
      <div className="flex items-center gap-1.5">
        {attention > 0 && (
          <span className="flex items-center gap-1 bg-[#f4a4a4]/15 text-[#f4a4a4] text-[10px] px-1.5 py-0.5 rounded-full font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#f4a4a4] animate-ping" />
            {attention}
          </span>
        )}
        {running > 0 && (
          <span className="flex items-center gap-1 bg-[#6ea7ff]/10 text-[#6ea7ff] text-[10px] px-1.5 py-0.5 rounded-full font-medium">
            <span className="w-1.5 h-1.5 rounded-full bg-[#6ea7ff] animate-pulse" />
            {running}
          </span>
        )}
        {attention === 0 && running === 0 && (
          <span className="text-[10px] text-[#f1ead9]/20">idle</span>
        )}

        {/* Mute toggle */}
        <button
          onClick={handleToggleMute}
          className="ml-1 p-1 rounded hover:bg-white/[0.06] transition-colors"
          title={muted ? "Unmute notifications" : "Mute notifications"}
        >
          <span className="text-[11px] text-[#f1ead9]/30 hover:text-[#f1ead9]/60">
            {muted ? "🔇" : "🔔"}
          </span>
        </button>
      </div>
    </div>
  );
}

export default App;
