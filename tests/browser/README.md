Run the portfolio interaction regression with Node.js and Chromium:

```sh
cd tests/browser
npm ci
npx playwright install chromium
npm test
```

The test builds rmap, renders temporary JSON projects, and opens the generated
HTML in headless Chromium. It needs no application server. Missing browser
dependencies fail the run. Temporary files and the browser are cleaned up.
