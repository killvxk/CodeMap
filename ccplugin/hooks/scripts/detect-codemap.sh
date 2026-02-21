#!/usr/bin/env bash
# detect-codemap.sh — SessionStart hook
# 检测当前工作目录是否有 .codemap/ 图谱，并检测可用的 codegraph 二进制
# 兼容 Linux / macOS / Windows Git Bash

# ── 检测 codegraph 二进制 ─────────────────────────────────────────────────────

CODEGRAPH_BIN=""

# 1. 优先使用 PATH 中的 codegraph
if command -v codegraph >/dev/null 2>&1; then
  CODEGRAPH_BIN="codegraph"
fi

# 2. 检查插件目录下的预编译二进制（ccplugin/bin/codegraph-*）
if [ -z "$CODEGRAPH_BIN" ] && [ -n "$CLAUDE_PLUGIN_ROOT" ]; then
  # 按平台选择二进制名称
  case "$(uname -s 2>/dev/null)" in
    Linux*)   _PLATFORM="linux" ;;
    Darwin*)  _PLATFORM="macos" ;;
    MINGW*|MSYS*|CYGWIN*) _PLATFORM="windows" ;;
    *)        _PLATFORM="" ;;
  esac

  if [ -n "$_PLATFORM" ]; then
    _BIN_PATH="$CLAUDE_PLUGIN_ROOT/bin/codegraph-$_PLATFORM"
    # Windows 下尝试 .exe 后缀
    if [ "$_PLATFORM" = "windows" ] && [ -f "${_BIN_PATH}.exe" ]; then
      CODEGRAPH_BIN="${_BIN_PATH}.exe"
    elif [ -f "$_BIN_PATH" ]; then
      CODEGRAPH_BIN="$_BIN_PATH"
    fi
  fi
fi

# 3. 检查 rust-cli 本地构建产物（开发环境）
if [ -z "$CODEGRAPH_BIN" ]; then
  for _CANDIDATE in \
    "./rust-cli/target/release/codegraph" \
    "./rust-cli/target/release/codegraph.exe" \
    "./rust-cli/target/debug/codegraph" \
    "./rust-cli/target/debug/codegraph.exe"
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
  echo "[CodeMap] 未找到 codegraph 二进制。请从 https://github.com/killvxk/CodeMap/releases 下载，或执行 cd rust-cli && cargo build --release 构建。"
fi
