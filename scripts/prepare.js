// 把游戏本体（game/index.html，原样不改）复制到 dist/ 供 Tauri 打包
const fs = require('fs'), path = require('path');
const root = path.join(__dirname, '..');
fs.mkdirSync(path.join(root, 'dist'), { recursive: true });
fs.copyFileSync(path.join(root, 'game', 'index.html'), path.join(root, 'dist', 'index.html'));
console.log('game copied to dist/index.html');
