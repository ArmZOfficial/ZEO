# ZEO Browser — แผนพัฒนา (plan.md)

> เบราว์เซอร์เขียนด้วย **Rust ทั้งตัว รวมถึง engine (Servo)** เน้น 3 อย่าง: **บล็อกโฆษณาระดับ Brave**, **ใช้ง่าย เข้าใจง่าย**, **ฟีเจอร์ครบแบบเบราว์เซอร์ชั้นนำ**

### บันทึกการแก้ไขแผน (ฉบับที่ 2)
| เรื่อง | ฉบับที่ 1 | ฉบับนี้ |
|---|---|---|
| ชื่อ | ZEN (ชนกับ Zen Browser) | **ZEO** |
| Engine | CEF (Chromium) | **Servo (Rust)** |
| ส่วนขยาย Chrome | หลัง v1 | **อยู่ใน v1** (แบบ subset ที่เราเขียนเอง) |
| Split view, แปลหน้าเว็บ | หลัง v1 | **อยู่ใน v1** |
| แพลตฟอร์ม | Windows ก่อน | Windows ก่อน (เหมือนเดิม) |
| ลำดับเฟส | ทำ shell ก่อน adblock | **Adblock มาก่อนทุกอย่าง** หลัง spike |

---

## 1. เป้าหมายและขอบเขต

### เป้าหมายหลัก
1. **Adblock ระดับ Brave ตั้งแต่ติดตั้งเสร็จ** — ไม่ต้องลงส่วนขยาย ไม่ต้องตั้งค่า
2. **ใช้ง่าย** — หน้าตาสะอาด ปุ่มน้อย การตั้งค่าอธิบายด้วยภาษาคน
3. **Rust ทั้งระบบ** — ตัวโปรแกรมและ engine เป็น Rust ทำให้แก้ engine ได้เองเมื่อจำเป็น (เช่น ป้องกัน fingerprint ระดับ engine แบบ Brave)
4. **เว็บที่คนใช้ประจำต้องใช้ได้** — มีรายการเว็บเป้าหมายที่วัดผลได้ (หัวข้อ 10) และมีทางออกให้ผู้ใช้เสมอเมื่อเว็บไหนยังไม่รองรับ
5. **เร็วและเบา**

### ข้อเท็จจริงที่ต้องยอมรับเมื่อเลือก Servo
Servo ยัง**เปิดเว็บจริงได้ไม่ครบเท่า Chromium/Firefox** ดังนั้น ZEO v1 จะ**ไม่**เปิดได้ทุกเว็บ แผนนี้จึง:
- กำหนด **"รายการเว็บเป้าหมาย"** และวัดความเข้ากันได้ทุก release (หัวข้อ 10)
- มีปุ่ม **"เปิดหน้านี้ในเบราว์เซอร์อื่น"** ในหน้าเตือนและเมนู เมื่อเว็บแสดงผลผิด
- ส่งแพตช์แก้ปัญหาที่เจอกลับไปที่ Servo upstream แทนการแยกโค้ดไปไกล

### สิ่งที่ไม่ทำใน v1 (Non-goals)
- ไม่เขียน engine ใหม่ตั้งแต่ศูนย์ — ใช้ Servo และแก้/เติมเท่าที่จำเป็น
- **DRM (Netflix, Disney+ ฯลฯ)** — Servo ไม่มี EME/Widevine
- ไม่ทำ mobile, crypto wallet, rewards, VPN, AI sidebar
- Sync ข้ามเครื่อง — หลัง v1

### แพลตฟอร์ม
- **v1: Windows 10/11 (x64)**
- v1.x: Linux, macOS — ใช้ `winit` + Servo ซึ่งข้ามแพลตฟอร์มอยู่แล้ว ห้ามเขียนโค้ดผูก Windows นอก module `platform/windows`

### ชื่อ ZEO
ยังไม่ได้ตรวจเครื่องหมายการค้า — ต้องเช็กก่อน Phase 6 (ปล่อยสาธารณะ) ชื่อ/ไอคอน/สีทั้งหมดเก็บที่ `zeo-branding` crate ที่เดียว

---

## 2. Web Engine: Servo

### ทำไม Servo
- เป็น Rust (ส่วน JavaScript ใช้ SpiderMonkey ผ่าน `mozjs` ซึ่งเป็น C++)
- มี **embedding API** (`Servo`, `WebView`, `WebViewDelegate`) ให้ฝังในโปรแกรมของเรา โดยมี `servoshell` เป็นตัวอย่างอ้างอิง
- เราแก้ engine ได้ตรงๆ → ทำของที่ Brave ทำได้เพราะ fork Chromium เช่น fingerprint farbling, ผูก adblock ลึกในชั้น network

### สิ่งที่ต้องพิสูจน์ใน Phase 0 (ห้ามข้าม)
embedding API ของ Servo ยังเปลี่ยนบ่อย ความสามารถด้านล่างต้อง**ยืนยันกับเวอร์ชันที่ใช้จริง**ก่อนลงทุนต่อ:

| ต้องทำได้ | ใช้กับ | ถ้าทำไม่ได้ |
|---|---|---|
| ดัก/ยกเลิก/ตอบแทน network request ทุกชนิด (รวม subresource) | Network blocking, redirect | แพตช์ชั้น `net` ของ Servo เอง |
| Inject script/CSS ก่อนสคริปต์ของหน้า ในทุก frame | Cosmetic, scriptlet, content script ของส่วนขยาย | แพตช์ script thread |
| หลาย WebView ในหน้าต่างเดียว วางตำแหน่ง/ขนาดเองได้ | แท็บ, split view, UI | — (จำเป็นมาก) |
| แยกโปรไฟล์ cookie/storage | หน้าต่างส่วนตัว, โปรไฟล์หลายชุด | แพตช์ storage |
| ดาวน์โหลดไฟล์, ขอสิทธิ์ (กล้อง/ตำแหน่ง/แจ้งเตือน), dialog | ฟีเจอร์พื้นฐาน | แพตช์ / เลื่อนฟีเจอร์ |
| เล่นวิดีโอ/เสียง บน Windows (ต้องแพ็ก GStreamer) | YouTube ฯลฯ | ระบุเป็นข้อจำกัด |

