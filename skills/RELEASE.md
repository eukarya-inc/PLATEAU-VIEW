# リリースフロー（内部向け）

このディレクトリ（`skills/`）は [Project-PLATEAU/skills](https://github.com/Project-PLATEAU/skills) に配布される。

## 仕組み

```
eukarya-inc/PLATEAU-VIEW (main)
  └ skills/                              ← 開発はここ
     ↓ build-skills-dist.yml が自動整形
eukarya-inc/PLATEAU-VIEW (skills-dist)   ← agentskills.io レイアウト
  ↓ 手動 push
Project-PLATEAU/skills (main)            ← gh skill install の対象
```

`skills/` 配下の変更が `main` に入ると `build-skills-dist` ワークフローが発火し、`skills-dist` ブランチに新しいコミットを **積み重ねる**。`skills-dist` には `README.md` がルート、各スキルが `skills/<name>/SKILL.md` にある agentskills.io 規約の配置で入る。履歴を残すので、配布リポジトリ側で `git log` を見れば各リリースのソース SHA が辿れる。

## 公開手順

```bash
git fetch origin skills-dist
git push git@github.com:Project-PLATEAU/skills.git \
  refs/remotes/origin/skills-dist:refs/heads/main
```

`refs/remotes/...:refs/heads/...` のように両側を完全修飾する必要がある（`origin/skills-dist:main` のような短縮形は git に拒否される）。通常は fast-forward push で済む。両ブランチの履歴が乖離してしまった場合は `--force` を付けて整合させる。

## 新しいスキルを追加するとき

ローカルの Claude Code でも自動的に有効になるよう、`.claude/skills/<name>` から `../../skills/<name>` への相対シンボリックリンクも作って一緒にコミットする。

```bash
ln -s ../../skills/<name> .claude/skills/<name>
```

## バリデーション

PR 時に `ci-skills.yml` が `gh skill publish --dry-run` で agentskills.io 仕様への準拠をチェックする。

## このファイルについて

`RELEASE.md` は配布対象外（`build-skills-dist.yml` の rsync で除外される）。開発リポジトリ内部のみのドキュメント。
