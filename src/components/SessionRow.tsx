import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../stores/sessions";

// Color palette aligned with Open Vibe Island's IslandDesignPalette
const phaseColors = {
  running: { dot: "bg-blue-400", bg: "border-blue-500/30", text: "text-blue-300" },
  "waiting-for-approval": { dot: "bg-rose-400", bg: "border-rose-500/40", text: "text-rose-300" },
  "waiting-for-answer": { dot: "bg-amber-400", bg: "border-amber-500/40", text: "text-amber-300" },
  completed: { dot: "bg-emerald-500/60", bg: "border-zinc-700/40", text: "text-zinc-500" },
};

function PhaseDot({ phase }: { phase: AgentSession["phase"] }) {
  const color = phaseColors[phase]?.dot || "bg-zinc-500";
  const isActive = phase !== "completed";
  return (
    <span className={`inline-block w-2 h-2 rounded-full ${color} ${isActive ? "animate-pulse" : ""}`} />
  );
}

function PhaseLabel({ phase }: { phase: AgentSession["phase"] }) {
  const labels: Record<string, string> = {
    running: "Running",
    "waiting-for-approval": "Needs approval",
    "waiting-for-answer": "Needs answer",
    completed: "Idle",
  };
  const textColor = phaseColors[phase]?.text || "text-zinc-500";
  return <span className={`text-[10px] font-medium ${textColor}`}>{labels[phase]}</span>;
}

function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 5000) return "now";
  if (diff < 60000) return `${Math.floor(diff / 1000)}s`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`;
  if (diff < 604800000) return `${Math.floor(diff / 86400000)}d`;
  const d = new Date(ts);
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

function shortenDir(dir: string): string {
  const parts = dir.split("/").filter(Boolean);
  return parts.length > 2 ? parts.slice(-2).join("/") : parts.join("/") || dir;
}

interface Props {
  session: AgentSession;
  compact?: boolean;
}

export function SessionRow({ session, compact }: Props) {
  const handlePermission = (approved: boolean) => {
    invoke("resolve_permission", { sessionId: session.id, approved });
  };

  const isActive = session.phase !== "completed";

  // Compact mode for completed sessions
  if (compact) {
    return (
      <div className="group rounded-lg bg-zinc-800/20 px-3 py-1.5 hover:bg-zinc-800/40 transition-all duration-200">
        <div className="flex items-center gap-2">
          <span
            className="w-1.5 h-1.5 rounded-full shrink-0 opacity-50"
            style={{ backgroundColor: session.brandColor }}
          />
          <span className="text-[11px] text-zinc-500 truncate flex-1">
            {session.title}
          </span>
          <span className="text-[10px] text-zinc-600">{timeAgo(session.updatedAt)}</span>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`rounded-xl border transition-all duration-300 ${
        session.phase === "waiting-for-approval"
          ? "bg-rose-950/20 border-rose-500/30 shadow-[0_0_12px_-3px_rgba(244,63,94,0.15)]"
          : session.phase === "waiting-for-answer"
            ? "bg-amber-950/20 border-amber-500/30"
            : isActive
              ? "bg-zinc-800/60 border-zinc-600/40"
              : "bg-zinc-800/40 border-zinc-700/30"
      }`}
    >
      {/* Left color bar */}
      <div className="flex">
        <div
          className={`w-[3px] rounded-l-xl shrink-0 transition-colors duration-300 ${
            isActive ? "" : "opacity-30"
          }`}
          style={{ backgroundColor: session.brandColor }}
        />

        <div className="flex-1 p-3 min-w-0">
          {/* Row 1: Agent + directory + phase */}
          <div className="flex items-center gap-1.5 mb-1">
            <span className="text-[11px] font-semibold text-zinc-300">
              {session.agentName}
            </span>
            <span className="text-[10px] text-zinc-600">·</span>
            <span className="text-[10px] text-zinc-500 truncate flex-1">
              {shortenDir(session.directory)}
            </span>
            <PhaseLabel phase={session.phase} />
            <PhaseDot phase={session.phase} />
          </div>

          {/* Row 2: Title */}
          <p className={`text-[13px] font-medium truncate mb-0.5 ${
            isActive ? "text-zinc-100" : "text-zinc-400"
          }`}>
            {session.title}
          </p>

          {/* Row 3: Summary (activity line) */}
          <div className="flex items-center justify-between gap-2">
            <p className={`text-[11px] truncate flex-1 ${
              isActive ? "text-zinc-400" : "text-zinc-600"
            }`}>
              {session.currentTool && isActive && (
                <span className="text-blue-400/80 font-mono mr-1">
                  {session.currentTool}
                </span>
              )}
              {session.summary}
            </p>
            <span className="text-[10px] text-zinc-600 shrink-0 tabular-nums">
              {timeAgo(session.updatedAt)}
            </span>
          </div>

          {/* Permission approval - 3 buttons like Vibe Island */}
          {session.phase === "waiting-for-approval" && session.pendingPermission && (
            <div className="mt-2.5 pt-2.5 border-t border-rose-500/15">
              <p className="text-[11px] text-rose-200/80 mb-2 font-mono leading-relaxed">
                {session.pendingPermission.description}
              </p>
              <div className="flex gap-1.5">
                <button
                  onClick={() => handlePermission(false)}
                  className="px-3 py-1.5 text-[11px] rounded-lg bg-zinc-700/60 hover:bg-zinc-600/80 text-zinc-400 hover:text-zinc-200 font-medium transition-all duration-150"
                >
                  Deny
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="flex-1 px-3 py-1.5 text-[11px] rounded-lg bg-amber-600/80 hover:bg-amber-500 text-white font-medium transition-all duration-150"
                >
                  Allow Once
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="px-3 py-1.5 text-[11px] rounded-lg bg-zinc-200/90 hover:bg-white text-zinc-900 font-medium transition-all duration-150"
                >
                  Always Allow
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