### กลยุทธ์ดูแล Servo
- **Pin เวอร์ชัน Servo** (crate หรือ git rev) อัปเดตเป็นรอบ เช่น ทุก 2–4 สัปดาห์ พร้อมรันชุดทดสอบทั้งหมด
- ถ้าต้องแพตช์ engine → ใช้ fork ที่ rebase ได้ (`zeo-servo` branch) และ**พยายามส่งเข้า upstream ทุกแพตช์** เพื่อไม่ให้ fork ห่างจนดูแลไม่ไหว
- ซ่อน Servo ไว้หลัง trait `WebEngine` ของเรา — โค้ดส่วนอื่นห้ามเรียก Servo ตรง

---

## 3. สถาปัตยกรรม

```
┌──────────────────────────── ZEO (Rust process) ─────────────────────────────┐
│ zeo-app        : winit window, event loop, compositing ของหลาย WebView      │
│ zeo-ui-bridge  : IPC ระหว่างหน้า zeo:// กับ Rust                             │
│ zeo-tabs       : แท็บ/หน้าต่าง/split view/session                            │
│ zeo-shields    : adblock + ความเป็นส่วนตัว                                   │
│ zeo-extensions : runtime ส่วนขยาย (subset ของ WebExtensions)                 │
│ zeo-translate  : แปลหน้าเว็บ                                                 │
│ zeo-store      : SQLite                                                      │
│      │                                                                       │
│ zeo-engine (trait) ──► zeo-engine-servo ──► Servo (+ แพตช์ของเราถ้าจำเป็น)  │
└───────────────────────────────────────────────────────────────────────────────┘
        ▲ UI View: zeo://ui (HTML)          ▲ Tab Views: เว็บจริง
```

### หลักการออกแบบ
- **UI ของเบราว์เซอร์เขียนด้วย HTML/CSS/TypeScript** แสดงผลใน Servo WebView พิเศษ `zeo://ui` — เราควบคุมเนื้อหาเองจึงเขียนให้อยู่ในชุดฟีเจอร์ที่ Servo รองรับดีได้ (เช่น flexbox/grid พื้นฐาน) และได้ทดสอบ Servo ไปในตัว
  - แผนสำรอง: ถ้า Phase 0 พบว่า Servo วาด UI ได้ไม่ลื่น ให้ทำแถบแท็บ/toolbar ด้วย `egui` (แบบ servoshell) และใช้ HTML เฉพาะหน้าตั้งค่า/ประวัติ
- **Logic ทั้งหมดอยู่ฝั่ง Rust** — UI เป็นแค่ตัวแสดงผล
- **IPC สิทธิ์สูงรับเฉพาะ origin `zeo://`** เว็บทั่วไปเรียกไม่ได้
- **Adblock ตัดสินใจก่อน request ออกจากเครื่อง**

### โครงสร้าง Cargo workspace
```
zeo/
├── Cargo.toml
├── crates/
│   ├── zeo-app/               # binary หลัก
│   ├── zeo-engine/            # trait WebEngine + ชนิดข้อมูลกลาง
│   ├── zeo-engine-servo/      # implementation ด้วย Servo
│   ├── zeo-shields/           # network, cosmetic, scriptlet, list updater, fingerprint
│   ├── zeo-extensions/        # manifest, content scripts, chrome.* API, ติดตั้ง .crx
│   ├── zeo-translate/         # ตรวจภาษา + แปลหน้า
│   ├── zeo-tabs/              # แท็บ, กลุ่ม, split view, session restore
│   ├── zeo-store/             # SQLite + migrations
│   ├── zeo-omnibox/           # address bar
│   ├── zeo-downloads/
│   ├── zeo-vault/             # password manager
│   ├── zeo-import/            # นำเข้าจาก Chrome/Edge/Firefox
│   ├── zeo-ipc/               # schema ข้อความ UI <-> Rust
│   ├── zeo-updater/
│   └── zeo-branding/
├── ui/                        # หน้า zeo:// (TypeScript, framework เบาๆ)
├── resources/                 # filter lists, scriptlets, โมเดลแปลภาษา (ถ้าแพ็กมา)
├── installer/
└── tests/
    ├── shields/               # หน้าทดสอบ adblock
    ├── compat/                # รายการเว็บเป้าหมาย + สคริปต์ตรวจ
    └── extensions/            # ส่วนขยายเป้าหมาย
```

### Dependency หลัก
| งาน | Crate |
|---|---|
| Web engine | `servo` (pin เวอร์ชัน/rev) |
| หน้าต่าง | `winit` |
| Adblock | `adblock` (adblock-rust ของ Brave, MPL-2.0) |
| ฐานข้อมูล | `rusqlite` (bundled, FTS5) |
| Async / network นอก engine | `tokio`, `reqwest` |
| URL / โดเมน | `url`, `psl` |
| ส่วนขยาย | `zip`, `serde_json`, ตรวจลายเซ็น CRX3 (protobuf: `prost`) |
| แปลภาษา | ประเมินใน Phase 0 (ดูหัวข้อ 6) |
| เข้ารหัส | `aes-gcm`, `argon2`, Windows DPAPI ผ่าน `windows` |
| Log / error | `tracing`, `thiserror`, `anyhow` |

---

