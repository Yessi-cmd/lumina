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
- 每次连接使用独立编号；战绩、详情和时间线缓存包含连接编号，旧连接结果不用于新连接。
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
  `gsm` 在国服选人阶段可能返回 403/404（见 Akari 日志），不用于 BP；加载阶段优先取 LCU `/lol-gameflow/v1/session`。
  排位名单不足 5v5 时，使用 league-session token 查询 SGP `gsm/v1/ledge/region/{platform}/puuid/{puuid}`，
  仅合并同一局缺失的成员并短时重试。
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
- 加载阶段：`/lol-gameflow/v1/session` 的 `gameData.teamOne/teamTwo` 提供双方 puuid，按自己所在队伍分我方/敌方。
  有时会直接少一条玩家记录；排位时显示缺失人数，并从 GSM 当前对局名单补齐，以便预取该玩家战绩。
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
- 对局分析先返回基础关系标签，再补充战力和时间线分析；失败时保留已有结果并提供重试。

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

### 3.3.5 选人阶段悬浮窗（`services/overlay_window.rs`、`services/client_window.rs`、`services/draft.rs`、`src/overlay/`）
仿 Akari-Yessi 的 BP 悬浮窗，设置项 `champSelectOverlay`（默认开）。
- 窗口：两个置顶窗口，无边框、透明、不可聚焦。不可聚焦（`focusable(false)`）是为了点它时不抢走客户端的键盘焦点。
  `overlay-allies` 贴在客户端左侧，`overlay-enemies` 贴在右侧；客户端贴着屏幕边时，悬浮窗收回屏幕内。
  每 250ms 用 Win32 `FindWindowW("RCLIENT")` + `GetWindowRect` 读客户端位置。只读窗口几何，不碰客户端进程。
  客户端最小化或不在前台时隐藏。
  布局按 720 高设计，前端用 CSS `zoom` 按客户端实际高度缩放，每行对齐客户端的五个席位。
  悬浮窗与主窗口加载同一个页面，`main.ts` 按窗口 label 挂载 `OverlayApp`。
  开启后，选人阶段不再把主窗口弹到前台，免得挡住客户端。
- 左侧：名单中的队友按楼层排列。数据是 SGP `q_420` 最近 50 场单双排，展示常用英雄前 3（场次、胜率、场均 K/D/A）。
- 右侧（`draft.rs`，推送 `overlay://draft`）：
  - 分路推断：三个信号相乘。
    - 英雄在各路的场次占比，取自 lolalytics 五个位置的榜单。
    - 该玩家 ban 的英雄所在的分路，因为 ban 常针对自己这一路。
    - 五人分路互斥：枚举所有不重复的分配，求每人每路的边际概率。
    客户端给了 `assignedPosition` 时直接用。
  - counter：取该英雄在推断分路的高分段对位数据。敌方英雄在误差之外输掉的对位排在前面，其次是其它劣势对位。
    已 ban、已选的英雄会去掉。
    我方该路已锁定时，改为显示我方英雄的对位胜率。
    我方已经没有未完成的选人时（例如对方是最后一手），提示无法 counter。
  - 选人状态没变的事件直接跳过。每次分析带一个递增编号，较慢的旧结果不会覆盖新的。

### 3.3.6 生涯分析（`services/career.rs`、`src/components/career/`）
战绩页召唤师卡片下的第二个标签。
- **范围**：近期 20 场；本赛季（1 月 1 日起，联盟赛季随年份开始）；生涯（战绩能翻到的全部）。
  后两者按 50 场一页翻 SGP，最多 400 场。可按模式筛选（SGP `tag=q_<queueId>`）。
- **同段位对比**：排位匹配会把段位相近的玩家放进同一局，所以同局其他玩家就是同段位样本。
  解析 SGP 对局时，每局记下三组分钟数据（`GameSummary.comparison`）：
  - 自己
  - 其余 9 人的平均
  - 同位置的敌方（对位）

  过半对局有对位数据时拿对位比，否则拿同局平均比。
- **雷达六维**：输出、承伤、发育、参团、视野、生存。
  每维 = 50 × 我 / 参照（生存取倒数），50 表示持平，100 表示两倍，限制在 0–100。
