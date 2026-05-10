[English](../external-data.md) | [日本語](external-data.md)

# 外部データソース

オセロ研究で使える公開棋譜・データセットと，それを `rs-othello-sim` に取り込む方法をまとめます．各形式の on-disk スキーマは [棋譜フォーマット](record-formats.md) を，注釈・評価に使う実機エンジンは [外部エンジン](external-engines.md) を参照してください．

## クイックリファレンス

| ソース | 形式 | 規模 | ライセンス | 取り込み |
|---|---|---|---|---|
| WTHOR ( フランスオセロ連盟 FFO) | `.wtb` バイナリ | 1977 年〜現在のマスター対局，年次更新 | 研究 / 非商用利用可．FFO への出典明記必須 | `othello-cli fetch wthor` ＋ `convert --input-format wthor` |
| GGS / GGF アーカイブ | `.ggf` テキスト | コンピュータ・オンライン対局の長年アーカイブ | アーカイブごとに条件確認 | `othello-cli convert --input-format ggf` |
| オンライン対戦サイト ( eOthello / Othello Quest / GGS) | サイトごと ( GGF か JSON が多い) | アカウント単位・大会単位 | サイトごとの利用規約 | GGF / JSON に変換後 |
| 自己生成 ( `selfplay`) | JSON + JSONL | 計算資源次第 | MIT ( 本リポジトリ) | ネイティブ |

## WTHOR — フランスオセロ連盟

- 配布: <https://www.ffothello.org/informatique/la-base-wthor/>
- 年次アーカイブは `wth_YYYY.wtb` ( 場合により `.zip` 内) で配布されます．補助テーブル `JOUEUR.JOU` ( 棋士 ID → 名前) ， `TOURNOI.TOU` ( 大会 ID → 名前) も同梱されます．
- フォーマット詳細: [`docs/record-formats.md` の WTHOR 節](record-formats.md) ．ヘッダ 16 byte ＋ 各局 68 byte ( メタ 8 + 手順 60) ．
- WTHOR は終局石数 ( `real_score`) と手の連続バイト列のみを持ちます．Pass は **暗黙** で記録されない ( 合法手が無いプレイヤーの手は単純にスキップされる) ため， `othello-io::wthor` の Reader は `Board::standard_8x8()` 上で再生し，合法手が無い手番に到達すると `Move::Pass` を挿入してから次のバイトに進みます．
- ライセンス: 研究・非商用利用は自由．再配布や学術利用には Fédération Française d'Othello のクレジットが必要．

```bash
# 0. 同梱の fetcher を使う ( 推奨．自動で展開・既存スキップ)
othello-cli fetch wthor --year 2023 --dest data/wthor/

# 範囲指定で複数年を一度に取得することも可能 ( 両端含む)
othello-cli fetch wthor --years 2020..2023 --dest data/wthor/

# 1. 取得 ( ブラウザ or curl)
curl -L -o wth_2023.zip https://www.ffothello.org/wthor/wth_2023.zip
unzip wth_2023.zip

# 2. ファイル全体の集計 ( 局数・勝率・平均手数・年)
./target/release/othello-cli inspect --file wth_2023.wtb --format wthor

# 3. 局単位の JSON に変換 ( data/wthor_2023/ に game_00001.json など)
./target/release/othello-cli convert \
  --input wth_2023.wtb --input-format wthor \
  --output-format json --output-dir data/wthor_2023/

# 4. 1 局を自動再生で確認
./target/release/othello-cli replay \
  --file data/wthor_2023/game_00001.json --format json \
  --auto --auto-delay 200
```

## GGF — Generic Game Format

- 仕様とアーカイブ索引: <http://www.skatgame.net/mburo/ggsa/ggf>
- GGF は GGS ( Generic Game Server) や複数のエンジンの自己テストで使われるテキスト形式 ( ノード木)．本リポジトリは Othello サブセット ( `GM` / `PB` / `PW` / `RE` / `BO` / `B[…]` / `W[…]` の各タグ) をサポートします．思考時間 ( `B[D3//1.234]`) はパース可能ですが書き出し時には省略されます．
- コミュニティアーカイブの多くは複数局を連結した `.ggf` テキストファイルです． `othello-cli convert` と `inspect` は単一・複数局の両方を扱えます．

```bash
# 複数局を含む GGF アーカイブを 1 局 1 ファイルの JSON に変換
./target/release/othello-cli convert \
  --input archive.ggf --input-format ggf \
  --output-format json --output-dir data/ggf_archive/
```

