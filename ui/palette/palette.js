(function () {
  const $ = (id) => document.getElementById(id);
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

  // คำสั่งคงที่ — id ตรงกับ palette:run ฝั่ง Rust
  const COMMANDS = [
    { id: "tab:new", icon: "plus", label: "แท็บใหม่", sub: "tab: new", keys: ["Ctrl", "T"] },
    { id: "window:new-private", icon: "private", label: "หน้าต่างส่วนตัวใหม่", sub: "window: private", keys: ["Ctrl", "⇧", "N"] },
    { id: "shields:toggle-site", icon: "shieldCheck", label: "เปิด/ปิด Shields สำหรับเว็บนี้", sub: "shields: toggle", keys: ["Alt", "S"] },
    { id: "view:toggle-bookmark-bar", icon: "star", label: "แสดง/ซ่อนแถบบุ๊กมาร์ก", sub: "view: bookmarks bar", keys: ["Ctrl", "⇧", "B"] },
    { id: "view:reader", icon: "reader", label: "โหมดอ่าน", sub: "view: reader", keys: ["F9"] },
    { id: "page:find", icon: "search", label: "ค้นหาในหน้า", sub: "page: find", keys: ["Ctrl", "F"] },
    { id: "open:history", icon: "history", label: "ประวัติ", sub: "open: history", keys: ["Ctrl", "H"] },
    { id: "open:downloads", icon: "download", label: "ดาวน์โหลด", sub: "open: downloads", keys: ["Ctrl", "J"] },
    { id: "privacy:clear", icon: "trash", label: "ล้างข้อมูลการท่องเว็บ", sub: "privacy: clear data", keys: ["Ctrl", "⇧", "Del"] },
    { id: "open:settings", icon: "settings", label: "ตั้งค่า", sub: "open: settings", keys: ["Ctrl", ","] }
  ];

  let tabs = [];      // [{ id, title, host, audible? }]
  let bookmarks = []; // [{ id, title, url }]
  let query = "";
  let sel = 0;
  let flat = [];

  // fuzzy แบบง่าย: ทุกตัวอักษรต้องเรียงตามลำดับ, ให้คะแนนตัวที่ติดกัน
  function score(text, q) {
    if (!q) return 1;
    const t = text.toLowerCase(); let ti = 0, s = 0, run = 0;
    for (const ch of q) {
      const i = t.indexOf(ch, ti);
      if (i < 0) return 0;
      run = i === ti ? run + 1 : 0; s += 1 + run; ti = i + 1;
    }
    return s + (t.startsWith(q) ? 10 : 0);
  }
  function highlight(text, q) {
    if (!q) return esc(text);
    const lower = text.toLowerCase(); let ti = 0, out = "";
    const hits = new Set();
    for (const ch of q) { const i = lower.indexOf(ch, ti); if (i < 0) return esc(text); hits.add(i); ti = i + 1; }
    [...text].forEach((c, i) => { out += hits.has(i) ? `<mark>${esc(c)}</mark>` : esc(c); });
    return out;
  }

  function build() {
    const q = query.trim().toLowerCase();
    const pick = (arr, f) => arr.map((x) => ({ x, s: score(f(x), q) })).filter((r) => r.s > 0).sort((a, b) => b.s - a.s).map((r) => r.x);
    const groups = [
      { name: "แท็บที่เปิดอยู่", items: pick(tabs, (t) => t.title + " " + t.host).map((t) => ({ kind: "tab", id: t.id, letter: (t.host || t.title || "?")[0].toUpperCase(), label: t.title, sub: t.host + (t.audible ? " · กำลังเล่นเสียง" : ""), keys: [] })) },
      { name: "คำสั่ง", items: pick(COMMANDS, (c) => c.label + " " + c.sub).map((c) => ({ kind: "cmd", ...c })) },
      { name: "บุ๊กมาร์ก", items: q ? pick(bookmarks, (b) => b.title + " " + b.url).slice(0, 5).map((b) => ({ kind: "bookmark", id: b.id, icon: "star", label: b.title, sub: b.url, keys: [] })) : [] }
    ].filter((g) => g.items.length);
    flat = groups.flatMap((g) => g.items);
    sel = Math.min(sel, Math.max(0, flat.length - 1));
    return groups;
  }

  function render() {
    const groups = build();
    const q = query.trim().toLowerCase();
    let idx = 0;
    $("list").innerHTML = groups.length ? groups.map((g) => `
      <div class="p-group">${esc(g.name)}</div>
      ${g.items.map((it) => {
        const i = idx++;
        return `<button class="p-item" role="option" data-i="${i}" aria-selected="${i === sel}">
          <span class="p-ico">${it.kind === "tab" ? esc(it.letter) : ZenIcons.svg(it.icon, 14)}</span>
          <span class="p-text"><span class="p-label">${highlight(it.label, q)}</span><span class="p-sub">${esc(it.sub)}</span></span>
          ${it.keys.map((k) => `<span class="p-key">${esc(k)}</span>`).join("")}
        </button>`;
      }).join("")}`).join("")
      : `<div class="p-empty">ไม่พบผลลัพธ์ — กด Enter เพื่อค้นหาบนเว็บ</div>`;
    $("list").querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }

  function run(i, newTab) {
    const it = flat[i];
    if (!it) { if (query.trim()) zen.send("nav:go", { input: query.trim(), newTab }); close(); return; }
    if (it.kind === "tab") zen.send("tab:select", { id: it.id });
    else if (it.kind === "bookmark") zen.send("bookmark:open", { id: it.id, newTab });
    else zen.send("palette:run", { id: it.id });
    close();
  }
  function close() { zen.send("palette:close"); if (zen.demo) { query = ""; $("cmd").value = ""; sel = 0; render(); } }

  $("cmd").addEventListener("input", (e) => { query = e.target.value; sel = 0; render(); });
  $("cmd").addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(flat.length - 1, sel + 1); render(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(0, sel - 1); render(); }
    else if (e.key === "Enter") { e.preventDefault(); run(sel, e.ctrlKey); }
    else if (e.key === "Escape") { e.preventDefault(); close(); }
  });
  $("list").addEventListener("mousemove", (e) => {
    const b = e.target.closest(".p-item"); if (!b) return;
    const i = Number(b.dataset.i); if (i !== sel) { sel = i; render(); }
  });
  $("list").addEventListener("click", (e) => { const b = e.target.closest(".p-item"); if (b) run(Number(b.dataset.i), e.ctrlKey); });
  $("scrim").onclick = close;

  zen.on("palette:data", (p) => { tabs = p.tabs || []; bookmarks = p.bookmarks || []; render(); });
  zen.on("palette:open", () => { query = ""; $("cmd").value = ""; sel = 0; render(); $("cmd").focus(); });

  ZenIcons.hydrate();
  if (zen.demo) {
    document.documentElement.classList.add("demo");
    tabs = [
      { id: "t1", title: "zed-industries/zed", host: "github.com" },
      { id: "t2", title: "Rust ใน 100 วินาที", host: "youtube.com", audible: true },
      { id: "t3", title: "adblock — Rust", host: "docs.rs" }
    ];
    bookmarks = [{ id: "b1", title: "GitHub", url: "github.com" }, { id: "b2", title: "docs.rs", url: "docs.rs" }, { id: "b3", title: "crates.io", url: "crates.io" }];
  } else {
    zen.send("ui:ready", { view: "palette" });
  }
  render();
  $("cmd").focus();
})();