- **其它数据**：
  - 段位：`/lol-ranked/v1/ranked-stats/{puuid}`，包括当前、最高、上赛季。
  - 生涯熟练度：英雄成就点。
  - 常用英雄。
  - 位置分布。
  - 近 60 场的胜负和近 10 场滚动胜率。
- **图表配色**：我 `#d97706`、参照 `#0b8fd0`，这一对已通过深色背景下的亮度、色盲区分度和对比度检查。
  所有图表都有图例，每个数据点有悬停提示。

### 3.3.7 段位记录（`services/rank_history.rs`、`src/views/RankView.vue`）
客户端只给当前段位，不给每局的胜点变化，所以 Lumina 自己记。
- **时机**：客户端连接时记一次起点；进入 `EndOfGame` 后每 5 秒读一次 `/lol-ranked/v1/current-ranked-stats`（最多 6 次，客户端更新段位会晚几秒）；回到 `Lobby` / `None` 时再补读一次。
- **记录**：单双排和灵活分开。队列的段位、胜点、胜负场数与该队列上一条不同，就新增一条记录（`RankPoint`）。
  已定级才记；新赛季胜负场数变小时当作新起点，不算差值。
- **胜点差**：把段位折成一个数（每个小段 100 点，黑铁 IV 0 点为 0，大师及以上共用一条刻度，从 2800 起），
  相减得到差值，所以升降级也能算出真实的胜点变化。
- **胜负**：只有两次记录之间刚好多打了一局才判断；多局合并或只有胜点变化（比如衰减）时不判断。
- **对局信息**：判断出胜负后，用绕过缓存的战绩请求（`MatchHistoryService::get_fresh`）找该队列上一条记录之后最新且胜负吻合的一局，
  附上对局编号、英雄、KDA，最多重试 3 次。找不到就只保留胜点变化。
- **存储**：`rank_history.json`（应用数据目录），按账号分开，每个账号最多保留 4000 条。写入先写临时文件再替换；
  文件损坏时改名为 `.bad` 保留，不覆盖。数据量小，暂不引入 SQLite。
- **前端**：`rank://updated` 事件触发刷新。走势图、今日 / 近 7 天 / 累计胜点、平均每胜每负、最长连胜连败、
  英雄加分 / 掉分榜都在前端由记录算出（`src/utils/rank.ts`）。

### 3.3.8 提醒与赛后（`services/tilt.rs`、`services/push.rs`、`services/honor.rs`）
三个都由设置开关控制，设置项见 `config.rs`。
- **连败止损**（`tilt_streak`，默认连败 3 把，0 关闭）：从段位记录里取最近连续的排位败局。
  两局之间隔超过 3 小时、或最近一局离现在超过 3 小时，就不算连败。
  新记录写入后检查一次（可推送到手机），开始排队（`Matchmaking`）时再检查一次（只在界面提醒）。
  警告存在 `AppState.tilt`，变化时 emit `tilt://warning`，前端在顶部显示横幅，点「知道了」清除。
- **手机推送**（`push_provider`：`off` / `bark` / `serverchan`）：找到对局（`ReadyCheck`）、进入选人（`ChampSelect`）、连败提醒三种消息，各有开关。
  Bark 走 `GET /{key}/{标题}/{内容}`（密钥也可以是自建服务器的完整地址），Server酱走 `POST https://sctapi.ftqq.com/{SendKey}.send`。
  消息直接从本机发往渠道，不经过其他服务器。设置页有「发送测试消息」。密钥明文存在 `settings.json`。
- **自动荣誉点赞**（`auto_honor`，默认关）：进入 `PreEndOfGame` 后每 2 秒读一次 `/lol-honor-v2/v1/ballot`（最多 8 次），
  有可点赞的队友就随机选一人，`POST /lol-honor-v2/v1/honor-player`，类型由 `honor_category` 决定。
  点赞接口的字段没有在真实客户端上验证过，失败只写日志，不影响其他功能。

### 3.3.9 搭档分析（`services/teammates.rs`、`src/components/career/TeammatesCard.vue`）
生涯分析页底部的「常一起玩的人」，范围和模式筛选跟生涯分析一致，复用它已取到的战绩页（`career::games`，命中缓存）。
- 只统计带全员名单的对局（SGP）。LCU 回退的战绩没有队友，重开和斗魂竞技场也不算。
- 同队至少 3 场才列出，最多 10 人，按同队场数排序；名字用 `/lol-summoner/v2/summoners/puuid/{puuid}` 并发查询，查不到就显示「未知玩家」。
- 另算「和这些人任意一人同队」与「其余对局」两组的胜率，用来看有没有固定队友时状态更好。

