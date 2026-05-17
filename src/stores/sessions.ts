import { create } from "zustand";

export interface AgentSession {
  id: string;
  agentId: string;
  agentName: string;
  brandColor: string;
  title: string;
  directory: string;
  phase: "running" | "waiting-for-approval" | "waiting-for-answer" | "completed";
  summary: string;
  updatedAt: number;
  model?: string;
  currentTool?: string;
  initialPrompt?: string;
  lastUserPrompt?: string;
  pendingPermission?: {
    id: string;
    title: string;
    description: string;
    toolName?: string;
  };
}

interface SessionStore {
  sessions: AgentSession[];
  setSessions: (sessions: AgentSession[]) => void;
}

export const useSessionStore = create<SessionStore>((set) => ({
  sessions: [],
  setSessions: (sessions) => set({ sessions }),
}));
