"""Unit tests for recent_updates.py (stdlib only; run via `make test`).

    python -m unittest utils/test_recent_updates.py
"""

import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import recent_updates as ru  # noqa: E402

TZ = timezone(timedelta(hours=2))


def _dt(s: str) -> datetime:
    return datetime.strptime(s, "%Y-%m-%d %H:%M").replace(tzinfo=TZ)


class GroupSessionsTest(unittest.TestCase):
    def test_creation_commit_never_folds_into_later_edits(self):
        """Publish Sep 28 14:10, fix 14:30, big rewrite Sep 29 10:20 (< 24h gap).

        Without a split the whole run reads as one "published Sep 29" row.
        The creation commit must stay its own session (dated the publish day)
        and the two later commits fold into one "updated" session dated Sep 29
        -- matching the note's `lastmod` / "Last updated" meta line."""
        first = _dt("2026-09-28 14:10")
        dated = [
            (first, 500, 0, True),
            (_dt("2026-09-28 14:30"), 5, 2, True),
            (_dt("2026-09-29 10:20"), 380, 60, True),
        ]
        sessions = ru.group_sessions(dated, first_dt=first)

        self.assertEqual(len(sessions), 2)
        update, creation = sessions  # newest first
        self.assertEqual(creation["start"], first)
        self.assertEqual(creation["end"], first)
        self.assertEqual(creation["added"], 500)
        self.assertEqual(update["start"], _dt("2026-09-28 14:30"))
        self.assertEqual(update["end"], _dt("2026-09-29 10:20"))
        self.assertEqual(update["added"], 385)
        self.assertEqual(update["removed"], 62)

    def test_edits_within_gap_still_fold(self):
        """The 24h coalescing is unchanged for non-creation commits."""
        first = _dt("2026-01-01 09:00")
        dated = [
            (first, 100, 0, True),
            (_dt("2026-03-10 22:00"), 10, 1, False),
            (_dt("2026-03-11 08:00"), 20, 3, True),
            (_dt("2026-05-01 12:00"), 7, 0, True),
        ]
        sessions = ru.group_sessions(dated, first_dt=first)
        self.assertEqual([s["end"] for s in sessions],
                         [_dt("2026-05-01 12:00"), _dt("2026-03-11 08:00"), first])
        self.assertEqual(sessions[1]["added"], 30)
        self.assertTrue(sessions[1]["bumped"])

    def test_no_first_dt_keeps_old_behaviour(self):
        dated = [(_dt("2026-09-28 14:10"), 1, 0, True), (_dt("2026-09-29 10:20"), 1, 0, True)]
        self.assertEqual(len(ru.group_sessions(dated)), 1)


class RelativeTest(unittest.TestCase):
    def test_calendar_days_not_elapsed_hours(self):
        now = _dt("2026-09-29 10:00")
        self.assertEqual(ru._relative(_dt("2026-09-28 14:10"), now), "yesterday")
        self.assertEqual(ru._relative(_dt("2026-09-29 01:00"), now), "today")
        self.assertEqual(ru._relative(_dt("2026-09-26 23:00"), now), "3 days ago")


class BadgeIsNewTest(unittest.TestCase):
    """NEW = latest content edit landed within NEW_GRACE_DAYS of the publish,
    judged at the note's own lastmod slot, not relative to today."""

    def test_same_day_tweak_stays_new(self):
        first = _dt("2026-08-21 09:35")
        self.assertTrue(ru.badge_is_new(_dt("2026-08-21 23:35"), first, grace_days=7))

    def test_next_day_rewrite_stays_new(self):
        first = _dt("2026-09-28 14:10")
        self.assertTrue(ru.badge_is_new(_dt("2026-09-29 10:20"), first, grace_days=7))

    def test_edit_after_grace_is_update(self):
        first = _dt("2026-09-28 14:10")
        self.assertFalse(ru.badge_is_new(_dt("2026-10-20 10:00"), first, grace_days=7))

    def test_boundary_inclusive(self):
        first = _dt("2026-09-01 12:00")
        self.assertTrue(ru.badge_is_new(_dt("2026-09-08 12:00"), first, grace_days=7))
        self.assertFalse(ru.badge_is_new(_dt("2026-09-08 12:01"), first, grace_days=7))

    def test_unknown_first_commit_is_update(self):
        self.assertFalse(ru.badge_is_new(_dt("2026-09-08 12:00"), None, grace_days=7))


if __name__ == "__main__":
    unittest.main()
