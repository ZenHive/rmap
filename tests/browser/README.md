Run the portfolio interaction regression with Node.js and Chromium:

```sh
cd tests/browser
npm ci
npx playwright install chromium
npm test
```

The tests build rmap, render temporary projects, and open the generated HTML
in headless Chromium. `portfolio.test.cjs` checks cross-repo cords and task
links. `detail-panel.test.cjs` opens a task that sets `context_refs` and
`checks`, and one that does not. Neither needs an application server. Missing
browser dependencies fail the run. Temporary files and the browser are cleaned
up.
