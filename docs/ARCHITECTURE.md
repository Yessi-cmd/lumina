# Lumina 架构

## 1. 技术栈

| 层 | 选型 |
|---|---|
| 后端 | Tauri 2、tokio、reqwest + tokio-tungstenite（LCU 走 native-tls / SChannel）、serde、sysinfo |
| 前端 | Vue 3、Vite、TypeScript、Pinia、vue-router、Tailwind CSS v4 |
| 类型同步 | 在 `src/api/index.ts` 手写。`tauri-specta` 需要在本机运行 debug 版才能生成 `bindings.ts`，维护者本机没有 Rust，暂不引入 |
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
- 窗口无原生边框（`decorations: false`），标题栏与最小化/最大化/关闭按钮由 `TitleBar.vue` 自绘，
  拖动区域用 `data-tauri-drag-region`。主题色集中在 `style.css`：zinc 色阶被重定义为 Lumina 的中性色，
  通用样式为 `.card` / `.btn-*` / `.field` / `.segmented`，图标为内联 SVG（`AppIcon.vue`），不引入组件库或图标包。

## 3. 模块

### 3.1 `clients/lcu`
- **发现**：每 2s 扫描 `LeagueClientUx.exe`，解析命令行 `--app-port`、`--remoting-auth-token`、`--rso_platform_id`。
- **降级**：腾讯服客户端以管理员运行时读不到命令行，改读安装目录 `lockfile`（`name:pid:port:password:protocol`），无需提权。
  lockfile 也读不到时，与 League Akari 一样提示「以管理员身份重启」（`services/elevation.rs`，经 UAC 启动提权实例）。
- **管理员权限**：国服（WeGame）客户端以管理员运行，连接信息只在其命令行里，lockfile 为空文件；
  因此 Lumina 的清单声明 `requireAdministrator`，启动时弹一次 UAC 即可（与 League Akari 一致）。
- **HTTP**：reqwest + Basic Auth（`riot:<token>`），编译期嵌入 `riotgames.pem` 作为唯一信任根（关闭系统根证书）。
  LCU 证书链到 Riot 2013 年的 v1/SHA-1 根证书，webpki（rustls）不接受，因此 LCU 用 native-tls（SChannel）；
  叶子证书不是签给 `127.0.0.1` 的，所以只跳过主机名校验，证书链照常校验。
- **WebSocket**：WAMP，发送 `[5, "OnJsonApiEvent"]` 订阅全部事件，`UriRouter` 按 URI 分发。
- **重连**：客户端退出 → `Disconnected`，清状态，继续扫描。
- **推送**：状态变化时 emit `lcu://snapshot`（完整快照）；阶段切换额外 emit `lcu://gameflow-phase`（`{ phase, previous }`）。

### 3.2 `clients/sgp`
- 服务器：内置 `resources/servers.json`（取自 League Akari 的内置配置，含国服各大区），后续支持远程更新。
  由 LCU `/riotclient/region-locale` 的 region + platformId 解析（`TENCENT` + `HN1` → `TENCENT_HN1`）；
  解析不到时仍可用，全部走 LCU。
- token：战绩接口用 LCU `/entitlements/v1/token` 的 accessToken（每次请求前现取）；
  `summoner-ledge` 等接口需要 `/lol-league-session/v1/league-session-token`，用到时再接入。
- 接口：M2 只用 `match-history-query` 的 SUMMARY（按 puuid 分页）。
  `gsm` 在国服选人阶段返回 403/404（见 Akari 日志），不用于 BP；加载阶段敌方名单直接取 LCU `/lol-gameflow/v1/session`。
- 降级：SGP 失败时回退到 LCU `/lol-match-history/v1/products/lol/{puuid}/matches`，
  结果带 `source` 与 `sgpError` 标注。
- 注意：客户端开加速器时，Lumina 直连 SGP 不一定走加速，失败会自动回退 LCU。

