# Vibe Island 调研分析报告

## 1. 产品概述

### 1.1 Vibe Island（商业版）

- **官网**: https://vibeisland.app/
- **定位**: macOS 原生 AI 编码代理监控中心
- **价格**: $19.99 一次性购买
- **技术栈**: Native Swift（非 Electron）
- **资源占用**: < 50MB RAM

Vibe Island 利用 Mac 的 Dynamic Island（刘海区域）或顶部浮动栏，为开发者创建统一的 AI 编码代理监控面板。核心理念：让你保持在工作流中，同时多个 AI 代理在后台持续工作。

### 1.2 Open Vibe Island（开源版）

- **仓库**: https://github.com/Octane0411/open-vibe-island
- **许可**: GPL v3
- **技术栈**: SwiftUI + AppKit, Swift 6.2
- **要求**: macOS 14+

---

## 2. 核心功能

| 功能 | 说明 |
|------|------|
| 集中监控 | 在刘海/顶栏实时显示多个 AI agent 的状态 |
| 权限审批 | 直接在面板中 GUI 审批工具调用权限，无需切换窗口 |
| 问题回答 | Agent 提问时直接在面板回复 |
| Plan 预览 | Markdown 渲染查看 agent 计划 |
| 终端跳转 | 一键精准跳回到正确的终端 tab/pane/tmux session |
| 用量追踪 | 追踪 Claude/Codex/Kimi 等的 API 配额使用情况 |
| 会话发现 | 自动从本地转录文件发现活跃会话 |
| 通知系统 | 权限请求和会话事件的自动高度通知面板 |
| 完全本地 | 无云端存储、无遥测数据收集 |

---

## 3. 支持的 Agent 与终端

### 3.1 AI Agents（16+）

| Agent | 集成方式 | 配置位置 |
|-------|----------|----------|
| Claude Code | Hook 集成 + JSONL 会话发现 + Status Line Bridge | `~/.claude/settings.json` |
| Codex CLI | Hook 集成 + 用量追踪 | `~/.codex/config.toml` |
| Codex Desktop | Hook + JSON-RPC + deep-link | 同上 |
| OpenCode | JS 插件集成 | `~/.config/opencode/plugins/` |
| Cursor | Hook 集成 | `~/.cursor/hooks.json` |
| Gemini CLI | Hook 集成 (fire-and-forget) | `~/.gemini/settings.json` |
| Kimi CLI | Hook 集成 (兼容 Claude 格式) | `~/.kimi/config.toml` |
| Qoder | Claude Code fork | `~/.qoder/settings.json` |
| Qwen Code | Claude Code fork | `~/.qwen/settings.json` |
| Factory | Claude Code fork | `~/.factory/settings.json` |
| CodeBuddy | Claude Code fork | `~/.codebuddy/settings.json` |

### 3.2 终端/IDE（18+）

- **Full 支持**: Terminal.app, Ghostty, iTerm2, WezTerm, cmux, Kaku, tmux, Zellij, Warp
- **Workspace 级别**: VS Code, Cursor, Windsurf, Trae, JetBrains 全家桶

---

## 4. 架构设计（Open Vibe Island）

### 4.1 整体架构

```
Agent (Claude Code / Codex / OpenCode / ...)
  ↓ hook event
OpenIslandHooks CLI (stdin → Unix socket)
  ↓ JSON envelope
BridgeServer (in-app)
  ↓ state update
Notch overlay UI ← 用户看到状态
  ↓ click
Jump back → 对应的终端/IDE
```

### 4.2 项目结构（4 个 Swift Target）

| Target | 职责 |
|--------|------|
| **OpenIslandApp** | SwiftUI + AppKit shell — 菜单栏、浮层面板、设置 |
| **OpenIslandCore** | 共享库 — 模型定义、桥接传输(Unix socket IPC)、hooks、会话持久化 |
| **OpenIslandHooks** | 轻量 CLI — 被 agent hook 调用，通过 Unix socket 转发 payload |
| **OpenIslandSetup** | 安装器 CLI — 管理各 agent 的 hook 配置文件 |

### 4.3 关键设计原则