## オンライン対戦サイト

オンラインでオセロを指すサイトのいくつかは棋譜エクスポートに対応していますが， **大量取得はほとんどの場合規約違反** です．アクセス前に最新の利用規約を確認してください．

| サイト | URL | 一般的なエクスポート手段 |
|---|---|---|
| eOthello | <https://www.eothello.com/> | プロフィール経由で 1 局ごとに GGF / JSON |
| Othello Quest ( WOC) | <https://www.othelloquest.com/> | アカウント単位のエクスポート．コミュニティツールあり |
| GGS ( レガシー) | 上記 GGF 索引にミラー | 複数局を束ねた GGF アーカイブ |

入手したデータはまず GGF または JSON に変換し，以降は通常の `convert` / `replay` / `inspect` パイプラインを通します．

## 合成・研究用データセット

Othello-AI 系の論文では大規模データセットを公開しているもの・再生成手順を示しているものがあります:

- **Othello-GPT** ( Li et al., 2022, [arXiv:2210.13382](https://arxiv.org/abs/2210.13382)) は 20 M 局の合成 60 ply 一様ランダム合法対戦で学習しています．本リポジトリで同等のデータを再生成できます:

  ```bash
  ./target/release/othello-cli selfplay \
    --board-size 8 --num-games 20000 \
    --black "random:seed=1" --white "random:seed=2" \
    --threads 8 --log-dir auto --save-records json
  ```

  集約された `runs/selfplay_*/all.jsonl` は `tools/tb_converter` の入力として便利です ( [tools-visualize.md](tools-visualize.md) 参照) ．

- **OLIVAW** ( Norelli & Panconesi, 2021, [arXiv:2103.17228](https://arxiv.org/abs/2103.17228)) は self-play のみで AlphaZero 風学習を行います．データセット自体は非公開ですが，レシピは [`nn:safetensors:`](nn-evaluator.md) 対戦相手と [Replay buffer](replay-buffer.md) を組み合わせた `selfplay` にそのまま対応します．

- **Edax / Egaroucid 注釈付与** ( [external-engines.md](external-engines.md) 参照) — 上記コーパスにエンジン評価値を付与すれば，教師あり学習用の注釈付き transition が得られます．

## ゼロから自前コーパスを作る

完全に分布をコントロールしたい研究では，ローカル生成が最も再現性が高いです:

```bash
# MCTS(200) vs Random を 1000 局，色入替あり，per-game JSON + 単一 JSONL を出力
./target/release/othello-cli selfplay \
  --board-size 8 --num-games 1000 --threads 8 \
  --black "mcts:200,seed=42" --white "random:seed=99" \
  --swap-colors --log-dir auto --save-records json

# 集計と TensorBoard 出力
uv run analyze-stats --input runs/selfplay_*/ --output runs/stats.csv
uv run jsonl-to-tb   --input runs/selfplay_*/all.jsonl --output runs/tb/
```

## 大規模コーパスの扱い

- WTHOR の per-game JSON は 1 局あたり 1 KB 程度です．1 年分の全棋譜でも 1 GB に収まります．
- ramdisk や `--threads N` は CPU 律速のときのみ意味があります．純粋な I/O 変換は `--threads 1` で十分なことが多いです．
- Replay buffer パイプラインでは JSON を介さず `transitions_from_record_both_sides` を直接使う方が高速です ( [replay-buffer.md](replay-buffer.md) 参照) ．
- 再現性のため， **オリジナルのアーカイブ** ( `.wtb` / `.ggf`) は必ず保存してください．変換後の JSON は派生物です．

## ライセンス・引用チェックリスト

- WTHOR — **Fédération Française d'Othello** をクレジットし，アーカイブの年を併記．
- GGS / GGF アーカイブ — Michael Buro の GGS ページとアーカイブ個別の README を引用．
- 合成データセット — 出典論文 ( `Li et al. 2022` ， `Norelli & Panconesi 2021`) を引用．
- 自己生成データ — 実験ノートに `selfplay` の正確な引数 ( seed・バージョン) を記録．

## 関連ドキュメント

- [棋譜フォーマット](record-formats.md) — JSON / GGF / WTHOR / JSONL の on-disk スキーマ．
- [CLI 利用ガイド](cli-usage.md) — `convert` / `inspect` / `selfplay` / `replay`．
- [外部エンジン](external-engines.md) — Edax / Egaroucid を使った評価・注釈．
- [Replay buffer](replay-buffer.md) — 棋譜を RL transition に変換する手順．
