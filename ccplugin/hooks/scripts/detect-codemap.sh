#!/usr/bin/env bash
# detect-codemap.sh — SessionStart hook
# 检测当前工作目录是否有 .codemap/ 图谱，输出提示信息

if [ -f ".codemap/graph.json" ]; then
  echo "[CodeMap] 检测到 .codemap/ 图谱已存在。建议使用 /codemap:codemap-load 加载项目上下文，或 /codemap:codemap-update 更新图谱。"
else
  echo "[CodeMap] 未检测到 .codemap/ 图谱。如需生成代码图谱，请使用 /codemap:codemap-scan。"
fi
