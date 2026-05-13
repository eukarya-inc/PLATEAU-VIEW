# PLATEAU Agent Skills

PLATEAU 配信サービスを AI コーディングエージェントから利用するための [Agent Skills](https://agentskills.io/specification)。

詳しくは <https://docs.plateauview.mlit.go.jp/skills/overview/> を参照。

## 収録スキル

| スキル | 用途 |
|---|---|
| [`plateau-api`](./plateau-api/) | PLATEAU 配信サービスの REST / GraphQL API を使ったデータ取得 |

## インストール

### Claude Code / Cursor / Codex CLI / Gemini CLI など（`gh skill`）

```bash
# 特定のスキルだけ（高速）
gh skill install Project-PLATEAU/skills plateau-api --agent claude-code --scope user

# インタラクティブに選ぶ
gh skill install Project-PLATEAU/skills
```

対応エージェント: Claude Code, GitHub Copilot, Cursor, Codex, Gemini CLI, Antigravity, Amp, Goose, Junie, OpenCode, Windsurf など。

### 手動インストール

```bash
git clone https://github.com/Project-PLATEAU/skills.git
cp -r skills/plateau-api ~/.claude/skills/
```

## ライセンス

MIT
