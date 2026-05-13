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

`skills/` 配下の変更が `main` に入ると `build-skills-dist` ワークフローが発火し、`skills-dist` ブランチを再生成する。`skills-dist` には `README.md` がルート、各スキルが `skills/<name>/SKILL.md` にある agentskills.io 規約の配置で入る。

## 公開手順

```bash
git fetch origin skills-dist
git push git@github.com:Project-PLATEAU/skills.git origin/skills-dist:main
```

force-push になるが、配布リポジトリはミラー扱いなので問題ない。

## バリデーション

PR 時に `ci-skills.yml` が `gh skill publish --dry-run` で agentskills.io 仕様への準拠をチェックする。

## このファイルについて

`RELEASE.md` は配布対象外（`build-skills-dist.yml` の rsync で除外される）。開発リポジトリ内部のみのドキュメント。
