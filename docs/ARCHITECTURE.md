# Lumina 架构

## 1. 技术栈

| 层 | 选型 |
|---|---|
| 后端 | Tauri 2、tokio、reqwest (rustls)、tokio-tungstenite、serde、sysinfo |
| 前端 | Vue 3、Vite、TypeScript、Pinia、vue-router、Tailwind CSS v4 |
| 类型同步 | `tauri-specta`：由 Rust 类型生成 `src/api/bindings.ts`（M1 引入） |
| 存储 | v0.1：内存 LRU + JSON 配置；v0.2：`rusqlite` |
| 构建 | pnpm + cargo；GitHub Actions（Windows）出安装包 |

## 2. 分层

```
┌──────────── 前端 (WebView2) ─────────────┐
│  views / components / stores (Pinia)      │
│        ▲ Tauri events     │ invoke()      │
└────────┼──────────────────┼───────────────┘
┌────────┴──────────────────▼───────────────┐
│ commands/   前端可调用接口（薄封装）          │
│ services/   战绩聚合、自动接受、对局编排       │
│ state/      AppState：连接、游戏阶段、选人会话 │
│ clients/    lcu / sgp / live 协议层          │
└──────┬──────────────┬──────────────┬──────┘
   LOL 客户端 (LCU)   Riot SGP     游戏内 :2999
```

- 所有状态由 Rust 维护，前端只做展示。
- LCU WebSocket 事件 → 更新 `state` → emit Tauri event → Pinia store。
- 前端不直接访问 LCU/SGP，拿不到任何 token。

## 3. 模块

### 3.1 `clients/lcu`
- **发现**：每 2s 扫描 `LeagueClientUx.exe`，解析命令行 `--app-port`、`--remoting-auth-token`、`--rso_platform_id`。
- **降级**：腾讯服客户端以管理员运行时读不到命令行，改读安装目录 `lockfile`（`name:pid:port:password:protocol`），无需提权。
- **HTTP**：reqwest + Basic Auth（`riot:<token>`），打包 `riotgames.pem` 校验证书，不关闭证书验证。
- **WebSocket**：WAMP，发送 `[5, "OnJsonApiEvent"]` 订阅全部事件，按 URI 分发。
- **重连**：客户端退出 → `Disconnected`，清状态，继续扫描。

### 3.2 `clients/sgp`
- token：LCU `/entitlements/v1/token`、`/lol-league-session/v1/league-session-token`。
- 服务器：内置 `resources/servers.json`，后续支持远程更新。
- 接口：`match-history-query`（战绩）、`summoner-ledge`（玩家）、`gsm`（进行中的对局）。
- 降级：SGP 不可用时回退到 LCU `/lol-match-history/...`，结果标注数据来源。

### 3.3 `state/gameflow`
监听 `/lol-gameflow/v1/gameflow-phase`：
`None → Lobby → Matchmaking → ReadyCheck → ChampSelect → InProgress → EndOfGame`

- `ReadyCheck` → 自动接受
- `ChampSelect` → 加载队友战绩
- `InProgress` → 加载 10 人战绩

### 3.4 性能
- 战绩请求并发上限 5（`tokio::sync::Semaphore`）。
- LRU 缓存 `(puuid, 分页)`，TTL 5 分钟。
- 游戏图片走自定义协议 `lcu-asset://`，由 Rust 代理并落盘缓存。

## 4. 目录

```
lumina/
├── src/                        # 前端
│   ├── api/                    # invoke 封装 + bindings.ts
│   ├── stores/                 # connection / gameflow / matchHistory / settings
│   ├── views/                  # Home / MatchHistory / OngoingGame / Settings
│   └── components/             # match/ player/ common/
└── src-tauri/
    ├── resources/              # riotgames.pem, servers.json
    └── src/
        ├── lib.rs              # 组装应用、注册 commands
        ├── error.rs
        ├── config.rs
        ├── clients/
        │   ├── lcu/            # discovery.rs http.rs ws.rs models.rs
        │   ├── sgp/            # token.rs servers.rs http.rs models.rs
        │   └── live.rs
        ├── state/              # mod.rs gameflow.rs
        ├── services/           # match_history.rs player_stats.rs auto_accept.rs ongoing_game.rs
        ├── commands/
        └── asset_proxy.rs
```

## 5. 里程碑

| 阶段 | 内容 | 验收 |
|---|---|---|
| M0 | 脚手架、CI、Release 流程 | CI 通过并产出安装包 |
| M1 | LCU 发现 + HTTP + WebSocket，连接状态页 | 实时显示游戏阶段，客户端重启可自动重连 |
| M2 | 战绩查询（LCU → SGP + 降级） | 可查自己与任意 Riot ID，图片正常 |
| M3 | 自动接受对局 | 匹配成功后自动接受，延迟可配 |
| M4 | 选人 / 游戏中战绩面板 | 进入选人 3s 内出现队友数据 |
| M5 | 打包发布 | 安装包 < 15MB |

**v0.1 不包含**：自动选/禁英雄、游戏内发消息、计时器、多窗口、SQLite。

## 6. 风险

- 只用 LCU/SGP，不碰游戏内存与进程注入，与 League Akari 做法一致。
- 腾讯服 SGP 服务器与 token 行为与外服不同，M2 需单独验证。
- 依赖 WebView2：Win11 自带；Win10 由 NSIS 安装包引导安装。