### 3.3 `state/gameflow`
监听 `/lol-gameflow/v1/gameflow-phase`：
`None → Lobby → Matchmaking → ReadyCheck → ChampSelect → InProgress → EndOfGame`

- `ReadyCheck` → 自动接受（M3，`services/auto_accept.rs`）：按设置延迟 0–10 秒后 POST `/lol-matchmaking/v1/ready-check/accept`。
  手动接受/拒绝（`/lol-matchmaking/v1/ready-check` 的 `playerResponse`）、点「取消本次」、关闭开关或离开 ReadyCheck 都会取消；
  倒计时通过 `auto-accept://state` 推给前端横幅。设置保存在配置目录的 `settings.json`（`config.rs`）。
- `ChampSelect` → 队友名单（`services/ongoing_game.rs`）
- `GameStart` / `InProgress` → 10 人名单

### 3.3.1 对局名单（M4）
- 选人阶段：读取 `/lol-champ-select/v1/session` 的 `myTeam/theirTeam`。
  `nameVisibilityType` 为 `HIDDEN` 时，在 Rust `clients/lcu/puuid.rs` 中按 Akari-Yessi
  的固定 16 字节 XOR 掩码解析 `obfuscatedPuuid`，恢复的 PUUID 进入正常名单及战绩预取流程。
  可见玩家直接使用原始 `puuid`；匿名标识缺失、格式错误或为空 UUID 时保留匿名队友占位或敌方隐藏计数，
  后续客户端提供有效身份时自动更新。
- 加载阶段：`/lol-gameflow/v1/session` 的 `gameData.teamOne/teamTwo` 出现双方 puuid，按自己所在队伍分我方/敌方。
- 名单变化时 emit `ongoing://roster`，并对新出现的 puuid 预取第一页战绩（20 场）进缓存；
  前端卡片请求同一页，基本直接命中缓存。进入 `None/Lobby/Matchmaking/ReadyCheck` 时清空名单。

### 3.3.2 玩家画像与标签（`services/player_profile.rs`、`services/roster_relations.rs`）
来源：League Akari 的 player-card tags + Akari-Yessi 的习惯分析（练英雄、近期异常、闪现键位、开黑推断）。
Lumina 把分析全部放到 Rust，并做了这些增强：

- **按队列取样**：当前是排位就用近期排位（单双/灵活合并），同队列样本不足 5 场才退回全部对局，卡片注明样本范围。
- **按位置基线**：伤害占比与该位置常见水平比较（辅助 10%、打野 17%、上单 22%、中下 26%），补刀只评价线上位置。
- **练英雄**：近期峡谷对局里当前英雄 ≤2 场时，再看英雄成就点（`/lol-champion-mastery/v1/{puuid}/champion-mastery`）：
  <2 万「练英雄」，≥10 万「熟练 N万」（以前常玩、最近没碰），拿不到成就点只标低置信的「近期少玩」。
- **证据与置信度**：每个标签都带说明文字（样本量、数值、阈值），样本少于 8 场标为低置信。
- **跨玩家关系**：用 SGP 返回的 10 人名单交叉比对同局 10 人的近期对局，推断开黑小队（同队 ≥3 场）和"遇见过"。
- **阈值集中**：所有阈值在 `player_profile.rs` 的 `limits` 模块，均有单元测试。

每局的队内占比来自 SGP SUMMARY（全部参与者），LCU 回退数据只有本人，此时相关标签不出现。

### 3.3.3 田忌赛马（`services/timeline.rs`、`services/matchup.rs`、`services/roster_insights.rs`）
- **时间线**：每人最近 8 场单双排/灵活对局读取 SGP DETAILS（`{区}_{gameId}/DETAILS`），同一局只取一次并缓存 400 局。
  得出 15 分钟前被敌方打野参与击杀次数（Akari 的「好抓」口径）和 10 分钟对位经济/补刀差。
