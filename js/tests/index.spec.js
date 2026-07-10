import { test, expect } from "@playwright/test";

// Load the built ESM bundle in the browser, initialize the wasm module, then
// exercise the Schedule API. Mirrors the Rust/Python NYSE-Monday vector.
async function run(page, body) {
  await page.goto("/dist/");
  return page.evaluate(async (src) => {
    const mod = await import("/dist/cdn/index.js");
    await mod.init();
    const fn = new Function("mod", src);
    return fn(mod);
  }, body);
}

const NYSE_MONDAY = JSON.stringify({
  freq: { type: "weekly", days: ["mon"], time: "17:30" },
  timezone: "America/New_York",
  overlays: [{ calendar: "nyse_holiday", rule: "exclude" }],
  makeup: "after",
  start: null,
  end: null,
});

test.describe("Schedule", () => {
  test("next makes up after an NYSE holiday", async ({ page }) => {
    const iso = await run(
      page,
      `const s = new mod.Schedule(${JSON.stringify(NYSE_MONDAY)});
       s.validate();
       return s.next(new Date(Date.UTC(2026, 0, 13))).toISOString();`,
    );
    // Mon 2026-01-19 (MLK) closed -> makeup after -> Tue 2026-01-20 17:30 ET.
    expect(iso).toBe("2026-01-20T22:30:00.000Z");
  });

  test("until[0] equals next, since is descending", async ({ page }) => {
    const res = await run(
      page,
      `const s = new mod.Schedule(${JSON.stringify(NYSE_MONDAY)});
       const after = new Date(Date.UTC(2026, 0, 13));
       const until = s.until(new Date(Date.UTC(2026, 1, 15)), after).map(d => d.toISOString());
       const since = s.since(new Date(Date.UTC(2025, 11, 15)), after).map(d => d.toISOString());
       return { first: until[0], next: s.next(after).toISOString(),
                sinceFirst: since[0], prev: s.previous(after).toISOString(),
                sinceDesc: since.join() === [...since].sort().reverse().join() };`,
    );
    expect(res.first).toBe(res.next);
    expect(res.sinceFirst).toBe(res.prev);
    expect(res.sinceDesc).toBe(true);
  });

  test("upcoming returns n occurrences", async ({ page }) => {
    const n = await run(
      page,
      `const s = new mod.Schedule(${JSON.stringify(NYSE_MONDAY)});
       return s.upcoming(3, new Date(Date.UTC(2026, 0, 13))).length;`,
    );
    expect(n).toBe(3);
  });

  test("end bound yields null", async ({ page }) => {
    const got = await run(
      page,
      `const s = new mod.Schedule(JSON.stringify({
         freq: { type: "daily", time: "12:00" }, timezone: "UTC",
         overlays: [], makeup: "none", start: null, end: "2026-01-03T00:00:00Z" }));
       return s.next(new Date(Date.UTC(2026, 0, 2, 13)));`,
    );
    expect(got).toBeNull();
  });

  test("invalid schedule throws", async ({ page }) => {
    const threw = await run(
      page,
      `try {
         new mod.Schedule("{not valid}");
         return false;
       } catch (e) { return true; }`,
    );
    expect(threw).toBe(true);
  });

  test("structurally invalid schedule throws on construction", async ({
    page,
  }) => {
    const threw = await run(
      page,
      `try {
         new mod.Schedule({ freq: { type: "weekly", days: [], time: "09:00" }, timezone: "UTC" });
         return false;
       } catch (e) { return true; }`,
    );
    expect(threw).toBe(true);
  });

  test("constructs from a typed spec object using model enums", async ({
    page,
  }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
       };
       const s = new mod.Schedule(spec);
       s.validate();
       return {
         next: s.next(new Date(Date.UTC(2026, 0, 13))).toISOString(),
         roundtrip: s.toObject().freq.type,
       };`,
    );
    expect(res.next).toBe("2026-01-20T22:30:00.000Z");
    expect(res.roundtrip).toBe("weekly");
  });

  test("round-trips max_makeup_hops from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         max_makeup_hops: 1,
       };
       const s = new mod.Schedule(spec);
       return s.toObject().max_makeup_hops;`,
    );
    expect(res).toBe(1);
  });

  test("round-trips makeup_failure from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         max_makeup_hops: 1,
         makeup_failure: mod.MakeupFailure.KeepOriginal,
       };
       const s = new mod.Schedule(spec);
       return s.toObject().makeup_failure;`,
    );
    expect(res).toBe("keep_original");
  });

  test("throws when makeup_failure is error", async ({ page }) => {
    const message = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         max_makeup_hops: 1,
         makeup_failure: mod.MakeupFailure.Error,
         makeup_only_on: [mod.Weekday.Mon],
       };
       const s = new mod.Schedule(spec);
       try {
         s.next(new Date(Date.UTC(2026, 0, 13)));
         return "";
       } catch (e) {
         return e.message;
       }`,
    );
    expect(message).toContain("makeup failed");
  });

  test("round-trips weekday makeup from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon, mod.Weekday.Fri], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: { mon: mod.Makeup.After, fri: mod.Makeup.Before, default: mod.Makeup.None },
       };
       const s = new mod.Schedule(spec);
       return s.toObject().makeup;`,
    );
    expect(res).toEqual({ mon: "after", fri: "before", default: "none" });
  });

  test("round-trips nearest makeup from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.Nearest,
       };
       const s = new mod.Schedule(spec);
       return s.toObject().makeup;`,
    );
    expect(res).toBe("nearest");
  });

  test("round-trips makeup_only_on from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         makeup_only_on: [mod.Weekday.Tue, mod.Weekday.Wed],
       };
       const s = new mod.Schedule(spec);
       return s.toObject().makeup_only_on;`,
    );
    expect(res).toEqual(["tue", "wed"]);
  });

  test("round-trips makeup target constraints from typed specs", async ({
    page,
  }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         makeup_within_week: true,
         makeup_exclude_weekends: true,
         makeup_before_next: true,
       };
       const s = new mod.Schedule(spec);
       return s.toObject();`,
    );
    expect(res.makeup_within_week).toBe(true);
    expect(res.makeup_exclude_weekends).toBe(true);
    expect(res.makeup_before_next).toBe(true);
  });

  test("round-trips cascade makeup from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "09:00" },
         timezone: "UTC",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: [
           { direction: mod.Makeup.After, max_hops: 1 },
           { direction: mod.Makeup.Before, max_hops: 3 },
           mod.Makeup.None,
         ],
       };
       const s = new mod.Schedule(spec);
       return s.toObject().makeup;`,
    );
    expect(res).toEqual([
      { direction: "after", max_hops: 1 },
      { direction: "before", max_hops: 3 },
      "none",
    ]);
  });

  test("round-trips skip_if_consecutive_excluded from typed specs", async ({
    page,
  }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         skip_if_consecutive_excluded: 2,
       };
       const s = new mod.Schedule(spec);
       return s.toObject().skip_if_consecutive_excluded;`,
    );
    expect(res).toBe(2);
  });

  test("round-trips max_skip_gap from typed specs", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "17:30" },
         timezone: "America/New_York",
         overlays: [{ calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude }],
         makeup: mod.Makeup.After,
         max_skip_gap: 3,
       };
       const s = new mod.Schedule(spec);
       return s.toObject().max_skip_gap;`,
    );
    expect(res).toBe(3);
  });

  test("throws when max_skip_gap is exceeded", async ({ page }) => {
    const message = await run(
      page,
      `const s = new mod.Schedule({
         freq: { type: "weekly", days: [mod.Weekday.Mon], time: "09:00" },
         timezone: "UTC",
         max_skip_gap: 1,
       });
       try {
         s.until(new Date(Date.UTC(2026, 0, 5)), new Date(Date.UTC(2026, 0, 1)));
         return "";
       } catch (e) {
         return e.message;
       }`,
    );
    expect(message).toContain("max_skip_gap");
  });

  test("round-trips overlay any groups and per-overlay makeup", async ({
    page,
  }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "daily", time: "09:00" },
         timezone: "UTC",
         overlays: [{
           any: [
             { calendar: mod.CalendarId.UsFederalHoliday, rule: mod.OverlayRule.Exclude },
             { calendar: mod.CalendarId.NyseHoliday, rule: mod.OverlayRule.Exclude, makeup: mod.Makeup.Before },
           ],
           makeup: mod.Makeup.None,
         }],
       };
       const s = new mod.Schedule(spec);
       return s.toObject().overlays;`,
    );
    expect(res).toEqual([
      {
        any: [
          { calendar: "us_federal_holiday", rule: "exclude" },
          { calendar: "nyse_holiday", rule: "exclude", makeup: "before" },
        ],
        makeup: "none",
      },
    ]);
  });

  test("round-trips custom calendar specs from typed specs", async ({
    page,
  }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "daily", time: "09:00" },
         timezone: "UTC",
         overlays: [{
           calendar: {
             union: [
               { dates: ["2026-07-03", "2026-07-04"] },
               { diff: [mod.CalendarId.UsFederalHoliday, mod.CalendarId.NyseHoliday] },
               { custom: "shutdown" },
             ],
           },
           rule: mod.OverlayRule.Exclude,
         }],
       };
       const s = new mod.Schedule(spec);
       return s.toObject().overlays;`,
    );
    expect(res).toEqual([
      {
        calendar: {
          union: [
            { dates: ["2026-07-03", "2026-07-04"] },
            { diff: ["us_federal_holiday", "nyse_holiday"] },
            { custom: "shutdown" },
          ],
        },
        rule: "exclude",
      },
    ]);
  });

  test("uses custom calendar provider callbacks", async ({ page }) => {
    const res = await run(
      page,
      `const spec = {
         freq: { type: "daily", time: "09:00" },
         timezone: "UTC",
         overlays: [{ calendar: { custom: "shutdown" }, rule: mod.OverlayRule.Exclude }],
       };
       const s = new mod.Schedule(spec, (name, date) => name === "shutdown" && date === "2026-08-14");
       return s.until(new Date(Date.UTC(2026, 7, 16)), new Date(Date.UTC(2026, 7, 13))).map(d => d.toISOString());`,
    );
    expect(res).toEqual([
      "2026-08-13T09:00:00.000Z",
      "2026-08-15T09:00:00.000Z",
    ]);
  });
});
