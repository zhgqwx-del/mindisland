# MindIsland — ��计方案

> 跨平台 AI Agent 监控面板，Windows + macOS 双���台支持

---

## 1. 产品定位

MindIsland 是一个轻量级桌面应用，运行在系统托盘中，实时监控本地 AI 编码代理的状态。开发者可以在不切换窗口的情况下：

- 查看所有活跃 agent 会话的状态（idle/busy/waiting）
- 审批权限请求（��件编辑、命令执行等）
- 回答 agent 提��的问题
- 一键跳转回对应的终端/IDE
- 追踪 token 用量

**与 Vibe Island 的差异**：
- 跨平台（Windows + macOS），不绑定 Dynamic Island
- 优先支持 UltraWork（基于 OpenCode 的 SSE/Plugin 机制）
- 开源

---

## 2. 技术栈

| 层 | 技术 | 理由 |
|----|------|------|
| 桌面框架 | **Tauri 2** | 跨平台、轻量（~5MB）、与 UltraWork 同栈 |
| 前端 | **React 19 + Vite 7 + Tailwind 4** | 与 UltraWork 一致，可共享��件 |
| 后端 (Rust) | tokio + serde + reqwest | 异步运行时，SSE 客户端，JSON 序列化 |
| IPC | Unix Socket (macOS) / Named Pipe (Windows) | Hook CLI 通信 |
| ��管理 | Bun | 快速安装 |

---

## 3. 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                       │
│                                                          │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ Session List │  │ Permission   │  │ Settings      │  │
│  │ (状态面板)   │  │ Panel (审批)  │  │ (配置管理)    │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
└────────────────────────┬────────────────────────────────┘
                         │ Tauri IPC (invoke / listen)
┌────────────────────────┴────────────────────────────────┐
│                    Rust Core                              │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              SessionManager                      │    │
│  │  - sessions: HashMap<String, AgentSession>       │    │
│  │  - emit events to frontend via app.emit()       │    │
│  └──────────────────────┬──────────────────────────┘    │
│                         │                                │
│  ┌──────────────────────┴──────────────────────────┐    │
│  │           AgentRegistry                          │    │
│  │  ┌────────────────┐  ┌─────────────────────┐   │    │
│  │  │ UltraWorkAgent │  │ ClaudeCodeAgent     │   │    │
│  │  │ (SSE client)   │  │ (Hook bridge)       │   │    │
│  │  └────────────────┘  └─────────────────────┘   │    │
│  │  ┌────────────────┐  ┌─────────────────────┐   │    │
│  │  │ CodexAgent     │  │ CustomAgent (扩展)   │   │    │
│  │  │ (Hook bridge)  │  │                     │   │    │
│  │  └────────────────┘  └─────────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │           Platform Services                      │    │
│  │  - TrayManager (系统托盘)                        │    │
│  │  - BridgeServer (IPC listener)                  │    │
│  │  - ProcessDiscovery (进程发现)                   │    │
│  │  - WindowJump (终端跳转)                         │    │
│  │  - NotificationService (通知)                   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│               Hook CLI (独立二进制)                       │
│  agent hook → stdin JSON → IPC send → Rust Core         │
│  支持: --source ultrawork|claude|codex|...              │
└─────────────────────────────────────────────────────────┘
```

---

## 4. 核心模块设计

### 4.1 Agent Trait (Rust)

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// 唯一标识
    fn id(&self) -> &str;
    /// 显示名
    fn display_name(&self) -> &str;
    /// 品牌色 (hex)
    fn brand_color(&self) -> &str;

    /// 启动监控（连接 SSE / 启动 IPC listener 等）
    async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<()>;
    /// 停止监控
    async fn stop(&self) -> Result<()>;

    /// 检测该 agent 是否在运行
    async fn detect(&self) -> bool;

    /// 回复权限请求
    async fn resolve_permission(&self, session_id: &str, approved: bool) -> Result<()>;
    /// 回答问题
    async fn answer_question(&self, session_id: &str, answer: &str) -> Result<()>;
}
```

### 4.2 UltraWork Adapter

```rust
pub struct UltraWorkAdapter {
    base_url: String,       // http://localhost:4096
    auth: String,           // Basic base64(opencode:test123)
    sse_handle: Option<JoinHandle<()>>,
}

impl AgentAdapter for UltraWorkAdapter {
    async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<()> {
        // 订阅 SSE: GET /event
        // 将 session.created/status/idle, message.*, permission.asked
        // 转换为统一的 AgentEvent 发送给 SessionManager
    }

    async fn detect(&self) -> bool {
        // GET /global/health ��� {"healthy": true}
        reqwest::get(&format!("{}/global/health", self.base_url))
            .await.map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn resolve_permission(&self, session_id: &str, approved: bool) -> Result<()> {
        // POST /permission/{id}/reply
    }
}
```

