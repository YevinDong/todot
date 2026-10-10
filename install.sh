#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
cd "$ROOT"
CACHE="$ROOT/.local"
RUST_VERSION=$(awk -F '"' '$1 ~ /^channel *= */ { print $2; exit }' "$ROOT/rust-toolchain.toml")

case "${1:-}" in
    '') RUN_TESTS=0 ;;
    --test) RUN_TESTS=1 ;;
    *) printf '用法：%s [--test]\n' "$0" >&2; exit 2 ;;
esac
if (( $# > 1 )); then printf '参数过多。\n' >&2; exit 2; fi
if [[ $(uname -s) != Linux ]]; then printf '此安装脚本仅支持 Linux。\n' >&2; exit 1; fi
case $(uname -m) in
    x86_64) TARGET=x86_64-unknown-linux-gnu ;;
    aarch64) TARGET=aarch64-unknown-linux-gnu ;;
    *) printf '不支持当前 CPU 架构。\n' >&2; exit 1 ;;
esac
for cmd in curl sha256sum cc flock; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        printf '缺少构建工具：%s。Ubuntu 可运行 sudo apt install build-essential curl util-linux。\n' "$cmd" >&2
        exit 1
    fi
done
if [[ -z "$RUST_VERSION" ]]; then
    printf 'rust-toolchain.toml 缺少 channel。\n' >&2
    exit 1
fi

mkdir -p "$CACHE/bin"
exec 9>"$CACHE/install.lock"
flock 9
export CARGO_HOME="$CACHE/cargo" RUSTUP_HOME="$CACHE/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
fetch() { curl -fsSL --retry 2 --connect-timeout 15 --proto '=https' --proto-redir '=https' --tlsv1.2 "$1" -o "$2"; }

if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
    printf '准备项目内 Rust 工具链，不修改系统 PATH。\n'
    url="https://static.rust-lang.org/rustup/dist/$TARGET/rustup-init"
    fetch "$url" "$CACHE/rustup-init"
    fetch "$url.sha256" "$CACHE/rustup-init.sha256"
    read -r digest _ < "$CACHE/rustup-init.sha256"
    printf '%s  %s\n' "$digest" "$CACHE/rustup-init" | sha256sum -c -
    chmod +x "$CACHE/rustup-init"
    "$CACHE/rustup-init" -y --no-modify-path --profile minimal --default-toolchain "$RUST_VERSION"
fi
if ! "$CARGO_HOME/bin/rustup" run "$RUST_VERSION" rustc --version >/dev/null 2>&1; then
    "$CARGO_HOME/bin/rustup" toolchain install "$RUST_VERSION" --profile minimal
fi

printf '从当前仓库源码编译 todot；首次需要下载依赖。\n'
"$CARGO_HOME/bin/cargo" "+$RUST_VERSION" build --release --locked \
    --manifest-path "$ROOT/Cargo.toml" --target-dir "$CACHE/target"
if (( RUN_TESTS )); then
    env -u TODO_DIR -u TODO_FILE -u DONE_FILE -u NOTES_DIR \
        XDG_CONFIG_HOME="$CACHE/test-config" \
        "$CARGO_HOME/bin/cargo" "+$RUST_VERSION" test --locked \
        --manifest-path "$ROOT/Cargo.toml" --target-dir "$CACHE/target" -- --test-threads=1
fi
cp -- "$CACHE/target/release/todot" "$CACHE/bin/todot.new"
chmod +x "$CACHE/bin/todot.new"
mv -f -- "$CACHE/bin/todot.new" "$CACHE/bin/todot"
INSTALL_DIR=${INSTALL_DIR:-${HOME:?HOME 未设置}/.local/bin}
mkdir -p -- "$INSTALL_DIR"
install -m 755 -- "$CACHE/bin/todot" "$INSTALL_DIR/todot.new"
mv -f -- "$INSTALL_DIR/todot.new" "$INSTALL_DIR/todot"
printf '安装完成：%s/todot。直接运行 todot。\n' "$INSTALL_DIR"
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) printf '请将此行加入 shell 配置：export PATH="%s:$PATH"\n' "$INSTALL_DIR" ;;
esac
