# Lumina

轻量、高性能的英雄联盟桌面助手，基于 Tauri 2（Rust）+ Vue 3。

通过本地 LCU API 与 Riot SGP 接口获取战绩、读取对局信息并操作客户端。不读写游戏内存，不注入进程。

## 功能（v0.1 规划）

- [ ] 自动连接 LOL 客户端（含腾讯服），断线自动重连
- [ ] 战绩查询：自己 / 按 Riot ID 搜索
- [ ] 选人阶段与游戏中的 10 人战绩面板
- [ ] 自动接受对局

详见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 开发

需要 Node.js 24+、pnpm、Rust stable，以及 Windows 上的 MSVC C++ 构建工具。

```sh
pnpm install
pnpm tauri dev      # 启动桌面应用
pnpm build          # 仅前端类型检查 + 构建
```

本机没有 Rust 环境时，推送后由 GitHub Actions 编译，安装包在 CI 运行页面的 Artifacts 中下载。
推送 `v*` 标签会自动生成草稿 Release。

## 致谢

架构与接口用法参考了 [League Akari](https://github.com/Hanxven/LeagueAkari)（MIT）。

## 声明

Lumina 不是 Riot Games 官方产品，与 Riot Games 无关联，也未获其认可。
League of Legends 及相关名称是 Riot Games, Inc. 的商标。

## License

[MIT](LICENSE)
