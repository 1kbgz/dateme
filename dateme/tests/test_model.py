from datetime import datetime, time, timezone

import pytest

from dateme import (
    AnyOverlay,
    CalendarId,
    Makeup,
    MakeupFailure,
    MakeupStep,
    MonthDay,
    MonthlyByDay,
    Nth,
    NthWeekday,
    Overlay,
    OverlayRule,
    Schedule,
    Weekday,
    WeekdayMakeup,
    Weekly,
    model,
)


def utc(y, m, d, hh=0, mm=0):
    return datetime(y, m, d, hh, mm, tzinfo=timezone.utc)


def nyse_monday_spec():
    return model.Schedule(
        freq=Weekly([Weekday.MON], "17:30"),
        timezone="America/New_York",
        overlays=[Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE)],
        makeup=Makeup.AFTER,
    )


def test_construct_from_typed_model():
    s = Schedule(nyse_monday_spec())
    s.validate()
    assert s.next(utc(2026, 1, 13)) == utc(2026, 1, 20, 22, 30)


def test_typed_dict_json_agree():
    spec = nyse_monday_spec()
    after = utc(2026, 1, 13)
    a = Schedule(spec).next(after)
    b = Schedule(spec.to_dict()).next(after)
    c = Schedule(spec.to_json()).next(after)
    assert a == b == c


def test_typed_model_serializes_max_makeup_hops():
    spec = nyse_monday_spec()
    capped = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        max_makeup_hops=1,
    )
    assert capped.to_dict()["max_makeup_hops"] == 1
    assert Schedule(capped).to_dict()["max_makeup_hops"] == 1


def test_typed_model_serializes_makeup_failure():
    spec = nyse_monday_spec()
    keep = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        max_makeup_hops=1,
        makeup_failure=MakeupFailure.KEEP_ORIGINAL,
    )
    assert keep.to_dict()["makeup_failure"] == "keep_original"
    assert Schedule(keep).to_dict()["makeup_failure"] == "keep_original"


def test_makeup_failure_error_raises_from_query():
    spec = nyse_monday_spec()
    strict = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        max_makeup_hops=1,
        makeup_failure=MakeupFailure.ERROR,
        makeup_only_on=[Weekday.MON],
    )
    with pytest.raises(ValueError, match="makeup failed"):
        Schedule(strict).next(utc(2026, 1, 13))


def test_typed_model_serializes_weekday_makeup():
    spec = nyse_monday_spec()
    weekday_makeup = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=WeekdayMakeup(mon=Makeup.AFTER, fri=Makeup.BEFORE, default=Makeup.NONE),
    )
    expected = {"mon": "after", "fri": "before", "default": "none"}
    assert weekday_makeup.to_dict()["makeup"] == expected
    assert Schedule(weekday_makeup).to_dict()["makeup"] == expected


def test_typed_model_serializes_nearest_makeup():
    spec = nyse_monday_spec()
    nearest = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=Makeup.NEAREST,
    )
    assert nearest.to_dict()["makeup"] == "nearest"
    assert Schedule(nearest).to_dict()["makeup"] == "nearest"


def test_typed_model_serializes_makeup_only_on():
    spec = nyse_monday_spec()
    restricted = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        makeup_only_on=[Weekday.TUE, Weekday.WED],
    )
    assert restricted.to_dict()["makeup_only_on"] == ["tue", "wed"]
    assert Schedule(restricted).to_dict()["makeup_only_on"] == ["tue", "wed"]


def test_typed_model_serializes_makeup_target_constraints():
    spec = nyse_monday_spec()
    constrained = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        makeup_within_week=True,
        makeup_exclude_weekends=True,
        makeup_before_next=True,
    )
    d = Schedule(constrained).to_dict()
    assert d["makeup_within_week"] is True
    assert d["makeup_exclude_weekends"] is True
    assert d["makeup_before_next"] is True


def test_typed_model_serializes_cascade_makeup():
    spec = nyse_monday_spec()
    cascade = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=[
            MakeupStep(Makeup.AFTER, max_hops=1),
            MakeupStep(Makeup.BEFORE, max_hops=3),
            Makeup.NONE,
        ],
    )
    expected = [
        {"direction": "after", "max_hops": 1},
        {"direction": "before", "max_hops": 3},
        "none",
    ]
    assert cascade.to_dict()["makeup"] == expected
    assert Schedule(cascade).to_dict()["makeup"] == expected


def test_typed_model_serializes_skip_if_consecutive_excluded():
    spec = nyse_monday_spec()
    threshold = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        skip_if_consecutive_excluded=2,
    )
    assert threshold.to_dict()["skip_if_consecutive_excluded"] == 2
    assert Schedule(threshold).to_dict()["skip_if_consecutive_excluded"] == 2


