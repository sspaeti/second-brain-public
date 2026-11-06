"""Logic of utils/revert-lastmod-only.sh: reads one `git diff -U0` stream of
the content submodule on stdin (prefixes pinned to a/ b/), decides per note
whether the diff is lastmod-only / meta-only / real, and restores from HEAD.
Runs with the submodule as cwd. See the shell wrapper for the rationale and
utils/test_revert_lastmod_only.py for the behaviour."""
import re
import subprocess
import sys

IGNORABLE = re.compile(r"^[+-](lastmod|status|status_source|categories|category_source):")
META = re.compile(r"^[+-](status|status_source|categories|category_source):")


def main() -> None:
    diff = sys.stdin.read()
    # Notes present in HEAD (one call; a note staged but not yet committed has nothing to revert to).
    in_head = set(
        subprocess.run(["git", "ls-tree", "-r", "--name-only", "-z", "HEAD"], capture_output=True, text=True, check=True)
        .stdout.split("\0")
    )
    for block in re.split(r"^(?=diff --git )", diff, flags=re.M):
        if not block.startswith("diff --git "):
            continue
        header = block.split("\n", 1)[0]
        path = header.partition(" b/")[2]  # unquoted thanks to core.quotepath=false
        if not path.endswith(".md"):
            continue
        if path not in in_head:
            continue
        # Changed content lines only: drop diff hunk/file headers, then drop the
        # lines a tag-only save or a category injection may legitimately touch.
        changed = [l for l in block.split("\n") if l.startswith(("+", "-")) and not l.startswith(("+++", "---"))]
        if any(not IGNORABLE.match(l) for l in changed):
            continue  # a real edit
        if any(META.match(l) for l in changed):
            # status or category changed: keep the working tree, put HEAD's lastmod back
            # (only when lastmod moved at all: the category line alone is a no-op here)
            if not any(l.startswith(("+lastmod:", "-lastmod:")) for l in changed):
                continue
            head = subprocess.run(["git", "show", f"HEAD:{path}"], capture_output=True, text=True, check=True).stdout
            m = re.search(r"^lastmod:.*$", head, re.M)
            if not m:
                continue
            head_lastmod = m.group(0)
            with open(path, encoding="utf-8") as f:
                text = f.read()
            with open(path, "w", encoding="utf-8") as f:
                f.write(re.sub(r"^lastmod:.*$", lambda _: head_lastmod, text, count=1, flags=re.M))
            print(f"restored lastmod (status-only change): {path}")
        else:
            head = subprocess.run(["git", "show", f"HEAD:{path}"], capture_output=True, check=True).stdout
            with open(path, "wb") as f:
                f.write(head)
            print(f"reverted (lastmod-only): {path}")


if __name__ == "__main__":
    main()
