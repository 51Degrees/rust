// TEMPORARY DIAGNOSTIC, see 51Degrees/rust issue 24. Remove with the
// branch that added it.
//
// Replicates the browser leg of the shared suite's SessionStorageCache
// test without Selenium: serve the same page, forward the include and
// the json endpoint to the running example, and record every request.
// The suite counts posts through its own proxy and reports only the
// count, so when the count is zero there is nothing to say why. This
// keeps the include the browser actually received, so the JavaScript
// bodies it carried can be counted.
const http = require('http');
const fs = require('fs');

const APP_PORT = Number(process.argv[2] || 8095);
const PORT = Number(process.argv[3] || 8099);
const INCLUDE_FILE = process.argv[4] || 'diag-include.js';
const log = [];

const PAGE = [
  '<!DOCTYPE html><html><head><title>diagnostic</title>',
  "<script async src='/51Degrees.core.js?fod-js-enable-cookies=true'></script>",
  '<script>',
  "window.addEventListener('load', function () {",
  "  var r = document.getElementById('results');",
  "  if (typeof fod === 'undefined') { r.setAttribute('data-state','no-fod'); return; }",
  '  fod.complete(function (data) {',
  '    var d = (data && data.device) || {};',
  "    document.getElementById('deviceid').textContent = d.deviceid || '(none)';",
  "    r.setAttribute('data-state','complete');",
  '  });',
  '});',
  '</script></head><body>',
  "<table id='results' data-state='pending'><tr><td id='deviceid'></td></tr></table>",
  '</body></html>'
].join('\n');

const server = http.createServer((req, res) => {
  const path = req.url.split('?')[0];
  if (path !== '/favicon.ico') {
    log.push(req.method + ' ' + path);
  }
  if (path === '/51Degrees.core.js' || path === '/51dpipeline/json') {
    const headers = Object.assign({}, req.headers);
    const up = http.request(
      { host: '127.0.0.1', port: APP_PORT, path: req.url, method: req.method, headers },
      (proxied) => {
        res.writeHead(proxied.statusCode, proxied.headers);
        if (path === '/51Degrees.core.js') {
          const chunks = [];
          proxied.on('data', c => { chunks.push(c); res.write(c); });
          proxied.on('end', () => {
            fs.writeFileSync(INCLUDE_FILE, Buffer.concat(chunks));
            res.end();
          });
          return;
        }
        proxied.pipe(res);
      });
    up.on('error', e => {
      log.push('PROXY ERROR ' + path + ' ' + e.message);
      res.writeHead(502); res.end();
    });
    req.pipe(up);
    return;
  }
  res.writeHead(200, { 'Content-Type': 'text/html' });
  res.end(PAGE);
});

server.listen(PORT, () => console.log('  diagnostic proxy listening on ' + PORT));

setTimeout(() => {
  console.log('  requests the browser made:');
  if (log.length === 0) { console.log('    (none)'); }
  for (const line of log) { console.log('    ' + line); }
  const posts = log.filter(l => l === 'POST /51dpipeline/json').length;
  console.log('  posts to the json endpoint: ' + posts);
  process.exit(0);
}, Number(process.env.DIAG_WAIT_MS || 25000));
