// Typed model for the dateme schedule spec. These types mirror the JSON schedule
// model; build a `ScheduleSpec` object and pass it to `new Schedule(...)`. The
// `as const` objects give runtime enum values for plain JavaScript callers.

export const Weekday = {
  Mon: "mon",
  Tue: "tue",
  Wed: "wed",
  Thu: "thu",
  Fri: "fri",
  Sat: "sat",
  Sun: "sun",
} as const;
export type Weekday = (typeof Weekday)[keyof typeof Weekday];

export const Nth = {
  First: "first",
  Second: "second",
  Third: "third",
  Fourth: "fourth",
  Fifth: "fifth",
  Last: "last",
} as const;
export type Nth = (typeof Nth)[keyof typeof Nth];

export const Makeup = {
  None: "none",
  Before: "before",
  After: "after",
} as const;
export type Makeup = (typeof Makeup)[keyof typeof Makeup];

export const MakeupFailure = {
  Skip: "skip",
  KeepOriginal: "keep_original",
} as const;
export type MakeupFailure = (typeof MakeupFailure)[keyof typeof MakeupFailure];

export const OverlayRule = {
  Exclude: "exclude",
  Only: "only",
} as const;
export type OverlayRule = (typeof OverlayRule)[keyof typeof OverlayRule];

export const CalendarId = {
  UsFederalHoliday: "us_federal_holiday",
  UsBusinessDay: "us_business_day",
  NyseHoliday: "nyse_holiday",
  NyseTradingDay: "nyse_trading_day",
} as const;
export type CalendarId = (typeof CalendarId)[keyof typeof CalendarId];

export type MonthDay = { type: "day"; value: number } | { type: "last" };

export interface NthWeekday {
  nth: Nth;
  weekday: Weekday;
}

export interface Overlay {
  calendar: CalendarId;
  rule: OverlayRule;
}

export type Frequency =
  | { type: "hourly"; minute: number }
  | { type: "daily"; time: string }
  | { type: "weekly"; days: Weekday[]; time: string }
  | { type: "monthly_by_day"; days: MonthDay[]; time: string }
  | { type: "monthly_by_weekday"; weekdays: NthWeekday[]; time: string }
  | { type: "yearly"; month: number; day: MonthDay; time: string };

export interface ScheduleSpec {
  freq: Frequency;
  timezone: string;
  overlays?: Overlay[];
  makeup?: Makeup;
  max_makeup_hops?: number | null;
  makeup_failure?: MakeupFailure;
  skip_if_consecutive_excluded?: number | null;
  start?: string | null;
  end?: string | null;
}
