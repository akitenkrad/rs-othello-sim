#!/usr/bin/env bash
# 簡易 GTP モックエンジン．
# 入力に対して固定的に合法手 D3 を返す．Othello のルールは検証しないモック．
# rs-othello-sim の ExternalEnginePlayer の通信レベルを確認するために使用する．

set -e

while IFS= read -r line; do
    # 先頭/末尾の空白除去 ( bash の trim 風)．
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"

    case "$line" in
        boardsize*|clear_board*|play*)
            printf "= \n\n"
            ;;
        genmove*)
            printf "= D3\n\n"
            ;;
        quit*)
            printf "= \n\n"
            exit 0
            ;;
        '')
            # 空行は無視
            ;;
        *)
            printf "? unknown command\n\n"
            ;;
    esac
done