def test_typed_model_serializes_max_skip_gap():
    spec = nyse_monday_spec()
    gap = model.Schedule(
        freq=spec.freq,
        timezone=spec.timezone,
        overlays=spec.overlays,
        makeup=spec.makeup,
        max_skip_gap=3,
    )
    assert gap.to_dict()["max_skip_gap"] == 3
    assert Schedule(gap).to_dict()["max_skip_gap"] == 3


def test_max_skip_gap_raises_from_query():
    spec = model.Schedule(
        freq=Weekly([Weekday.MON], "09:00"),
        timezone="UTC",
        max_skip_gap=1,
    )
    with pytest.raises(ValueError, match="max_skip_gap"):
        Schedule(spec).until(utc(2026, 1, 5), utc(2026, 1, 1))


def test_typed_model_serializes_overlay_any_and_overlay_makeup():
    overlay = AnyOverlay(
        [
            Overlay(CalendarId.US_FEDERAL_HOLIDAY, OverlayRule.EXCLUDE),
            Overlay(CalendarId.NYSE_HOLIDAY, OverlayRule.EXCLUDE, makeup=Makeup.BEFORE),
        ],
        makeup=Makeup.NONE,
    )
    spec = model.Schedule(
        freq=model.Daily("09:00"),
        timezone="UTC",
        overlays=[overlay],
    )
    expected = [
        {
            "any": [
                {"calendar": "us_federal_holiday", "rule": "exclude"},
                {"calendar": "nyse_holiday", "rule": "exclude", "makeup": "before"},
            ],
            "makeup": "none",
        }
    ]
    assert spec.to_dict()["overlays"] == expected
    assert Schedule(spec).to_dict()["overlays"] == expected


def test_host_validation_skip_if_consecutive_excluded():
    spec = nyse_monday_spec()
    with pytest.raises(ValueError):
        model.Schedule(
            freq=spec.freq,
            timezone=spec.timezone,
            overlays=spec.overlays,
            makeup=spec.makeup,
            skip_if_consecutive_excluded=0,
        )


def test_construct_from_plain_dict():
    s = Schedule(
        {
            "freq": {"type": "daily", "time": "12:00"},
            "timezone": "UTC",
        }
    )
    assert s.next(utc(2026, 1, 1)) == utc(2026, 1, 1, 12, 0)


def test_to_dict_roundtrip():
    spec = nyse_monday_spec()
    s = Schedule(spec)
    d = s.to_dict()
    assert d["freq"] == {"type": "weekly", "days": ["mon"], "time": "17:30"}
    # rebuilding from the dict yields an identical schedule
    assert Schedule(d).to_json() == s.to_json()


def test_model_serializes_all_frequencies():
    specs = [
        model.Hourly(30),
        model.Daily("09:00"),
        Weekly([Weekday.MON, Weekday.WED], "17:00"),
        MonthlyByDay([MonthDay.day(1), MonthDay.last()], "12:00"),
        model.MonthlyByWeekday([NthWeekday(Nth.FIRST, Weekday.TUE)], "09:00"),
        model.Yearly(7, MonthDay.day(4), "12:00"),
    ]
    for freq in specs:
        s = Schedule(model.Schedule(freq=freq, timezone="UTC"))
        s.validate()  # every built frequency is structurally valid


def test_time_accepts_datetime_time():
    spec = model.Schedule(freq=model.Daily(time(9, 30)), timezone="UTC")
    assert spec.to_dict()["freq"]["time"] == "09:30"


def test_bounds_accept_datetime():
    spec = model.Schedule(
        freq=model.Daily("12:00"),
        timezone="UTC",
        start=utc(2026, 6, 1),
    )
    s = Schedule(spec)
    assert s.next(utc(2026, 1, 1)) == utc(2026, 6, 1, 12, 0)


def test_host_validation_hourly_minute():
    with pytest.raises(ValueError):
        model.Hourly(99)


def test_host_validation_empty_weekly():
    with pytest.raises(ValueError):
        Weekly([], "09:00")


def test_host_validation_monthday_range():
    with pytest.raises(ValueError):
        MonthDay.day(32)


def test_construction_validates_raw_dict():
    # host builders can be bypassed with a raw dict; the engine validates on
    # construction, so an invalid schedule raises immediately.
    with pytest.raises(ValueError):
        Schedule({"freq": {"type": "weekly", "days": [], "time": "09:00"}, "timezone": "UTC"})


def test_construction_rejects_out_of_range_month():
    # An out-of-range yearly month is rejected at construction rather than
    # crashing the interpreter later.
    with pytest.raises(ValueError):
        Schedule(
            {
                "freq": {"type": "yearly", "month": 13, "day": {"type": "day", "value": 1}, "time": "12:00"},
                "timezone": "UTC",
            }
        )


def test_construction_rejects_bad_skip_if_consecutive_excluded():
    with pytest.raises(ValueError):
        Schedule(
            {
                "freq": {"type": "daily", "time": "09:00"},
                "timezone": "UTC",
                "skip_if_consecutive_excluded": 0,
            }
        )
