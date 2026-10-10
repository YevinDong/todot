#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
HOME=${HOME:?HOME 未设置}
SOURCE=${1:-$HOME/.todo-tui}
DATA=${2:-$HOME/.todot}
if (( $# > 2 )); then printf '用法：%s [旧目录] [新目录]\n' "$0" >&2; exit 2; fi
if [[ $# == 0 && ! -d $SOURCE ]]; then SOURCE=$ROOT; fi
if [[ ! -d $SOURCE || $DATA != /* ]]; then
    printf '旧目录必须存在，新目录必须为绝对路径。\n' >&2
    exit 2
fi
if [[ -e $DATA || -L $DATA ]]; then
    printf '目标已存在，未覆盖任何数据：%s\n' "$DATA" >&2
    exit 1
fi
for exe in /proc/[0-9]*/exe; do
    running=$(readlink -- "$exe" 2>/dev/null || true)
    case "$running" in
        */tuxedo|*/tuxedo\ \(deleted\)|*/todot|*/todot\ \(deleted\))
            printf '程序仍在运行，请先正常退出，再迁移数据。\n' >&2; exit 1 ;;
    esac
done
umask 077
mkdir -p -- "$(dirname -- "$DATA")"
# mkdir fails if another process creates the target; never merge or overwrite.
mkdir -- "$DATA"
for name in todo.txt done.txt inbox.txt inbox.txt.tuxedo-staging inbox.txt.todot-staging notes config .config; do
    [[ -e $SOURCE/$name ]] || continue
    target=$name
    [[ $name != .config ]] || target=config
    [[ $name != inbox.txt.tuxedo-staging ]] || target=inbox.txt.todot-staging
    if [[ -e $DATA/$target || -L $DATA/$target ]]; then
        printf '迁移目标冲突，已复制文件保留，请检查：%s\n' "$DATA/$target" >&2
        exit 1
    fi
    cp -a -- "$SOURCE/$name" "$DATA/$target"
    diff -qr -- "$SOURCE/$name" "$DATA/$target" >/dev/null
    printf '已复制并验证：%s\n' "$name"
done
if [[ -d $DATA/config/tuxedo && ! -e $DATA/config/todot && ! -L $DATA/config/todot ]]; then
    mv -- "$DATA/config/tuxedo" "$DATA/config/todot"
fi
chmod 700 "$DATA"
printf '迁移完成：%s；旧目录保留。\n' "$DATA"