### 4.3 Claude Code Adapter

```rust
pub struct ClaudeCodeAdapter {
    bridge_listener: Option<JoinHandle<()>>,
}

impl AgentAdapter for ClaudeCodeAdapter {
    async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<()> {
        // 启动 IPC server (Unix Socket / Named Pipe)
        // 接收 Hook CLI 转发的 JSON payload
        // 解析 ClaudeHookPayload → AgentEvent
    }

    async fn detect(&self) -> bool {
        // 检查 ~/.claude/ 是否存在
        // ps 查找 claude 进程
    }
}
```

### 4.4 统一事件模型

```rust
#[derive(Debug, Clone, Serialize)]
pub enum AgentEvent {
    SessionCreated {
        agent_id: String,
        session_id: String,
        title: String,
        directory: String,
    },
    SessionStatusChanged {
        session_id: String,
        status: SessionStatus,  // Idle, Busy, WaitingPermission, WaitingAnswer
    },
    MessageDelta {
        session_id: String,
        text: String,
    },
    PermissionRequested {
        session_id: String,
        title: String,
        description: String,
        tool_name: Option<String>,
    },
    QuestionAsked {
        session_id: String,
        question: String,
    },
    SessionCompleted {
        session_id: String,
        summary: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub enum SessionStatus {
    Idle,
    Busy,
    WaitingPermission,
    WaitingAnswer,
    Completed,
    Error(String),
}
```

### 4.5 前端状态管理

```typescript
// stores/sessions.ts
interface AgentSession {
  id: string;
  agentId: string;        // "ultrawork" | "claude-code" | "codex"
  agentName: string;
  brandColor: string;
  title: string;
  directory: string;
  status: "idle" | "busy" | "waiting-permission" | "waiting-answer" | "completed";
  lastActivity: string;   // 最近活动摘要
  updatedAt: number;

  // 可选：权限/问题请求
  pendingPermission?: PermissionRequest;
  pendingQuestion?: QuestionRequest;
}
```

---

## 5. UI 设计

### 5.1 系统托盘

```
┌────────────────────────────┐
│  🟢 MindIsland             │  ← 托盘图标，颜色表示总��态
│                            │     🟢 全部空闲
│                            │     🟡 有��在运行
│                            │     🔴 有等待审批
└────────────────────────────┘
```

### 5.2 弹出面板（点击托盘图标）

```
┌─────────────────────────────────────┐
│  MindIsland          ⚙️  │ 宽 360px
├─────────────────────────────────────┤
│                                     │
│  ● UltraWork · workspace2          │  ← 会话行
│    🟡 Running: edit src/index.ts    │
│    2s ago                           │
│                                     │
│  ● Claude Code · vibeisland         │
│    🔴 Permission: Run npm install   │
│    [Allow] [Deny]                   │  ← 内��审批
│                                     │
│  ● UltraWork · project-x            │
│    🟢 Idle · "Refactored auth"      │
│    5m ago                           │
│                                     │
├─────────────────────────────────────┤
│  Sessions: 3  │  Tokens: 45.2k     │
└─────────────────────────────────────┘
```

### 5.3 实现���式

Tauri 2 不支持原生托盘弹窗面板，采用**浮动窗口**模式：
- 点击托盘图标 → 获取图标坐标 → 在附近创建/显示无边框窗口
- 窗口失去焦点时自动隐藏
- macOS: 窗口出现在菜单栏下方
- Windows: 窗口出现在任务栏上方

```rust
// 创建弹出面板窗口
fn create_panel_window(app: &AppHandle) -> Window {
    WindowBuilder::new(app, "panel")
        .title("MindIsland")
        .inner_size(360.0, 480.0)
        .decorations(false)         // 无边框
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)             // 默认隐藏
        .build()
        .unwrap()
}
```

---

## 6. 跨平台差异处理