### 3.3.10 自动 BP（`services/auto_select.rs`、`src/views/AutoSelectView.vue`）
设置项 `auto_select`：自动禁用、自动选择两个开关，锁定延迟（0–10 秒），以及按分路的预设。
预设的键是 `TOP / JUNGLE / MIDDLE / BOTTOM / UTILITY / ANY`，每个分路各有一份「禁用」和「选择」列表，最多 10 个，越靠前越优先。
- **触发**：监听 `/lol-champ-select/v1/session`。自己的 ban 或 pick 轮到（`isInProgress`）且还没有预选英雄时才动手；
  已经点了英雄就不替你换。同一个 action 只处理一次，离开选人阶段后清空记录。
- **取哪个**：先取该分路的列表，再接「通用」（`ANY`）的列表；分路取自 `myTeam[].assignedPosition`，没有时只用「通用」。
  按顺序取第一个客户端允许的：禁用看 `bannable-champion-ids`，选择看 `pickable-champion-ids`。
  禁用还会跳过自己和队友正在预选（`championId`）或已表态（`championPickIntent`）的英雄，免得禁掉队友要玩的。
- **动作**：`PATCH /lol-champ-select/v1/session/actions/{id}`（`{championId}`）预选，等延迟后重新读一次会话，
  只有这个 action 还开着、且预选的还是刚才那个英雄，才 `POST .../complete` 锁定；期间你换了英雄就不锁。
- 预选与锁定前均校验连接、阶段和设置；修改自动 BP 设置、离开选人或断线会使待执行任务失效。

### 3.3.11 客户端小工具（`services/client_tools.rs`、`src/views/ToolsView.vue`）
- **在线状态与签名**：`PUT /lol-chat/v1/me`，状态为 `chat`（在线）/ `away`（离开）/ `offline`（隐身），签名最多 100 字。
- **生涯背景**：`/lol-champions/v1/inventories/{summonerId}/champions/{id}/skins` 列出某英雄的皮肤和是否拥有，
  `POST /lol-summoner/v1/current-summoner/summoner-profile`（`{key: "backgroundSkinId", value}`）设置，当前背景从同一路径读取。
- **重启客户端窗口**：`POST /riotclient/kill-and-restart-ux`，只重启客户端界面进程，游戏不受影响；前端要点两次确认。
  重启会断开 LCU 连接，`lcu_connection` 会自动重连。
- 以上接口没有在真实客户端上验证过，失败时把客户端返回的错误显示出来。

### 3.3.12 悬停提示
全局只有一个提示层（`TooltipLayer.vue`），元素上用 `v-tip`，同一时刻只显示一个，不会互相叠。
- 装备、召唤师技能、符文的名称和说明来自 LCU 的 `items.json`、`summoner-spells.json`、`perks.json`。
  客户端的富文本标签在后端转成纯文本。
- 从一个元素移到另一个时，提示框直接滑过去。

### 3.4 后端安全

- 安装包必须提供有效的 SHA-256 摘要，缺失时只提供发布页手动下载；大小限制为 256 MiB。
- Windows 更新目录创建时设置受保护 ACL，所有者为 Administrators，仅允许管理员和 SYSTEM 访问，禁止复用已有目录。
- 使用固定本地文件名排他创建安装包，不使用发布资源名称拼接本地路径；下载完成后重新读取磁盘文件校验。
- 校验至启动安装程序期间保持禁止写入、删除的文件句柄，并锁定目录防止重命名。失败时清理本次文件，启动成功后保留供安装程序使用。
- 推送失败仅记录状态码或固定错误分类，不保留包含推送密钥的 URL 和底层错误链。
- 图片代理只接受资源目录内的 ASCII 路径片段，拒绝编码、点目录、反斜杠和 URL 分隔符。

### 3.5 性能
- 战绩请求并发上限 5（`tokio::sync::Semaphore`）。
- LRU 缓存 `(puuid, 分页)`，TTL 5 分钟。
- 同一连接的相同战绩页请求共享执行锁，预取与玩家卡片不会并行重复拉取同一页。
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
