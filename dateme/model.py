"""Typed builders for the dateme schedule model.

These dataclasses and enums mirror the JSON schedule model. Build a structure
from them and pass it to :class:`dateme.Schedule` (or call ``to_dict()`` /
``to_json()``). Construction performs light validation; the authoritative
structural check remains :meth:`dateme.Schedule.validate`.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from datetime import date as Date, datetime, time
from enum import Enum

__all__ = [
    "Weekday",
    "Nth",
    "Makeup",
    "MakeupStep",
    "WeekdayMakeup",
    "OverlayRule",
    "CalendarId",
    "CalendarDates",
    "CalendarUnion",
    "CalendarDiff",
    "CustomCalendar",
    "MakeupFailure",
    "MonthDay",
    "NthWeekday",
    "Overlay",
    "AnyOverlay",
    "Frequency",
    "Hourly",
    "Daily",
    "Weekly",
    "EveryNDays",
    "EveryNWeeks",
    "MonthlyByDay",
    "MonthlyByWeekday",
    "Yearly",
    "Quarterly",
    "CustomCron",
    "Schedule",
]


class Weekday(str, Enum):
    MON = "mon"
    TUE = "tue"
    WED = "wed"
    THU = "thu"
    FRI = "fri"
    SAT = "sat"
    SUN = "sun"


class Nth(str, Enum):
    FIRST = "first"
    SECOND = "second"
    THIRD = "third"
    FOURTH = "fourth"
    FIFTH = "fifth"
    LAST = "last"


class Makeup(str, Enum):
    NONE = "none"
    BEFORE = "before"
    AFTER = "after"
    NEAREST = "nearest"


class MakeupFailure(str, Enum):
    SKIP = "skip"
    KEEP_ORIGINAL = "keep_original"
    ERROR = "error"


@dataclass(frozen=True)
class MakeupStep:
    """One cascade makeup step."""

    direction: Makeup
    max_hops: int | None = None

    def to_dict(self) -> dict:
        return {"direction": self.direction.value, "max_hops": self.max_hops}


@dataclass(frozen=True)
class WeekdayMakeup:
    """Makeup directions selected by the excluded date's weekday."""

    mon: Makeup | None = None
    tue: Makeup | None = None
    wed: Makeup | None = None
    thu: Makeup | None = None
    fri: Makeup | None = None
    sat: Makeup | None = None
    sun: Makeup | None = None
    default: Makeup | None = None

    def to_dict(self) -> dict:
        out = {}
        for name in ("mon", "tue", "wed", "thu", "fri", "sat", "sun", "default"):
            value = getattr(self, name)
            if value is not None:
                out[name] = value.value
        return out


class OverlayRule(str, Enum):
    EXCLUDE = "exclude"
    ONLY = "only"


class CalendarId(str, Enum):
    US_FEDERAL_HOLIDAY = "us_federal_holiday"
    US_BUSINESS_DAY = "us_business_day"
    NYSE_HOLIDAY = "nyse_holiday"
    NYSE_TRADING_DAY = "nyse_trading_day"


def _date_str(value: str | Date | datetime) -> str:
    if isinstance(value, datetime):
        return value.date().isoformat()
    if isinstance(value, Date):
        return value.isoformat()
    return value


@dataclass(frozen=True)
class CalendarDates:
    """An inline set of local dates."""

    dates: list[str | Date | datetime]

    def to_dict(self) -> dict:
        return {"dates": [_date_str(d) for d in self.dates]}


@dataclass(frozen=True)
class CalendarUnion:
    """A calendar that contains dates present in any child calendar."""

    union: list[CalendarSpec]

    def __post_init__(self) -> None:
        if not self.union:
            raise ValueError("calendar union is empty")

    def to_dict(self) -> dict:
        return {"union": [_calendar_to_json(c) for c in self.union]}


@dataclass(frozen=True)
class CalendarDiff:
    """A calendar containing dates in the first child, minus following children."""

    diff: list[CalendarSpec]

    def __post_init__(self) -> None:
        if not self.diff:
            raise ValueError("calendar diff is empty")

    def to_dict(self) -> dict:
        return {"diff": [_calendar_to_json(c) for c in self.diff]}


@dataclass(frozen=True)
class CustomCalendar:
    """A named calendar resolved by a runtime provider."""

    custom: str

    def __post_init__(self) -> None:
        if not self.custom:
            raise ValueError("custom calendar name is empty")

    def to_dict(self) -> dict:
        return {"custom": self.custom}


CalendarSpec = CalendarId | CalendarDates | CalendarUnion | CalendarDiff | CustomCalendar


def _calendar_to_json(value: CalendarSpec) -> str | dict:
    if isinstance(value, CalendarId):
        return value.value
    return value.to_dict()