| 功能 | macOS | Windows |
|------|-------|---------|
| 托盘图标 | NSStatusItem | Shell_NotifyIcon |
| 弹窗位置 | 菜单栏下方 | 任务栏上方 |
| Hook IPC | Unix Domain Socket | Named Pipe |
| 进程发现 | `sysinfo` crate | `sysinfo` crate |
| 终端跳转 | AppleScript / AX API | Windows Terminal wt.exe |
| 通知 | NSUserNotification | Toast (winrt) |
| 自启动 | launchd plist | Registry HKCU\Run |
| 配置路径 | `~/.config/mindisland/` | `%APPDATA%\mindisland\` |

**Rust 条件编译示例**：
```rust
pub fn ipc_path() -> PathBuf {
    #[cfg(unix)]
    { PathBuf::from("/tmp/mindisland.sock") }

    #[cfg(windows)]
    { PathBuf::from(r"\\.\pipe\mindisland") }
}
```

---

## 7. 项目结构

```
mindisland/
├── DESIGN.md                    # 本文档
├── package.json                 # Bun workspace root
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs             # 入口
│   │   ├── lib.rs              # Tauri setup + commands
│   │   ├── tray.rs             # 系统托盘管理
│   │   ├── panel.rs            # 弹出面板窗口管理
│   │   ├── session.rs          # SessionManager
│   │   ├── event.rs            # AgentEvent 定义
│   │   ├── agents/
│   │   │   ├── mod.rs          # AgentAdapter trait
│   │   │   ├── ultrawork.rs    # UltraWork SSE adapter
│   │   │   ├── claude.rs       # Claude Code hook adapter
│   │   │   └── codex.rs        # Codex hook adapter
│   │   ├── bridge/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs       # IPC server (socket/pipe)
│   │   │   └── protocol.rs     # Wire protocol (NDJSON)
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── macos.rs        # macOS 特定 (AX, AppleScript)
│   │       └── windows.rs      # Windows 特定 (COM, wt.exe)
│   └── icons/
├── src/                         # React 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── SessionList.tsx     # 会话列表
│   │   ├── SessionRow.tsx      # 单个会话行
│   │   ├── PermissionPanel.tsx # 权限审批 UI
│   │   ├── QuestionPanel.tsx   # 问题回答 UI
│   │   └── StatusBadge.tsx     # 状态标记
│   ├── stores/
│   │   └── sessions.ts        # 状态管理 (zustand)
│   ├── hooks/
│   │   └── useTauriEvents.ts  # 监听 Tauri 事件
│   └── lib/
│       └── tauri.ts           # Tauri IPC 封装
├── cli/                        # Hook CLI (独立 Rust binary)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # stdin → IPC forward
└── vite.config.ts
```

---

## 8. 开发路线图

### Phase 1: Claude Code MVP ✅ DONE

- [x] Tauri 2 项目初始化 (React 19 + Vite 7 + Tailwind 4)
- [x] 系统托盘 + 点击弹出浮动窗口 + 失焦自动收起
- [x] macOS Dock 隐藏 (Accessory activation policy)
- [x] Claude Code hook adapter (Unix Socket bridge)
- [x] Hook 脚本注册到 `~/.claude/settings.json`
- [x] SessionManager 事件驱动状态管理
- [x] 前端会话列表 + 实时状态 + 工具活动详情
- [x] 面板刷新：emit + panel-opened + 2s 轮询

### Phase 2: Claude Code 打磨 + OpenCode

- [ ] Claude Code 会话发现（扫描 `~/.claude/projects/` JSONL transcript）
- [ ] 权限审批 GUI（PermissionRequest 事件 → 面板审批 → hook stdout 回传）
- [ ] OpenCode adapter（复用 Claude Code hook 格式）
- [ ] 系统通知（有权限请求时弹系统通知）
- [ ] Stop 后状态正确变为 completed（绿色）
- [ ] 会话过期自动清理（超过 1 小时无活动的 completed 会话）

### Phase 3: UltraWork + 多 Agent

- [ ] 重新启用 UltraWork SSE adapter
- [ ] UltraWork 现有会话加载（REST API + 最后消息）
- [ ] 多 Agent 并行显示 + 按 agent 分组/筛选
- [ ] Claude Code fork 支持（Qoder/Qwen/Factory/CodeBuddy — 同 hook 格式，不同 `--source`）

### Phase 4: Windows + 发布

- [ ] Windows Named Pipe 替代 Unix Socket
- [ ] Windows 系统托盘 + 弹窗位置
- [ ] CI/CD 双平台打包 (DMG + MSI)
- [ ] 自动更新 (Sparkle / tauri-plugin-updater)
- [ ] Hook 自动安装器（settings.json 管理）

### Phase 5: 高级功能

- [ ] 终端精准跳转（AppleScript / Accessibility API）
- [ ] Token 用量追踪面板
- [ ] 插件系统
- [ ] Codex / Gemini CLI / Cursor adapter

---

## 9. MVP Scope���严格最小集）

第一版只做：

| 功能 | 包含 | 不包含 |
|------|------|--------|
| Agent 支持 | UltraWork (SSE) | Claude Code, Codex |
| 平台 | macOS | Windows (Phase 4) |
| UI | 会话列表 + 状态 | 权限审批、问题回答 |
| 交互 | 查看 | 跳转、审批 |
| 通知 | 无 | 系统通知 |

**MVP 验收标准**: 
1. 点击托盘图标弹出面板
2. 面板显示 UltraWork 所有活跃会话
3. 每个会话显示：名称、状态（idle/busy）、最近活动
4. 实时更新（SSE 推送）

---

## 10. 关键技术决策记录

| 决策 | 选择 | 备��� | 理由 |
|------|------|------|------|
| 框架 | Tauri 2 | Electron | 轻量、与 UltraWork 同栈 |
| 前端 | React | Vue/Svelte | 与 UltraWork 一致，可复用 |
| 状态管理 | Zustand | Redux/Jotai | 轻量、简单 |
| UltraWork 集成 | SSE | Plugin | SSE 零侵入、无需重启 |
| Claude Code 集成 | Hook + IPC | 无 | 官方支持的扩展方式 |
| IPC 协议 | NDJSON | gRPC/protobuf | 简单、可读、与 Open Island 兼容 |
| 弹窗实现 | 浮动窗口 | WebView popup | Tauri 2 无原生 tray popup |

---

## 11. 配置文件

```jsonc
// ~/.config/mindisland/config.json (macOS)
// %APPDATA%\mindisland\config.json (Windows)
{
  "agents": {
    "ultrawork": {
      "enabled": true,
      "url": "http://localhost:4096",
      "auth": "Basic b3BlbmNvZGU6dGVzdDEyMw=="
    },
    "claude-code": {
      "enabled": false,
      "hook_installed": false
    }
  },
  "ui": {
    "panel_width": 360,
    "panel_max_height": 600,
    "show_token_usage": true
  },
  "general": {
    "launch_at_login": false,
    "notification_sound": true
  }
}
```

---

## 12. 与 Open Vibe Island 的关系

MindIsland 不是 Open Vibe Island 的 fork，而是独立项目：
- Open Island: Swift-only, macOS-only, 复杂（4 个 target）
- MindIsland: Rust + Web, 跨平台, 简化架构

但��鉴了 Open Island 的设计理念：
- Hook fail-open 原则
- NDJSON bridge 协议
- Session state reducer 模式
- Agent adapter 分离

---

## 13. 已验证的技术结论（Lessons Learned）

### Claude Code Hook 集成

- **Payload 格式**: snake_case（`session_id`, `hook_event_name`, `tool_name`, `tool_input`）
- **Hook 注册**: `~/.claude/settings.json` 的 `hooks` 字段，每个事件可有多个 hook（与 Vibe Island 共存）
- **Hook 调用时机**: settings.json 在 Claude Code 进程启动时读取，中途修改需要新会话才生效
- **Subagent hooks**: `agent_id` 不为 null 时是子 agent 的 hook，应跳过（父 session 有 SubagentStart/Stop）

### IPC (Unix Socket)

- **Socket 路径**: `/tmp/mindisland-claude.sock`
- **启动时序**: MindIsland 启动后 socket bind 需要几百毫秒，hook 脚本需有重试逻辑
- **协议**: NDJSON（每行一个 JSON 对象）
- **Hook 脚本**: 必须用 temp file 中转 stdin（避免 shell 转义破坏 JSON），然后 python3 读文件发 socket

### Tauri 2 面板

- **Dock 隐藏**: `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` 比 Info.plist 可靠
- **透明窗口**: 需要 `macos-private-api` feature + Cargo.toml feature
- **Tray icon**: 需要 `image-png` feature 才能用 `Image::from_bytes`
- **emit 到隐藏窗口**: 不可靠，需要面板打开时主动 invoke + 轮询兜底
- **浮动面板**: 用 WebviewWindowBuilder 动态创建，`decorations(false)` + `always_on_top(true)` + `skip_taskbar(true)`

### UltraWork SSE（已验证，暂未集成）

- **端点**: `GET http://localhost:4096/event`（Basic auth `opencode:test123`）
- **事件格式**: SSE `data: {"type":"...", "properties":{...}}`
- **现有会话**: `GET /session` 拉取列表，`GET /session/{id}/message` 拉取最后消息
- **插件方式也可行**: `~/.config/ultrawork/opencode.json` 添加 `"plugin": ["./plugins/xxx.mjs"]`