## 4. ZEO Shields — ระบบ Adblock (หัวใจของโปรเจกต์)

### 4.1 ชั้นการป้องกัน

| ชั้น | ทำอะไร | วิธีทำใน ZEO |
|---|---|---|
| **1. Network blocking** | ตัด request โฆษณา/ตัวติดตามก่อนโหลด | `adblock::Engine::check_network_request` ใน request hook ของ Servo |
| **2. Cosmetic filtering** | ซ่อนกรอบโฆษณาที่เหลือ | ขอ selector ตาม URL → inject CSS ตอนเริ่มโหลด + MutationObserver |
| **3. Scriptlet injection** | แก้เว็บที่ต้าน adblock / โฆษณาในวิดีโอ | resource ชุด uBlock Origin, inject ก่อนสคริปต์ของหน้า |
| **4. Redirect resources** | แทนสคริปต์ที่บล็อกด้วยของปลอม กันเว็บพัง | ตอบกลับด้วย resource ในเครื่อง |
| **5. Tracking param stripping** | ลบ `fbclid`, `gclid`, `utm_*` | `removeparam` rule + รายการของเราเอง |
| **6. HTTPS upgrade** | บังคับ HTTPS | ลอง https ก่อน, พังค่อยถาม |
| **7. Third-party cookie blocking** | บล็อก cookie ข้ามเว็บ | นโยบาย cookie ใน storage ของ Servo |
| **8. Cookie banner blocking** | ซ่อนป๊อปอัปขอคุกกี้ | filter list เฉพาะทาง |
| **9. Fingerprint protection (ระดับ engine)** | สุ่ม canvas/WebGL/audio/fonts ต่อเว็บต่อ session แบบ Brave | **แพตช์ใน Servo โดยตรง** — ทำได้เพราะ engine เป็น Rust ที่เราแก้ได้ |
| **10. CNAME uncloaking** | จับตัวติดตามซ่อนหลัง subdomain | resolve CNAME ในชั้น net ของ Servo แล้วตรวจซ้ำ |

ข้อ 9–10 ทำได้ดีกว่าแผนเดิม (CEF) เพราะเข้าถึงโค้ด engine ได้

### 4.2 Filter lists
เปิดเป็นค่าเริ่มต้น: EasyList, EasyPrivacy, uBlock Origin filters (ads, privacy, badware, quick fixes, unbreak), Peter Lowe's list, **EasyList Thailand** (เปิดอัตโนมัติตามภาษาเครื่อง)
เลือกเปิดเพิ่มได้: cookie banner, annoyances, social widgets, list จาก URL

### 4.3 การอัปเดตและประสิทธิภาพ
- แพ็ก list ล่าสุดมากับตัวติดตั้ง → บล็อกได้ทันทีแม้ออฟไลน์
- อัปเดตเบื้องหลังทุก ~24 ชม. (`ETag`), ตรวจ checksum
- Serialize engine ที่ compile แล้วลงดิสก์ → โหลดในหลักมิลลิวินาที
- สลับ engine แบบ atomic (`arc-swap`)
- งบเวลา: ตรวจ 1 request **< 0.1 ms** เฉลี่ย

### 4.4 UX ของ Shields
- ไอคอนโล่บน address bar + จำนวนที่บล็อก
- แผงเล็ก: สวิตช์ **"ป้องกันเว็บนี้: เปิด/ปิด"**, ตัวเลข, ปุ่ม **"เว็บนี้แสดงผลผิดปกติ?"** (ปิดเฉพาะเว็บนี้แล้วโหลดใหม่), ตัวเลือกเพิ่มเติมพับไว้
- ระดับ: **มาตรฐาน** (ค่าเริ่มต้น) / เข้มงวด / ปิด
- New Tab แสดงสถิติรวม
- Element picker: คลิกขวา → "ซ่อนสิ่งนี้" (v1)

### 4.5 การทดสอบ adblock
- หน้า HTML ในเครื่องจำลองโฆษณาแต่ละชนิด → ตรวจอัตโนมัติว่าบล็อก/ซ่อนได้
- เว็บจริง ~50 เว็บ (ไทย + ต่างประเทศ) เช็กทุก release: โฆษณาหาย **และ** เว็บไม่พัง
- เทียบผลกับ Brave และ uBlock Origin บนหน้าทดสอบสาธารณะ
- Benchmark ด้วย `criterion`

### 4.6 เชื่อมกับระบบส่วนขยาย
`declarativeNetRequest` ของส่วนขยาย (หัวข้อ 5) แปลงเป็นกฎแล้วตรวจผ่าน pipeline เดียวกับ Shields — ไม่มี hook ซ้อนสองชุด

---

## 5. ส่วนขยาย (Extensions) — v1

### ความจริงที่ต้องรู้
Servo **ไม่มีระบบส่วนขยาย** ZEO ต้องเขียน runtime เองทั้งหมด จึงทำได้แค่ **subset ของ WebExtensions (Manifest V3)** ส่วนขยายที่ใช้ API นอกชุดนี้ หรือใช้ฟีเจอร์เว็บที่ Servo ยังไม่รองรับ จะใช้ไม่ได้ — ต้องบอกผู้ใช้ตรงๆ ตอนติดตั้ง

