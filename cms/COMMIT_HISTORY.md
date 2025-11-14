# Commit History Preservation

## Current Approach

The Re:Earth CMS code was copied into PLATEAU-VIEW-3.0 using a standard `cp` command. This approach does **not** preserve the Git commit history from the original `reearth-cms` repository.

## Why History Wasn't Preserved

1. **Simplicity**: Direct copy is straightforward and avoids complex Git operations
2. **Clean separation**: The code is being renamed and restructured for PLATEAU
3. **Different module paths**: All imports and module names were changed
4. **Go workspace integration**: The CMS is now part of a larger monorepo

## If You Need Commit History

If commit history preservation is important, you have several options:

### Option 1: Git Subtree (Recommended for ongoing sync)

```bash
cd /Users/dexter/active/PLATEAU-VIEW-3.0
git subtree add --prefix=cms https://github.com/reearth/reearth-cms.git main --squash
```

**Pros:**
- Preserves history with a single merge commit
- Easier to pull future updates from upstream
- Part of standard Git

**Cons:**
- History is in a separate branch
- Requires rebasing changes made to paths/module names

### Option 2: Git Filter-Branch (For one-time migration)

```bash
# In a separate clone of reearth-cms
git clone https://github.com/reearth/reearth-cms.git reearth-cms-filtered
cd reearth-cms-filtered

# Rewrite history to move everything under cms/
git filter-branch --tree-filter '
  mkdir -p cms
  git ls-tree --name-only $GIT_COMMIT | xargs -I {} mv {} cms/
' HEAD

# Then merge into PLATEAU repo
cd /Users/dexter/active/PLATEAU-VIEW-3.0
git remote add cms-filtered ../reearth-cms-filtered
git fetch cms-filtered
git merge cms-filtered/main --allow-unrelated-histories
```

**Pros:**
- Preserves full commit history
- All commits maintain their original timestamps and authors

**Cons:**
- Very complex and time-consuming
- Rewrites entire Git history
- Still requires updating all module paths in history
- High risk of conflicts

### Option 3: Keep Upstream Remote (Lightest)

```bash
cd /Users/dexter/active/PLATEAU-VIEW-3.0/cms
git remote add upstream https://github.com/reearth/reearth-cms.git
git fetch upstream
```

**Pros:**
- Simple to set up
- Can reference upstream commits
- Easy to see diffs against upstream

**Cons:**
- Doesn't actually import history
- History is only available via `git log upstream/main`

## Recommendation

For this use case, **the current approach (direct copy) is recommended** because:

1. The code has been significantly modified (module names, paths, package names)
2. PLATEAU CMS will diverge from Re:Earth CMS over time
3. The original history is still available at https://github.com/reearth/reearth-cms
4. All builds and tests pass successfully with the current setup

If you need to reference the original history:
- Check the upstream repository: https://github.com/reearth/reearth-cms
- Git blame won't work, but you can search commits on GitHub
- For specific file history, check: `https://github.com/reearth/reearth-cms/commits/main/path/to/file`

## Future Synchronization

If you want to pull updates from Re:Earth CMS in the future:

1. Clone both repositories side by side
2. Use a diff tool to compare changes
3. Manually port relevant changes to PLATEAU CMS
4. Test thoroughly after each sync

This manual approach is safer than automated Git merging given the structural differences.
