// ZEO UI ↔ Rust bridge (ฝั่ง UI ของ zeo-ipc)
//
// ฝั่ง Rust ต้องเปิดให้หน้า zeo:// เท่านั้นเรียกได้ (plan.md หัวข้อ 3, 8):
//   - UI → Rust : window.__zenNative.postMessage(JSON.stringify({ type, payload }))
//                 (ผูกผ่าน CEF message router / cefQuery หรือ WebView2 postMessage)
//   - Rust → UI : เรียก window.zen.receive({ type, payload }) ด้วย ExecuteJavaScript
//
// ถ้าไม่มี native (เปิดไฟล์ในเบราว์เซอร์ธรรมดา) จะเข้าโหมด demo:
// ส่งข้อความไปที่ console และให้แต่ละหน้าใช้ข้อมูลตัวอย่างแทน
(function () {
  const listeners = new Map();
  const native = window.__zenNative;

  const zen = {
    demo: !native,
    send(type, payload = {}) {
      const msg = { type, payload };
      if (native) native.postMessage(JSON.stringify(msg));
      else console.debug("[zen:send]", type, payload);
    },
    on(type, fn) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type).add(fn);
      return () => listeners.get(type).delete(fn);
    },
    receive(msg) {
      const m = typeof msg === "string" ? JSON.parse(msg) : msg;
      (listeners.get(m.type) || []).forEach((fn) => fn(m.payload));
    },
    setAccent(hex) { document.documentElement.style.setProperty("--accent", hex); }
  };

  window.zen = zen;
  zen.on("theme:accent", ({ hex }) => zen.setAccent(hex));
})();

/* ข้อความทั้งหมด (สรุป) — ให้ตรงกับ enum ใน crates/zeo-ipc

UI → Rust
  nav:back | nav:forward | nav:reload                {}
  nav:go                                             { input }           // omnibox: URL หรือคำค้น
  tab:select | tab:close                             { id }
  tab:new                                            { private?: bool }
  window:new                                         { private?: bool }
  window:minimize | window:maximize | window:close   {}
  bookmark:add | bookmark:update                     { url, title, folderId }
  bookmark:remove                                    { url }
  bookmark:open                                      { id }
  shields:set-site                                   { host, enabled }
  shields:set-level                                  { level }           // "standard" | "strict" | "off"
  shields:report-broken                              { host }            // ปิด Shields เว็บนี้ + reload
  page:find | page:zoom                              { delta? }
  open:internal                                      { page }            // "bookmarks" | "history" | "downloads" | "settings"
  ui:popover                                         { open: bool, rect } // ให้ Rust ขยาย/หด UI view ตอนเปิดเมนู
  ui:ready                                           { view }            // "chrome" | "newtab" | "palette"
  palette:run                                        { id }              // id ของคำสั่งใน palette.js (COMMANDS)
  palette:close                                      {}

Rust → UI
  state:tabs        { tabs: [{ id, title, host, favicon?, audible?, active }], group?: { name, color } }
  state:nav         { url, host, secure, canBack, canForward, loading, bookmarked }
  state:bookmarks   { bar: [{ id, title, url?, favicon?, folder? }], folders: [{ id, name }] }
  state:shields     { host, enabled, level, ads, trackers, savedMB }
  state:profile     { name, label, avatar? }
  state:zoom        { percent }
  stats:global      { blocked, savedBytes, savedSeconds }
  theme:accent      { hex }
  ui:bookmark-bar   { visible }
  newtab:sites      { sites: [{ title, url, favicon? }] }
  search:engine     { name }
  palette:open      {}
  palette:data      { tabs: [{ id, title, host, audible? }], bookmarks: [{ id, title, url }] }
*/
