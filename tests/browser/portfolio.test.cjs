const assert = require("node:assert/strict");
const { test } = require("node:test");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs/promises");
const path = require("node:path");
const { pathToFileURL } = require("node:url");
const { chromium } = require("playwright");

test("portfolio cords and clickable task references share aliases and collisions", async () => {
  const root = path.resolve(__dirname, "../..");
  const dir = await fs.mkdtemp(path.join(root, "target/portfolio-browser-"));
  let browser;
  try {
    const refs = ["TARGET", "first/target.json", "target.json", "__PROTO__", "missing", "constructor"];
    const envelope = (project, title, cross_repo = []) => ({
      project,
      phases: { "1": { name: "Work", order: 1 } },
      task: [{ id: 1, phase: 1, status: "pending", title, cross_repo }],
    });
    const projects = [
      envelope("Target", "First target"),
      envelope("TARGET", "Duplicate project"),
      envelope("target.json", "Basename collision"),
      envelope("__proto__", "Special alias"),
      envelope("Source", "Source task", [
        ...refs.map(repo => ({ repo, task_id: 1, relation: "blocks" })),
        { repo: "TARGET", task_id: 999, relation: "blocks" },
      ]),
    ];
    const files = ["first/target.json", "duplicate.json", "collision.json", "special.json", "source.json"];
    await fs.mkdir(path.join(dir, "first"));
    for (let i = 0; i < files.length; i++) {
      await fs.writeFile(path.join(dir, files[i]), JSON.stringify(projects[i]));
    }
    execFileSync(path.join(root, "target/debug/rmap"), [
      "render", "--html", "--multi", ...files, "--out", "portfolio.html",
    ], { cwd: dir });

    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    const url = pathToFileURL(path.join(dir, "portfolio.html")).href;
    await page.goto(url);
    const data = await page.locator("#rmap-data").textContent();
    assert.deepEqual(JSON.parse(data).projects, projects);
    assert.deepEqual(JSON.parse(await page.locator("#rmap-relations").textContent()), [
      { source: "source-4", target: "target-0", relation: "blocks" },
      { source: "source-4", target: "proto-3", relation: "blocks" },
    ]);
    assert.equal(await page.locator("#relations-overlay .rel-edge").count(), 2);

    for (const [alias, repo, title] of [
      [refs[0], 0, "First target"],
      [refs[1], 0, "First target"],
      [refs[2], 0, "First target"],
      [refs[3], 3, "Special alias"],
    ]) {
      await page.goto(url + "#task=4:1");
      const link = page.locator("button.ref").filter({ has: page.getByText("blocks · " + alias, { exact: true }) });
      await link.click();
      assert.equal(new URL(page.url()).hash, "#task=" + repo + ":1");
      assert.equal(await page.locator(".detail-title").textContent(), title);
    }
    await page.goto(url + "#task=4:1");
    const missing = page.locator(".ref-missing");
    assert.equal(await missing.count(), 3);
    assert.deepEqual(await missing.locator("b").allTextContents(), ["1", "1", "999"]);
    assert.equal(await missing.locator("button").count(), 0);
    assert.deepEqual(errors, []);
  } finally {
    if (browser) await browser.close();
    await fs.rm(dir, { recursive: true, force: true });
  }
});
