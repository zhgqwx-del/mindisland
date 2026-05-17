// MindIsland Plugin for OpenCode
// Forwards OpenCode events to MindIsland via Unix socket.
// Handles permission and question replies via HTTP POST back to OpenCode.
// Fails open — if MindIsland is not running, OpenCode continues unaffected.
import { Socket } from "net";
import { createRequire } from "module";
const require = createRequire(import.meta.url);
const os = require("os");
const fs = require("fs");

const SOCKET = "/tmp/mindisland-claude.sock";

// --- Socket helpers ---

function socketExists() {
  try { return fs.statSync(SOCKET).isSocket(); } catch { return false; }
}

function sendFireAndForget(json) {
  if (!socketExists()) return;
  try {
    const sock = new Socket();
    sock.on("error", () => sock.destroy());
    sock.on("connect", () => {
      try {
        sock.write(JSON.stringify(json) + "\n");
        sock.end();
      } catch { sock.destroy(); }
    });
    sock.setTimeout(3000, () => sock.destroy());
    sock.connect({ path: SOCKET });
  } catch {}
}

function sendAndWaitResponse(json, timeoutMs = 300000) {
  if (!socketExists()) return Promise.resolve(null);
  return new Promise((resolve) => {
    let settled = false;
    const settle = (value) => {
      if (settled) return;
      settled = true;
      resolve(value);
    };
    try {
      const sock = new Socket();
      let buf = "";
      sock.on("error", () => settle(null));
      sock.on("data", (data) => { buf += data.toString(); });
      sock.on("end", () => {
        try { settle(JSON.parse(buf)); } catch { settle(null); }
      });
      sock.on("connect", () => {
        try {
          sock.write(JSON.stringify(json) + "\n");
        } catch { sock.destroy(); settle(null); }
      });
      sock.setTimeout(timeoutMs, () => { sock.destroy(); settle(null); });
      sock.connect({ path: SOCKET });
    } catch { settle(null); }
  });
}

// --- Plugin entry point ---

