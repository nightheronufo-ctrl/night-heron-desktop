// 夜鹭骑行 · 桌面小窗外壳（注入到游戏页面，不改游戏本体）
// © 2026 不明飞行鹭 · CC BY-NC-ND 4.0
(function () {
  if (window.top !== window || window.__nhShell) return;
  window.__nhShell = true;

  var isMac = /Mac/.test(navigator.platform || navigator.userAgent);
  var inv = function (c, a) {
    try { return window.__TAURI__ ? window.__TAURI__.core.invoke(c, a || {}) : Promise.resolve(); }
    catch (e) { return Promise.resolve(); }
  };
  var lang = function () {
    try { return typeof LANG !== 'undefined' ? LANG : (localStorage.getItem('nightheron_lang') || 'zh'); }
    catch (e) { return 'zh'; }
  };
  var T = {
    zh: { pin: '一直置顶', unpin: '取消置顶', min: '最小化', close: '关闭', drag: '拖动这里移动窗口 · 双击全屏' },
    en: { pin: 'Always on top', unpin: 'Stop staying on top', min: 'Minimize', close: 'Close', drag: 'Drag to move · double-click for full screen' }
  };

  var R = 12; // 圆角
  var css = [
    // 圆角卡片：Mac 用透明窗口裁出圆角；Windows 11 由系统给圆角
    'html.nh-mac{background:transparent!important;clip-path:inset(0 round ' + R + 'px);-webkit-clip-path:inset(0 round ' + R + 'px);}',
    'html.nh-mac body{background:transparent!important;}',
    'html.nh-mac.nh-full{clip-path:none;-webkit-clip-path:none;}',
    // 悬停拖动条
    '#nhbar{position:fixed;left:0;right:0;top:0;height:30px;z-index:2147483000;display:flex;align-items:center;gap:6px;padding:0 8px 0 12px;box-sizing:border-box;',
    '  background:linear-gradient(rgba(12,20,34,.50),rgba(12,20,34,.30));backdrop-filter:blur(8px) saturate(1.2);-webkit-backdrop-filter:blur(8px) saturate(1.2);',
    '  border-bottom:1px solid rgba(255,255,255,.10);color:rgba(255,255,255,.88);font:500 12px/1 "PingFang SC","Microsoft YaHei","Noto Sans SC",system-ui,sans-serif;letter-spacing:.08em;',
    '  transform:translateY(-100%);opacity:0;transition:transform .22s ease,opacity .22s ease;user-select:none;-webkit-user-select:none;cursor:grab;}',
    'html.nh-bar #nhbar{transform:none;opacity:1;}',
    '#nhbar .nh-t{flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;pointer-events:none;opacity:.85}',
    '#nhbar .nh-t i{font-style:normal;opacity:.55;margin-left:8px;letter-spacing:.02em}',
    '#nhbar button{all:unset;box-sizing:border-box;width:22px;height:22px;border-radius:50%;display:grid;place-items:center;cursor:pointer;color:#fff;background:rgba(255,255,255,.10);transition:background .15s,transform .15s}',
    '#nhbar button:hover{background:rgba(255,255,255,.26)}',
    '#nhbar button:active{transform:scale(.92)}',
    '#nhbar button.nh-x:hover{background:#e0605a}',
    '#nhbar button.on{background:rgba(255,214,120,.92);color:#2a2a2a}',
    '#nhbar svg{width:12px;height:12px;display:block}',
    // 拖动条出现时，游戏右上角的按钮和左上角地名一起往下让一点
    '#snd,#bag,#mind,#wake,#place,#hint{transition:translate .22s ease;}',
    'html.nh-bar #snd,html.nh-bar #bag,html.nh-bar #mind,html.nh-bar #wake,html.nh-bar #place,html.nh-bar #hint{translate:0 22px;}'
  ].join('\n');

  var ICON = {
    pin: '<svg viewBox="0 0 16 16" fill="currentColor"><path d="M10.6 1.4l4 4-1.4.7-2.6 2.6.3 3.3-1.4 1.4-2.8-2.8-3.6 3.6-.8-.8 3.6-3.6L3.1 7l1.4-1.4 3.3.3 2.6-2.6z"/></svg>',
    min: '<svg viewBox="0 0 16 16"><path d="M3.5 8.5h9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>',
    close: '<svg viewBox="0 0 16 16"><path d="M4.2 4.2l7.6 7.6M11.8 4.2l-7.6 7.6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>'
  };

  var state = { pin: false, full: false };
  var bar, bPin, bMin, bClose, title;

  function label() {
    var t = T[lang() === 'en' ? 'en' : 'zh'];
    title.innerHTML = (lang() === 'en' ? 'Night Heron on Wheels' : '夜鹭骑行') + '<i>© 不明飞行鹭</i>';
    bar.title = t.drag;
    bPin.title = state.pin ? t.unpin : t.pin;
    bPin.setAttribute('aria-label', bPin.title);
    bMin.title = t.min; bMin.setAttribute('aria-label', t.min);
    bClose.title = t.close; bClose.setAttribute('aria-label', t.close);
  }
  function apply(s) {
    if (s) { state.pin = !!s.pin; state.full = !!s.full; }
    document.documentElement.classList.toggle('nh-full', state.full);
    if (bPin) { bPin.classList.toggle('on', state.pin); label(); }
  }

  // 悬停显示：鼠标进入窗口时浮出，移开或 3 秒不动就收起（指着拖动条时不收）
  var hideT = 0, overBar = false;
  function show() {
    document.documentElement.classList.add('nh-bar');
    clearTimeout(hideT);
    hideT = setTimeout(function () { if (!overBar) hide(); }, 3000);
  }
  function hide() { clearTimeout(hideT); document.documentElement.classList.remove('nh-bar'); }

  function btn(cls, svg, fn) {
    var b = document.createElement('button');
    b.className = cls; b.innerHTML = svg; b.type = 'button';
    b.addEventListener('mousedown', function (e) { e.stopPropagation(); });
    b.addEventListener('click', function (e) { e.stopPropagation(); fn(); });
    b.addEventListener('dblclick', function (e) { e.stopPropagation(); });
    return b;
  }

  function build() {
    if (document.getElementById('nhbar')) return;
    var st = document.createElement('style'); st.id = 'nhstyle'; st.textContent = css;
    document.head.appendChild(st);
    if (isMac) document.documentElement.classList.add('nh-mac');

    bar = document.createElement('div'); bar.id = 'nhbar';
    title = document.createElement('div'); title.className = 'nh-t';
    bPin = btn('nh-pin', ICON.pin, function () { inv('nh_toggle_pin'); });
    bMin = btn('nh-min', ICON.min, function () { hide(); inv('nh_minimize'); });
    bClose = btn('nh-x', ICON.close, function () { inv('nh_close'); });
    bar.append(title, bPin, bMin, bClose);
    bar.addEventListener('mousedown', function (e) {
      if (e.button !== 0) return;
      e.preventDefault();
      if (e.detail === 2) inv('nh_toggle_full'); else if (e.detail === 1) inv('nh_drag');
    });
    bar.addEventListener('mouseenter', function () { overBar = true; show(); });
    bar.addEventListener('mouseleave', function () { overBar = false; show(); });
    // 不让拖动条上的点击穿到游戏里
    ['click', 'pointerdown', 'pointerup', 'touchstart'].forEach(function (k) {
      bar.addEventListener(k, function (e) { e.stopPropagation(); });
    });
    document.body.appendChild(bar);

    document.addEventListener('mousemove', show, true);
    document.documentElement.addEventListener('mouseleave', function () { overBar = false; hide(); });
    window.addEventListener('blur', function () { if (!overBar) hide(); });
    window.addEventListener('keydown', function (e) {
      if (e.key === 'F11') { e.preventDefault(); inv('nh_toggle_full'); }
    });

    label();
    inv('nh_state').then(function (s) { apply(s); });
    try { window.__TAURI__.event.listen('nh-state', function (ev) { apply(ev.payload); }); } catch (e) {}

    // 跟随游戏语言切换（托盘菜单和提示文字）
    inv('nh_set_lang', { lang: lang() });
    if (typeof window.setLang === 'function' && !window.setLang.__nh) {
      var orig = window.setLang;
      window.setLang = function () {
        var r = orig.apply(this, arguments);
        label(); inv('nh_set_lang', { lang: lang() });
        return r;
      };
      window.setLang.__nh = true;
    }
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();
})();
