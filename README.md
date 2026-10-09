# todo-tui：中文 Linux 待办终端

基于 [Tuxedo v2026.8.1](https://github.com/webstonehq/tuxedo/releases/tag/v2026.8.1) 的中文定制版。**修改后的 Rust 源码直接保存在本仓库**，新环境克隆后即可重新构建；后续直接改 `src/`，不再维护汉化补丁。

上游来源、修改范围及 MIT 许可证见 [UPSTREAM.md](UPSTREAM.md) 和 [LICENSE](LICENSE)。Cargo 包和可执行文件仍叫 `tuxedo`，GitHub 仓库叫 `todo-tui`。

## 新环境安装

支持 Linux x86_64 / aarch64，使用 glibc；已验证 Ubuntu 24.04 x86_64。aarch64 尚未实测。不支持 Windows、macOS 或 Alpine/musl。建议 UTF-8 终端、中文等宽字体，以及至少 120 列 × 36 行窗口。

Ubuntu / Debian 准备工具：

```bash
sudo apt update
sudo apt install -y git build-essential curl util-linux python3
```

本仓库是私有仓库，需要有访问权限的 GitHub SSH 密钥：

```bash
git clone git@github.com:YevinDong/todo-tui.git
cd todo-tui
./install.sh
./start.sh
```

也可以用已登录的 GitHub CLI 克隆：`gh repo clone YevinDong/todo-tui`。

- `install.sh` 只编译当前仓库源码，不下载或覆盖上游源码。
- Rust 版本由 `rust-toolchain.toml` 固定为 `1.99.0`，依赖由 `Cargo.lock` 固定；构建使用 `--locked`。
- Rust 工具链、依赖缓存、编译结果与最终二进制均保存在项目 `.local/`，不修改系统 PATH。
- 首次构建需要联网访问 Rust 下载服务和 crates.io，耗时与机器配置有关；缓存完整后可复用。
- 新克隆的项目不会带真实待办；首次启动创建空 `todo.txt`。
- 从其他目录运行 `/完整路径/todo-tui/start.sh`，仍只使用此项目的数据与配置。

## 常用操作

| 按键 | 操作 |
|---|---|
| `j` / `k` 或上下箭头 | 移动 |
| `n` / `e` | 新建 / 编辑任务 |
| `/` | 搜索任务 |
| `Space` | 完成 / 取消完成 |
| `p` | 切换优先级 |
| `d` | 删除；可用 `u` 撤销 |
| `a` | 在待办与已完成视图间切换 |
| `t` | 显示 / 隐藏未来任务 |
| `:` | 命令菜单，可输入中文搜索 |
| `Enter` | 展开完整详情；`j/k` 滚动、`PgUp/PgDn` 翻页、`Home/End` 首尾、`Esc` 返回 |
| `[` / `]` | 显示 / 隐藏筛选区域、详情区域 |
| `?` / `,` | 帮助 / 设置 |
| `O` | 创建或打开所选任务的笔记，使用 `VISUAL` / `EDITOR` 指定的编辑器 |
| `s` | 手机录入二维码 |
| `Ctrl+L` | 强制重绘当前界面，保留未保存输入 |
| `q` | 在主列表中退出 |

任务按 `todo.txt` 格式保存，例如：

```text
(A) 整理发布说明 +工作 @电脑 due:2026-10-20
每周整理笔记 +个人 @家里 due:2026-10-25 rec:1w
```

菜单、帮助、设置、提示及手机页面均已汉化。中文任务可以输入、搜索；**不支持中文自然语言日期解析**。日期用日历选择器或 `due:YYYY-MM-DD`，也可以使用上游支持的英文日期表达式。

终端不足 100 列时，列表全宽，筛选与详情移到下方。筛选最多 3 行，当前选项优先显示；详情为预览，按 `Enter` 查看完整内容。高度不足时优先保留列表，隐藏的详情仍可展开。旋转屏幕或弹出键盘时自动调整，不修改面板开关设置。

切回终端、调整窗口大小及外部编辑器返回时会完整重绘。若终端不发送焦点通知或仍出现残影，按 `Ctrl+L`。

## 数据与隐私

| 本地路径 | 用途 | 是否进入 Git |
|---|---|---|
| `todo.txt` | 当前待办 | 否 |
| `done.txt` | 已完成归档 | 否 |
| `inbox.txt` | 手机录入或待处理条目 | 否 |
| `notes/` | 任务笔记 | 否 |
| `.config/` | 设置、手机访问令牌等 | 否 |
| `.local/` | 工具链、缓存、二进制、临时检查资料 | 否 |
| `todo.md` | 原有文件，不读取、不导入、不修改 | 否 |

**GitHub 只保存工程代码，不同步个人待办。** 换机器时，如需继续原任务，请另行安全复制 `todo.txt`、`done.txt` 和 `notes/`；复制前退出旧 TUI。新环境的 `.config/` 可重新生成，不建议公开或直接分享其中的访问令牌。

`start.sh` 固定当前项目的数据和配置路径，拒绝其他文件路径、`--sample` 及未知命令，避免误打开或覆盖数据。支持 `./start.sh --help`、`./start.sh --version`、`./start.sh serve`。

### 手机录入

在 TUI 按 `s`，或运行 `./start.sh serve`，会启动局域网 HTTP 服务。仅在可信局域网使用：HTTP 不加密，URL 中的访问令牌不应分享。关闭二维码弹窗不会停止服务，退出 Tuxedo 才会停止。

## 继续开发

主要文件：

```text
src/                       Rust 程序源码与手机页面
Cargo.toml / Cargo.lock    包信息与锁定依赖
rust-toolchain.toml         固定 Rust 工具链
install.sh / start.sh       构建与项目隔离启动
tests/check.py              中文交互及终端恢复的伪终端回归检查
tests/snapshots.rs          上游保留的界面快照测试
tests/snapshots/            中文界面的快照基线
.github/workflows/check.yml GitHub Actions 构建与测试
```

修改 `src/` 后：

```bash
./install.sh --test
python3 tests/check.py
./start.sh
```

`--test` 会检查库、主程序、界面快照和文档测试；Rust 测试的配置隔离在 `.local/test-config/`。Python 检查使用 `.local/` 下的临时项目，验证路径隔离、中文输入/归档、焦点恢复、缩放、编辑器返回，以及 `Ctrl+L` 保留输入，不修改真实待办。

构建成功只会替换 `.local/bin/tuxedo`，已运行的旧进程不会自动更新。请正常退出旧 TUI，再启动新版。

提交代码：

```bash
git status --short
git add src Cargo.toml Cargo.lock README.md
# 按实际修改补充其他工程文件；不要使用 -f 强制加入个人数据
git commit -m "Fix terminal redraw behavior"
git push origin main
```

提交消息使用英文。每次 push / pull request 会自动执行 GitHub Actions 检查，不自动发布或覆盖其他机器上的程序。

### 界面快照变更

如果有意改变界面文字或布局，可更新基线；不要用更新快照掩盖意外回归：

```bash
INSTA_UPDATE=always ./install.sh --test
git diff -- tests/snapshots
./install.sh --test
python3 tests/check.py
```

先检查快照差异，再提交快照和对应源码。无需额外安装 `cargo-insta`。

## 上游更新

本项目独立维护，官方自更新指引已在启动脚本中关闭。`./start.sh update` 只提示重新编译；它不会安装官方英文二进制。上游升级需要手动移植并验证，详见 [UPSTREAM.md](UPSTREAM.md)。
