#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
BIN="$ROOT/.local/bin/tuxedo"
if [[ ! -x "$BIN" ]]; then
    printf '尚未安装本项目的中文 Tuxedo。请先运行：%s/install.sh\n' "$ROOT" >&2
    exit 1
fi

export TODO_DIR="$ROOT" TODO_FILE="$ROOT/todo.txt" DONE_FILE="$ROOT/done.txt"
export XDG_CONFIG_HOME="$ROOT/.config" NOTES_DIR="$ROOT/notes" TUXEDO_NO_UPDATE_CHECK=1
mkdir -p "$XDG_CONFIG_HOME/tuxedo"
chmod 700 "$XDG_CONFIG_HOME"

# 本项目直接维护源码，不使用官方更新指引。
if [[ ${1:-} == update ]]; then
    printf '本项目使用中文定制版。修改源码后重新编译请运行：%s/install.sh\n' "$ROOT"
    exit 0
fi
if [[ ${1:-} == --sample ]]; then
    printf '本项目不提供示例模式，避免混入示例数据。\n' >&2
    exit 2
fi
if (( $# == 0 )); then
    if [[ ! -t 0 || ! -t 1 ]]; then
        printf '交互界面需要终端。脚本调用可使用 ./start.sh list 或 ./start.sh --help。\n' >&2
        exit 1
    fi
    exec "$BIN" "$TODO_FILE"
fi
case "$1" in
    --help|-h|--version|-V|--json|--force|-f|add|a|append|app|prepend|prep|replace|pri|p|depri|dp|done|do|complete|del|rm|archive|list|ls|listall|lsa|listpri|lsp|listproj|lsprj|listcon|lsc) ;;
    *) printf '未知命令：%s。此脚本不接受其他文件路径，请使用 --help 查看命令。\n' "$1" >&2; exit 2 ;;
esac
exec "$BIN" "$@"