### ขอบเขต v1
| ส่วน | รายละเอียด |
|---|---|
| ติดตั้ง | จากไฟล์ `.crx` / `.zip` / โฟลเดอร์ (โหมดนักพัฒนา); ตรวจลายเซ็น CRX3 |
| Chrome Web Store | ปุ่ม "เพิ่มลงใน ZEO" บนหน้า Web Store (ดาวน์โหลด .crx แล้วติดตั้ง) — ต้องตรวจเงื่อนไขการใช้งานของ Google ก่อนเปิดใช้ |
| Manifest | MV3 (และ MV2 แบบอ่านได้เท่าที่ map ได้) |
| Content scripts | `matches`, `run_at`, `all_frames`, CSS — ใช้ระบบ inject เดียวกับ Shields, แยก isolated world |
| Background | service worker — **ขึ้นกับการรองรับ Service Worker ของ Servo**; ถ้าไม่พอ ให้รันเป็น background page ที่ซ่อนอยู่แทน |
| API ชุดแรก | `runtime` (messaging), `storage` (local/sync→local), `tabs` (พื้นฐาน), `scripting`, `contextMenus`, `action` (ปุ่ม + popup), `i18n`, `alarms`, `notifications`, `declarativeNetRequest` |
| สิทธิ์ | แสดงหน้าขอสิทธิ์เป็นภาษาคน, จำกัด host permission ได้รายเว็บ |
| UI | หน้า `zeo://extensions` เปิด/ปิด/ลบ, ปุ่มส่วนขยายบน toolbar (ปักหมุดได้) |

### เกณฑ์วัด
กำหนด **"ส่วนขยายเป้าหมาย 10 ตัว"** ที่คนใช้บ่อยและใช้ API อยู่ในชุดข้างบน (เช่น ประเภทโหมดมืด, ตัวจัดการรหัสผ่าน, ข้ามสปอนเซอร์ในวิดีโอ, จดโน้ต, แปลศัพท์) — v1 ผ่านเมื่อใช้ได้ ≥ 8/10 ตัว รายชื่อจริงเลือกตอนเริ่ม Phase 5 และทดสอบอัตโนมัติใน `tests/extensions/`

> หมายเหตุ: ไม่จำเป็นต้องลง adblock extension เพราะ Shields มีในตัวแล้ว

---

## 6. Split view และแปลหน้าเว็บ — v1

### Split view
- เปิด 2 แท็บในหน้าต่างเดียว แบ่งซ้าย/ขวา ลากเส้นแบ่งปรับขนาดได้
- วิธีเปิด: ลากแท็บไปขอบหน้าต่าง / คลิกขวาแท็บ → "เปิดแบบแบ่งจอ" / `Ctrl+Shift+S` (ยืนยันคีย์ไม่ชนกับคีย์ลัดอื่น)
- Address bar ตามช่องที่โฟกัสอยู่, ปุ่ม "สลับฝั่ง" และ "ออกจากแบ่งจอ"
- บันทึกใน session restore
- ใช้ความสามารถหลาย WebView ในหน้าต่างเดียว (ต้องผ่าน Phase 0)

### แปลหน้าเว็บ
- ตรวจภาษาหน้าเว็บ → ถ้าไม่ใช่ภาษาของผู้ใช้ แสดงแถบเล็ก **"แปลหน้านี้เป็นภาษาไทย?"**
- แปลแบบรักษาโครงหน้า (แปลเฉพาะ text node, ทยอยแปลส่วนที่มองเห็นก่อน), ปุ่ม "ดูต้นฉบับ"
- **ทางเลือกเทคนิค (ตัดสินใจใน Phase 0):**

| ทางเลือก | ข้อดี | ข้อเสีย |
|---|---|---|
| **แปลในเครื่อง** (เช่น โมเดล Bergamot/Firefox Translations หรือโมเดลผ่าน CTranslate2) | เป็นส่วนตัว ไม่ส่งข้อมูลออก ใช้ออฟไลน์ได้ | ต้องตรวจว่ามีโมเดลคู่ภาษา **ไทย** หรือไม่; บางโมเดลมี license ห้ามใช้เชิงพาณิชย์ (เช่น NLLB = CC-BY-NC) ต้องตรวจก่อนใช้ |
| **บริการแปลบนคลาวด์** | คุณภาพดี รองรับทุกภาษา | ส่งข้อความในหน้าออกไปภายนอก → ต้อง**ปิดเป็นค่าเริ่มต้นและถามผู้ใช้ก่อน**, มีค่าใช้จ่าย API |

**แนวทางที่แนะนำ:** แปลในเครื่องเป็นค่าเริ่มต้นสำหรับคู่ภาษาที่มีโมเดลดีและ license ใช้ได้ ส่วนคู่ภาษาที่ยังไม่มี ให้เปิดคลาวด์ได้แบบ opt-in โดยแจ้งชัดว่าข้อความจะถูกส่งออก โมเดลดาวน์โหลดตามภาษาที่ใช้ ไม่แพ็กทั้งหมดในตัวติดตั้ง

---

## 7. ฟีเจอร์เบราว์เซอร์ทั้งหมด

### 7.1 MVP (Alpha)
**แท็บและหน้าต่าง:** เปิด/ปิด/สลับ/ลากเรียง, ลากออกเป็นหน้าต่างใหม่, ปักหมุด, ทำซ้ำ, ปิดเสียง, เปิดแท็บที่ปิดคืน, กู้ session หลัง crash
**การนำทาง:** ย้อน/ไปหน้า/โหลดใหม่/หยุด/หน้าแรก, Omnibox (URL หรือคำค้น + แนะนำจากประวัติ/บุ๊กมาร์ก/แท็บ/search engine), เลือก search engine, ค้นหาในหน้า, ซูม, เต็มจอ, พิมพ์
**ข้อมูลผู้ใช้:** ประวัติ, บุ๊กมาร์ก + แถบบุ๊กมาร์ก, ดาวน์โหลด, หน้าต่างส่วนตัว, ล้างข้อมูล
**ความปลอดภัย:** แม่กุญแจ + ข้อมูลใบรับรอง, หน้าเตือน, สิทธิ์รายเว็บ (กล้อง/ไมค์/ตำแหน่ง/แจ้งเตือน/ป๊อปอัป)
**พื้นฐานอื่น:** เมนูคลิกขวา, คีย์ลัดแบบ Chrome, ตั้งเป็นเบราว์เซอร์เริ่มต้น, หน้าตั้งค่า, ธีมสว่าง/มืด, ภาษา UI ไทย + อังกฤษ, ปุ่ม **"เปิดในเบราว์เซอร์อื่น"**
**เครื่องมือนักพัฒนา:** Servo ไม่มี DevTools ในตัวแบบ Chrome — v1 ใช้การเชื่อมต่อ remote debugging ของ Servo (ถ้ามีในเวอร์ชันที่ใช้) แล้วค่อยพิจารณาทำ DevTools ของเราเองหลัง v1

