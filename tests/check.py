#!/usr/bin/env python3
"""隔离项目数据，检查启动脚本、中文 TUI 和 UTF-8 任务往返。"""
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
BINARY = ROOT / '.local/bin/tuxedo'
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
    shutil.copy2(ROOT / 'start.sh', project / 'start.sh')
    launcher = project / 'start.sh'
    editor = project / '编辑器.sh'
    editor.write_text("#!/bin/sh\nprintf 'EDITOR_SENTINEL\\n'\n")
    editor.chmod(0o755)
    env = dict(os.environ, TERM='xterm-256color', LANG='C.UTF-8',
               VISUAL=str(editor), EDITOR=str(project / '不存在的编辑器'))
    # 模拟环境中已有其他待办路径；启动脚本不能误写它们。
    env.update(TODO_FILE=str(Path(temp) / '外部.txt'), DONE_FILE=str(Path(temp) / '外部归档.txt'))

    def run(*args, expected=0):
        result = subprocess.run([str(launcher), *args], cwd=temp, env=env,
                                text=True, capture_output=True, timeout=10)
        assert result.returncode == expected, result.stderr
        return result.stdout + result.stderr

    assert '尚未安装' in run('--help', expected=1)
    shutil.copy2(BINARY, project / '.local/bin/tuxedo')
    assert '中文定制版' in run('--version')
    assert '用法：' in run('--help')
    assert '中文定制版' in run('update')
    assert '需要终端' in run(expected=1)
    assert '示例模式' in run('--sample', expected=2)
    wrong_file = Path(temp) / '误传.txt'
    assert '未知命令' in run(str(wrong_file), expected=2)
    assert not wrong_file.exists()

    raw = '(A) 检查中文保存 +工作 @电脑 due:2099-12-31'
    run('add', raw)
    listing = json.loads(run('--json', 'list'))
    assert '检查中文保存 +工作 @电脑 due:2099-12-31' in (project / 'todo.txt').read_text()
    assert '检查中文保存' in json.dumps(listing, ensure_ascii=False)
    assert not (Path(temp) / '外部.txt').exists()
    assert not (Path(temp) / '外部归档.txt').exists()

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

    def screen(keys=b'', *, raw=False):
        if keys:
            os.write(master, keys)
        output = bytearray()
        deadline = time.monotonic() + 1.5
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
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
        focused = screen(b'\x1b[O\x1b[I', raw=True)
        assert '\x1b[2J' in focused and '优先级' in plain(focused), '恢复焦点应清屏并完整重绘'
        # 同尺寸 SIGWINCH 不会触发 Ratatui autoresize，也必须重绘。
        os.kill(process.pid, signal.SIGWINCH)
        assert '\x1b[2J' in screen(raw=True), '同尺寸窗口恢复应重绘'
        fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack('HHHH', 36, 140, 0, 0))
        os.kill(process.pid, signal.SIGWINCH)
        assert '优先级' in screen(), '窗口缩放后应重绘'
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
        assert '新建任务' in screen(b'n')
        screen('交互录入中文'.encode())
        refreshed = screen(b'\x0c', raw=True)
        assert '\x1b[2J' in refreshed and '交互录入中文' in plain(refreshed), 'Ctrl+L 应重绘并保留未保存输入'
        refocused = screen(b'\x1b[O\x1b[I', raw=True)
        assert '\x1b[2J' in refocused and '交互录入中文' in plain(refocused), '恢复焦点不能丢失未保存输入'
        assert '已添加' in screen(b'\r')
        exited = screen(b'q', raw=True)
        assert '\x1b[?1004l' in exited, '退出时应关闭终端焦点通知'
        assert process.wait(timeout=5) == 0
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=5)
        os.close(master)

    assert '交互录入中文' in (project / 'todo.txt').read_text()
    run('done', '1')
    run('archive')
    assert '检查中文保存' in (project / 'done.txt').read_text()
    assert '检查中文保存' not in (project / 'todo.txt').read_text()
    assert '交互录入中文' in (project / 'todo.txt').read_text()

print('通过：路径隔离、中文交互/归档、焦点恢复/缩放/编辑器返回重绘、Ctrl+L 保留输入。')
