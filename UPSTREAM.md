# 上游来源与本地维护

本项目是独立维护的中文定制版，不是 Tuxedo 官方发行版，也不是 GitHub 平台上的 fork 仓库。

- 上游：https://github.com/webstonehq/tuxedo
- 基础版本：[`v2026.8.1`](https://github.com/webstonehq/tuxedo/releases/tag/v2026.8.1)
- 最初导入的源码包：`https://codeload.github.com/webstonehq/tuxedo/tar.gz/refs/tags/v2026.8.1`
- 源码包 SHA256：`3135e38b61bdf12f751143b5f704ebd3b1ec6f25dd7625baeb7f7f30e56b13ea`
- 许可证：MIT，原版权声明保留在 [`LICENSE`](LICENSE)。

从该版本保留了完整 Rust 程序源码、Cargo.lock、单元测试、界面快照测试及截图生成示例。未复制上游发布流水线、Nix 配置、宣传图片和示例待办文件。

## 已纳入源码的修改

- 菜单、帮助、设置、编辑弹窗、命令菜单、日历、手机录入页面等中文界面。
- 中文双宽字符布局、输入光标、滚动和补全宽度修正。
- 终端焦点通知；恢复焦点、窗口尺寸变化及外部编辑器返回时完整重绘。
- `Ctrl+L` 手动重绘，保留未保存的输入。
- 修复编辑器选择中的 `VISUAL` 环境变量。
- 项目统一命名为 todot；全局命令直接启动，默认数据及配置隔离到 ~/.todot，保留显式路径覆盖。
- 不在启动时检查官方更新，update 只提示重新编译，避免覆盖定制版。

早期汉化补丁已经合入 `src/`，不再作为构建输入，也不必重新导出补丁。

## 后续升级

本仓库不包含上游 Git 历史。升级时对照上游新版本的源码变更，按需移植到 `src/`，重新检查中文界面，并运行 `./install.sh --test` 与 `python3 tests/check.py`。不要仅修改 Cargo.toml 中的版本号，也不要重新下载上游源码覆盖当前 `src/`。
