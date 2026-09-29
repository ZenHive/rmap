const assert = require("node:assert/strict");
const { test } = require("node:test");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs/promises");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { chromium } = require("playwright");

const TASKS = `
schema_version = 2
project = "detail_panel"
default_branch = "main"

[phases.1]
name = "Board"
order = 1
status = "in_progress"

[bundles.panel]
phase = 1
order = 1
description = "Detail panel"

[[task]]
id = 1
phase = 1
bundle = "panel"
status = "pending"
title = "Task with read-first and reviewer checks"
scores = { d = 2, b = 4, u = 2 }
body = "Show both lists beside the sections that already render."
acceptance_criteria = ["Both sections are visible"]
checks = ["cargo test --test delegate", "mix test test/harness/roadmap_test.exs"]
out_of_scope = ["Changing the data island"]
context_refs = ["DESIGN.md", "https://example.com/adr/0012", "src/<b>nope.rs"]
files_to_modify = ["templates/_board.js"]

[[task]]
id = 2
phase = 1
bundle = "panel"
status = "pending"
title = "Task without the new lists"
scores = { d = 2, b = 4, u = 2 }
body = "Today's sections only."
acceptance_criteria = ["No empty headings"]
out_of_scope = ["Inventing read-first or reviewer checks"]
files_to_modify = ["templates/_styles.css"]
`;

const HINT = "Hints for the reviewer, not an automated gate. rmap does not execute these commands; the reviewer runs and judges them.";

async function headings(page) {
  return page.locator(".detail-back h3").evaluateAll((nodes) =>
    nodes.map((node) => node.childNodes[0].textContent.trim())
  );
}

test("detail panel shows read-first and reviewer checks only when set", async () => {
  const root = path.resolve(__dirname, "../..");
  const dir = await fs.mkdtemp(path.join(root, "target/detail-browser-"));
  let browser;
  try {
    await fs.mkdir(path.join(dir, "roadmap"));
    await fs.writeFile(path.join(dir, "roadmap/tasks.toml"), TASKS);
    await fs.writeFile(path.join(dir, "ROADMAP.md"), "# Roadmap\n\n<!-- TASKS:BEGIN phase=1 -->\n<!-- TASKS:END -->\n");
    execFileSync(path.join(root, "target/debug/rmap"), ["render", "--html"], {
      cwd: dir,
      env: { ...process.env, RMAP_TODAY: "2026-09-29" },
    });

    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const url = pathToFileURL(path.join(dir, "roadmap/dist/index.html")).href;
    await page.goto(url);

    const island = JSON.parse(await page.locator("#rmap-data").textContent());
    const withLists = island.task.find((task) => task.id === 1);
    const withoutLists = island.task.find((task) => task.id === 2);
    assert.deepEqual(withLists.context_refs, ["DESIGN.md", "https://example.com/adr/0012", "src/<b>nope.rs"]);
    assert.deepEqual(withLists.checks, ["cargo test --test delegate", "mix test test/harness/roadmap_test.exs"]);
    assert.equal(Object.hasOwn(withoutLists, "context_refs"), false);
    assert.equal(Object.hasOwn(withoutLists, "checks"), false);

    await page.locator('.task-card[data-id="1"]').click();
    assert.equal(new URL(page.url()).hash, "#task=0:1");
    assert.deepEqual(await headings(page), [
      "Read first",
      "Spec",
      "Acceptance criteria",
      "Reviewer checks",
      "Out of scope",
      "Files",
    ]);
    const read = page.locator("section.spec", { has: page.getByRole("heading", { name: "Read first", exact: true }) });
    assert.match(await read.innerText(), /Read these before starting/);
    assert.equal(await read.locator("code").nth(0).textContent(), "DESIGN.md");
    const link = read.locator("a");
    assert.equal(await link.getAttribute("href"), "https://example.com/adr/0012");
    assert.equal(await link.getAttribute("target"), "_blank");
    assert.equal(await link.getAttribute("rel"), "noopener");
    assert.equal(await read.locator("b, img").count(), 0);
    assert.match(await read.innerText(), /src\/<b>nope\.rs/);

    const checks = page.locator("section.spec", { has: page.getByRole("heading", { name: "Reviewer checks", exact: true }) });
    assert.equal(await checks.locator("p").textContent(), HINT);
    assert.deepEqual(await checks.locator("li").allTextContents(), [
      "cargo test --test delegate",
      "mix test test/harness/roadmap_test.exs",
    ]);

    await page.locator('.task-card[data-id="2"]').click();
    assert.equal(new URL(page.url()).hash, "#task=0:2");
    assert.equal(await page.locator(".detail-title").textContent(), "Task without the new lists");
    assert.deepEqual(await headings(page), ["Spec", "Acceptance criteria", "Out of scope", "Files"]);
    assert.equal(await page.locator(".detail-back").getByRole("heading", { name: "Read first" }).count(), 0);
    assert.equal(await page.locator(".detail-back").getByRole("heading", { name: "Reviewer checks" }).count(), 0);
    assert.match(await page.locator(".detail-back").innerText(), /Today's sections only/);
    assert.match(await page.locator(".detail-back").innerText(), /No empty headings/);
    assert.match(await page.locator(".detail-back").innerText(), /templates\/_styles\.css/);

    await page.setViewportSize({ width: 390, height: 844 });
    await page.keyboard.press("Escape");
    await page.locator('.task-card[data-id="1"]').click();
    assert.deepEqual(await headings(page), [
      "Read first",
      "Spec",
      "Acceptance criteria",
      "Reviewer checks",
      "Out of scope",
      "Files",
    ]);
    assert.equal(await page.locator("section.spec", {
      has: page.getByRole("heading", { name: "Reviewer checks", exact: true }),
    }).locator("p").textContent(), HINT);
    assert.deepEqual(errors, []);
  } finally {
    if (browser) await browser.close();
    await fs.rm(dir, { recursive: true, force: true });
  }
});
