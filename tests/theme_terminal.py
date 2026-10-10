#!/usr/bin/env python3
"""用隔离 PTY 模拟终端响应；不读取或修改真实待办配置。"""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / '.local/bin/todot'
CONFIG = 'theme = Muted Slate\ntheme_mode = auto\ntheme_dark = Nord\ntheme_light = Dawn\n'
DARK = b'\x1b[?997;1n'
LIGHT = b'\x1b[?997;2n'
NORD = b'48;2;46;52;64m'
DAWN = b'48;2;250;246;240m'
MUTED = b'48;2;26;29;35m'
MATRIX = b'48;2;10;18;10m'


def plain(data):
    return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', data.decode('utf-8', errors='replace'))


class Terminal:
    def __init__(self, directory, *, config=CONFIG, scheme=DARK, background=None):
        self.directory = Path(directory)
        self.directory.mkdir()
        self.config = self.directory / 'config/todot/config.toml'
        self.config.parent.mkdir(parents=True)
        self.config.write_text(config)
        self.todo = self.directory / 'todo.txt'
        self.todo.write_text('测试主题\n')
        self.scheme, self.background = scheme, background
        self.queries = []
        self.query_buffer = b''
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 160, 0, 0))
        env = dict(os.environ, HOME=str(self.directory), XDG_CONFIG_HOME=str(self.directory / 'config'),
                   NOTES_DIR=str(self.directory / 'notes'), TERM='xterm-256color', LANG='C.UTF-8')
        for key in ['TODO_DIR', 'TODO_FILE', 'DONE_FILE', 'COLORFGBG']:
            env.pop(key, None)

        def controlling_terminal():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        self.process = subprocess.Popen([str(BINARY), str(self.todo)], stdin=slave, stdout=slave,
                                        stderr=slave, env=env, preexec_fn=controlling_terminal)
        os.close(slave)

    def screen(self, keys=b'', duration=0.8):
        if keys:
            os.write(self.master, keys)
        output = bytearray()
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if not select.select([self.master], [], [], min(0.05, max(0, deadline - time.monotonic())))[0]:
                continue
            chunk = os.read(self.master, 65536)
            output.extend(chunk)
            self.query_buffer += chunk
            while True:
                matches = [(self.query_buffer.find(q), q, response) for q, response in [
                    (b'\x1b[?996n', self.scheme), (b'\x1b]11;?\x1b\\', self.background)]
                    if q in self.query_buffer]
                if not matches:
                    self.query_buffer = self.query_buffer[-8:]
                    break
                index, query, response = min(matches)
                self.queries.append(query)
                self.query_buffer = self.query_buffer[index + len(query):]
                if response:
                    # 跨 read 的碎片响应必须与用户按键共用解析器。
                    split = min(5, len(response) - 1)
                    os.write(self.master, response[:split])
                    time.sleep(0.01)
                    os.write(self.master, response[split:])
        assert self.process.poll() is None, 'TUI 意外退出'
        return bytes(output)

    def close(self):
        self.process.terminate()
        self.process.wait(timeout=5)
        os.close(self.master)