def _makeup_to_json(value: Makeup | WeekdayMakeup | list[Makeup | MakeupStep]) -> str | dict | list:
    if isinstance(value, Makeup):
        return value.value
    if isinstance(value, WeekdayMakeup):
        return value.to_dict()
    return [step.value if isinstance(step, Makeup) else step.to_dict() for step in value]


def _time_str(value: str | time) -> str:
    """Normalize a time-of-day to ``"HH:MM"``."""
    if isinstance(value, time):
        return f"{value.hour:02}:{value.minute:02}"
    return value


def _instant_str(value: str | datetime | None) -> str | None:
    """Normalize a UTC instant bound to an RFC 3339 string, or ``None``."""
    if value is None:
        return None
    if isinstance(value, datetime):
        return value.isoformat()
    return value


@dataclass(frozen=True)
class MonthDay:
    """A day within a month: a fixed ``value`` (1-31), or the last day when ``None``."""

    value: int | None = None

    def __post_init__(self) -> None:
        if self.value is not None and not 1 <= self.value <= 31:
            raise ValueError(f"month day {self.value} out of range 1..=31")

    @classmethod
    def day(cls, value: int) -> MonthDay:
        return cls(value)

    @classmethod
    def last(cls) -> MonthDay:
        return cls(None)

    def to_dict(self) -> dict:
        if self.value is None:
            return {"type": "last"}
        return {"type": "day", "value": self.value}


@dataclass(frozen=True)
class NthWeekday:
    """An ordinal weekday within a month, e.g. the third Tuesday."""

    nth: Nth
    weekday: Weekday

    def to_dict(self) -> dict:
        return {"nth": self.nth.value, "weekday": self.weekday.value}


@dataclass(frozen=True)
class Overlay:
    """A calendar filter applied to occurrences."""

    calendar: CalendarSpec
    rule: OverlayRule
    makeup: Makeup | WeekdayMakeup | list[Makeup | MakeupStep] | None = None

    def to_dict(self) -> dict:
        out = {"calendar": _calendar_to_json(self.calendar), "rule": self.rule.value}
        if self.makeup is not None:
            out["makeup"] = _makeup_to_json(self.makeup)
        return out


@dataclass(frozen=True)
class AnyOverlay:
    """An overlay group that passes when any child overlay passes."""

    any: list[Overlay | AnyOverlay]
    makeup: Makeup | WeekdayMakeup | list[Makeup | MakeupStep] | None = None

    def __post_init__(self) -> None:
        if not self.any:
            raise ValueError("overlay any group is empty")

    def to_dict(self) -> dict:
        out = {"any": [o.to_dict() for o in self.any]}
        if self.makeup is not None:
            out["makeup"] = _makeup_to_json(self.makeup)
        return out


class Frequency:
    """Base class for the recurrence frequencies."""

    def to_dict(self) -> dict:  # pragma: no cover - overridden
        raise NotImplementedError


@dataclass(frozen=True)
class Hourly(Frequency):
    """Every hour, at ``minute`` past the hour."""

    minute: int

    def __post_init__(self) -> None:
        if not 0 <= self.minute <= 59:
            raise ValueError(f"minute {self.minute} out of range 0..=59")

    def to_dict(self) -> dict:
        return {"type": "hourly", "minute": self.minute}


@dataclass(frozen=True)
class Daily(Frequency):
    """Every day at ``time``."""

    time: str | time

    def to_dict(self) -> dict:
        return {"type": "daily", "time": _time_str(self.time)}


@dataclass(frozen=True)
class Weekly(Frequency):
    """Every selected weekday at ``time``."""

    days: list[Weekday]
    time: str | time

    def __post_init__(self) -> None:
        if not self.days:
            raise ValueError("weekly days selection is empty")

    def to_dict(self) -> dict:
        return {"type": "weekly", "days": [d.value for d in self.days], "time": _time_str(self.time)}


@dataclass(frozen=True)
class EveryNDays(Frequency):
    """Every ``interval`` days from ``start_date`` at ``time``."""

    interval: int
    start_date: str | Date | datetime
    time: str | time

    def __post_init__(self) -> None:
        if self.interval < 1:
            raise ValueError("every_n_days interval must be at least 1")

    def to_dict(self) -> dict:
        return {
            "type": "every_n_days",
            "interval": self.interval,
            "start_date": _date_str(self.start_date),
            "time": _time_str(self.time),
        }


