#!/usr/bin/env bash
# detect-codemap.sh — SessionStart hook
# 检测 codegraph 二进制和 .codemap/ 图谱状态
# 查找优先级: PATH > ~/.codemap/bin/ > 插件目录 > 开发构建

# ── 平台检测 ──────────────────────────────────────────────────────────────────

case "$(uname -s 2>/dev/null)" in
  Linux*)   _OS="linux" ;;
  Darwin*)  _OS="macos" ;;
  MINGW*|MSYS*|CYGWIN*) _OS="windows" ;;
  *)        _OS="" ;;
esac

case "$(uname -m 2>/dev/null)" in
  x86_64|amd64)  _ARCH="x86_64" ;;
  aarch64|arm64) _ARCH="aarch64" ;;
  *)             _ARCH="" ;;
esac

_BIN_NAME=""
if [ -n "$_OS" ] && [ -n "$_ARCH" ]; then
  _BIN_NAME="codegraph-${_ARCH}-${_OS}"
  [ "$_OS" = "windows" ] && _BIN_NAME="${_BIN_NAME}.exe"
fi

CODEMAP_HOME="${CODEMAP_HOME:-$HOME/.codemap}"
CODEMAP_BIN_DIR="${CODEMAP_HOME}/bin"

# ── 多级查找 codegraph 二进制 ─────────────────────────────────────────────────

CODEGRAPH_BIN=""

# 1. PATH 中的 codegraph（用户全局安装）
if command -v codegraph >/dev/null 2>&1; then
  CODEGRAPH_BIN="codegraph"
fi

# 1b. PATH 中的 arch-specific 名称
if [ -z "$CODEGRAPH_BIN" ] && [ -n "$_BIN_NAME" ]; then
  if command -v "$_BIN_NAME" >/dev/null 2>&1; then
    CODEGRAPH_BIN="$(command -v "$_BIN_NAME")"
  fi
fi

# 2. ~/.codemap/bin/（用户级专用目录）
if [ -z "$CODEGRAPH_BIN" ] && [ -n "$_BIN_NAME" ] && [ -f "${CODEMAP_BIN_DIR}/${_BIN_NAME}" ]; then
  CODEGRAPH_BIN="${CODEMAP_BIN_DIR}/${_BIN_NAME}"
fi

# 3. 插件目录（向后兼容）
if [ -z "$CODEGRAPH_BIN" ] && [ -n "$CLAUDE_PLUGIN_ROOT" ] && [ -n "$_BIN_NAME" ]; then
  if [ -f "$CLAUDE_PLUGIN_ROOT/bin/${_BIN_NAME}" ]; then
    CODEGRAPH_BIN="$CLAUDE_PLUGIN_ROOT/bin/${_BIN_NAME}"
  fi
fi

# 4. 开发构建 (rust-cli/target/)
if [ -z "$CODEGRAPH_BIN" ]; then
  _DEV_NAME="codegraph"
  [ "$_OS" = "windows" ] && _DEV_NAME="codegraph.exe"
  for _CANDIDATE in \
    "./rust-cli/target/release/${_DEV_NAME}" \
    "./rust-cli/target/release/${_DEV_NAME}" \
    "./rust-cli/target/debug/${_DEV_NAME}" \
    "./rust-cli/target/debug/${_DEV_NAME}"
  do
    if [ -f "$_CANDIDATE" ]; then
      CODEGRAPH_BIN="$_CANDIDATE"
      break
    fi
  done
fi

# ── 检测 .codemap/ 图谱 ───────────────────────────────────────────────────────

if [ -f ".codemap/graph.json" ]; then
  echo "[CodeMap] 检测到 .codemap/ 图谱已存在。建议使用 /codemap:codemap-load 加载项目上下文，或 /codemap:codemap-update 更新图谱。"
else
  echo "[CodeMap] 未检测到 .codemap/ 图谱。如需生成代码图谱，请使用 /codemap:codemap-scan。"
fi

# ── 输出二进制状态 ────────────────────────────────────────────────────────────

if [ -n "$CODEGRAPH_BIN" ]; then
  echo "[CodeMap] codegraph 引擎：$CODEGRAPH_BIN"
else
  echo "[CodeMap] 未找到 codegraph 二进制。首次执行命令时将自动从 GitHub Releases 下载，"
  echo "[CodeMap] 或手动放置到: ~/.codemap/bin/${_BIN_NAME}"
fi
