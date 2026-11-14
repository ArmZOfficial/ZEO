// ZEO chrome — แสดงผลจาก state ที่ Rust ส่งมา; ทุกการกระทำส่งกลับผ่าน zen.send
(function () {
  const $ = (id) => document.getElementById(id);
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const I = (name, size, sw) => ZenIcons.svg(name, size, sw);

  const LEVELS = { standard: "มาตรฐาน", strict: "เข้มงวด", off: "ปิด" };

  // ---------- state ----------
  const state = {
    tabs: [], group: null,
    nav: { url: "", host: "", secure: true, canBack: false, canForward: false, loading: false, bookmarked: false },
    bookmarks: { bar: [], folders: [] },
    shields: { host: "", enabled: true, level: "standard", ads: 0, trackers: 0, savedMB: 0 },
    profile: { name: "", label: "" },
    zoom: 100,
    pop: null,       // "menu" | "bookmark" | "shields" | null
    editingUrl: false
  };

  // ---------- render ----------
  function renderTabs() {
    $("tabs").innerHTML = state.tabs.map((t) => `
      <button class="tab" role="tab" data-id="${esc(t.id)}" aria-selected="${t.active ? "true" : "false"}" title="${esc(t.title)}">
        <span class="fav">${t.favicon ? `<img src="${esc(t.favicon)}" alt="">` : esc((t.host || t.title || "?")[0].toUpperCase())}</span>
        <span class="title">${esc(t.title)}</span>
        ${t.audible ? `<span class="audible">${I("volume", 11, 2)}</span>` : ""}
        <span class="x" data-close="${esc(t.id)}" role="button" aria-label="ปิดแท็บ">${I("close", 9, 2.4)}</span>
      </button>`).join("");
    const g = state.group;
    $("group-chip").hidden = !g;
    if (g) {
      $("group-name").textContent = g.name;
      $("group-chip").querySelector(".dot").style.background = g.color || "var(--accent)";
    }
  }

  function renderNav() {
    const n = state.nav;
    $("btn-back").disabled = !n.canBack;
    $("btn-forward").disabled = !n.canForward;
    $("omni-host").textContent = n.host || "แท็บใหม่";
    $("omni-secure").textContent = n.host ? (n.secure ? " · ปลอดภัย" : " · ไม่ปลอดภัย") : "";
    $("btn-star").classList.toggle("on", !!n.bookmarked);
    $("btn-star").setAttribute("aria-label", n.bookmarked ? "แก้ไขบุ๊กมาร์ก (Ctrl+D)" : "เพิ่มบุ๊กมาร์ก (Ctrl+D)");
  }

  function renderBookmarks() {
    $("bm-list").innerHTML = state.bookmarks.bar.map((b) => `
      <button class="bm-item" data-bm="${esc(b.id)}" title="${esc(b.url || b.title)}">
        ${b.folder ? I("folder", 12, 2) : `<span class="fav">${b.favicon ? `<img src="${esc(b.favicon)}" alt="">` : esc((b.title || "?")[0].toUpperCase())}</span>`}
        ${esc(b.title)}
      </button>`).join("");
    $("bm-folder").innerHTML = state.bookmarks.folders.map((f) => `<option value="${esc(f.id)}">${esc(f.name)}</option>`).join("");
  }

  function renderShields() {
    const s = state.shields;
    const on = s.enabled && s.level !== "off";
    $("shield-count").textContent = on ? String(s.ads + s.trackers) : "0";
    $("btn-shields").classList.toggle("off", !on);
    $("sh-host").textContent = s.host;
    $("sh-toggle").setAttribute("aria-pressed", String(on));
    $("sh-label").textContent = on ? "บล็อกโฆษณาและตัวติดตามอยู่" : "ปิดการป้องกันสำหรับเว็บนี้";
    $("sh-ads").textContent = on ? s.ads : 0;
    $("sh-trackers").textContent = on ? s.trackers : 0;
    $("sh-mb").textContent = on ? Number(s.savedMB).toFixed(1) : "0.0";
    $("pop-shields").classList.toggle("off", !on);
    document.querySelectorAll("#sh-level [data-level]").forEach((b) => b.setAttribute("aria-checked", String(b.dataset.level === s.level)));
  }

  function renderProfile() {
    const p = state.profile;
    $("profile-label").textContent = p.label || p.name;
    const av = $("btn-profile");
    if (p.avatar) av.innerHTML = `<img src="${esc(p.avatar)}" alt="">`;
    else av.textContent = (p.name || "?")[0].toUpperCase();
  }

  function renderZoom() { $("zoom-val").textContent = state.zoom + "%"; }

  function setPop(name) {
    state.pop = state.pop === name ? null : name;
    ["menu", "bookmark", "shields"].forEach((p) => { $("pop-" + p).hidden = state.pop !== p; });
    $("btn-profile").setAttribute("aria-expanded", String(state.pop === "menu"));
    if (state.pop === "bookmark") {
      $("bm-name").value = currentTab()?.title || state.nav.host;
      setTimeout(() => $("bm-name").select(), 0);
    }
    // ให้ Rust รู้ว่าต้องให้ UI view ทับหน้าเว็บ (popover ล้นลงมาเกินแถบ)
    const el = state.pop && $("pop-" + state.pop);
    zen.send("ui:popover", { open: !!state.pop, rect: el ? el.getBoundingClientRect().toJSON() : null });
  }

  const currentTab = () => state.tabs.find((t) => t.active);

  // ---------- omnibox ----------
  function editUrl(on) {
    state.editingUrl = on;
    $("omni-display").hidden = on;
    $("omni-input").hidden = !on;
    if (on) { $("omni-input").value = state.nav.url; $("omni-input").focus(); $("omni-input").select(); }
  }
  $("omnibox").addEventListener("click", (e) => { if (!e.target.closest("button") && !state.editingUrl) editUrl(true); });
  $("omni-input").addEventListener("keydown", (e) => {
    if (e.key === "Enter") { zen.send("nav:go", { input: e.target.value.trim() }); editUrl(false); }
    if (e.key === "Escape") editUrl(false);
  });
  $("omni-input").addEventListener("blur", () => editUrl(false));

  // ---------- events ----------
  $("btn-back").onclick = () => zen.send("nav:back");
  $("btn-forward").onclick = () => zen.send("nav:forward");
  $("btn-reload").onclick = () => zen.send("nav:reload");
  $("btn-newtab").onclick = () => zen.send("tab:new");
  $("btn-downloads").onclick = () => zen.send("open:internal", { page: "downloads" });
  $("btn-profile").onclick = () => setPop("menu");
  $("btn-shields").onclick = () => setPop("shields");
  $("btn-star").onclick = () => {
    if (!state.nav.bookmarked) {
      zen.send("bookmark:add", { url: state.nav.url, title: currentTab()?.title || "", folderId: state.bookmarks.folders[0]?.id });
      state.nav.bookmarked = true; renderNav();
    }
    setPop("bookmark");
  };
  document.querySelectorAll("[data-win]").forEach((b) => (b.onclick = () => zen.send("window:" + b.dataset.win)));

  $("tabs").addEventListener("click", (e) => {
    const close = e.target.closest("[data-close]");
    if (close) { e.stopPropagation(); zen.send("tab:close", { id: close.dataset.close }); if (zen.demo) demo.closeTab(close.dataset.close); return; }
    const tab = e.target.closest(".tab");
    if (tab) { zen.send("tab:select", { id: tab.dataset.id }); if (zen.demo) demo.selectTab(tab.dataset.id); }
  });
  $("tabs").addEventListener("auxclick", (e) => {
    const tab = e.target.closest(".tab");
    if (tab && e.button === 1) zen.send("tab:close", { id: tab.dataset.id });
  });

  $("bm-list").addEventListener("click", (e) => {
    const b = e.target.closest("[data-bm]");
    if (b) zen.send("bookmark:open", { id: b.dataset.bm });
  });

  $("pop-bookmark").addEventListener("submit", (e) => {
    e.preventDefault();
    zen.send("bookmark:update", { url: state.nav.url, title: $("bm-name").value, folderId: $("bm-folder").value });
    setPop(null);
  });
  $("bm-remove").onclick = () => {
    zen.send("bookmark:remove", { url: state.nav.url });
    state.nav.bookmarked = false; renderNav(); setPop(null);
  };

  $("sh-toggle").onclick = () => {
    state.shields.enabled = !state.shields.enabled;
    if (state.shields.enabled && state.shields.level === "off") state.shields.level = "standard";
    zen.send("shields:set-site", { host: state.shields.host, enabled: state.shields.enabled });
    renderShields();
  };
  $("sh-level").addEventListener("click", (e) => {
    const b = e.target.closest("[data-level]");
    if (!b) return;
    state.shields.level = b.dataset.level;
    zen.send("shields:set-level", { level: b.dataset.level });
    renderShields();
  });
  $("sh-broken").onclick = () => { zen.send("shields:report-broken", { host: state.shields.host }); setPop(null); };

  $("pop-menu").addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b || b.classList.contains("zoom-btn")) return;
    if (b.dataset.cmd === "window:new-private") zen.send("window:new", { private: true });
    else if (b.dataset.cmd) zen.send(b.dataset.cmd);
    else if (b.dataset.open) zen.send("open:internal", { page: b.dataset.open });
    if (b.dataset.pop) { state.pop = null; setPop(b.dataset.pop); return; }
    setPop(null);
  });
  document.querySelectorAll("[data-open]").forEach((b) => {
    if (!b.closest("#pop-menu")) b.onclick = () => zen.send("open:internal", { page: b.dataset.open });
  });
  const zoomBy = (d) => { state.zoom = Math.max(30, Math.min(300, state.zoom + d)); zen.send("page:zoom", { delta: d }); renderZoom(); };
  $("zoom-in").onclick = () => zoomBy(10);
  $("zoom-out").onclick = () => zoomBy(-10);

  // ปิด popover เมื่อคลิกข้างนอก / กด Esc
  document.addEventListener("mousedown", (e) => {
    if (!state.pop) return;
    if (e.target.closest(".pop, #btn-profile, #btn-shields, #btn-star")) return;
    setPop(null);
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && state.pop) setPop(null);
    if (e.ctrlKey && e.key.toLowerCase() === "l") { e.preventDefault(); editUrl(true); }
    if (e.ctrlKey && e.key.toLowerCase() === "d") { e.preventDefault(); $("btn-star").click(); }
  });

  // ---------- รับ state จาก Rust ----------
  zen.on("state:tabs", (p) => { state.tabs = p.tabs; state.group = p.group || null; renderTabs(); });
  zen.on("state:nav", (p) => { Object.assign(state.nav, p); renderNav(); });
  zen.on("state:bookmarks", (p) => { state.bookmarks = p; renderBookmarks(); });
  zen.on("state:shields", (p) => { Object.assign(state.shields, p); renderShields(); });
  zen.on("state:profile", (p) => { state.profile = p; renderProfile(); });
  zen.on("state:zoom", (p) => { state.zoom = p.percent; renderZoom(); });
  zen.on("ui:bookmark-bar", (p) => { $("bookmark-bar").hidden = !p.visible; });

  ZenIcons.hydrate();

  // ---------- โหมด demo (ไม่มี native) ----------
  const demo = {
    pages: {
      t1: { title: "zed-industries/zed", host: "github.com", url: "https://github.com/zed-industries/zed", ads: 0, trackers: 6, mb: 0.4, bm: true },
      t2: { title: "Rust ใน 100 วินาที", host: "youtube.com", url: "https://youtube.com/watch?v=5C_HPTJg5ek", ads: 11, trackers: 23, mb: 8.2, audible: true },
      t3: { title: "adblock — Rust", host: "docs.rs", url: "https://docs.rs/adblock/latest", ads: 0, trackers: 2, mb: 0.1, bm: true },
      t4: { title: "ข่าวเทคโนโลยีวันนี้", host: "sanook.com", url: "https://sanook.com/hitech", ads: 37, trackers: 41, mb: 12.6 },
      t5: { title: "cef — crates.io", host: "crates.io", url: "https://crates.io/crates/cef", ads: 0, trackers: 1, mb: 0 }
    },
    active: "t1",
    push() {
      const ids = Object.keys(this.pages);
      zen.receive({ type: "state:tabs", payload: { group: { name: "งาน" }, tabs: ids.map((id) => ({ id, title: this.pages[id].title, host: this.pages[id].host, audible: !!this.pages[id].audible, active: id === this.active })) } });
      const p = this.pages[this.active];
      if (!p) return;
      zen.receive({ type: "state:nav", payload: { url: p.url, host: p.host, secure: true, canBack: true, canForward: false, bookmarked: !!p.bm } });
      zen.receive({ type: "state:shields", payload: { host: p.host, ads: p.ads, trackers: p.trackers, savedMB: p.mb } });
    },
    selectTab(id) { this.active = id; this.push(); },
    closeTab(id) {
      delete this.pages[id];
      if (this.active === id) this.active = Object.keys(this.pages)[0];
      this.push();
    }
  };

  if (zen.demo) {
    document.documentElement.classList.add("demo");
    zen.receive({ type: "state:profile", payload: { name: "ArmZ", label: "ArmZ · ส่วนตัว" } });
    zen.receive({ type: "state:bookmarks", payload: {
      folders: [{ id: "bar", name: "แถบบุ๊กมาร์ก" }, { id: "zen-dev", name: "ZEO Dev" }, { id: "rust", name: "Rust" }, { id: "other", name: "บุ๊กมาร์กอื่นๆ" }],
      bar: [
        { id: "zen-dev", title: "ZEO Dev", folder: true },
        { id: "rust", title: "Rust", folder: true },
        { id: "b1", title: "GitHub", url: "https://github.com" },
        { id: "b2", title: "docs.rs", url: "https://docs.rs" },
        { id: "b3", title: "crates.io", url: "https://crates.io" },
        { id: "b4", title: "YouTube", url: "https://youtube.com" },
        { id: "b5", title: "Sanook", url: "https://sanook.com" }
      ] } });
    demo.push();
  } else {
    zen.send("ui:ready", { view: "chrome" });
  }
  renderZoom();
})();
