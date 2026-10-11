#!/usr/bin/env bash
# 打包 Web 测试页到 testweb/。
# 用法：scripts/build_web.sh
# 测试：python -m http.server 8000 -d testweb  （或任意静态服务器）→ http://localhost:8000
set -euo pipefail
cd "$(dirname "$0")/.."

cargo build --release --target wasm32-unknown-unknown --example web_demo

wasm-bindgen --target web --out-dir testweb --out-name web_demo \
    target/wasm32-unknown-unknown/release/examples/web_demo.wasm

cat > testweb/index.html <<'HTML'
<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>starfish-we web demo</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; background: #16181d; }
  canvas { display: block; width: 100vw; height: 100vh; background: #232733; }
</style>
</head>
<body>
<canvas id="starfish-we"></canvas>
<script type="module">
  import init from './web_demo.js';
  init().catch((e) => {
    document.body.textContent = 'Init failed: ' + e;
  });
</script>
</body>
</html>
HTML

echo "打包完成 → testweb/"
echo "测试：python -m http.server 8000 -d testweb  然后浏览器打开 http://localhost:8000"