- **Hooks fail open**: 如果 Open Island 没运行，agent 不受影响继续工作
- **Session State Reducer**: `SessionState.apply` 作为单一状态来源
- **进程发现**: 通过 `ps`/`lsof` 匹配活跃 agent 进程
- **会话持久化**: 跨应用重启保持会话状态

---

## 5. 与 Claude Code 的集成细节

### 5.1 Hook 机制

Claude Code 支持在 `~/.claude/settings.json` 中注册 hooks，在关键事件时触发外部命令：

- `SessionStart` — 会话开始
- `UserPromptSubmit` — 用户提交 prompt
- `PreToolUse` — 工具调用前（可阻止）
- `PostToolUse` — 工具调用后
- `Stop` — 会话结束
- `Notification` — 通知事件

### 5.2 会话发现

扫描 `~/.claude/projects/` 下的 JSONL 转录文件，自动发现并恢复活跃会话。

### 5.3 Status Line Bridge

可选安装的状态行桥接：写入 managed `statusLine.command` 到 `~/.open-island/bin/open-island-statusline`，将 Claude Code 运行状态实时同步到 UI。

### 5.4 用量追踪

读取 Claude 本地缓存的 5 小时和 7 天用量窗口数据（`rate_limits`），缓存到 `/tmp/open-island-rl.json`。

---

## 6. OpenCode 集成方式

OpenCode 的集成不同于 Claude Code 的 hook 方式，使用的是 **JS 插件机制**：

- 插件位置: `~/.config/opencode/plugins/`
- 首次启动时自动安装插件
- 接收事件: session lifecycle, tool use, permission, question
- 支持: 权限审批流程、问题回答流程
- 进程检测: 通过 `ps` 命令

---

## 7. 应用场景

1. **多 Agent 并行开发** — 同时运行多个 Claude Code/Codex 实例做不同任务，统一监控
2. **权限管理效率** — Agent 需要权限时无需切到终端逐个审批
3. **上下文保持** — 写文档/开会/review 时后台 agent 状态一目了然，一键跳回
4. **用量可视化** — 避免不知不觉超出 API 配额
5. **团队重度 AI 编码** — 多终端多会话的 vibe coding 工作流

---

## 8. 局限性分析

| 局限 | 说明 |
|------|------|
| **仅 macOS** | 完全依赖 Swift/SwiftUI/AppKit，无法跨平台 |
| **Dynamic Island 依赖** | UI 设计围绕 macOS 刘海，Windows/Linux 无此概念 |
| **Agent 发现硬编码** | 每个 agent 的配置路径、hook 格式都需要单独适配 |
| **Unix Socket IPC** | Windows 不原生支持 Unix Socket（需 Named Pipe 替代） |

---

## 9. 跨平台方案初步思考

如果要做一个兼容 Windows 的类似工具：

### 9.1 技术栈选型

| 方案 | 优点 | 缺点 |
|------|------|------|
| **Tauri (Rust + Web)** | 跨平台、轻量、原生性能 | 前端 Web 技术 |
| **Electron** | 生态成熟、跨平台 | 资源占用大 |
| **Flutter Desktop** | 跨平台 UI、性能好 | 桌面生态不成熟 |
| **.NET MAUI / WPF + 条件编译** | Windows 原生体验好 | macOS 适配差 |

### 9.2 IPC 方案

- macOS: Unix Domain Socket
- Windows: Named Pipes 或 TCP localhost
- 跨平台统一: TCP localhost / gRPC / WebSocket

### 9.3 UI 形态

- macOS: 菜单栏 + 浮层（类似 Dynamic Island）
- Windows: 系统托盘 + 弹出面板 / 或 Overlay 窗口

---

## 10. UltraWork 监控分析

### 10.1 问题

UltraWork 是基于 OpenCode 的桌面 agent（Tauri 应用），配置在 `~/.config/ultrawork`（而非 `~/.config/opencode`），导致 Vibe Island / Open Island 无法自动发现和监控。

### 10.2 UltraWork 当前配置结构

```
~/.config/ultrawork/
├── opencode.json          # OpenCode 格式的配置（MCP servers、model、provider）
├── package.json           # Node.js 依赖 (@opencode-ai/plugin 1.3.13)
├── package-lock.json
└── node_modules/
    ├── @opencode-ai/      # OpenCode 插件 SDK
    └── zod/
```

