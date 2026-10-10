#!/usr/bin/env python3
"""隔离用户数据，检查原生命令、中文 TUI 和 UTF-8 任务往返。"""
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / '.local/bin/todot'
assert BINARY.is_file(), '先运行 ./install.sh'

# GitHub 只保存工程；真实待办、令牌和构建缓存必须保持未跟踪。
if (ROOT / '.git').exists():
    private_files = {'todo.md', 'todo.txt', 'done.txt', 'inbox.txt'}
    private_dirs = {'.local', '.config', 'target', 'notes'}
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    assert not any(path in private_files or path.split('/')[0] in private_dirs for path in tracked)
    for path in sorted(private_files | {name + '/privacy-check' for name in private_dirs}):
        assert subprocess.run(['git', 'check-ignore', '--quiet', path], cwd=ROOT).returncode == 0, path

with tempfile.TemporaryDirectory(prefix='中文检查 ', dir=ROOT / '.local') as temp:
    project = Path(temp) / '待办 项目'
    (project / '.local/bin').mkdir(parents=True)
    launcher = project / '.local/bin/todot'
    shutil.copy2(BINARY, launcher)
    home = Path(temp) / '用户目录'
    data = home / '.todot'
    editor = project / '编辑器.sh'
    editor.write_text("#!/bin/sh\nprintf 'EDITOR_SENTINEL\\n'\n")
    editor.chmod(0o755)
    env = dict(os.environ, TERM='xterm-256color', LANG='C.UTF-8',
               HOME=str(home), VISUAL=str(editor), EDITOR=str(project / '不存在的编辑器'))
    for key in ['TODO_TUI_HOME', 'TODO_DIR', 'TODO_FILE', 'DONE_FILE', 'NOTES_DIR', 'XDG_CONFIG_HOME']:
        env.pop(key, None)
    cwd_todo = Path(temp) / 'todo.txt'
    cwd_todo.write_text('当前目录任务不应隐式打开\n')

    def run(*args, expected=0):
        result = subprocess.run([str(launcher), *args], cwd=temp, env=env,
                                text=True, capture_output=True, timeout=10)
        assert result.returncode == expected, result.stderr
        return result.stdout + result.stderr

    assert 'todot' in run('--version')
    assert '用法：' in run('--help')
    assert '中文定制版' in run('update')
    assert not data.exists(), '帮助和版本不能创建数据'
    assert '未知选项' in run('--unknown', expected=2)

    raw = '(A) 检查中文保存 +工作 @电脑 due:2099-12-31'
    run('add', raw)
    listing = json.loads(run('--json', 'list'))
    assert '检查中文保存 +工作 @电脑 due:2099-12-31' in (data / 'todo.txt').read_text()
    assert not (project / 'todo.txt').exists()
    assert not (project / '.config').exists()
    assert data.stat().st_mode & 0o777 == 0o700
    assert '检查中文保存' in json.dumps(listing, ensure_ascii=False)
    assert cwd_todo.read_text() == '当前目录任务不应隐式打开\n'

    # 明确指定的环境变量仍生效，TUI 和 CLI 共用同一解析器。
    override = Path(temp) / '独立数据'
    override_env = dict(env, TODO_DIR=str(override))
    result = subprocess.run([str(launcher), 'add', '独立任务'], env=override_env,
                            capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr
    assert '独立任务' in (override / 'todo.txt').read_text()
    explicit_file, explicit_done = override / '指定.txt', override / '归档.txt'
    override_env.update(TODO_FILE=str(explicit_file), DONE_FILE=str(explicit_done))
    for args in [('add', '指定任务'), ('done', '1'), ('archive',)]:
        result = subprocess.run([str(launcher), *args], env=override_env,
                                capture_output=True, text=True, timeout=10)
        assert result.returncode == 0, result.stderr
    assert '指定任务' in explicit_done.read_text()
    assert '指定任务' not in (override / 'todo.txt').read_text()

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))

    def controlling_terminal():
        os.setsid()
        fcntl.ioctl(0, termios.TIOCSCTTY, 0)

    process = subprocess.Popen([str(launcher)], cwd=temp, env=env,
                               stdin=slave, stdout=slave, stderr=slave,
                               preexec_fn=controlling_terminal)
    os.close(slave)

    def plain(text):
        return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', text)

    def screen(keys=b'', *, raw=False, duration=1.5):
        if keys:
            os.write(master, keys)
        output = bytearray()
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([master], [], [], min(0.1, max(0, deadline - time.monotonic())))[0]:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    break
                if not chunk:
                    break
                output.extend(chunk)
        text = output.decode('utf-8', errors='replace')
        return text if raw else plain(text)

    try:
        initial = screen(raw=True)
        assert '优先级' in plain(initial)
        assert '\x1b[?1004h' in initial, '启动时应启用终端焦点通知'
        pending = screen(b'\x1b[O\x1b[I', raw=True, duration=0.15)
        assert '\x1b[2J' not in pending, '恢复焦点不能立即清屏'
        focused = screen(raw=True, duration=0.5)
        assert '\x1b[2J' in focused and '优先级' in plain(focused), '恢复焦点 300ms 后应清屏并完整重绘'
        pending = screen(b'\x1b[I', raw=True, duration=0.1)
        cancelled = screen(b'\x1b[O', raw=True, duration=0.5)
        assert '\x1b[2J' not in pending + cancelled, '再次失焦应取消待执行重绘'
        # 同尺寸 SIGWINCH 不会触发 Ratatui autoresize，也必须重绘。
        os.kill(process.pid, signal.SIGWINCH)
        assert '\x1b[2J' in screen(raw=True), '同尺寸窗口恢复应重绘'
        fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 36, 140, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        assert '优先级' in screen(), '窗口缩放后应重绘'
        # 手机窄屏：上下布局、完整详情、缩小高度后仍可返回列表。
        fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 40, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        mobile = screen()
        assert 'Enter' in mobile and '展开' in mobile, mobile
        expanded = screen(b'\r')
        assert '创建日期' in expanded and '检查中文保存' in expanded, expanded
        screen(b'j\x1b[6~')
        fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 16, 40, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        assert '详情' in screen(), '键盘弹出后详情应重新布局'
        restored = screen(b'\x1b', raw=True)
        assert '展开' in plain(restored), 'Esc 应恢复列表与预览'
        fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 36, 140, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        assert '优先级' in screen(), '宽屏应恢复三栏'
        help_text = screen(b'?')
        assert '导航' in help_text and '记录格式' in help_text, help_text
        screen(b'\x1b')
        settings = screen(b',')
        assert '设置' in settings and '快捷键' in settings, settings
        screen(b'\x1b')
        commands = screen(':归档'.encode())
        assert '归档已完成任务' in commands, commands
        screen(b'\x1b')
        editor_return = screen(b'O', raw=True)
        assert 'EDITOR_SENTINEL' in editor_return, '应使用 VISUAL 指定的编辑器'
        assert '\x1b[2J' in editor_return and '优先级' in plain(editor_return), '编辑器返回后应完整重绘'
        screen(b'x')
        assert '检查中文保存' in screen(b'H'), 'H 显示完成但未归档的任务'
        assert '暂无已完成任务' in screen(b'a'), 'a 只显示归档，不自动归档'
        screen(b'a')
        screen(b'A')
        assert '检查中文保存' in (data / 'done.txt').read_text()
        assert '检查中文保存' in screen(b'a'), 'A 后 a 应读取同目录的 done.txt'
        screen(b'x')  # 归档视图中恢复任务。
        screen(b'a')
        assert '检查中文保存' in (data / 'todo.txt').read_text()
        assert (data / 'config/todot/config.toml').is_file(), '设置也应使用默认数据根目录'
        assert '新建任务' in screen(b'n')
        screen('交互录入中文'.encode())
        refreshed = screen(b'\x0c', raw=True)
        assert '\x1b[2J' in refreshed and '交互录入中文' in plain(refreshed), 'Ctrl+L 应重绘并保留未保存输入'
        refocused = screen(b'\x1b[O\x1b[I', raw=True)
        assert '\x1b[2J' in refocused and '交互录入中文' in plain(refocused), '恢复焦点不能丢失未保存输入'
        assert '输入中' in screen(b'\t'), 'Tab 应切换到正文框'
        body_text = '# 正文首段\n\n  缩进 +不是项目 @不是场景 due:2099-01-01\n```rust\nlet value = 1;\n```\n正文末段 BODY_TAIL'
        pasted = screen(b'\x1b[200~' + body_text.encode() + b'\x1b[201~')
        assert 'BODY_TAIL' in pasted, '多行粘贴应显示正文尾部'
        screen(b'\r')
        body_text += '\n'
        assert '交互录入中文' not in (data / 'todo.txt').read_text(), '正文 Enter 只能换行，不能保存'
        assert '已添加' in screen(b'\x13'), 'Ctrl-S 应保存标题和正文'
        saved_line = next(line for line in (data / 'todo.txt').read_text().splitlines() if '交互录入中文' in line)
        note_rel = next(token[5:] for token in saved_line.split() if token.startswith('note:'))
        assert (data / 'notes' / note_rel).read_text() == body_text, '正文必须逐字保存'
        assert '+不是项目' not in saved_line and 'due:2099-01-01' not in saved_line
        expanded_body = screen(b'\r')
        assert '正文首段' in expanded_body and 'BODY_TAIL' in expanded_body, '详情应显示正文'
        assert 'note:' not in expanded_body, '详情不能显示内部正文文件链接'
        screen(b'\x1b')
        screen(b'dd')
        assert '交互录入中文' not in (data / 'todo.txt').read_text()
        assert not (data / 'notes' / note_rel).exists(), '删除任务必须同时删除正文文件'
        screen(b'u')
        assert '交互录入中文' in (data / 'todo.txt').read_text()
        assert (data / 'notes' / note_rel).read_text() == body_text, '撤销必须恢复正文'
        exited = screen(b'q', raw=True)
        assert '\x1b[?1004l' in exited, '退出时应关闭终端焦点通知'
        assert process.wait(timeout=5) == 0
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        os.close(master)

    assert '交互录入中文' in (data / 'todo.txt').read_text()
    run('done', '1')
    run('archive')
    assert '检查中文保存' in (data / 'done.txt').read_text()
    assert '检查中文保存' not in (data / 'todo.txt').read_text()
    assert '交互录入中文' in (data / 'todo.txt').read_text()
    assert (data / 'notes' / note_rel).read_text() == body_text, '完成或归档不能删除正文'
    run('--force', 'del', '1')
    assert not (data / 'notes' / note_rel).exists(), 'CLI 删除也必须清理正文'

    # 显式 cp 迁移旧数据；目标已存在时不得覆盖或重复导入。
    legacy = Path(temp) / '旧数据'
    (legacy / 'notes').mkdir(parents=True)
    (legacy / 'config/tuxedo').mkdir(parents=True)
    originals = {'todo.txt': '旧任务 note:旧正文.md\n', 'done.txt': 'x 已完成\n',
                 'inbox.txt': '收件箱\n', 'notes/旧正文.md': '# 正文\n',
                 'inbox.txt.tuxedo-staging': '恢复收件箱\n',
                 'config/tuxedo/config.toml': 'theme = Dawn\n'}
    for name, content in originals.items():
        (legacy / name).write_text(content)
    migrated = Path(temp) / '迁移目录'
    migration_env = dict(env, TODO_DIR=str(migrated))
    result = subprocess.run([str(ROOT / 'migrate-data.sh'), str(legacy), str(migrated)], env=migration_env,
                            capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr
    for name, content in originals.items():
        assert (legacy / name).read_text() == content
        target = name.replace('config/tuxedo/', 'config/todot/').replace('.tuxedo-staging', '.todot-staging')
        assert (migrated / target).read_text() == content
    (legacy / 'todo.txt').write_text('不会重复导入\n')
    result = subprocess.run([str(ROOT / 'migrate-data.sh'), str(legacy), str(migrated)], env=migration_env,
                            capture_output=True, text=True, timeout=10)
    assert result.returncode != 0 and '未覆盖' in result.stderr
    assert (migrated / 'todo.txt').read_text() == originals['todo.txt']
    assert not (legacy / 'notes/旧正文.md').is_symlink()

print('通过：用户数据目录/安全迁移、中文交互/归档、正文编辑/多行粘贴/详情/删除联动、窄屏布局、焦点恢复/缩放/编辑器返回、Ctrl+L 保留输入。')
