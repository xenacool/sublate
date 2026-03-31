const http = require('http');
const fs = require('fs');
const path = require('path');

const publicDir = path.resolve(__dirname, process.argv[2] || 'target/dx/web/debug/web/public');
const port = 8080;

http.createServer((req, res) => {
    let urlPath = req.url.split('?')[0];
    let filePath = path.join(publicDir, urlPath === '/' ? 'index.html' : urlPath);
    
    if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
        filePath = path.join(publicDir, 'index.html');
    }
    
    const ext = path.extname(filePath);
    const contentTypes = {
        '.html': 'text/html',
        '.js': 'text/javascript',
        '.css': 'text/css',
        '.wasm': 'application/wasm',
        '.svg': 'image/svg+xml',
        '.ico': 'image/x-icon',
    };
    
    try {
        const data = fs.readFileSync(filePath);
        res.writeHead(200, { 'Content-Type': contentTypes[ext] || 'text/plain' });
        res.end(data);
    } catch (err) {
        res.writeHead(404);
        res.end('Not found');
    }
}).listen(port, () => {
    console.log(`SPA Server running at http://localhost:${port}`);
});