关键发现：
- UltraWork 使用 OpenCode 的配置 schema (`$schema: "https://opencode.ai/config.json"`)
- 已安装了 `@opencode-ai/plugin` 1.3.13 — **说明插件机制可用**
- 使用自定义 model provider (`opencode/big-pickle`)
- 有 MCP servers（browser、knowledge-base）
- 是 Tauri 桌面应用

### 10.3 为什么 Vibe Island 无法监控 UltraWork

Open Island 对 OpenCode 的集成方式是 **JS 插件机制**（源码分析 `OpenCodePluginInstallationManager.swift`）：

```swift
// 硬编码路径 — 这是根本原因
openCodeConfigDirectory = ~/.config/opencode/
pluginsDirectory = ~/.config/opencode/plugins/
pluginFileURL = ~/.config/opencode/plugins/open-island.js
configURL = ~/.config/opencode/config.json
```

具体问题：
1. **路径硬编码**: 插件安装器只认 `~/.config/opencode/`，不认 `~/.config/ultrawork/`
2. **插件注册位置**: 在 `config.json` 的 `"plugin"` 数组注册，但 UltraWork 用的是 `opencode.json`
3. **进程发现**: `ActiveAgentProcessDiscovery` 通过 `ps` 匹配 "opencode" 进程名
4. **桌面应用封装**: Tauri 打包后进程名是 "ultrawork" 或类似名称，不在扫描范围

### 10.4 解决思路

**短期方案**: 在 `~/.config/ultrawork/` 中手动安装 open-island 插件
- 将 `open-island.js` 复制到 `~/.config/ultrawork/plugins/`
- 在 `opencode.json` 中添加 `"plugin": ["file://~/.config/ultrawork/plugins/open-island.js"]`

**长期方案**: 自建跨平台监控工具（见下节）

---

## 11. 自建跨平台方案设计（初步）

### 11.1 目标

构建一个类似 Vibe Island 的 AI Agent 监控工具，满足：
1. **跨平台**: Windows + macOS（Linux 可选）
2. **支持 UltraWork**: 基于 `~/.config/ultrawork` 配置
3. **可扩展**: 易于添加新 agent 支持（插件化 adapter）
4. **本地优先**: 无需云端

### 11.2 技术栈选型

| 方案 | 跨平台 | 性能 | 包大小 | 适合度 | 推荐度 |
|------|--------|------|--------|--------|--------|
| **Tauri 2 (Rust + Web)** | Win/Mac/Linux | 优 | ~5-10MB | UltraWork 同栈 | ★★★★★ |
| Electron | Win/Mac/Linux | 中 | ~100MB+ | 生态成熟 | ★★★ |
| Flutter Desktop | Win/Mac/Linux | 优 | ~20MB | UI 好 | ★★★ |
| .NET MAUI | Win/Mac | 中 | 大 | Windows 原生好 | ★★ |

**推荐 Tauri 2**：
- UltraWork 本身就是 Tauri 应用，技术栈统一，可共享 Rust 基础设施
- Rust 后端性能好，天然适合 IPC/进程监控/文件系统 watch
- 前端 Web 自由度高（React/Vue/Svelte）
- 原生系统托盘支持
- 包体小（对比 Electron 小 10x+），资源占用低

### 11.3 架构设计

```
┌─────────────────────────────────────────────────┐
│                   UI Layer (Web)                 │
│  System Tray Popup / Overlay Window             │
│  - Session list & status                        │
│  - Permission approval panel                    │
│  - Jump-back buttons                            │
│  - Usage tracking dashboard                     │
└─────────────────────┬───────────────────────────┘
                      │ Tauri IPC (commands/events)
┌─────────────────────┴───────────────────────────┐
│               Rust Core                          │
│                                                  │
│  ┌──────────────┐  ┌──────────────────────┐     │
│  │ Bridge Server │  │ Session State Manager│     │
│  │ (IPC Listen)  │  │ (Event Reducer)      │     │
│  └──────┬───────┘  └──────────────────────┘     │
│         │                                        │
│  ┌──────┴───────────────────────────────────┐   │
│  │ Agent Adapters (trait AgentAdapter)       │   │
│  │ ┌────────────┐ ┌──────────┐ ┌─────────┐ │   │
│  │ │ UltraWork  │ │ Claude   │ │ Codex   │ │   │
│  │ │ Adapter    │ │ Adapter  │ │ Adapter │ │   │
│  │ └────────────┘ └──────────┘ └─────────┘ │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │ Platform Services                         │   │
│  │ - IPC: Unix Socket (Mac) / Named Pipe    │   │
│  │        (Windows)                          │   │
│  │ - Process Discovery: sysinfo crate       │   │
│  │ - File Watch: notify crate               │   │
│  │ - Terminal Jump: platform-specific        │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  Hook CLI (独立二进制)                           │
│  agent hook 触发 → 读 stdin → 发送到 Bridge     │
│  支持: --source ultrawork/claude/codex/...      │
└─────────────────────────────────────────────────┘
```

