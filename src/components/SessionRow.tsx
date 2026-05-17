import { invoke } from "@tauri-apps/api/core";
import type { AgentSession } from "../stores/sessions";

function PhaseDot({ phase }: { phase: AgentSession["phase"] }) {
  const styles: Record<string, string> = {
    running: "bg-yellow-400 animate-pulse",
    "waiting-for-approval": "bg-red-400 animate-pulse",
    "waiting-for-answer": "bg-orange-400 animate-pulse",
    completed: "bg-zinc-500",
  };
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${styles[phase] || "bg-zinc-500"}`}
    />
  );
}

function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 5000) return "just now";
  if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;
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
    invoke("resolve_permission", {
      sessionId: session.id,
      approved,
    });
  };

  const isActive =
    session.phase === "running" ||
    session.phase === "waiting-for-approval" ||
    session.phase === "waiting-for-answer";

  // Compact mode for completed sessions — single line
  if (compact) {
    return (
      <div className="rounded-lg bg-zinc-800/30 border border-zinc-800/40 px-3 py-2 opacity-60">
        <div className="flex items-center gap-2">
          <span
            className="w-2 h-2 rounded-sm shrink-0"
            style={{ backgroundColor: session.brandColor }}
          />
          <span className="text-xs text-zinc-400 truncate flex-1">
            {session.title}
          </span>
          <span className="text-xs text-zinc-600 shrink-0">
            {timeAgo(session.updatedAt)}
          </span>
          <PhaseDot phase={session.phase} />
        </div>
        <p className="text-xs text-zinc-600 truncate mt-0.5 pl-4">
          {session.summary}
        </p>
      </div>
    );
  }

  return (
    <div
      className={`rounded-lg border p-3 transition-colors ${
        session.phase === "waiting-for-approval" || session.phase === "waiting-for-answer"
          ? "bg-red-950/30 border-red-500/40"
          : isActive
            ? "bg-zinc-800/80 border-zinc-600/50"
            : "bg-zinc-800/50 border-zinc-700/40"
      }`}
    >
      {/* Header */}
      <div className="flex items-center gap-2 mb-1">
        <span
          className="w-2.5 h-2.5 rounded-sm shrink-0"
          style={{ backgroundColor: session.brandColor }}
        />
        <span className="text-xs font-medium text-zinc-300">
          {session.agentName}
        </span>
        <span className="text-xs text-zinc-600">·</span>
        <span className="text-xs text-zinc-500 truncate flex-1">
          {shortenDir(session.directory)}
        </span>
        <PhaseDot phase={session.phase} />
      </div>

      {/* Title */}
      <p className="text-sm text-zinc-200 truncate mb-1">{session.title}</p>

      {/* Summary + time */}
      <div className="flex items-center justify-between gap-2">
        <p className={`text-xs truncate flex-1 ${isActive ? "text-zinc-300" : "text-zinc-400"}`}>
          {session.summary}
        </p>
        <span className="text-xs text-zinc-600 shrink-0">
          {timeAgo(session.updatedAt)}
        </span>
      </div>

      {/* Permission approval bar */}
      {session.phase === "waiting-for-approval" && session.pendingPermission && (
        <div className="mt-2 pt-2 border-t border-red-500/20">
          <p className="text-xs text-orange-300 mb-2 truncate">
            {session.pendingPermission.description}
          </p>
          <div className="flex gap-2">
            <button
              onClick={() => handlePermission(true)}
              className="flex-1 text-xs px-3 py-1.5 rounded bg-green-600 hover:bg-green-500 text-white font-medium transition-colors"
            >
              Allow
            </button>
            <button
              onClick={() => handlePermission(false)}
              className="flex-1 text-xs px-3 py-1.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-300 font-medium transition-colors"
            >
              Deny
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