- **排位样本**：每人除共享的第一页外，再按 `q_420` / `q_440` 各拉 20 场，合并后取最近 20 场排位。
- **战力值**（0–100，50 为平均）只看单双排和灵活（`player_profile::RankedForm`），匹配、娱乐、人机都不算。
  每场权重按时间衰减（往前 6 场减半），不在本局位置的对局按 30% 计。
  = 50 + 胜率项（加权胜率向 50% 收缩 4 场，权重 50）+ 表现项（±12）+ 对线项（每 40 经济 1 分，±12）。
  表现项逐场对比同位置平均：死亡/10 分钟、参团率、伤害占比、经济占比，每项限制在 ±1，
  不看 KDA，一两局刷出来的数据拉不动整体。
  + 位置项：本局位置占近期排位的比例，30% 为 0，每 +10% 加 2 分，范围 −6~+5（补位扣得比本位置加得多）。
  排位不足 5 场不计算。
- **标签**：队内比中位数高/低 ≥6 → 我方「上等马 / 下等马」、敌方「硬骨头 / 软柿子」；
  同位置战力差 ≥10 → 「对位优势 / 劣势」；本局位置占近期排位 <15%（≥8 场）→「补位」；好抓 / 非常好抓 / 难抓；对线强 / 弱（±350 经济）。
  同一事实对敌我语气相反（敌方好抓是机会，我方好抓是提醒）。
- **建议**：敌方软柿子与硬骨头、最大优势路与劣势路；选人阶段只有我方信息时提示需要照顾的队友。
- 所有阈值在各模块的 `limits` 中，均有单元测试。

### 3.3.4 选英雄助手（`clients/lolalytics.rs`、`services/champion_assist.rs`）
- 数据：lolalytics 网站自用接口 `a1.lolalytics.com/mega/`：`ep=list`（某位置全部英雄的排名、Tier 1–15、胜率/选取/禁用）
  与 `ep=build-full`（`c` 为小写英文名，含最常用/最高胜率两套符文、召唤师技能、加点、出门装、核心装、克制关系）。
  外服排位（queue=420，近 30 天），分段由设置 `statsTier` 决定，默认翡翠及以上。非官方接口，内存缓存 6 小时。
- 已拥有英雄：LCU `/lol-champions/v1/owned-champions-minimal`。
- 克制关系：`ep=counter`（同位置对位），用高分段数据（设置 `matchupTier`，默认钻石+），因为低分段胜负更多取决于个人操作。
  使用归一化优势 `d2`（扣除双方英雄整体胜率后的差值），不看原始对位胜率；按 95% 置信区间判定：
  |优势| 超过误差才算「克制 / 被克制」，其余为「均势」，少于 200 场为「样本不足」。
  选人阶段敌方身份隐藏但已锁英雄可见（`Roster.enemyChampions`），逐个给出对位判断。
- 摇摆位：对位数据带对手的常走位置（`defaultLane`），常走本路的敌方英雄排在前面；只有一个时自动当作对位，
  其余标「常走 X」。也可点击或搜索手动指定对位，切换候选英雄时保留，前端从 `all` 中直接查，不再请求。
- 一键应用：删除旧的 `Lumina` 前缀符文页后新建；页数已满时改写当前可编辑页。
  召唤师技能用 `PATCH /lol-champ-select/v1/session/my-selection`，保持玩家原来闪现所在的键位。

### 3.4 性能
- 战绩请求并发上限 5（`tokio::sync::Semaphore`）。
- LRU 缓存 `(puuid, 分页)`，TTL 5 分钟。
- 游戏图片走自定义协议 `lcu-asset://`（WebView 中为 `http://lcu-asset.localhost/<LCU 路径>`），
  只放行 `/lol-game-data/assets/`，由 Rust 带认证取回并缓存到 app cache 目录。

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
        │   ├── lcu/            # discovery.rs http.rs ws.rs events.rs models.rs
        │   ├── sgp/            # token.rs servers.rs http.rs models.rs
        │   └── live.rs
        ├── state/              # mod.rs gameflow.rs
        ├── services/           # lcu_connection.rs match_history.rs player_stats.rs auto_accept.rs ongoing_game.rs
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