### 11.4 核心模块设计

#### Agent Adapter Trait (Rust)

```rust
trait AgentAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn config_dir(&self) -> PathBuf;
    fn detect_process(&self) -> Vec<AgentProcess>;
    fn discover_sessions(&self) -> Vec<Session>;
    fn install_hooks(&self) -> Result<()>;
    fn uninstall_hooks(&self) -> Result<()>;
    fn parse_hook_event(&self, payload: &[u8]) -> Result<AgentEvent>;
}
```

#### UltraWork Adapter

```rust
struct UltraWorkAdapter {
    config_dir: PathBuf,  // ~/.config/ultrawork
}

impl AgentAdapter for UltraWorkAdapter {
    fn config_dir(&self) -> PathBuf {
        dirs::config_dir().unwrap().join("ultrawork")
    }
    
    fn install_hooks(&self) -> Result<()> {
        // 安装 JS 插件到 ~/.config/ultrawork/plugins/
        // 或注册 hook command 到 opencode.json
    }
    
    fn parse_hook_event(&self, payload: &[u8]) -> Result<AgentEvent> {
        // 复用 OpenCode 的 payload 格式
        parse_opencode_payload(payload)
    }
}
```

### 11.5 IPC 跨平台方案

```rust
#[cfg(unix)]
fn create_listener() -> Listener {
    UnixListener::bind("/tmp/agent-monitor.sock")
}

#[cfg(windows)]
fn create_listener() -> Listener {
    // 使用 windows-named-pipes crate
    NamedPipeServer::create(r"\\.\pipe\agent-monitor")
}
```

### 11.6 UI 形态设计

| 平台 | 主界面位置 | 交互方式 |
|------|-----------|----------|
| macOS | 菜单栏图标 + 弹出面板 | 点击菜单栏图标展开 |
| Windows | 系统托盘 + 弹出面板 | 点击托盘图标展开 |
| 通用 | 可选 overlay 窗口 | 置顶浮窗模式 |

### 11.7 Windows 特有实现

| 特性 | macOS | Windows |
|------|-------|---------|
| UI 入口 | NSStatusItem (菜单栏) | System Tray (Shell_NotifyIcon) |
| IPC | Unix Domain Socket | Named Pipe |
| 进程发现 | `sysinfo` crate (跨平台) | 同上 |
| 终端跳转 | AppleScript/Accessibility API | Windows Terminal JSON fragment / COM |
| 通知 | NSUserNotification | Toast notification (winrt) |
| 自启动 | launchd plist | Registry HKCU\Run |
| 文件监控 | FSEvents | ReadDirectoryChangesW |

### 11.8 UltraWork 集成具体方案

由于 UltraWork 已安装了 `@opencode-ai/plugin`，最直接的集成方式：

**Step 1**: 编写 JS 插件（兼容 OpenCode Plugin SDK）
```javascript
// ~/.config/ultrawork/plugins/agent-monitor.js
import { definePlugin } from "@opencode-ai/plugin";

export default definePlugin({
  name: "agent-monitor",
  hooks: {
    onSessionStart(ctx) { sendToBridge("SessionStart", ctx); },
    onToolUse(ctx) { sendToBridge("PreToolUse", ctx); },
    onPermissionRequest(ctx) { return sendToBridgeAndWait("PermissionRequest", ctx); },
    onStop(ctx) { sendToBridge("Stop", ctx); },
  }
});
```

