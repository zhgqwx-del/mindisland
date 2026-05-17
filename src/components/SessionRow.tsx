import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../stores/sessions";

function PhaseDot({ phase }: { phase: AgentSession["phase"] }) {
  const color: Record<string, string> = {
    running: "bg-[#6ea7ff]",
    "waiting-for-approval": "bg-[#f4a4a4]",
    "waiting-for-answer": "bg-[#ffd58a]",
    completed: "bg-[#6fb982]",
  };
  const isWaiting = phase === "waiting-for-approval" || phase === "waiting-for-answer";
  return (
    <span className="relative flex items-center justify-center w-2.5 h-2.5">
      {isWaiting && (
        <span className={`absolute w-2.5 h-2.5 rounded-full ${color[phase]} animate-ping opacity-40`} />
      )}
      <span className={`relative w-2 h-2 rounded-full ${color[phase]} ${
        phase === "running" ? "animate-pulse" : ""
      }`} />
    </span>
  );
}

function PhaseLabel({ phase }: { phase: AgentSession["phase"] }) {
  const config: Record<string, { label: string; style: string }> = {
    running: { label: "Running", style: "text-[#6ea7ff]" },
    "waiting-for-approval": { label: "Approval", style: "text-[#f4a4a4]" },
    "waiting-for-answer": { label: "Question", style: "text-[#ffd58a]" },
    completed: { label: "Done", style: "text-[#6fb982]/60" },
  };
  const c = config[phase] || config.completed;
  return <span className={`text-[10px] font-medium ${c.style}`}>{c.label}</span>;
}

function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 5000) return "now";
  if (diff < 60000) return `${Math.floor(diff / 1000)}s`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`;
  return `${Math.floor(diff / 86400000)}d`;
}

function shortenDir(dir: string): string {
  const parts = dir.split("/").filter(Boolean);
  return parts.length > 2 ? parts.slice(-2).join("/") : parts.join("/") || dir;
}

export function SessionRow({ session }: { session: AgentSession }) {
  const handlePermission = (approved: boolean) => {
    invoke("resolve_permission", { sessionId: session.id, approved });
  };

  const isActive = session.phase !== "completed";
  const isWaiting = session.phase === "waiting-for-approval" || session.phase === "waiting-for-answer";

  return (
    <div
      className={`rounded-xl border overflow-hidden transition-all duration-300 ${
        session.phase === "waiting-for-approval"
          ? "bg-[#f4a4a4]/[0.06] border-[#f4a4a4]/20 shadow-[0_0_20px_-4px_rgba(244,164,164,0.12)]"
          : session.phase === "waiting-for-answer"
            ? "bg-[#ffd58a]/[0.06] border-[#ffd58a]/20"
            : isActive
              ? "bg-white/[0.03] border-white/[0.06]"
              : "bg-white/[0.02] border-white/[0.04]"
      }`}
    >
      <div className="flex">
        {/* Left status bar */}
        <div
          className={`w-[3px] shrink-0 transition-all duration-500 ${
            isActive ? "opacity-100" : "opacity-20"
          }`}
          style={{
            backgroundColor: isWaiting
              ? session.phase === "waiting-for-approval" ? "#f4a4a4" : "#ffd58a"
              : session.brandColor,
          }}
        />

        <div className="flex-1 px-3 py-2.5 min-w-0">
          {/* Header line */}
          <div className="flex items-center gap-1.5 mb-1">
            <span className="text-[11px] font-semibold text-[#f1ead9]/80">
              {session.agentName}
            </span>
            <span className="text-[10px] text-[#f1ead9]/20">·</span>
            <span className="text-[10px] text-[#f1ead9]/30 truncate flex-1 font-mono">
              {shortenDir(session.directory)}
            </span>
            <PhaseLabel phase={session.phase} />
            <PhaseDot phase={session.phase} />
          </div>

          {/* Title */}
          <p className={`text-[13px] font-medium truncate leading-tight ${
            isActive ? "text-[#f1ead9]/90" : "text-[#f1ead9]/40"
          }`}>
            {session.title}
          </p>

          {/* Activity line */}
          <div className="flex items-center justify-between gap-2 mt-1">
            <p className={`text-[11px] truncate flex-1 leading-tight ${
              isActive ? "text-[#f1ead9]/50" : "text-[#f1ead9]/25"
            }`}>
              {session.currentTool && isActive && (
                <span className="text-[#6ea7ff]/60 font-mono text-[10px] mr-1">
                  {session.currentTool}
                </span>
              )}
              {session.summary}
            </p>
            <span className="text-[10px] text-[#f1ead9]/20 shrink-0 tabular-nums">
              {timeAgo(session.updatedAt)}
            </span>
          </div>

          {/* Permission approval — 3 buttons */}
          {session.phase === "waiting-for-approval" && session.pendingPermission && (
            <div className="mt-3 pt-2.5 border-t border-[#f4a4a4]/10">
              <p className="text-[11px] text-[#f1ead9]/60 mb-2.5 font-mono leading-relaxed break-all">
                {session.pendingPermission.description}
              </p>
              <div className="flex gap-1.5">
                <button
                  onClick={() => handlePermission(false)}
                  className="px-3 py-[6px] text-[11px] rounded-lg
                    bg-white/[0.065] hover:bg-white/[0.12]
                    text-[#f1ead9]/60 hover:text-[#f1ead9]/90
                    border border-white/[0.06]
                    font-medium transition-all duration-150"
                >
                  Deny
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="flex-1 px-3 py-[6px] text-[11px] rounded-lg
                    bg-[#e7a762]/80 hover:bg-[#e7a762]
                    text-white
                    border border-[#e7a762]/40
                    font-medium transition-all duration-150"
                >
                  Allow Once
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="px-3 py-[6px] text-[11px] rounded-lg
                    bg-[#f1ead9]/90 hover:bg-[#f1ead9]
                    text-[#0d0d0f]
                    border border-[#f1ead9]/40
                    font-medium transition-all duration-150"
                >
                  Always Allow
                </button>
              </div>
            </div>
          )}

          {/* Question answer — waiting for answer */}
          {session.phase === "waiting-for-answer" && (
            <div className="mt-3 pt-2.5 border-t border-[#ffd58a]/10">
              <p className="text-[11px] text-[#f1ead9]/60 mb-2.5">
                {session.summary}
              </p>
              <button
                onClick={() => handlePermission(true)}
                className="w-full px-3 py-[6px] text-[11px] rounded-lg
                  bg-[#ffd58a]/20 hover:bg-[#ffd58a]/30
                  text-[#ffd58a] border border-[#ffd58a]/20
                  font-medium transition-all duration-150"
              >
                Answer in Terminal
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
