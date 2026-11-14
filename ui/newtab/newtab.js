(function () {
  const $ = (id) => document.getElementById(id);
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

  // นาฬิกา
  const fmtTime = new Intl.DateTimeFormat("th-TH", { hour: "2-digit", minute: "2-digit", hour12: false });
  const fmtDate = new Intl.DateTimeFormat("th-TH", { weekday: "long", day: "numeric", month: "long" });
  function tick() { const d = new Date(); $("time").textContent = fmtTime.format(d); $("date").textContent = fmtDate.format(d); }
  tick(); setInterval(tick, 10_000);

  // ค้นหา
  $("search").addEventListener("submit", (e) => {
    e.preventDefault();
    const v = $("q").value.trim();
    if (v) zen.send("nav:go", { input: v });
  });
  $("engine").onclick = () => zen.send("open:internal", { page: "settings#search" });

  // เว็บที่ใช้บ่อย
  function renderSites(sites) {
    $("sites").innerHTML = sites.map((s) => `
      <a class="site" href="${esc(s.url)}">
        <span class="ico">${s.add ? ZenIcons.svg("plus", 16, 2) : s.favicon ? `<img src="${esc(s.favicon)}" alt="">` : esc((s.title || "?")[0].toUpperCase())}</span>
        <span class="name">${esc(s.title)}</span>
      </a>`).join("");
  }

  // สถิติ Shields
  const nf = new Intl.NumberFormat("th-TH");
  function bytes(n) { return n >= 1e9 ? [(n / 1e9).toFixed(1), "GB"] : [String(Math.round(n / 1e6)), "MB"]; }
  function duration(sec) { return sec >= 3600 ? [(sec / 3600).toFixed(1), "ชม."] : [String(Math.round(sec / 60)), "นาที"]; }
  function setValue(el, [num, unit]) { el.textContent = num; const u = document.createElement("small"); u.textContent = unit; el.append(u); }
  function renderStats(s) {
    $("st-blocked").textContent = nf.format(s.blocked);
    setValue($("st-bytes"), bytes(s.savedBytes));
    setValue($("st-time"), duration(s.savedSeconds));
  }

  zen.on("newtab:sites", (p) => renderSites(p.sites));
  zen.on("stats:global", renderStats);
  zen.on("search:engine", (p) => { $("engine").textContent = p.name; });

  ZenIcons.hydrate();

  if (zen.demo) {
    document.documentElement.classList.add("demo");
    renderSites([
      { title: "GitHub", url: "https://github.com" },
      { title: "YouTube", url: "https://youtube.com" },
      { title: "docs.rs", url: "https://docs.rs" },
      { title: "crates.io", url: "https://crates.io" },
      { title: "Sanook", url: "https://sanook.com" },
      { title: "เพิ่ม", url: "zeo://settings#newtab", add: true }
    ]);
    renderStats({ blocked: 12480, savedBytes: 1.2e9, savedSeconds: 840 });
  } else {
    zen.send("ui:ready", { view: "newtab" });
  }
})();
