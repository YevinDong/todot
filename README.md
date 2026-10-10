# todot

基于 [Tuxedo](https://github.com/webstonehq/tuxedo) 的中文待办终端应用，使用 Rust 构建，以 `todo.txt` 纯文本格式管理任务，提供终端交互界面和命令行操作。

本项目基于 Tuxedo `v2026.8.1` 独立维护，采用 MIT 许可证。上游来源及维护说明见 [UPSTREAM.md](UPSTREAM.md)。

## 功能

- **任务管理**：新建、编辑、完成、归档、优先级设置、批量操作和撤销。
- **分类与筛选**：项目标签、场景标签、模糊搜索、保存筛选条件及多种排序方式。
- **日期与重复任务**：截止日期、延后显示、日历选择器和周期性任务。
- **正文与笔记**：内置多行编辑器，支持粘贴文本、Markdown 和代码；提供侧栏预览、详情弹窗及外部编辑器入口。
- **终端体验**：中文界面、键盘操作、自定义快捷键、命令菜单和窄屏自适应布局。
- **主题设置**：固定主题或跟随终端深浅模式，分别配置深色和浅色主题。
- **手机录入**：通过局域网网页和二维码录入任务，自动合并到待办列表。
- **命令行接口**：支持任务增删改查、归档及 JSON 输出，便于脚本集成。

## 构建与安装

安装脚本支持 Linux x86_64 / aarch64（glibc）。需要 Git、C 编译工具、curl 和 util-linux。

Ubuntu / Debian：

```bash
sudo apt update
sudo apt install -y git build-essential curl util-linux

git clone git@github.com:YevinDong/todot.git
cd todot
```

### 使用安装脚本

```bash
./install.sh
```

脚本按 `rust-toolchain.toml` 准备 Rust 工具链，使用锁定依赖构建 release 版本，并安装到 `~/.local/bin/todot`。工具链和构建缓存位于项目 `.local/`。

将安装目录加入 `PATH`，并将以下配置保存到 `~/.bashrc` 或对应 shell 配置文件：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

指定其他安装目录：

```bash
INSTALL_DIR=/usr/local/bin ./install.sh
```

安装目录需具备写入权限。更新程序时，在仓库中拉取最新代码后重新运行安装脚本。

### 使用 Cargo

已有 Rust / rustup 环境时，可以直接构建：

```bash
cargo build --release --locked
./target/release/todot
```

手动安装构建结果：

```bash
install -Dm755 target/release/todot "$HOME/.local/bin/todot"
```

## 运行

### 终端界面

```bash
todot                      # 使用默认数据目录
todot /path/to/todo.txt    # 打开指定待办文件
TODO_DIR=/path/to/data todot
```

默认数据目录为 `~/.todot/`，首次运行创建空待办文件。程序可从任意工作目录启动。

### 命令行

```bash
todot add "整理发布说明 +工作 @电脑 due:2026-12-01"
todot list
todot done 1
todot archive
todot --json list
todot --help
```

任务编号对应文件行号，从 `1` 开始，可通过 `todot list` 查看。完整命令和参数见 `todot --help`。

## 终端操作

| 按键 | 操作 |
|---|---|
| `j` / `k`、上下箭头 | 移动光标 |
| `n` | 新建任务 |
| `i` / `e` | 编辑任务（输入模式 / 普通模式） |
| `x` | 切换完成状态；在归档中恢复任务 |
| `p` | 切换优先级 |
| `r` | 修改截止日期 |
| `v` / `Space` | 进入多选 / 勾选任务 |
| `dd` / `u` | 删除 / 撤销待办操作 |
| `/` | 模糊搜索 |
| `fp` / `fc` | 按项目 / 场景筛选 |
| `ff` / `fs` | 选择 / 保存筛选 |
| `S` | 切换排序 |
| `H` / `F` | 显示或隐藏已完成 / 未来任务 |
| `A` / `a` | 归档已完成任务 / 切换归档视图 |
| `Enter` | 查看任务详情 |
| `[` / `]` | 切换筛选栏 / 详情栏 |
| `:` / `Ctrl-P` | 打开命令菜单 |
| `T` | 设置主题 |
| `s` | 打开手机录入二维码 |
| `?` / `,` | 帮助 / 设置 |
| `Ctrl+L` | 重新检测终端主题并重绘 |
| `q` | 退出 |

完成状态与归档分开管理：`x` 标记完成，`H` 可在当前列表查看；`A` 将已完成任务移入归档，`a` 查看归档。归档删除不可撤销。

### 正文编辑

新建或编辑任务时，按 `Tab` 在标题和正文间切换；正文支持多行输入和粘贴。按 `Ctrl-S` 保存，`Esc` 返回标题普通模式。正文随任务关联保存，可在侧栏或 `Enter` 打开的详情中查看。

按 `O` 可使用外部编辑器打开笔记，优先使用 `VISUAL`、`EDITOR`，其次尝试 `nvim`、`vim`、`vi`。

### 手机录入

在终端界面按 `s`，扫描二维码后通过网页添加任务。服务随程序退出而关闭。手机录入使用带访问令牌的 HTTP 服务，仅在可信局域网使用，请勿公开访问链接。

## 任务格式

任务使用 `todo.txt` 格式，一行一条：

```text
(A) 整理发布说明 +工作 @电脑 due:2026-12-01
每周整理笔记 +个人 @家里 due:2026-12-06 rec:1w
x 2026-11-20 2026-11-18 完成环境配置 +工作
```

| 字段 | 含义 |
|---|---|
| `(A)` | 优先级，范围 `A`–`Z` |
| `+项目` | 项目标签 |
| `@场景` | 场景标签 |
| `due:YYYY-MM-DD` | 截止日期 |
| `t:YYYY-MM-DD` | 开始显示日期 |
| `rec:1w` | 每周重复；支持 `d`、`w`、`m`、`y`、`b` 周期单位 |
| `rec:+1w` | 按截止日期计算下一次重复 |
| `x` | 已完成任务前缀 |

## 数据与配置

默认目录结构：

```text
~/.todot/
├── todo.txt               # 当前任务
├── done.txt               # 已完成归档
├── inbox.txt              # 待合并任务
├── notes/                 # 正文与笔记
└── config/todot/
    ├── config.toml        # 界面与主题设置
    ├── keybinds.toml      # 自定义快捷键
    └── themes/            # 自定义主题
```

个人数据保存在本地，不通过 Git 同步。备份时复制整个数据目录，以保留任务、正文和配置。

### 路径设置

| 环境变量 | 用途 |
|---|---|
| `TODO_DIR` | 数据根目录，默认 `~/.todot` |
| `TODO_FILE` | 指定待办文件 |
| `DONE_FILE` | 指定归档文件 |
| `NOTES_DIR` | 指定正文与笔记目录 |
| `XDG_CONFIG_HOME` | 指定配置根目录，配置位于其下的 `todot/` |
| `VISUAL` / `EDITOR` | 外部编辑器 |

待办路径优先级为：显式文件参数 → `TODO_FILE` → `TODO_DIR/todo.txt` → `~/.todot/todo.txt`。归档默认位于待办文件旁；配置和正文默认位于数据根目录下。

### 主题配置

在界面按 `T` 设置主题，或编辑 `config.toml`：

```toml
theme = Muted Slate
theme_mode = auto
theme_dark = Nord
theme_light = Dawn
```

`theme_mode = fixed` 使用固定主题，`auto` 跟随终端深浅模式。更改终端配色后，可按 `Ctrl+L` 重新检测。

### 数据迁移

退出程序后，使用迁移脚本复制数据到尚不存在的目标目录：

```bash
./migrate-data.sh /path/to/old-data /path/to/new-data
```

脚本复制任务、归档、正文和配置并验证内容，保留源目录；兼容旧版 `~/.todo-tui` 数据结构。

## 测试

```bash
./install.sh --test
python3 tests/check.py
python3 tests/theme_terminal.py
```

Rust 测试覆盖核心逻辑和界面快照；Python 回归测试覆盖终端交互、数据路径、正文编辑及主题检测，需安装 Python 3。

## 许可证

[MIT](LICENSE)。保留 Tuxedo 及相关依赖的版权声明。