### 7.2 v1.0 (เทียบเบราว์เซอร์ชั้นนำ)
- Password manager + autofill + สร้างรหัสผ่าน
- Autofill ที่อยู่/ฟอร์ม
- นำเข้าจาก Chrome / Edge / Firefox + ขั้นตอนเปิดครั้งแรก
- โปรไฟล์หลายชุด
- กลุ่มแท็บ, ค้นหาแท็บ, แท็บแนวตั้ง, พักแท็บที่ไม่ได้ใช้
- **Split view** (หัวข้อ 6)
- **แปลหน้าเว็บ** (หัวข้อ 6)
- **ส่วนขยาย** (หัวข้อ 5)
- Reader mode, Picture-in-Picture, ดู PDF ในตัว (ใช้ pdf.js หาก Servo รันได้ดีพอ ไม่งั้นเรนเดอร์ด้วย crate Rust), จับภาพหน้าจอ
- ตัวจัดการงาน, Auto-update

### 7.3 หลัง v1
Sync แบบ end-to-end, Linux/macOS, DevTools ของเราเอง, AI ผู้ช่วย (ถ้าต้องการ)

---

## 8. หลักการ UX — "ใช้ง่าย เข้าใจง่าย"

1. หน้าจอหลักมีแค่: แท็บ, ย้อน/ไปหน้า/โหลดใหม่, address bar, โล่ Shields, ปุ่มส่วนขยาย, เมนู
2. ค่าเริ่มต้นต้องดีที่สุดแล้ว — คนส่วนใหญ่ไม่ต้องเปิดหน้าตั้งค่าเลย
3. ภาษาคน ไม่ใช่ศัพท์เทคนิค
4. การตั้งค่าทุกข้อมีคำอธิบาย 1 บรรทัด + ช่องค้นหา
5. ตัวเลือกขั้นสูงพับเก็บไว้
6. เปิดครั้งแรก 3 ขั้น ≤ 30 วินาที: นำเข้า → ตั้งเป็นค่าเริ่มต้น → ธีม (ข้ามได้ทุกขั้น)
7. คีย์ลัดเหมือน Chrome
8. หน้า error ต้องบอกทางแก้ — รวมถึงกรณี "เว็บนี้ยังไม่รองรับใน ZEO" พร้อมปุ่มเปิดในเบราว์เซอร์อื่น
9. ไม่มี telemetry โดยค่าเริ่มต้น; ฟีเจอร์ที่ส่งข้อมูลออก (แปลบนคลาวด์, crash report) ต้องถามก่อน

---

## 9. ข้อมูล ความปลอดภัย

### การจัดเก็บ — `%LOCALAPPDATA%\ZEO\User Data\<โปรไฟล์>\`
| ข้อมูล | รูปแบบ |
|---|---|
| ประวัติ, บุ๊กมาร์ก, ดาวน์โหลด, สิทธิ์รายเว็บ | SQLite (`zeo.db`) + FTS5 |
| การตั้งค่า | JSON มี schema version |
| Session | snapshot เขียนแบบ atomic |
| รหัสผ่าน | `vault.db` เข้ารหัส AES-GCM, กุญแจผูก DPAPI |
| ส่วนขยาย | `Extensions/<id>/<version>/` + storage แยกต่อส่วนขยาย |
| Filter lists, โมเดลแปลภาษา | `Shields/`, `Translate/` |

ทุก schema มี migration ตั้งแต่วันแรก หน้าต่างส่วนตัวเก็บในหน่วยความจำเท่านั้น

### ความปลอดภัย — ความเสี่ยงสูงขึ้นเมื่อใช้ Servo
Servo **ยังไม่ผ่านการ harden ด้านความปลอดภัยระดับ Chromium/Firefox** (โหมด multiprocess และ sandbox ยังอยู่ระหว่างพัฒนา) ข้อดีคือโค้ดส่วนใหญ่เป็น Rust ที่ปลอดภัยด้านหน่วยความจำ แต่ JavaScript engine (SpiderMonkey) เป็น C++

แนวทาง:
- เปิด multiprocess + sandbox ของ Servo ทันทีที่ใช้งานได้บน Windows; ถ้ายังไม่ได้ ให้ระบุสถานะ "Beta — ไม่แนะนำสำหรับเว็บธนาคาร" ใน UI อย่างตรงไปตรงมา
- ตามอัปเดต Servo/mozjs สม่ำเสมอ โดยเฉพาะแพตช์ความปลอดภัยของ SpiderMonkey
- Fuzzing (`cargo fuzz`) ส่วนที่เราเขียนเอง: IPC, parser manifest ส่วนขยาย, omnibox
- IPC สิทธิ์สูงรับเฉพาะ `zeo://`, validate ทุก field; ส่วนขยายเข้าถึงได้เฉพาะ API ที่ประกาศสิทธิ์
- หน้า `zeo://` ใช้ CSP เข้มงวด
- อัปเดตโปรแกรม/filter list/โมเดล ผ่าน HTTPS + ตรวจลายเซ็น
- Code signing ตัวติดตั้ง, Mark-of-the-Web ให้ไฟล์ดาวน์โหลด
- `cargo audit` + `cargo deny` ใน CI