**Step 2**: 安装 Hook CLI
```bash
# 注册到 ultrawork 的 plugin 配置
# 在 opencode.json 中添加 plugin 引用
```

**Step 3**: Bridge 接收并处理事件

---

## 12. UltraWork 源码深度分析

### 12.1 项目架构

UltraWork 是一个 **Tauri 2 + React 19 + Bun** 的桌面应用，通过 vendor 方式内嵌 OpenCode Server：

```
ultrawork/
├── packages/
│   ├── client/desktop/          # Tauri 桌面应用 (React + Rust)
│   │   ├── src/                 # React 前端 (Vite 7, Tailwind 4)
│   │   └── src-tauri/           # Rust 后端 (Tauri 2)
│   ├── channel/gateway/         # IM 网关 (DingTalk, WeChat)
│   ├── core/api-client/         # TypeScript API 客户端 SDK
│   └── core/knowledge/          # 知识库服务
├── vendor/opencode/             # OpenCode 子模块 (核心引擎)
└── patches/                     # 针对 OpenCode 的定制补丁
```

### 12.2 关键标识

| 项目 | 值 |
|------|-----|
| Bundle ID | `com.ultrawork.app` |
| Product Name | `Ultrawork` |
| 环境变量 | `OPENCODE_APP_NAME=ultrawork` |
| OpenCode Server 端口 | `:4096` |
| Channel Gateway 端口 | `:4097` |
| Knowledge Sidecar 端口 | `:4098` |
| 配置目录 | `~/.config/ultrawork/` |
| 数据目录 | `~/.local/share/ultrawork/` |
| Session Map | `~/.ultrawork/session-map.json` |

### 12.3 插件系统详解

**插件加载流程** (源码 `vendor/opencode/packages/opencode/src/plugin/index.ts`):

```
1. 读取 cfg.plugin_origins (来自 opencode.json 的 "plugin" 字段)
2. 等待依赖安装完成 (config.waitForDependencies)
3. PluginLoader.loadExternal() 加载每个插件
4. 对每个加载成功的插件调用 applyPlugin(load, input, hooks)
5. 插件返回 Hooks 对象，注册到全局 hooks 列表
6. 调用每个 hook 的 config() 通知当前配置
7. 订阅 Bus 事件并转发给插件的 event() hook
```

**关键代码** (第139行):
```typescript
const plugins = Flag.OPENCODE_PURE ? [] : (cfg.plugin_origins ?? [])
```

**plugin_origins 来源**: 从 `opencode.json` 的 `"plugin"` 字段解析：
```json
{
  "plugin": [
    "npm:my-plugin@^1.0",
    "file:./plugins/local-plugin",
    ["npm:plugin-with-opts@^2.0", { "key": "value" }]
  ]
}
```

### 12.4 可用的 Plugin Hooks

```typescript
interface Hooks {
  // 接收所有 Bus 事件（最有用的 hook！）
  event?: (input: { event: Event }) => Promise<void>
  
  // 配置变更通知
  config?: (input: Config) => Promise<void>
  
  // 权限拦截（可以 allow/deny）
  "permission.ask"?: (input: Permission, output: { status: "ask"|"deny"|"allow" }) => Promise<void>
  
  // 工具执行前后
  "tool.execute.before"?: (input, output: { args: any }) => Promise<void>
  "tool.execute.after"?: (input, output: { title, output, metadata }) => Promise<void>
  
  // 命令执行前
  "command.execute.before"?: (input, output: { parts: Part[] }) => Promise<void>
  
  // 消息拦截
  "chat.message"?: (input: { sessionID, agent, model, messageID }, output: { message, parts }) => Promise<void>
  
  // Shell 环境定制
  "shell.env"?: (input: { cwd, sessionID }, output: { env: Record<string, string> }) => Promise<void>
  
  // 自定义工具
  tool?: { [key: string]: ToolDefinition }
}
```

### 12.5 Bus 事件系统（通过 event hook 可获取）

UltraWork/OpenCode 的所有内部事件都通过 Bus 发布，插件的 `event` hook 可以接收全部：

