// ZEO icon set — เส้น 24x24, stroke เดียวกันทั้งชุด
// ใช้: ZenIcons.svg("shield", 14)  หรือ  <i data-icon="shield" data-size="14"></i> แล้วเรียก ZenIcons.hydrate()
(function () {
  const P = {
    back: "M15 18l-6-6 6-6",
    forward: "M9 18l6-6-6-6",
    reload: "M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7",
    star: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z",
    shield: "M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6z",
    shieldCheck: "M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6zM9 12l2 2 4-4",
    download: "M12 4v11M7 10l5 5 5-5M5 20h14",
    plus: "M12 5v14M5 12h14",
    close: "M6 6l12 12M18 6L6 18",
    minimize: "M5 12h14",
    maximize: "M5 5h14v14H5z",
    folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    volume: "M4 9v6h4l5 4V5L8 9zM16.5 8.5a5 5 0 0 1 0 7",
    window: "M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zM4 9h16",
    private: "M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.9 5.1A10 10 0 0 1 12 5c5 0 9 4.5 10 7a13 13 0 0 1-2.4 3.4M6.6 6.6C4.4 8 2.8 10 2 12c1 2.5 5 7 10 7a9.8 9.8 0 0 0 5.4-1.6",
    search: "M11 4a7 7 0 1 1 0 14a7 7 0 0 1 0-14zM20 20l-4-4",
    zoom: "M11 4a7 7 0 1 1 0 14a7 7 0 0 1 0-14zM20 20l-4-4M8 11h6M11 8v6",
    history: "M3 12a9 9 0 1 0 3-6.7M3 4v5h5M12 7v5l3 2",
    settings: "M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4",
    sidebar: "M4 5h16v14H4zM9 5v14",
    reader: "M4 5a2 2 0 0 1 2-2h12v16H6a2 2 0 0 0-2 2zM4 5v16M8 7h6M8 11h6",
    trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
    lock: "M5 11h14v10H5zM8 11V8a4 4 0 0 1 8 0v3",
    warning: "M12 9v4M12 17h.01M10.3 3.9L2.5 17.5a2 2 0 0 0 1.7 3h15.6a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
    chevronDown: "M6 9l6 6 6-6"
  };
  function svg(name, size = 14, strokeWidth = 1.9) {
    const d = P[name];
    if (!d) { console.warn("[ZenIcons] ไม่มีไอคอน:", name); return ""; }
    return `<svg aria-hidden="true" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="${strokeWidth}" stroke-linecap="round" stroke-linejoin="round"><path d="${d}"></path></svg>`;
  }
  function hydrate(root = document) {
    root.querySelectorAll("[data-icon]").forEach((el) => {
      el.outerHTML = svg(el.dataset.icon, Number(el.dataset.size || 14), Number(el.dataset.stroke || 1.9));
    });
  }
  window.ZenIcons = { svg, hydrate, names: Object.keys(P) };
})();