---

## 10. ความเข้ากันได้กับเว็บ (งานต่อเนื่องทุกเฟส)

- **รายการเว็บเป้าหมาย ~50 เว็บ** แบ่ง 3 ระดับ:
  - **ระดับ A (ต้องใช้ได้ใน v1):** ค้นหา, วิกิพีเดีย, เว็บข่าวไทย, YouTube (ดูวิดีโอ), Gmail/อีเมลแบบเว็บ, โซเชียลหลัก, ช้อปปิ้งออนไลน์หลักในไทย
  - **ระดับ B (ควรใช้ได้):** เอกสารออนไลน์, แผนที่, แชตบนเว็บ
  - **ระดับ C (อาจใช้ไม่ได้ใน v1):** เว็บธนาคาร, เว็บแอปหนัก, สตรีมมิงที่ใช้ DRM
- สคริปต์ตรวจอัตโนมัติ: โหลดหน้า → ภาพหน้าจอ → เทียบกับภาพอ้างอิง + ตรวจ console error
- ติดตามคะแนน Web Platform Tests (WPT) ของ Servo ในฟีเจอร์ที่เว็บเป้าหมายใช้
- ทุกปัญหาที่เจอ: แยกเป็น (1) bug ของเรา (2) bug/ฟีเจอร์ที่ขาดของ Servo → เปิด issue/ส่งแพตช์ upstream
- **เกณฑ์ v1:** ระดับ A ใช้ได้ ≥ 90%, ระดับ B ≥ 70%

---

## 11. แผนงานเป็นเฟส (Adblock มาก่อน)

> ประมาณเวลาสำหรับนักพัฒนา 1 คนเต็มเวลา — ตัวเลขคร่าวๆ การใช้ Servo และการทำส่วนขยายเองทำให้ยาวกว่าแผนเดิมมาก

### Milestone 0 — Prototype บน WebView2 (เสร็จแล้ว)
Prototype ทำได้ภายในวันเดียว เพื่อให้มีของที่ใช้งานได้ก่อนลงทุนกับ Servo **engine เป็น WebView2 ชั่วคราว** ส่วนทิศทางระยะยาวยังเป็น Servo ตามหัวข้อ 2
- [x] `zeo-shields` ไม่ผูกกับ engine: EasyList + EasyPrivacy เก็บใน `resources/filters/` ใช้เฉพาะกฎ network แยกนับโฆษณาและตัวติดตาม และกฎยกเว้นทำงานข้ามรายการ
- [x] `zeo-app`: หน้าต่างไม่มีกรอบ มี WebView2 สองตัวใช้ environment ร่วมกัน (UI จาก `ui/` ที่ `https://zeo.ui/` และแท็บหน้าเว็บ 1 แท็บ) ดักทุก request ด้วย `WebResourceRequested`
- [x] ต่อ UI แล้ว: นำทาง, omnibox (URL หรือค้นด้วย DuckDuckGo), ปุ่มหน้าต่าง, แผง Shields (ปิดรายเว็บ / ระดับ / เว็บแสดงผลผิด), ซูม
- [x] ประหยัดแรม: ตัดกฎ cosmetic, ย่อหน้าต่างแล้วซ่อน view และตั้ง `MemoryUsageTargetLevel=Low`, ปิด autofill/บันทึกรหัสผ่าน, release profile (LTO fat, codegen-units 1, panic abort, strip)
- ยังไม่ทำ: cosmetic filtering (มีกรอบว่างค้างตรงที่เคยเป็นโฆษณา), "MB ประหยัด" (แสดง 0), แท็บหลายแท็บ, บุ๊กมาร์ก, newtab, palette, Mica, ระดับ "เข้มงวด" ยังทำงานเท่ากับ "มาตรฐาน", ตอนเปิด popover พื้นที่แถบเดียวกันทึบเป็นสีเข้ม
- ผลวัดจริง: ดูหัวข้อ 12

### Phase 0 — Spike พิสูจน์ Servo (2–3 สัปดาห์)
- [ ] ตั้ง workspace, CI (build + clippy + test), pin เวอร์ชัน Servo
- [ ] Build Servo บน Windows ได้ (รวม GStreamer สำหรับสื่อ)
- [ ] หน้าต่าง winit + WebView เปิดเว็บได้
- [ ] ตรวจทุกข้อในตาราง "สิ่งที่ต้องพิสูจน์" (หัวข้อ 2)
- [ ] ต่อ `adblock` crate เข้ากับ request hook + EasyList
- [ ] ลองวาด UI ด้วย HTML ใน WebView ที่สอง → ตัดสินใจ HTML หรือ egui
- [ ] ประเมินทางเลือกแปลภาษา (มีโมเดลไทย? license ใช้ได้?)
- [ ] เปิดเว็บระดับ A ทั้งหมด จดผลเป็น baseline

**ผ่านเมื่อ:** เว็บข่าวที่มีโฆษณาเยอะเปิดใน Servo แล้วโฆษณาหาย, ทุกข้อในตารางหัวข้อ 2 มีคำตอบว่า "ได้" หรือ "ต้องแพตช์ (ประเมินแล้ว X สัปดาห์)"
**ถ้าไม่ผ่าน:** หยุดคิดใหม่ก่อนลงทุนต่อ — ทางเลือกคือรอ Servo พัฒนาต่อ หรือกลับไปใช้ CEF ตามแผนฉบับที่ 1 (ทั้งคู่ใช้ `zeo-shields` ชุดเดียวกันได้ เพราะซ่อนหลัง trait `WebEngine`)