@dataclass(frozen=True)
class EveryNWeeks(Frequency):
    """Every ``interval`` weeks from ``start_date`` on selected weekdays."""

    interval: int
    start_date: str | Date | datetime
    days: list[Weekday]
    time: str | time

    def __post_init__(self) -> None:
        if self.interval < 1:
            raise ValueError("every_n_weeks interval must be at least 1")
        if not self.days:
            raise ValueError("every_n_weeks days selection is empty")

    def to_dict(self) -> dict:
        return {
            "type": "every_n_weeks",
            "interval": self.interval,
            "start_date": _date_str(self.start_date),
            "days": [d.value for d in self.days],
            "time": _time_str(self.time),
        }


@dataclass(frozen=True)
class MonthlyByDay(Frequency):
    """Selected days-of-month at ``time``."""

    days: list[MonthDay]
    time: str | time

    def __post_init__(self) -> None:
        if not self.days:
            raise ValueError("monthly days selection is empty")

    def to_dict(self) -> dict:
        return {"type": "monthly_by_day", "days": [d.to_dict() for d in self.days], "time": _time_str(self.time)}


@dataclass(frozen=True)
class MonthlyByWeekday(Frequency):
    """Selected nth-weekdays at ``time``."""

    weekdays: list[NthWeekday]
    time: str | time

    def __post_init__(self) -> None:
        if not self.weekdays:
            raise ValueError("monthly weekdays selection is empty")

    def to_dict(self) -> dict:
        return {"type": "monthly_by_weekday", "weekdays": [w.to_dict() for w in self.weekdays], "time": _time_str(self.time)}


@dataclass(frozen=True)
class Yearly(Frequency):
    """Once a year in ``month`` on ``day`` at ``time``."""

    month: int
    day: MonthDay
    time: str | time

    def __post_init__(self) -> None:
        if not 1 <= self.month <= 12:
            raise ValueError(f"month {self.month} out of range 1..=12")

    def to_dict(self) -> dict:
        return {"type": "yearly", "month": self.month, "day": self.day.to_dict(), "time": _time_str(self.time)}


@dataclass(frozen=True)
class Quarterly(Frequency):
    """Every quarter on ``month`` within quarter (1-3), on ``day`` at ``time``."""

    month: int
    day: MonthDay
    time: str | time

    def __post_init__(self) -> None:
        if not 1 <= self.month <= 3:
            raise ValueError(f"quarter month {self.month} out of range 1..=3")

    def to_dict(self) -> dict:
        return {"type": "quarterly", "month": self.month, "day": self.day.to_dict(), "time": _time_str(self.time)}


@dataclass(frozen=True)
class CustomCron(Frequency):
    """Five-field cron expression in schedule-local time."""

    expr: str

    def to_dict(self) -> dict:
        return {"type": "custom_cron", "expr": self.expr}


@dataclass(frozen=True)
class Schedule:
    """A complete schedule spec.

    Pass an instance to :class:`dateme.Schedule` to compute occurrences, or call
    :meth:`to_dict` / :meth:`to_json` for the storable form.
    """

    freq: Frequency
    timezone: str
    overlays: list[Overlay | AnyOverlay] = field(default_factory=list)
    makeup: Makeup | WeekdayMakeup | list[Makeup | MakeupStep] = Makeup.NONE
    max_makeup_hops: int | None = None
    makeup_failure: MakeupFailure = MakeupFailure.SKIP
    makeup_only_on: list[Weekday] | None = None
    makeup_within_week: bool = False
    makeup_exclude_weekends: bool = False
    makeup_before_next: bool = False
    skip_if_consecutive_excluded: int | None = None
    max_skip_gap: int | None = None
    start: str | datetime | None = None
    end: str | datetime | None = None

    def __post_init__(self) -> None:
        if self.skip_if_consecutive_excluded is not None and self.skip_if_consecutive_excluded < 1:
            raise ValueError("skip_if_consecutive_excluded must be at least 1")
        if self.max_skip_gap is not None and self.max_skip_gap < 1:
            raise ValueError("max_skip_gap must be at least 1")

    def to_dict(self) -> dict:
        return {
            "freq": self.freq.to_dict(),
            "timezone": self.timezone,
            "overlays": [o.to_dict() for o in self.overlays],
            "makeup": _makeup_to_json(self.makeup),
            "max_makeup_hops": self.max_makeup_hops,
            "makeup_failure": self.makeup_failure.value,
            "makeup_only_on": None if self.makeup_only_on is None else [d.value for d in self.makeup_only_on],
            "makeup_within_week": self.makeup_within_week,
            "makeup_exclude_weekends": self.makeup_exclude_weekends,
            "makeup_before_next": self.makeup_before_next,
            "skip_if_consecutive_excluded": self.skip_if_consecutive_excluded,
            "max_skip_gap": self.max_skip_gap,
            "start": _instant_str(self.start),
            "end": _instant_str(self.end),
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict())
