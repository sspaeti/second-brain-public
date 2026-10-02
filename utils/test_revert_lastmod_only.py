"""utils/revert-lastmod-only.sh against a throwaway git repo.

The script normally cds into ../content; the tests pass an explicit directory
as its first argument instead. Run via `make test` or
`python -m unittest utils/test_revert_lastmod_only.py`.
"""
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "revert-lastmod-only.sh"

HEAD_NOTE = """---
createddate: 2023-12-22
lastmod: 2026-05-01 09:43:34
title: "SQL IDEs"
---
SQL IDEs are the butter and bread.
"""


class RevertLastmodOnlyTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        for args in (["init", "-q"], ["config", "user.email", "t@t"], ["config", "user.name", "t"]):
            subprocess.run(["git", "-C", str(self.repo), *args], check=True)
        self.note = self.repo / "sql ides.md"
        self.note.write_text(HEAD_NOTE, encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "add", "-A"], check=True)
        subprocess.run(["git", "-C", str(self.repo), "commit", "-q", "-m", "head"], check=True)

    def tearDown(self):
        self.tmp.cleanup()

    def run_script(self) -> str:
        return subprocess.run([str(SCRIPT), str(self.repo)], capture_output=True, text=True, check=True).stdout

    def test_lastmod_only_change_is_fully_restored(self):
        self.note.write_text(HEAD_NOTE.replace("2026-05-01 09:43:34", "2026-10-01 13:37:29"), encoding="utf-8")
        out = self.run_script()
        self.assertEqual(self.note.read_text(encoding="utf-8"), HEAD_NOTE)
        self.assertIn("reverted (lastmod-only)", out)

    def test_status_lines_are_kept_but_lastmod_is_restored(self):
        bumped = HEAD_NOTE.replace("2026-05-01 09:43:34", "2026-10-01 13:37:29").replace(
            'title: "SQL IDEs"', 'status: growing\nstatus_source: manual\ntitle: "SQL IDEs"'
        )
        self.note.write_text(bumped, encoding="utf-8")
        out = self.run_script()
        text = self.note.read_text(encoding="utf-8")
        self.assertIn("lastmod: 2026-05-01 09:43:34\n", text, "old lastmod is back")
        self.assertIn("status: growing\nstatus_source: manual\n", text, "the override survives")
        self.assertNotIn("2026-10-01", text)
        self.assertIn("restored lastmod (status-only change)", out)

    def test_removed_status_lines_also_keep_working_tree_and_restore_lastmod(self):
        with_status = HEAD_NOTE.replace('title: "SQL IDEs"', 'status: growing\nstatus_source: manual\ntitle: "SQL IDEs"')
        self.note.write_text(with_status, encoding="utf-8")
        subprocess.run(["git", "-C", str(self.repo), "commit", "-qam", "with status"], check=True)
        # author removed the tag; the copy step bumps lastmod again
        self.note.write_text(HEAD_NOTE.replace("2026-05-01 09:43:34", "2026-10-02 08:00:00"), encoding="utf-8")
        self.run_script()
        text = self.note.read_text(encoding="utf-8")
        self.assertNotIn("status:", text)
        self.assertIn("lastmod: 2026-05-01 09:43:34\n", text)

    def test_real_body_change_is_left_alone(self):
        edited = HEAD_NOTE.replace("2026-05-01 09:43:34", "2026-10-01 13:37:29").replace(
            "butter and bread.", "butter and bread of every data engineer."
        )
        self.note.write_text(edited, encoding="utf-8")
        out = self.run_script()
        self.assertEqual(self.note.read_text(encoding="utf-8"), edited)
        self.assertEqual(out, "")


if __name__ == "__main__":
    unittest.main()