| 事件类型 | 触发时机 |
|----------|----------|
| `session.status` | 会话状态变化 (idle/busy/retry) |
| `session.idle` | 会话空闲 |
| `session.diff` | 文件变更 |
| `permission.asked` | 权限请求 |
| `permission.replied` | 权限回复 |
| `question.asked` | 问题提问 |
| `question.replied` | 问题回答 |
| `message.part.delta` | 消息增量更新 |
| `file.edited` | 文件被编辑 |
| `command.executed` | 命令执行完成 |
| `pty.created/updated/exited` | PTY 生命周期 |
| `server.connected` | 服务器连接 |
| `global.disposed` | 实例销毁 |

### 12.6 另一种监控方式：SSE 事件流

UltraWork 的 OpenCode Server 暴露了 SSE 端点：
```
GET http://localhost:4096/event
Authorization: Basic b3BlbmNvZGU6dGVzdDEyMw==  (opencode:test123)
Accept: text/event-stream
```

所有 Bus 事件都会通过这个 SSE 推送，外部监控工具可以直接订阅而无需安装插件。

---

## 13. UltraWork 插件加载实测方案

### 13.1 当前状态确认

`~/.config/ultrawork/opencode.json` 当前**没有 `"plugin"` 字段**，但：
- 已安装 `@opencode-ai/plugin@1.3.13`（在 `~/.config/ultrawork/package.json` 中）
- OpenCode 引擎支持 `plugin_origins` 字段
- 插件加载器 `PluginLoader.loadExternal()` 完全可用

### 13.2 实测步骤

**Step 1: 编写最小测试插件**

```javascript
// ~/.config/ultrawork/plugins/monitor-test.mjs
export const server = async (input, options) => {
  console.log("[monitor-test] Plugin loaded!", input.directory);
  
  return {
    async event({ event }) {
      // 将事件转发到外部监控
      const msg = JSON.stringify({
        type: event.type,
        properties: event.properties,
        timestamp: Date.now()
      });
      
      // 方式1: 写入文件（最简单的验证）
      const fs = await import("fs");
      fs.appendFileSync("/tmp/ultrawork-events.log", msg + "\n");
      
      // 方式2: 发送到 Unix Socket / Named Pipe（正式方案）
      // const net = await import("net");
      // const client = net.connect("/tmp/agent-monitor.sock");
      // client.write(msg + "\n");
      // client.end();
    },
    
    async "permission.ask"(input, output) {
      console.log("[monitor-test] Permission asked:", input.title);
      // output.status 保持 "ask" 不变，不影响正常流程
    }
  };
};
```

**Step 2: 在 opencode.json 中注册插件**

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": [
    "file:./plugins/monitor-test.mjs"
  ],
  "mcp": { ... },
  "model": "opencode/big-pickle",
  "provider": { ... }
}
```

**Step 3: 重启 UltraWork 并验证**

```bash
# 监控事件日志
tail -f /tmp/ultrawork-events.log

# 在 UltraWork 中执行任何操作，应该能看到事件输出
```

### 13.3 备选验证方式：直接订阅 SSE

如果插件方式有问题，可以直接用 SSE 验证事件流是否可用：

```bash
curl -N -H "Authorization: Basic $(echo -n 'opencode:test123' | base64)" \
  http://localhost:4096/event
