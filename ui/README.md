# ZEO UI — Glass · Graphite

หน้า `zeo://` ของ ZEO Browser ตามดีไซน์ใน Claude Design เป็น HTML/CSS/JS ล้วน ไม่มี build step และไม่โหลดอะไรจากภายนอก จึงผ่าน CSP เข้มตาม plan.md หัวข้อ 8

## ลองดูทันที

เปิดไฟล์เหล่านี้ใน Chrome หรือ Edge ได้เลย ถ้าไม่มี native bridge จะเข้า **โหมด demo** และใช้ข้อมูลตัวอย่างให้เอง

1. `chrome/index.html` แถบบน 3 แถว (แท็บ, URL, บุ๊กมาร์ก) พร้อมเมนูโปรไฟล์ แผงบุ๊กมาร์ก และแผง Shields
2. `newtab/index.html` หน้าแท็บใหม่
3. `palette/index.html` Command Palette (Ctrl+K)

ในโหมด demo ทุกการกระทำจะถูก log ไว้ใน DevTools console ในรูปแบบ `[zen:send] type payload`

## โครงสร้าง

```
ui/
├── shared/
│   ├── tokens.css     # สี ฟอนต์ กระจก ขนาด — แก้ธีมที่นี่ที่เดียว
│   ├── icons.js       # ไอคอน SVG ทั้งชุด (ZenIcons.svg / data-icon)
│   ├── bridge.js      # ตัวกลาง IPC กับ Rust + รายการข้อความทั้งหมด
│   └── fonts/         # Inter, IBM Plex Sans Thai, JetBrains Mono (OFL)
├── chrome/            # zeo://ui      — แถบบนของหน้าต่าง
├── newtab/            # zeo://newtab
└── palette/           # zeo://palette — overlay
```

## ต่อเข้ากับฝั่ง Rust

> **สถานะ Milestone 0 (WebView2):** `crates/zeo-app` เสิร์ฟโฟลเดอร์นี้ผ่าน virtual host `https://zeo.ui/` (แทน scheme `zeo://` ไปก่อน) และรับข้อความจาก origin นี้เท่านั้น ส่วนที่ต่อแล้วคือ `ui:ready`, `nav:*`, `window:*`, `ui:popover`, `shields:*`, `page:zoom`, `tab:new`, `tab:close` ข้อความอื่นถูกบันทึกเป็น `UNHANDLED`

1. **เสิร์ฟไฟล์**: ลงทะเบียน scheme `zeo://` (CEF: `CefRegisterSchemeHandlerFactory`) ให้ map `zeo://ui/*` → `ui/chrome/*`, `zeo://newtab/*` → `ui/newtab/*`, `zeo://palette/*` → `ui/palette/*` และ `zeo://shared/*` → `ui/shared/*`
   - ทุกหน้าอ้าง `../shared/...` จึงต้องเสิร์ฟให้ path สัมพัทธ์ตรงกัน หรือจะแพ็กทั้งโฟลเดอร์ `ui/` เป็นรากเดียวก็ได้
2. **Bridge**: ฉีด `window.__zenNative` ที่มี `postMessage(string)` ก่อนโหลดหน้า (CEF message router หรือ WebView2 `chrome.webview.postMessage`)
   - Rust ส่ง state กลับด้วย `window.zen.receive({type, payload})`
   - รายการข้อความทั้งหมดอยู่ท้ายไฟล์ `shared/bridge.js` ให้ทำ enum ใน `crates/zeo-ipc` ให้ตรงกัน
   - ตรวจ origin ฝั่ง Rust ทุกข้อความ: รับเฉพาะจาก `zeo://`
3. **ลำดับเริ่มต้น**: หน้าแต่ละหน้าส่ง `ui:ready {view}` มาก่อน Rust จึงค่อย push state ชุดแรก
   - chrome ต้องได้ `state:tabs`, `state:nav`, `state:bookmarks`, `state:shields` และ `state:profile`
4. **วาง view**: UI view (chrome) เต็มหน้าต่าง แล้ววาง tab view ทับพื้นที่ `#viewport` (ใต้แถว 3 คือ y = 100px หรือ 70px ถ้าซ่อนแถบบุ๊กมาร์ก)
   - เมื่อ chrome ส่ง `ui:popover {open:true, rect}` ให้ยก UI view ขึ้นมาเหนือ tab view ชั่วคราว ไม่งั้นเมนูจะโดนหน้าเว็บบัง
   - ทางเลือกคือทำ popover เป็น child window แยกแบบที่ Chrome ทำ
5. **กระจกจริงบน Windows**: `backdrop-filter` เบลอได้แค่ของที่อยู่ในหน้าเดียวกัน ถ้าจะให้เห็นวอลเปเปอร์ข้างหลังแบบในดีไซน์ ต้องทำดังนี้
   - เปิด Mica หรือ Acrylic ให้หน้าต่าง เช่นใช้ crate `window-vibrancy`: `apply_acrylic` / `apply_mica`
   - ตั้งพื้นหลังของ CEF browser เป็นโปร่งใส (`background_color = 0x00000000`)
   - ปรับ `--glass-window` ใน `tokens.css` ให้เข้ม/อ่อนตามที่ชอบ
6. **ลากหน้าต่าง**: แถว 1–2 ใช้ `-webkit-app-region: drag` ซึ่ง CEF และ WebView2 รองรับ ปุ่มทั้งหมดเป็น `no-drag`
   - ถ้าใช้หน้าต่างแบบไม่มีกรอบ ให้ส่ง `window:minimize`, `window:maximize` และ `window:close` ไปจัดการฝั่ง Rust

## คีย์ลัดที่ทำไว้ในหน้า

- **chrome**: `Ctrl+L` แก้ URL, `Ctrl+D` บุ๊กมาร์ก, `Esc` ปิด popover, คลิกปุ่มกลางเมาส์ที่แท็บเพื่อปิดแท็บ
- **palette**: `↑` `↓` เลือก, `Enter` เปิด, `Ctrl+Enter` เปิดในแท็บใหม่, `Esc` ปิด

คีย์ลัดระดับโปรแกรม เช่น `Ctrl+T`, `Ctrl+K` และ `Ctrl+Shift+B` ควรดักที่ฝั่ง Rust แล้วสั่ง UI ผ่าน bridge เพราะต้องทำงานได้แม้ focus อยู่ในหน้าเว็บ

## ปรับธีม

- **สีเน้น (Shields)**: ใช้ `zen.receive({type:"theme:accent", payload:{hex:"#7DD3FC"}})` หรือแก้ `--accent` ใน `tokens.css`
- **ขนาดแถว**: `--row-top`, `--row-tabs`, `--row-bookmarks`
- **ไอคอนใหม่**: เพิ่ม path ใน `shared/icons.js` (viewBox 24, เส้นอย่างเดียว)

## License ฟอนต์

ฟอนต์ทั้งสามใช้ SIL Open Font License 1.1 ไฟล์ license อยู่ใน `shared/fonts/` ให้ใส่ชื่อไว้ในหน้า credits ตาม plan.md หัวข้อ 12