### Phase 1 — ZEO Shields เต็มรูปแบบ (5–7 สัปดาห์)
ใช้หน้าต่างทดสอบเรียบๆ (ช่อง URL + WebView) พอ ยังไม่ต้องมี UI สวย
- [ ] Network blocking + list หลายชุด + แพ็กมากับโปรแกรม
- [ ] Cosmetic filtering + scriptlet + redirect resources
- [ ] ตัวอัปเดต list, serialize engine, สลับแบบ atomic
- [ ] ลบ tracking param, HTTPS upgrade, บล็อก third-party cookie, cookie banner
- [ ] **Fingerprint protection ระดับ engine** (แพตช์ Servo)
- [ ] CNAME uncloaking
- [ ] ชุดทดสอบ adblock อัตโนมัติ + benchmark

**ผ่านเมื่อ:** หน้าทดสอบ adblock ได้คะแนนใกล้เคียง Brave, เว็บข่าวไทย 10 อันดับแรกไม่มีโฆษณาและไม่พัง (เท่าที่ Servo แสดงผลได้)

### Phase 2 — เบราว์เซอร์ขั้นต่ำ (5–7 สัปดาห์)
- [ ] `zeo-engine` trait + `zeo-engine-servo` เต็มรูปแบบ
- [ ] UI `zeo://ui` + IPC
- [ ] แท็บ, การนำทาง, address bar พื้นฐาน
- [ ] แผง Shields + ตัวนับ + ปิดรายเว็บ
- [ ] เมนูคลิกขวา, คีย์ลัด, popup, ปุ่ม "เปิดในเบราว์เซอร์อื่น"
- [ ] `zeo-store` + ประวัติ

**ผ่านเมื่อ:** ท่องเว็บระดับ A ต่อเนื่อง 1 ชั่วโมงโดยไม่ crash

### Phase 3 — ฟีเจอร์ใช้งานประจำวัน (6–8 สัปดาห์)
- [ ] Omnibox เต็มรูปแบบ, บุ๊กมาร์ก, หน้าประวัติ, ดาวน์โหลด
- [ ] Session restore, เปิดแท็บที่ปิดคืน
- [ ] หน้าต่างส่วนตัว, ล้างข้อมูล
- [ ] สิทธิ์รายเว็บ, ใบรับรอง/หน้าเตือน
- [ ] ค้นหาในหน้า, ซูม, เต็มจอ, พิมพ์
- [ ] หน้าตั้งค่า, ธีม, New Tab + สถิติ Shields, i18n ไทย/อังกฤษ

**ผ่านเมื่อ:** ใช้เป็นเบราว์เซอร์หลักกับเว็บระดับ A ได้ 1 สัปดาห์ = **Alpha**

### Phase 4 — Split view + แปลหน้าเว็บ + ฟีเจอร์ชั้นนำ (8–10 สัปดาห์)
- [ ] Split view
- [ ] แปลหน้าเว็บ (ในเครื่อง + คลาวด์ opt-in)
- [ ] Password manager + autofill
- [ ] นำเข้าจาก Chrome/Edge/Firefox + ขั้นตอนเปิดครั้งแรก
- [ ] โปรไฟล์หลายชุด, กลุ่มแท็บ, ค้นหาแท็บ, แท็บแนวตั้ง, พักแท็บ
- [ ] Reader mode, PiP, PDF, จับภาพหน้าจอ, ตัวจัดการงาน
- [ ] Element picker

### Phase 5 — ส่วนขยาย (8–12 สัปดาห์)
- [ ] เลือกรายชื่อส่วนขยายเป้าหมาย 10 ตัว
- [ ] Parser manifest, ติดตั้ง .crx/.zip, ตรวจลายเซ็น
- [ ] Content scripts (ใช้ระบบ inject ของ Shields)
- [ ] Background (service worker หรือ background page สำรอง)
- [ ] `chrome.*` API ชุดแรก + `declarativeNetRequest` ผ่าน Shields
- [ ] หน้าขอสิทธิ์, `zeo://extensions`, ปุ่มบน toolbar + popup
- [ ] ติดตั้งจาก Chrome Web Store (หลังตรวจเงื่อนไขของ Google)

**ผ่านเมื่อ:** ส่วนขยายเป้าหมายใช้ได้ ≥ 8/10 = **Beta**

### Phase 6 — เตรียมปล่อย (4–6 สัปดาห์)
- [ ] ตัวติดตั้ง + code signing + auto-update
- [ ] ตั้งเป็นเบราว์เซอร์เริ่มต้น, protocol/file handler
- [ ] Crash reporter (ถามก่อน)
- [ ] ปรับประสิทธิภาพ, การเข้าถึง (keyboard, screen reader)
- [ ] สถานะ sandbox/ความปลอดภัย ชัดเจนใน UI
- [ ] **ตรวจเครื่องหมายการค้าชื่อ ZEO**, เว็บไซต์, เอกสาร, หน้า credits/license

**ผ่านเมื่อ:** ติดตั้งบนเครื่อง Windows สะอาดได้, อัปเดตตัวเองได้, เกณฑ์ compat ในหัวข้อ 10 ผ่าน = **v1.0**

**รวมประมาณ 38–53 สัปดาห์** (1 คน) ไม่รวมเวลาที่อาจต้องรอ/แพตช์ Servo

---

## 12. เป้าหมายประสิทธิภาพ

| ตัวชี้วัด | เป้า |
|---|---|
| Cold start จนพร้อมพิมพ์ | < 1.5 วินาที |
| โหลด adblock engine จาก cache | < 50 ms |
| ตรวจ 1 network request | < 0.1 ms เฉลี่ย |
| RAM รวม 10 แท็บเว็บทั่วไป | น้อยกว่า Chrome ที่เปิดแท็บเดียวกัน |
| UI | 60 fps ตอนสลับ/ลากแท็บ |