```

如果 UltraWork 正在运行且 OpenCode Server 已启动，应该能看到事件流。

### 13.4 预期结果与风险

**预期可行**：
- OpenCode 引擎完全支持 `plugin` 字段
- UltraWork 的 patch 没有移除插件功能（只改了 config path 和 plugin version pinning）
- `@opencode-ai/plugin` 已安装在 `~/.config/ultrawork/`

**潜在风险**：
1. UltraWork 的 patch 可能设置了 `OPENCODE_PURE=true`（跳过插件），需要检查
2. 文件类插件的 resolve 路径可能不正确（相对于 `~/.config/ultrawork/` 还是 `cwd`）
3. UltraWork 桌面应用重启后可能需要重新初始化插件
4. 插件版本兼容性（pinned 到 1.3.13���但 plugin SDK 的 API 应该稳定）

### 13.5 最终监控方案对比

| 方案 | 复杂度 | 侵入性 | 实时性 | 跨平台 | 推荐 |
|------|--------|--------|--------|--------|------|
| **Plugin (event hook)** | 中 | 低（只读事件） | 实时 | 是 | ★★★★★ |
| **SSE 订阅** | 低 | 零（纯外部） | 实时 | 是 | ★★★★ |
| **文件系统 Watch** | 高 | 零 | 延迟 | 是 | ★★ |
| **进程监控** | 中 | 零 | 延迟 | 是 | ★★ |

**推荐组合**: Plugin (event hook) 作为主方案 + SSE 作为备选/验证方案。

两者都是跨平台的（Node.js 插件在 Windows/macOS 都能运行），且 SSE 方案完全不需要修改 UltraWork 配置。

---

## 14. 待调研问题（更新）

1. ~~UltraWork 插件加载~~ → ✅ **已验证成功** (spec 格式: `"./plugins/xxx.mjs"`)
2. ~~OPENCODE_PURE 标志~~ → ✅ **未设置** (插件系统启用)
3. ~~SSE 端口可达性~~ → ✅ **可达** (localhost:4096, Basic auth opencode:test123)
4. ~~重启后插件加载~~ → ✅ **42条事件成功接收** (含 session/message/tool 全链路)
5. **Windows 路径映射**: `~/.config/ultrawork/` 在 Windows 对应什么路径？
6. **Tauri 2 系统托盘**: 弹窗 UI 实现的具体限制
7. **与 UltraWork 深度集成**: 是否值得作为内置模块

### 实测事件链（一次完整对话）

```
session.created → session.updated → message.updated (user msg)
→ session.status {busy} → message.updated (assistant msg)
→ message.part.updated (step-start) → message.part.delta × N (streaming tokens)
→ message.part.updated (reasoning done) → message.part.updated (text done)
→ message.part.updated (step-finish) → message.updated (final)
→ session.status {idle} → session.idle → session.diff
```

总计 42 条事件，覆盖了会话的完整生命周期。

---

## 15. 实测验证结果

### 15.1 SSE 事件流 ✅ 已验证成功

```bash
# 健康检查
curl http://localhost:4096/global/health
# → {"healthy":true,"version":"0.0.0--202604220629"}

# SSE 事件订阅
curl -N -H "Authorization: Basic b3BlbmNvZGU6dGVzdDEyMw==" http://localhost:4096/event
# → data: {"type":"server.connected","properties":{}}

# 会话列表
curl -H "Authorization: Basic b3BlbmNvZGU6dGVzdDEyMw==" http://localhost:4096/session
# → 返回 2 个活跃会话
```

**结论**: UltraWork 运行时 OpenCode Server `:4096` 完全可达，SSE 事件流正常推送。

### 15.2 OPENCODE_PURE ✅ 确认未设置

- Tauri 启动 sidecar 参数：`serve --port 4096`
- 环境变量仅：`OPENCODE_SERVER_PASSWORD=test123`, `OPENCODE_APP_NAME=ultrawork`
- **无 `--pure`，无 `OPENCODE_PURE`**
- **插件系统完全可用**

### 15.3 插件注册 ✅ 已配置（待重启验证）

已在 `~/.config/ultrawork/opencode.json` 添加：
```json
"plugin": ["file:./plugins/monitor-test.mjs"]
```

测试插件 `~/.config/ultrawork/plugins/monitor-test.mjs` 会将所有事件写入 `/tmp/ultrawork-monitor-test.log`。

**✅ 已验证成功！** 重启后 `/tmp/ultrawork-monitor-test.log` 正常输出。

注意：插件 spec 格式必须是 `"./plugins/monitor-test.mjs"`（相对路径），而非 `"file:./plugins/..."`。
OpenCode 的 `isPathPluginSpec()` 只识别 `"file://..."`, `"./..."`, 或绝对路径。

### 15.4 最终方案确定

| 方案 | 状态 | 特点 |
|------|------|------|
| **SSE 外部订阅** | ✅ 已验证可用 | 零侵入、立即可用、无需重启 |
| **Plugin event hook** | ✅ 已验证可用 | 更丰富上下文、可拦截权限 |

**推荐实现路径**:
1. 先用 SSE 方案快速实现基础监控（会话状态、事件流）
2. 后续用 Plugin 方案实现深度集成（权限审批代理、问题回答）
3. 外部监控工具通过 Tauri 2 构建跨平台 UI（系统托盘 + 弹窗）