with tempfile.TemporaryDirectory(prefix='theme-check-', dir=ROOT / '.local') as temp:
    sessions = []

    def terminal(name, **kwargs):
        session = Terminal(Path(temp) / name, **kwargs)
        sessions.append(session)
        return session

    try:
        legacy = terminal('legacy', config='theme = Dawn\n')
        assert DAWN in legacy.screen()
        assert not legacy.queries, '旧固定配置不应自动查询终端'
        assert 'Ctrl+L' in plain(legacy.screen(b'?'))
        legacy.screen(b'\x1b')
        assert DAWN in legacy.screen(b'\x0c'), '固定模式重测不能改变主题'

        auto = terminal('semantic')
        assert NORD in auto.screen()
        assert auto.queries == [b'\x1b[?996n'], '明确模式成功时不应查询背景'
        initial_config = auto.config.read_bytes()
        title, body = '主题刷新保留标题', '正文第一行\n正文第二行'
        auto.screen(b'n' + title.encode())
        auto.screen(b'\t\x1b[200~' + body.encode() + b'\x1b[201~')
        auto.scheme = LIGHT
        updated = auto.screen(b'\x0c')
        assert DAWN in updated and '\x1b[2J' in updated.decode(), 'Ctrl+L 应重测并完整重绘'
        assert title in plain(updated) and '正文第一行' in plain(updated), '重测不能丢输入'
        assert auto.config.read_bytes() == initial_config, '检测不能写配置'
        auto.screen(b'\x13')
        raw_task = auto.todo.read_text()
        assert title in raw_task
        note = re.search(r'note:([^\s]+)', raw_task).group(1)
        assert (auto.directory / 'notes' / note).read_text() == body

        auto.screen(b'Tj')  # 在浅色槽位预览 Nord。
        auto.scheme = DARK
        preview = auto.screen(b'\x0c')
        assert NORD in preview and '浅色：Nord' in plain(preview), '检测不得覆盖槽位预览'
        restored = auto.screen(b'\x1b')
        assert NORD in restored, '取消后应用最新深色检测结果'
        assert auto.config.read_bytes() == initial_config, '取消不得保存预览'

        settings = auto.screen(b',T')
        assert '固定／回退' in plain(settings), '设置页 T 应打开同一主题设置'
        auto.screen(b'j\tjjm\r')  # 深色 Nord→Matrix；浅色 Dawn→Matrix；切固定并保存。
        saved = auto.config.read_text()
        assert 'theme = Muted Slate' in saved
        assert 'theme_dark = Matrix' in saved and 'theme_light = Matrix' in saved
        assert 'theme_mode = fixed' in saved and '设置' in plain(auto.screen(b'\x0c'))
        auto.screen(b'Tm\r')  # 再次启用跟随模式。
        assert 'theme_mode = auto' in auto.config.read_text()
        assert MATRIX in auto.screen(b'\x0c')
        auto.screen(b'D')  # 其他设置保存不能覆写双主题或固定回退主题。
        saved = auto.config.read_text()
        assert 'theme = Muted Slate' in saved and 'theme_dark = Matrix' in saved and 'theme_light = Matrix' in saved
        auto.config.write_text(CONFIG)
        assert NORD in auto.screen(), '配置热加载应保留已检测的深色模式'

        for index, (rgb, ending) in enumerate([
            (b'f/f/f', b'\x07'), (b'ff/ff/ff', b'\x1b\\'),
            (b'fff/fff/fff', b'\x07'), (b'ffff/ffff/ffff', b'\x1b\\'),
        ]):
            fallback = terminal(f'background-{index}', scheme=None, background=b'\x1b]11;rgb:' + rgb + ending)
            assert DAWN in fallback.screen(), f'OSC 11 应支持 {rgb!r} 与 BEL/ST'
            assert fallback.queries == [b'\x1b[?996n', b'\x1b]11;?\x1b\\']
            late = fallback.screen(DARK)
            assert NORD in late, '延迟的明确模式响应优先于背景推断'

        failed_save = terminal('failed-save')
        failed_save.screen()
        original = failed_save.config.read_text()
        failed_save.screen(b'Tmj')
        failed_save.config.unlink()
        failed_save.config.mkdir()  # 配置目标变成目录，原子重命名必须失败。
        error = failed_save.screen(b'\r')
        assert '保存配置失败' in plain(error), '失败时不能显示保存成功'
        assert '固定／回退' in plain(failed_save.screen(b'\x0c')), '失败后保留主题编辑以便重试'
        failed_save.config.rmdir()
        failed_save.config.write_text(original)
        failed_save.screen(b'\r')
        assert 'theme_mode = fixed' in failed_save.config.read_text()
        assert 'theme_dark = Matrix' in failed_save.config.read_text()

        unsupported = terminal('unsupported', scheme=None)
        assert MUTED in unsupported.screen()
        failure = unsupported.screen(b'\x0cnNo response')
        assert '无法检测' in plain(failure), '检测失败应可见'
        unsupported.screen(b'\r')
        assert 'No response' in unsupported.todo.read_text(), '检测超时不能阻塞或吞输入'

        malformed = terminal('malformed', scheme=b'\x1b[?997;3n', background=b'\x1b]11;rgb:invalid\x07')
        malformed.screen()
        malformed.screen(b'\x1b[?998;1nnAfter invalid report\r')
        assert 'After invalid report' in malformed.todo.read_text(), '未知／非法响应不能吞后续按键'
    finally:
        for session in sessions:
            session.close()

print('通过：固定/双主题配置、启动与 Ctrl+L 检测、OSC 11 回退、取消/保存/热加载、碎片/延迟/非法响应、未保存输入保留。')