### ผลวัด Milestone 0 (2026-11-15, WebView2 157, Windows 11)
Private memory รวมทุก process ของเบราว์เซอร์ วัด 25 วินาทีหลังเปิดหน้า ครั้งเดียวต่อหน้า (ยังไม่ได้เฉลี่ยหลายรอบ) Edge เปิดด้วยโปรไฟล์ใหม่และไม่มี adblock

| หน้า | ZEO แถบ Win32 | ZEO UI HTML (ใช้จริง) | Edge |
|---|---|---|---|
| DuckDuckGo | 249 MB | 286 MB | 287 MB |
| Sanook ข่าว (โฆษณาเยอะ) | 327 MB | 394 MB | 831 MB |
| Wikipedia | 241 MB | 275 MB | 293 MB |

- UI HTML กินแรมเพิ่มประมาณ 35–67 MB เทียบกับแถบ Win32 (ต้องมี renderer เพิ่มอีกตัว)
- หน้าที่โฆษณาเยอะ ZEO ใช้แรมน้อยกว่า Edge ประมาณครึ่งหนึ่ง เพราะไม่โหลดสคริปต์โฆษณา/ตัวติดตาม ส่วนหน้าที่ไม่มีโฆษณา ใช้พอๆ กัน
- โหลด filter list 37–53 ms, เปิดโปรแกรมจนเริ่มโหลดหน้า 330–380 ms
- ทางลดต่อ: serialize engine ลงดิสก์, ปิด UI renderer ตอนย่อหน้าต่าง, วัดหลายรอบแล้วเฉลี่ย, ทดลอง browser arguments ของ WebView2 ทีละตัว

---

## 13. การทดสอบและคุณภาพ

- **Unit:** omnibox, store/migration, shields, parser manifest, การตรวจภาษา
- **Integration:** Servo จริงกับ web server ในเครื่อง (นำทาง, ดาวน์โหลด, สิทธิ์, inject script)
- **Adblock regression** (หัวข้อ 4.5), **Web compat** (หัวข้อ 10), **Extensions** (หัวข้อ 5)
- **UI:** หน้า `zeo://` ทดสอบแยกด้วย mock IPC
- **Fuzzing:** IPC, manifest, URL parsing
- **CI ทุก commit:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny`
- **CI รายคืน:** build Servo ใหม่ + รันชุด compat และ adblock เต็ม

---

## 14. ความเสี่ยง

| ความเสี่ยง | ผลกระทบ | แนวทางรับมือ |
|---|---|---|
| Servo เปิดเว็บจริงไม่ได้ตามเป้า | **สูงมาก** | วัดตั้งแต่ Phase 0, จัดระดับเว็บ, ปุ่มเปิดในเบราว์เซอร์อื่น, ส่งแพตช์ upstream, trait `WebEngine` ทำให้ย้ายไป CEF ได้ |
| Embedding API ของ Servo เปลี่ยนบ่อย | สูง | Pin เวอร์ชัน, อัปเดตเป็นรอบ, ทุกการเรียก Servo อยู่ใน `zeo-engine-servo` |
| ความปลอดภัย (sandbox ยังไม่พร้อม) | สูง | ดูหัวข้อ 9, บอกสถานะตรงๆ, ไม่โฆษณาว่าปลอดภัยสำหรับธนาคารจนกว่าจะพร้อม |
| ส่วนขยายต้องเขียน runtime เองทั้งหมด | สูง | จำกัด API ชุดแรก, วัดด้วยส่วนขยายเป้าหมาย 10 ตัว, แยกเป็นเฟสท้าย |
| Fork Servo ห่างจาก upstream | กลาง | ส่งแพตช์เข้า upstream ทุกครั้ง, rebase สม่ำเสมอ |
| วิดีโอ/YouTube บน Windows (GStreamer) | กลาง | ทดสอบใน Phase 0, แพ็ก GStreamer ที่จำเป็นกับตัวติดตั้ง |
| ไม่มีโมเดลแปลไทยที่ license ใช้ได้ | กลาง | ใช้คลาวด์แบบ opt-in สำหรับภาษาไทยก่อน |
| เว็บใหญ่เปลี่ยนวิธีต้าน adblock | กลาง | พึ่ง list ชุมชน (uBO quick fixes) อัปเดตถี่ |
| ขอบเขตใหญ่เกินกำลังคนเดียว | สูง | ยึดลำดับเฟส, ไม่ข้ามเกณฑ์ผ่าน |
| เงื่อนไข Chrome Web Store / license | กลาง | ตรวจก่อนเปิดฟีเจอร์ติดตั้งจาก Store; Servo/adblock-rust = MPL-2.0, filter lists หลาย license → หน้า credits |
| ชื่อ ZEO อาจซ้ำ | ต่ำ–กลาง | ตรวจก่อน Phase 6, เปลี่ยนที่ `zeo-branding` ที่เดียว |

---

## 15. ขั้นตอนถัดไปทันที

1. ติดตั้งเครื่องมือตามเอกสาร build ของ Servo บน Windows (Rust MSVC, Visual Studio Build Tools, Python, LLVM/clang, GStreamer ฯลฯ — ใช้รายการล่าสุดจากเอกสาร Servo)
2. `git init` + workspace ตามหัวข้อ 3
3. Build และรัน `servoshell` ให้ได้ก่อน เพื่อยืนยันว่า toolchain ใช้ได้
4. สร้าง `zeo-app` ขั้นต่ำ: หน้าต่าง + WebView เดียว
5. ต่อ `adblock` crate เข้ากับ request hook แล้วโหลด EasyList
6. ไล่ตรวจตารางหัวข้อ 2 ทีละข้อ แล้วบันทึกผลลงไฟล์ `docs/phase0-results.md`
