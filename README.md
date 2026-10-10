# todot：中文 Linux 待办终端

基于 [Tuxedo v2026.8.1](https://github.com/webstonehq/tuxedo/releases/tag/v2026.8.1) 的中文定制版。**修改后的 Rust 源码直接保存在本仓库**，新环境克隆后即可重新构建；后续直接改 `src/`，不再维护汉化补丁。

上游来源、修改范围及 MIT 许可证见 [UPSTREAM.md](UPSTREAM.md) 和 [LICENSE](LICENSE)。项目、GitHub 仓库、Cargo 包和可执行文件统一叫 `todot`。

列表只显示任务标题与属性；正文使用内置多行编辑器录入，在侧栏或居中的详情弹窗中查看。个人待办、正文和设置默认保存在 `~/.todot/`，不进入 Git。

## 新环境安装

支持 Linux x86_64 / aarch64，使用 glibc；已验证 Ubuntu 24.04 x86_64。aarch64 尚未实测。不支持 Windows、macOS 或 Alpine/musl。建议 UTF-8 终端、中文等宽字体，以及至少 120 列 × 36 行窗口。

Ubuntu / Debian 准备工具：

```bash
sudo apt update
sudo apt install -y git build-essential curl util-linux python3
```

本仓库是私有仓库，需要有访问权限的 GitHub SSH 密钥：

```bash
git clone git@github.com:YevinDong/todot.git
cd todot
./install.sh
export PATH="$HOME/.local/bin:$PATH"  # 加入 ~/.bashrc 或对应 shell 配置
todot
```

也可以用已登录的 GitHub CLI 克隆：`gh repo clone YevinDong/todot`。

- `install.sh` 只编译当前仓库源码，不下载或覆盖上游源码。
- Rust 版本由 `rust-toolchain.toml` 固定为 `1.99.0`，依赖由 `Cargo.lock` 固定；构建使用 `--locked`。
- Rust 工具链、依赖缓存与编译结果保存在项目 `.local/`；二进制安装到 `~/.local/bin/todot`，也可用 `INSTALL_DIR=/usr/local/bin ./install.sh` 指定 PATH 内的安装目录（需要写入权限）。
- 首次构建需要联网访问 Rust 下载服务和 crates.io，耗时与机器配置有关；缓存完整后可复用。
- 待办、正文及设置默认保存在当前用户的 `~/.todot/`，与项目目录分离。
- 首次启动复用该用户已有数据；没有旧数据时创建空 `~/.todot/todo.txt`。
- 在任何目录直接运行 `todot`，默认使用同一用户的数据；不会隐式读取当前目录的 `todo.txt`。

## 常用操作

以下为默认快捷键；自定义绑定保存在数据目录的 `config/todot/keybinds.toml`。

| 按键 | 操作 |
|---|---|
| `j` / `k` 或上下箭头 | 移动 |
| `n` / `i` | 新建任务 / 以输入模式编辑任务 |
| `e` | 以标题普通模式编辑；按 `i` 进入输入模式 |
| `/` | 搜索任务 |
| `x` | 完成 / 取消完成；归档视图中恢复任务 |
| `v` / `Space` | 进入多选 / 勾选任务；`dd` 删除所选，`x` 完成所选 |
| `p` | 切换优先级 |
| `dd` | 连续按两次 `d` 删除；待办删除可用 `u` 撤销，归档删除不可撤销 |
| `a` | 在待办与归档（`done.txt`）视图间切换 |
| `A` | 将已完成的待办移入归档 |
| `H` | 在当前列表显示 / 隐藏已完成但未归档的任务 |
| `t` | 显示 / 隐藏未来任务 |
| `:` | 命令菜单，可输入中文搜索 |
| `Enter` | 打开居中的详情弹窗，优先显示正文；`j/k` 滚动、`PgUp/PgDn` 翻页、`Home/End` 首尾、`Esc` 返回 |
| `[` / `]` | 显示 / 隐藏筛选区域、详情区域 |
| `?` / `,` | 帮助 / 设置 |
| `O` | 可选：使用外部编辑器打开笔记，依次尝试 `VISUAL`、`EDITOR`、`nvim`、`vim`、`vi` |
| `s` | 手机录入二维码 |
| `T` | 设置固定主题及深色／浅色主题 |
| `Ctrl+L` | 重新检测终端深浅模式并强制重绘，保留未保存输入 |
| `q` | 在主列表中退出 |

任务按 `todo.txt` 格式保存，例如：

```text
(A) 整理发布说明 +工作 @电脑 due:2026-10-20
每周整理笔记 +个人 @家里 due:2026-10-25 rec:1w
```

### 深色／浅色主题

按 `T` 打开主题设置（设置页也使用同一选择器）：

- `m`：切换固定模式／跟随终端。
- `Tab` / `Shift-Tab`：选择固定／回退、深色、浅色槽位。
- `j/k` / 上下箭头 / `T`：预览所选槽位的主题；允许两个槽位选择同一主题。
- `Enter`：保存全部主题设置；`Esc`：取消全部修改，返回原页面。
- `Ctrl+L`：重新检测并重绘，不丢失未保存的标题、正文或主题预览。

示例配置（旧配置没有 `theme_mode` 时保持固定模式）：

```toml
theme = Muted Slate
theme_mode = auto
theme_dark = Nord
theme_light = Dawn
```

跟随模式在启动或启用时检测终端；改变终端配色后按 `Ctrl+L` 更新。优先查询终端明确的深浅偏好（CSI ? 996），无响应时查询默认背景（OSC 11），用亮度推断。检测异步进行，每阶段等待最多 300ms，不阻塞输入。启动检测失败使用 `theme` 回退；后续失败保留最近的主题。不存在的主题名回退到固定主题。

跟随的是终端，不直接查询操作系统。终端必须支持并传递查询响应；SSH、tmux 等中间层可能影响检测。当前不订阅实时变化通知；Windows 使用固定主题回退。选择器打开期间不会被检测结果打断，关闭后应用最新结果。自动切换不写配置，其他设置保存和配置热加载会保留双主题槽位。

### 内置正文编辑

不需要安装或启动外部编辑器：

1. 按 `n` 新建任务，填写标题及任务属性；按 `i` 直接编辑已有待办。归档任务先按 `x` 恢复，再编辑。
2. 按 `Tab` 切到正文，输入或粘贴 Markdown、代码和长文本。
3. 按 `Ctrl-S` 同时保存标题与正文。
4. 在列表中选中任务，侧栏预览正文；按 `Enter` 查看完整正文与任务属性，不显示内部正文文件链接。

| 编辑器按键 | 操作 |
|---|---|
| `Tab` / `Shift-Tab` / `F6` | 切换标题与正文；标题补全弹层打开时，`Tab` 先确认补全 |
| 正文中的 `Enter` | 换行，不提交任务 |
| 正文中的上下箭头、`Home` / `End` | 移动光标、到行首／行尾；长内容自动滚动 |
| `Ctrl-S` / 标题框的 `Enter` | 保存标题与正文 |
| 正文中的 `Esc` | 回到标题普通模式，再按 `Esc` 取消编辑 |

标题框保持单行，只解析标题、优先级、项目、场景及日期。多行粘贴使用终端的括号粘贴模式，只能放入正文框，避免误建任务。

正文保留原始 Markdown、代码、空白及任务样式词语；其中的 `+标签`、`@场景`、`due:` 等不会参与任务解析。当前显示纯文本原文，不做 Markdown 渲染。

### 正文保存与删除

`todo.txt` 仍是一行一条任务。新正文保存到数据目录中的 `notes/tasks/<随机标识>.md`，由程序维护关联，不需要手工创建文件或填写路径；修改标题不会改变正文文件名。已有 `note:<路径>` 关联继续使用，不重写旧笔记模板。

| 任务操作 | 正文处理 |
|---|---|
| 完成 / 取消完成 | 保留 |
| 将已完成任务归档到 `done.txt` | 保留 |
| 删除待办、批量删除、删除归档任务 | 删除关联正文；仍被其他待办或归档任务引用时保留 |
| 在当前会话中用 `u` 撤销待办删除 | 恢复任务和缺失的正文文件，不覆盖同名已有文件 |

重复任务沿用同一正文关联，因此只有最后一个引用删除时才清理文件。指向正文目录之外的旧笔记不会自动删除。归档删除不可撤销；普通正文编辑不参与 `u` 撤销，退出程序或撤销记录清空后无法撤销待办删除。

保存失败保留输入；检测到外部修改时拒绝覆盖。任务与正文分文件保存，普通写入失败会尝试回滚，但不保证异常退出或掉电时两个文件一起提交。

`O` 仅用于可选的外部编辑器流程：优先使用 `VISUAL`、`EDITOR`，再尝试 `nvim`、`vim`、`vi`；可执行文件不存在时继续降级，其他启动或退出错误在界面中提示，不退出 TUI。

### 日期、窄屏与终端恢复

菜单、帮助、设置、提示及手机页面均已汉化。中文任务可以输入、搜索；**不支持中文自然语言日期解析**。日期用日历选择器或 `due:YYYY-MM-DD`，也可以使用上游支持的英文日期表达式。

终端不足 100 列时，列表全宽，筛选与详情移到下方。筛选最多 3 行，当前选项优先显示；详情为预览，按 `Enter` 查看完整内容。高度不足时优先保留列表，隐藏的详情仍可展开。旋转屏幕或弹出键盘时自动调整，不修改面板开关设置。

切回终端后延迟 300ms 完整重绘，期间仍可输入；再次失焦会取消待执行重绘。调整窗口大小及外部编辑器返回时立即完整重绘。若终端不发送焦点通知或仍出现残影，按 `Ctrl+L`。

## 数据与隐私

| 本地路径 | 用途 | 是否进入 Git |
|---|---|---|
| `~/.todot/todo.txt` | 当前待办 | 否 |
| `~/.todot/done.txt` | 已完成归档 | 否 |
| `~/.todot/inbox.txt` | 手机录入或待处理条目 | 否 |
| `~/.todot/notes/` | 正文及任务笔记 | 否 |
| `~/.todot/config/` | 设置、快捷键、主题、手机访问令牌等 | 否 |
| `项目/.local/` | 工具链、缓存、二进制、临时检查资料 | 否 |
| `项目/todo.md` | 原有文件，不读取、不导入、不修改 | 否 |

**GitHub 只保存工程代码，不同步个人待办。** 数据根目录权限为 `700`，新文件默认只允许当前用户访问。常用设置文件为 `~/.todot/config/todot/config.toml`；`config/` 包含手机访问令牌，不应公开。

### 旧数据迁移

先按 `q` 正常退出旧 TUI，再复制数据：

```bash
./migrate-data.sh "$HOME/.todo-tui" "$HOME/.todot"
todot
```

脚本使用 `cp -a` 复制待办、归档、收件箱、正文和配置，验证内容，保留旧目录。旧 `config/tuxedo/` 改为 `config/todot/`；旧项目 `.config/` 复制为 `config/`。不传参数时优先从 `~/.todo-tui` 迁移，否则从旧项目目录迁移。**目标已存在时拒绝覆盖或合并；复制失败保留已复制文件并报错，需检查后再启动。** 相对 `note:` 关联继续使用，绝对外部笔记路径不搬动。

`x` 只标记完成，任务仍在 `todo.txt`；`H` 可查看。按 `A` 才移入 `done.txt`，之后按 `a` 查看归档。

### 备份与独立数据

换机器前退出 TUI，再安全复制整个 `~/.todot/`，包含待办、归档、正文和配置；只复制 `todo.txt` 不会带上正文。新机器克隆代码并构建后，`todot` 会复用已有用户数据。

需要另一套独立数据时，指定专用目录的绝对路径，不要使用系统目录或混有其他文件的目录：

```bash
TODO_DIR=/完整路径/另一套数据 todot
# 如果是迁移已有数据，先退出程序，再复制到不存在的新目录：
./migrate-data.sh "$HOME/.todot" /完整路径/另一套数据
```

### 启动与命令行

无需启动脚本，路径规则由程序自身处理，TUI 与 CLI 一致：

```bash
todot              # 打开 TUI
todot list         # 命令行查看待办
todot --json list  # JSON 输出，供脚本读取
todot --help       # 查看命令
```

待办路径优先级：显式 `todot FILE` → `TODO_FILE` → `TODO_DIR/todo.txt` → `~/.todot/todo.txt`。归档默认是待办文件旁的 `done.txt`，可用 `DONE_FILE` 覆盖。配置默认 `<数据根目录>/config/todot/`，保留 `XDG_CONFIG_HOME` 覆盖；正文默认 `<数据根目录>/notes/`，保留 `NOTES_DIR` 和配置中的 `notes_dir` 覆盖。数据根目录为 `TODO_DIR` 或 `~/.todot`。其他参数（包括 `--sample`）保留；帮助和版本不触碰数据。

### 手机录入

在 TUI 按 `s`，会启动局域网 HTTP 服务。仅在可信局域网使用：HTTP 不加密，URL 中的访问令牌不应分享。关闭二维码弹窗不会停止服务，退出 todot 才会停止。

## 继续开发

主要文件：

```text
src/                        Rust 程序源码与手机页面
Cargo.toml / Cargo.lock      包信息与锁定依赖
rust-toolchain.toml          固定 Rust 工具链
install.sh                  构建并安装全局 todot 命令
migrate-data.sh              旧项目数据复制、验证与安全迁移
vendor/crossterm/            带终端主题响应解析补丁的 crossterm 0.29（MIT）
tests/theme_terminal.py     深浅检测、主题设置及查询期间输入回归
tests/check.py              用户数据隔离、正文交互与终端恢复回归
tests/snapshots.rs           界面快照、正文与详情弹窗渲染回归
tests/snapshots/             中文界面的快照基线
.github/workflows/check.yml  GitHub Actions 构建与测试
```

修改 `src/` 后：

```bash
bash -n install.sh migrate-data.sh
./install.sh --test
python3 tests/check.py
python3 tests/theme_terminal.py
todot
```

`--test` 会检查库、主程序、界面快照和文档测试；Rust 测试的配置隔离在 `.local/test-config/`。Python 检查使用 `.local/` 下的临时项目和隔离 HOME，验证数据迁移、路径隔离、中文输入/归档、正文多行粘贴与详情、删除联动与撤销、CLI 删除、焦点恢复、缩放、编辑器返回，以及 `Ctrl+L` 保留输入，不修改真实待办。

构建成功替换安装目录中的 `todot` 和项目 `.local/bin/todot`，已运行的旧进程不会自动更新。请正常退出旧 TUI，再启动新版。

提交代码：

```bash
git status --short
git add README.md src tests install.sh migrate-data.sh Cargo.toml Cargo.lock
# 按实际修改补充其他工程文件；不要使用 -f 强制加入个人数据
git diff --cached --stat
git diff --cached --check
git commit -m "Add built-in task body editing and isolated user data"
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

本项目独立维护，不在启动时检查上游更新。`todot update` 只提示重新编译；它不会安装官方英文二进制。上游升级需要手动移植并验证，详见 [UPSTREAM.md](UPSTREAM.md)。
