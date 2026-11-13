# 夜鹭骑行 · 桌面小窗版

把《夜鹭骑行》放进桌面角落的一张小画卡里：没有系统边框，只有圆角和淡淡的投影，开着慢慢看就好。

## 用法

- 默认停在屏幕右下角，大小约 480×300。拖边角可以改大小，下次打开会记住位置和大小。
- 鼠标移到画面上，顶部会浮出一条半透明细条：按住细条拖动窗口，双击细条全屏；右边是 **置顶 / 最小化 / 关闭**。
- 拖到屏幕边缘附近松手，会自动吸过去，贴在离边缘一点点的位置。
- 托盘（Mac 是菜单栏）里的小夜鹭：显示 / 隐藏窗口、一直置顶、全屏、回到右下角、退出。Windows 上左键点一下托盘图标就能显示 / 隐藏。
- `F11` 切换全屏。

## 第一次打开

安装包没有做付费的开发者签名，所以系统会提醒一次：

- **Windows**：如果出现“Windows 已保护你的电脑”，点 **更多信息 → 仍要运行**。
- **Mac**：打开 dmg，把“夜鹭骑行”拖进“应用程序”。第一次打开如果提示“无法验证开发者”，到 **系统设置 → 隐私与安全性**，在下方点 **仍要打开**。

## 存档

进度保存在这个桌面版自己的存储里，和浏览器里玩的网页版是两份不同的存档。

## 自己构建

```bash
npm ci
npm run build        # 会先把 game/index.html 复制到 dist/，再用 Tauri 打包
```

推送到 `main` 分支后，GitHub Actions 会自动构建 Windows（.exe 安装包 + 免安装版）和 macOS（通用 .dmg，Intel / Apple 芯片都能用），在 Actions 页面的 Artifacts 里下载。

游戏本体 `game/index.html` 原样打包，没有修改；小窗外壳在 `src-tauri/src/`。

---

## English

**Night Heron on Wheels**, as a small borderless card in the corner of your desktop. Hover to show a drag bar with *Always on top / Minimize / Close*; drag near a screen edge to snap to it; the tray icon shows, hides, pins or full-screens the window. Position and size are remembered.

---

© 2026 不明飞行鹭 · 由 不明飞行鹭 构思与设计 · 代码、美术与音效由 AI（Claude）生成 · 免费分享 · 禁止商业用途 · [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/deed.zh-hans)
