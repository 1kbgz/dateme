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
});
