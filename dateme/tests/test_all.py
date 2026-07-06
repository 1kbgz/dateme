from datetime import datetime, timezone

import pytest

from dateme import Schedule


def utc(y, m, d, hh=0, mm=0):
    return datetime(y, m, d, hh, mm, tzinfo=timezone.utc)


NYSE_MONDAY = {
    "freq": {"type": "weekly", "days": ["mon"], "time": "17:30"},
    "timezone": "America/New_York",
    "overlays": [{"calendar": "nyse_holiday", "rule": "exclude"}],
    "makeup": "after",
    "start": None,
    "end": None,
}


def make(spec):
    import json

    return Schedule.from_json(json.dumps(spec))


def test_next_makeup_after_over_mlk():
    s = make(NYSE_MONDAY)
    s.validate()
    # Mon 2026-01-19 is MLK Day (NYSE closed) -> makeup after -> Tue 2026-01-20 17:30 ET.
    assert s.next(utc(2026, 1, 13)) == utc(2026, 1, 20, 22, 30)
    # A regular Monday (Jan 5) fires normally at 17:30 ET that day.
    assert s.next(utc(2026, 1, 5)) == utc(2026, 1, 5, 22, 30)


def test_previous_and_since_descending():
    s = make(NYSE_MONDAY)
    before = utc(2026, 1, 13)
    assert s.previous(before) == utc(2026, 1, 12, 22, 30)
    series = s.since(utc(2025, 12, 15), before)
    assert series[0] == s.previous(before)  # since(start)[0] == previous()
    assert series == sorted(series, reverse=True)


def test_until_first_equals_next():
    s = make(NYSE_MONDAY)
    after = utc(2026, 1, 13)
    series = s.until(utc(2026, 2, 15), after)
    assert series[0] == s.next(after)  # until(end)[0] == next()
    assert series == sorted(series)


def test_upcoming_count():
    s = make(NYSE_MONDAY)
    assert len(s.upcoming(3, utc(2026, 1, 13))) == 3


def test_end_bound_returns_none():
    s = make(
        {
            "freq": {"type": "daily", "time": "12:00"},
            "timezone": "UTC",
            "overlays": [],
            "makeup": "none",
            "start": None,
            "end": "2026-01-03T00:00:00Z",
        }
    )
    assert s.next(utc(2026, 1, 2, 13)) is None


def test_validate_rejects_bad_schedule():
    with pytest.raises(ValueError):
        make(
            {
                "freq": {"type": "hourly", "minute": 99},
                "timezone": "UTC",
                "overlays": [],
                "makeup": "none",
                "start": None,
                "end": None,
            }
        ).validate()


def test_invalid_json_raises():
    with pytest.raises(ValueError):
        Schedule.from_json("{not valid}")


def test_roundtrip_json():
    s = make(NYSE_MONDAY)
    again = Schedule.from_json(s.to_json())
    assert again.to_json() == s.to_json()
