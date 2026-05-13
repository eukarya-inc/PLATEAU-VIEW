---
title: PLATEAU Agent Skills
description: AI コーディングエージェント向けの Agent Skills 配布リポジトリと利用方法
---

PLATEAU Agent Skills は、AI コーディングエージェント（Claude Code、GitHub Copilot、Cursor、Codex CLI、Gemini CLI など）に PLATEAU 配信サービスの利用知識をインストールするための [Agent Skills](https://agentskills.io/) パッケージです。

:::caution[試験的な提供 - 利用上の注意]
- **動作保証・SLA はありません** - 可用性やパフォーマンスの保証は一切行いません
- **予告なく変更されることがあります** - スキルの追加・削除・変更、内容の変更などが事前告知なく行われることがあります
- **本番環境での利用は推奨しません** - 検証・評価目的での利用を想定しています
- **サポートは限定的です** - 問い合わせへの対応は保証されません

本サービスの利用により生じたいかなる損害についても、提供者は一切の責任を負いません。
:::

## Agent Skills とは

Agent Skills は、AI コーディングエージェントに特定ドメインの知識・手順を渡すための、移植可能な指示書フォーマットです。`SKILL.md` という Markdown ファイル（YAML フロントマターで `name` / `description` を持つ）と関連リソースを 1 つのフォルダにまとめた形式で、[agentskills.io](https://agentskills.io/specification) の仕様に従います。

エージェントはタスクの内容に応じて該当スキルを自動的に読み込み、その指示に従って動作します。PLATEAU 配信 API の URL 書式や、CityGML API の使い分けといった「人間が毎回説明する必要のある手順」をスキルとして配布することで、エージェントの応答精度が向上します。

## 配布リポジトリ

PLATEAU の Agent Skills は [`Project-PLATEAU/skills`](https://github.com/Project-PLATEAU/skills) で配布しています。

## 利用可能なスキル

| スキル | 用途 |
|---|---|
| `plateau-api` | PLATEAU 配信サービスの REST / GraphQL API を使ったデータ取得（3D Tiles, MVT, CityGML 等） |

今後、用語集スキルや仕様書スキル等を追加予定です。

## インストール

### GitHub CLI 経由（推奨）

[`gh skill`](https://cli.github.com/manual/gh_skill) コマンドを使うと、Claude Code・GitHub Copilot・Cursor・Codex CLI・Gemini CLI など多数のエージェントに横断的にインストールできます。

```bash
# Claude Code（ユーザースコープ）にインストール
gh skill install Project-PLATEAU/skills plateau-api --agent claude-code --scope user

# インタラクティブに対象エージェント・スキルを選ぶ
gh skill install Project-PLATEAU/skills
```

対応エージェント: GitHub Copilot, Claude Code, Cursor, Codex CLI, Gemini CLI, Antigravity, Amp, Goose, Junie, OpenCode, Windsurf など。

### Claude Code プラグイン経由

将来サポート予定。

### 手動インストール

```bash
git clone https://github.com/Project-PLATEAU/skills.git
cp -r skills/skills/plateau-api ~/.claude/skills/
```

Claude Code 以外のエージェントについては、各エージェントの skills ディレクトリ（例: `.cursor/rules/`, `.codex/skills/` 等）にコピーしてください。

## MCP Server / llms.txt との関係

PLATEAU 配信サービスは AI クライアント向けに 3 つのアクセス手段を提供しています。用途に応じて使い分けてください。

| 手段 | 特徴 | 推奨用途 |
|---|---|---|
| **Agent Skills** | エージェントに事前インストールする指示書 | コーディングエージェントから API を呼び出す際の手順を埋め込みたい |
| **[MCP Server](/mcp/overview/)** | 実行時にデータを取得する HTTP サーバー | 最新のデータカタログ・属性情報を動的に問い合わせたい |
| **[llms-full.txt](https://docs.plateauview.mlit.go.jp/llms-full.txt)** | ドキュメント全文の Markdown | エージェントに公式ドキュメント全体を読ませたい |

Agent Skills は **静的なノウハウ**、MCP Server は **動的なデータ取得**、llms.txt は **網羅的なリファレンス** という役割分担です。組み合わせて使うと相互補完的に動作します。

## ライセンス

[MIT License](https://github.com/Project-PLATEAU/skills/blob/main/LICENSE)
