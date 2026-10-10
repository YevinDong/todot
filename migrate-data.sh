#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
DATA=${TODO_TUI_HOME:-${HOME:?HOME 未设置}/.todo-tui}
if [[ $DATA != /* ]]; then printf 'TODO_TUI_HOME 必须为绝对路径。\n' >&2; exit 2; fi
if [[ -e $DATA || -L $DATA ]]; then
    printf '目标已存在，未覆盖任何数据：%s\n' "$DATA" >&2
    exit 1
fi
for exe in /proc/[0-9]*/exe; do
    running=$(readlink -- "$exe" 2>/dev/null || true)
    if [[ $running == "$ROOT/.local/bin/tuxedo" || $running == "$ROOT/.local/bin/tuxedo (deleted)" ]]; then
        printf '旧程序仍在运行，请先正常退出，再迁移数据。\n' >&2
        exit 1
    fi
done
printf '复制后保留项目中的原始数据。\n'
umask 077
mkdir -p -- "$(dirname -- "$DATA")"
STAGING=$(mktemp -d "$(dirname -- "$DATA")/.todo-tui-migrate.XXXXXX")
trap 'rm -rf -- "$STAGING"' EXIT
for name in todo.txt done.txt inbox.txt inbox.txt.tuxedo-staging notes .config; do
    [[ -e $ROOT/$name ]] || continue
    target=$name
    [[ $name != .config ]] || target=config
    cp -a -- "$ROOT/$name" "$STAGING/$target"
    diff -qr -- "$ROOT/$name" "$STAGING/$target" >/dev/null
    printf '已验证：%s\n' "$name"
done
chmod -R go-rwx "$STAGING"
chmod 700 "$STAGING"
# -T treats DATA as the destination itself; -n never overwrites a concurrent creator.
mv -Tn -- "$STAGING" "$DATA"
if [[ -d $STAGING ]]; then printf '目标被其他进程创建，迁移已取消。\n' >&2; exit 1; fi
trap - EXIT
printf '迁移完成：%s；旧文件保留，不再由新版读取。\n' "$DATA"
