import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../stores/sessions";
import { useSessionStore } from "../stores/sessions";

const phaseColor: Record<string, string> = {
  running: "#6ea7ff",
  "waiting-for-approval": "#f4a4a4",
  "waiting-for-answer": "#ffd58a",
  completed: "#6fb982",
};

function PhaseDot({ phase }: { phase: AgentSession["phase"] }) {
  const color = phaseColor[phase] || "#6fb982";
  const isWaiting = phase === "waiting-for-approval" || phase === "waiting-for-answer";
  return (
    <span className="relative flex items-center justify-center w-2.5 h-2.5">
      {isWaiting && (
        <span
          className="absolute w-2.5 h-2.5 rounded-full animate-ping opacity-40"
          style={{ backgroundColor: color }}
        />
      )}
      <span
        className={`relative w-2 h-2 rounded-full ${phase === "running" ? "animate-pulse" : ""}`}
        style={{ backgroundColor: color }}
      />
    </span>
  );
}

function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 5000) return "now";
  if (diff < 60000) return `${Math.floor(diff / 1000)}s`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`;
  return `${Math.floor(diff / 86400000)}d`;
}

/** Extract meaningful workspace name, skipping common subdirectory names */
function workspaceName(dir: string): string {
  const skip = new Set([
    "src", "src-tauri", "packages", "apps", "lib", "dist", "build",
    "client", "server", "desktop", "frontend", "backend", "core",
  ]);
  const parts = dir.split("/").filter(Boolean);
  for (let i = parts.length - 1; i >= 0; i--) {
    if (!skip.has(parts[i])) return parts[i];
  }
  return parts[parts.length - 1] || dir;
}

function formatModel(model?: string): string | null {
  if (!model) return null;
  return model
    .replace("claude-", "")
    .replace("-4-6", " 4.6")
    .replace("opus", "Opus")
    .replace("sonnet", "Sonnet")
    .replace("haiku", "Haiku");
}

export function SessionRow({ session }: { session: AgentSession }) {
  const setSessions = useSessionStore((s) => s.setSessions);
  const sessions = useSessionStore((s) => s.sessions);

  const handlePermission = (approved: boolean) => {
    // Optimistic UI update — reflect state change immediately
    setSessions(
      sessions.map((s) =>
        s.id === session.id
          ? {
              ...s,
              phase: approved ? "running" as const : "completed" as const,
              pendingPermission: undefined,
              summary: approved ? "Permission approved" : "Permission denied",
            }
          : s
      )
    );
    invoke("resolve_permission", { sessionId: session.id, approved });
  };

  const isActive = session.phase !== "completed";
  const isWaiting = session.phase === "waiting-for-approval" || session.phase === "waiting-for-answer";
  const workspace = workspaceName(session.directory);
  const model = formatModel(session.model);

  // Build headline like Vibe Island: "workspace · initial prompt..."
  const headlinePrompt = session.initialPrompt;

  return (
    <div
      className={`rounded-xl overflow-hidden transition-all duration-300 ${
        isWaiting
          ? ""
          : isActive
            ? "bg-white/[0.03]"
            : "bg-white/[0.015]"
      }`}
      style={isWaiting ? { backgroundColor: `${phaseColor[session.phase]}15` } : undefined}
    >
      <div className="flex">
        {/* Left status bar */}
        <div
          className="w-[3px] shrink-0 transition-all duration-500"
          style={{
            backgroundColor: isWaiting ? phaseColor[session.phase] : session.brandColor,
            opacity: isActive ? 1 : 0.2,
          }}
        />

        <div className="flex-1 px-3 py-2 min-w-0">
          {/* Line 1: Headline — workspace · initial_prompt + right badges */}
          <div className="flex items-center gap-1.5 mb-0.5">
            <p className={`text-[13px] font-medium truncate flex-1 ${
              isActive ? "text-[#f1ead9]/90" : "text-[#f1ead9]/35"
            }`}>
              {workspace}
              {headlinePrompt && (
                <span className="text-[#f1ead9]/40 font-normal"> · {headlinePrompt}</span>
              )}
            </p>

            {/* Right side badges */}
            <span
              className="text-[9px] font-medium px-1.5 py-[1px] rounded-full border shrink-0"
              style={{
                color: session.brandColor,
                borderColor: `${session.brandColor}40`,
                backgroundColor: `${session.brandColor}15`,
              }}
            >
              {session.agentName.split(" ").pop()}
            </span>
            <span className="text-[10px] tabular-nums text-[#f1ead9]/20 shrink-0">
              {timeAgo(session.updatedAt)}
            </span>
            <PhaseDot phase={session.phase} />
          </div>

          {/* Line 2: "You: last_prompt" */}
          {session.lastUserPrompt && (
            <p className={`text-[11px] truncate leading-tight ${
              isActive ? "text-[#f1ead9]/45" : "text-[#f1ead9]/20"
            }`}>
              <span className="text-[#f1ead9]/25">You: </span>
              {session.lastUserPrompt}
            </p>
          )}

          {/* Line 3: Activity — tool use or current action */}
          {session.currentTool && isActive && (
            <p className="text-[11px] truncate leading-tight mt-0.5 text-[#f1ead9]/35">
              <span className="text-[#6ea7ff]/40 font-mono text-[10px] mr-1">
                {session.currentTool}
              </span>
              {!session.summary.startsWith("Prompt: ") && session.summary}
            </p>
          )}

          {/* Line 4: Assistant's last response */}
          {session.lastAssistantMessage && (
            <p className={`text-[11px] truncate leading-tight mt-0.5 ${
              isActive ? "text-[#f1ead9]/30" : "text-[#f1ead9]/20"
            }`}>
              {session.lastAssistantMessage}
            </p>
          )}

          {/* Model badge */}
          {model && isActive && (
            <div className="mt-1">
              <span className="text-[9px] text-[#f1ead9]/15 bg-white/[0.03] px-1.5 py-[1px] rounded">
                {model}
              </span>
            </div>
          )}

          {/* Permission approval */}
          {session.phase === "waiting-for-approval" && session.pendingPermission && (
            <div className="mt-2 pt-2 border-t border-[#f4a4a4]/10">
              <p className="text-[11px] text-[#f1ead9]/50 mb-2 font-mono leading-relaxed break-all line-clamp-3">
                {session.pendingPermission.description}
              </p>
              <div className="flex gap-1.5">
                <button
                  onClick={() => handlePermission(false)}
                  className="px-3 py-[5px] text-[11px] rounded-lg
                    bg-white/[0.065] hover:bg-white/[0.12]
                    text-[#f1ead9]/50 hover:text-[#f1ead9]/80
                    font-medium transition-all duration-150 active:scale-95"
                >
                  Deny
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="flex-1 px-3 py-[5px] text-[11px] rounded-lg
                    bg-[#e7a762]/80 hover:bg-[#e7a762]
                    text-white
                    font-medium transition-all duration-150 active:scale-95"
                >
                  Allow Once
                </button>
                <button
                  onClick={() => handlePermission(true)}
                  className="px-3 py-[5px] text-[11px] rounded-lg
                    bg-[#f1ead9]/90 hover:bg-[#f1ead9]
                    text-[#0d0d0f]
                    font-medium transition-all duration-150 active:scale-95"
                >
                  Always Allow
                </button>
              </div>
            </div>
          )}

          {/* Question */}
          {session.phase === "waiting-for-answer" && (
            <div className="mt-2 pt-2 border-t border-[#ffd58a]/10">
              <p className="text-[11px] text-[#f1ead9]/50 mb-2">{session.summary}</p>
              <button
                onClick={() => handlePermission(true)}
                className="w-full px-3 py-[5px] text-[11px] rounded-lg
                  bg-[#ffd58a]/20 hover:bg-[#ffd58a]/30
                  text-[#ffd58a]
                  font-medium transition-all duration-150 active:scale-95"
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