export default async ({ client, serverUrl, directory }) => {
  const serverOrigin = serverUrl?.origin || "http://127.0.0.1:4096";

  // Auth for Electron sidecar (dynamic port + Basic auth)
  const sidecarUser = process.env.OPENCODE_SERVER_USERNAME;
  const sidecarPass = process.env.OPENCODE_SERVER_PASSWORD;
  const authHeader = (sidecarUser && sidecarPass)
    ? "Basic " + Buffer.from(`${sidecarUser}:${sidecarPass}`).toString("base64")
    : null;

  function buildHeaders(cwd) {
    const headers = { "Content-Type": "application/json" };
    if (authHeader) headers.Authorization = authHeader;
    const dir = cwd || directory;
    if (dir) headers["x-opencode-directory"] = encodeURIComponent(dir);
    return headers;
  }

  async function postReply(path, body, cwd) {
    try {
      const response = await fetch(`${serverOrigin}${path}`, {
        method: "POST", headers: buildHeaders(cwd), body: JSON.stringify(body),
      });
      if (response?.ok) return response;
    } catch {}
    // Fallback to client SDK if direct fetch fails
    try {
      const rawClient = client?._client;
      if (rawClient?.post) {
        return await rawClient.post({ url: path, headers: buildHeaders(cwd), body });
      }
    } catch {}
    return null;
  }

  // --- State tracking ---

  const msgRoles = new Map();       // messageID → { role, sessionID }
  const sessionCwd = new Map();     // sessionID → cwd
  const sessionText = new Map();    // sessionID → { lastUserText, lastAssistantText }
  const knownSessions = new Set();  // sessionIDs we've sent SessionStart for
  const pendingParts = new Map();   // messageID → [parts] (buffered until role known)

  function getSessionText(sid) {
    if (!sessionText.has(sid)) sessionText.set(sid, { lastUserText: "", lastAssistantText: "" });
    return sessionText.get(sid);
  }

  // --- Event building ---

  function base(sessionId, extra) {
    return {
      session_id: sessionId,
      _source: "opencode",
      ...extra,
    };
  }

  function ensureSessionStarted(sessionId, cwd) {
    if (!sessionId || knownSessions.has(sessionId)) return null;
    knownSessions.add(sessionId);
    const effectiveCwd = cwd || directory || undefined;
    if (effectiveCwd) sessionCwd.set(sessionId, effectiveCwd);
    return base(`opencode-${sessionId}`, {
      hook_event_name: "SessionStart",
      cwd: effectiveCwd,
    });
  }

  function clip(text, max) {
    if (!text) return "";
    const first = text.split("\n")[0] || text;
    if (first.length <= max) return first;
    return first.slice(0, max) + "...";
  }

  // --- Event mapping ---

  function mapEvent(ev) {
    const t = ev.type;
    const p = ev.properties || {};

    // session.created
    if (t === "session.created" && p.info) {
      const cwd = p.info.directory || "";
      if (cwd) sessionCwd.set(p.info.id, cwd);
      return ensureSessionStarted(p.info.id, cwd);
    }

    // session.deleted
    if (t === "session.deleted" && p.info) {
      sessionText.delete(p.info.id);
      sessionCwd.delete(p.info.id);
      knownSessions.delete(p.info.id);
      return base(`opencode-${p.info.id}`, { hook_event_name: "SessionEnd" });
    }

    // session.updated (title change, archive, directory)
    if (t === "session.updated" && p.info) {
      if (p.info.directory) sessionCwd.set(p.info.id, p.info.directory);
      if (p.info.time?.archived) {
        sessionText.delete(p.info.id);
        sessionCwd.delete(p.info.id);
        knownSessions.delete(p.info.id);
        return base(`opencode-${p.info.id}`, { hook_event_name: "SessionEnd" });
      }
      // Ensure session is known (title update implies activity)
      return ensureSessionStarted(p.info.id, sessionCwd.get(p.info.id));
    }

    // session.status / session.idle → Stop
    if ((t === "session.status" || t === "session.idle") && p.sessionID) {
      const sid = `opencode-${p.sessionID}`;
      const s = getSessionText(p.sessionID);
      const cwd = sessionCwd.get(p.sessionID);
      if (t === "session.idle" || p.status?.type === "idle") {
        return base(sid, {
          hook_event_name: "Stop",
          cwd,
          last_assistant_message: s.lastAssistantText || undefined,
        });
      }
    }

    // message.updated → track role for buffered parts
    if (t === "message.updated" && p.info?.id && p.info?.sessionID) {
      msgRoles.set(p.info.id, { role: p.info.role, sessionID: p.info.sessionID });
      if (msgRoles.size > 200) msgRoles.delete(msgRoles.keys().next().value);
      // Flush any pending parts that were waiting for role info
      const buffered = pendingParts.get(p.info.id) || [];
      pendingParts.delete(p.info.id);
      return buffered
        .map((part) => mapEvent({
          type: "message.part.updated",
          properties: { sessionID: p.info.sessionID, part },
        }))
        .filter(Boolean)
        .flat();
    }

    // message.part.updated (text) → UserPromptSubmit or track assistant text
    if (t === "message.part.updated" && p.part?.type === "text" && p.part?.messageID) {
      const meta = msgRoles.get(p.part.messageID);
      if (!meta) {
        // Buffer until we know the role
        const list = pendingParts.get(p.part.messageID) || [];
        list.push(p.part);
        pendingParts.set(p.part.messageID, list.slice(-20));
        if (pendingParts.size > 200) pendingParts.delete(pendingParts.keys().next().value);
        return null;
      }
      const s = getSessionText(meta.sessionID);
      const cwd = sessionCwd.get(meta.sessionID);
      const text = p.part.text || "";
      if (meta.role === "user" && text) {
        s.lastUserText = text;
        const start = ensureSessionStarted(meta.sessionID, cwd);
        const submit = base(`opencode-${meta.sessionID}`, {
          hook_event_name: "UserPromptSubmit",
          cwd,
          prompt: text,
        });
        return start ? [start, submit] : submit;
      }
      if (meta.role === "assistant" && text) {
        s.lastAssistantText = text;
      }
      return null;
    }

    // message.part.updated (tool) → PreToolUse / PostToolUse
    if (t === "message.part.updated" && p.part?.type === "tool") {
      const sessionID = p.part.sessionID || (p.part.messageID && msgRoles.get(p.part.messageID)?.sessionID);
      if (!sessionID) {
        if (p.part.messageID) {
          const list = pendingParts.get(p.part.messageID) || [];
          list.push(p.part);
          pendingParts.set(p.part.messageID, list.slice(-20));
          if (pendingParts.size > 200) pendingParts.delete(pendingParts.keys().next().value);
        }
        return null;
      }
      const sid = `opencode-${sessionID}`;
      const st = p.part.state?.status;
      const cwd = sessionCwd.get(sessionID);
      const toolName = (p.part.tool || "").charAt(0).toUpperCase() + (p.part.tool || "").slice(1);
      if (st === "running" || st === "pending") {
        return base(sid, {
          hook_event_name: "PreToolUse",
          cwd,
          tool_name: toolName,
          tool_input: p.part.state?.input || {},
        });
      }
      if (st === "completed" || st === "error") {
        return base(sid, {
          hook_event_name: "PostToolUse",
          cwd,
          tool_name: toolName,
        });
      }
    }

    // message.part.delta → track streaming assistant text
    if (t === "message.part.delta" && p.sessionID && p.field === "text") {
      getSessionText(p.sessionID).lastAssistantText =
        (getSessionText(p.sessionID).lastAssistantText || "") + (p.delta || "");
      return null;
    }

    // permission.asked → PermissionRequest (bidirectional)
    if (t === "permission.asked" && p.id && p.sessionID) {
      const toolName = (p.permission || "").charAt(0).toUpperCase() + (p.permission || "").slice(1);
      const patterns = p.patterns || [];
      const toolInput = { patterns, metadata: p.metadata };
      if (p.permission === "bash" && patterns.length > 0) {
        toolInput.command = patterns.join(" && ");
      }
      if ((p.permission === "edit" || p.permission === "write") && patterns.length > 0) {
        toolInput.file_path = patterns[0];
      }
      return {
        _type: "permission",
        _requestId: p.id,
        _sessionID: p.sessionID,
        event: base(`opencode-${p.sessionID}`, {
          hook_event_name: "PermissionRequest",
          cwd: sessionCwd.get(p.sessionID),
          tool_name: toolName,
          tool_input: toolInput,
        }),
      };
    }

    // question.asked → PermissionRequest (AskUserQuestion, bidirectional)
    if (t === "question.asked" && p.id && p.sessionID) {
      return {
        _type: "question",
        _requestId: p.id,
        _sessionID: p.sessionID,
        event: base(`opencode-${p.sessionID}`, {
          hook_event_name: "PermissionRequest",
          cwd: sessionCwd.get(p.sessionID),
          tool_name: "AskUserQuestion",
          tool_input: {
            questions: (p.questions || []).map((q) => ({
              question: q.question || "",
              header: q.header || "",
              options: (q.options || []).map((o) => ({ label: o.label, description: o.description })),
              multiSelect: q.multiple || false,
            })),
          },
        }),
      };
    }

    // permission.replied / question.replied → PostToolUse (confirmation)
    if ((t === "permission.replied" || t === "question.replied" || t === "question.rejected") && p.sessionID) {
      return base(`opencode-${p.sessionID}`, {
        hook_event_name: "PostToolUse",
        cwd: sessionCwd.get(p.sessionID),
      });
    }

    return null;
  }

  // --- Event dispatch ---

  async function dispatchMapped(mapped) {
    if (!mapped) return;
    if (Array.isArray(mapped)) {
      for (const item of mapped) await dispatchMapped(item);
      return;
    }

    // Permission: send to socket, wait for response, POST reply to OpenCode
    if (mapped._type === "permission") {
      const { _requestId, _sessionID, event } = mapped;
      const cwd = sessionCwd.get(_sessionID);
      sendAndWaitResponse(event).then(async (response) => {
        if (!response) return;
        const behavior = response?.hookSpecificOutput?.decision?.behavior;
        if (!behavior) return;
        const reply = behavior === "allow" ? "once"
          : behavior === "always" ? "always" : "reject";
        const reason = response?.hookSpecificOutput?.decision?.reason;
        await postReply(`/permission/${_requestId}/reply`, { reply, message: reason }, cwd);
      });
      return;
    }

    // Question: send to socket, wait for response, POST reply to OpenCode
    if (mapped._type === "question") {
      const { _requestId, _sessionID, event } = mapped;
      const cwd = sessionCwd.get(_sessionID);
      sendAndWaitResponse(event).then(async (response) => {
        if (!response) return;
        const answers = response?.hookSpecificOutput?.decision?.updatedInput?.answers;
        if (!answers) return;
        const qs = event.tool_input?.questions || [];
        const answerArray = qs.length > 0
          ? qs.map((q) => answers[q.header] ? [answers[q.header]] : []).filter((a) => a.length > 0)
          : Object.values(answers).map((v) => [v]);
        await postReply(`/question/${_requestId}/reply`, { answers: answerArray }, cwd);
      });
      return;
    }

    // Regular event: fire-and-forget
    sendFireAndForget(mapped);
  }

  // Dedup: avoid processing the same event twice
  const seenEvents = new Map();

  function eventKey(ev) {
    if (ev?.id) return `id:${ev.id}`;
    const p = ev?.properties || {};
    const part = p.part || {};
    const status = p.status?.type || part.state?.status || "";
    return [
      ev?.type || "", p.sessionID || p.info?.id || part.sessionID || "",
      p.messageID || part.messageID || "", p.partID || part.id || p.id || "", status,
    ].join(":");
  }

  function shouldProcess(ev) {
    const key = eventKey(ev);
    if (!key) return true;
    if (seenEvents.has(key)) return false;
    seenEvents.set(key, Date.now());
    // Cleanup old entries
    if (seenEvents.size > 500) {
      const cutoff = Date.now() - 600000;
      for (const [k, ts] of seenEvents) {
        if (ts < cutoff || seenEvents.size > 800) seenEvents.delete(k);
        if (seenEvents.size <= 500) break;
      }
    }
    return true;
  }

  // --- Global event bridge (for Electron sidecar) ---

  function normalizeSyncType(type) {
    return (type || "").replace(/\.\d+$/, "");
  }

  function syncEventToBusEvent(syncEvent) {
    const type = normalizeSyncType(syncEvent?.type);
    if (!["session.created", "session.updated", "session.deleted",
      "message.updated", "message.part.updated"].includes(type)) return null;
    return { id: syncEvent.id, type, properties: syncEvent.data || {} };
  }

  async function startGlobalEventBridge() {
    const shouldStart = process.env.OPENCODE_CLIENT === "desktop"
      || Boolean(authHeader)
      || (serverUrl?.port && parseInt(serverUrl.port) !== 4096);
    if (!shouldStart) return;

    let delay = 1000;
    while (true) {
      try {
        const response = await fetch(`${serverOrigin}/global/event`, {
          method: "GET", headers: buildHeaders(directory),
        });
        if (!response?.ok || !response.body) throw new Error("unavailable");
        delay = 1000;
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";
        while (true) {
          const { value, done } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          buffer = buffer.replace(/\r\n/g, "\n");
          let idx;
          while ((idx = buffer.indexOf("\n\n")) !== -1) {
            const chunk = buffer.slice(0, idx);
            buffer = buffer.slice(idx + 2);
            const data = chunk.split(/\r?\n/)
              .filter((line) => line.startsWith("data:"))
              .map((line) => line.slice(5).trimStart())
              .join("\n");
            if (!data) continue;
            try {
              const globalEvent = JSON.parse(data);
              if (globalEvent.directory && directory && globalEvent.directory !== directory) continue;
              const payload = globalEvent.payload || {};
              if (payload.type !== "sync") continue;
              const event = syncEventToBusEvent(payload.syncEvent);
              if (event && shouldProcess(event)) {
                await dispatchMapped(mapEvent(event));
              }
            } catch {}
          }
        }
      } catch {}
      await new Promise((resolve) => setTimeout(resolve, delay));
      delay = Math.min(delay * 2, 30000);
    }
  }

  void startGlobalEventBridge();

  // --- Plugin hooks ---

  return {
    event: async ({ event }) => {
      if (!event || !shouldProcess(event)) return;
      await dispatchMapped(mapEvent(event));
    },
  };
};
