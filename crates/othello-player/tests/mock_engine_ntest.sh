#!/usr/bin/env bash
# 簡易 ntest モックエンジン．
# `set game <board>` を受けたあと `go` で D3 を返す．他コマンドは無視．

set -e

while IFS= read -r line; do
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"
    case "$line" in
        "set game"*)
            # 局面を受け取ったが何もしない
            ;;
        "go"*)
            printf "D3\n"
            ;;
        "quit"*)
            exit 0
            ;;
        *)
            ;;
    esac
done
