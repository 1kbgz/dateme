import { Schedule } from "../src/ts/index";
import type { ScheduleSpec } from "../src/ts/model";

const frequency = { type: "daily", time: "12:00" } as const;
const canonical: ScheduleSpec = { frequency, timezone: "UTC" };
const legacy: ScheduleSpec = { freq: frequency, timezone: "UTC" };

// @ts-expect-error A schedule requires a frequency.
const missing: ScheduleSpec = { timezone: "UTC" };
// @ts-expect-error The aliases cannot be supplied together.
const duplicate: ScheduleSpec = { frequency, freq: frequency, timezone: "UTC" };

const schedule = new Schedule(canonical);
new Schedule(legacy);
const objectFrequency: string = schedule.toObject().frequency.type;
const jsonFrequency: string = schedule.toJSON().frequency.type;

void missing;
void duplicate;
void objectFrequency;
void jsonFrequency;
