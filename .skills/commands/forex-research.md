---
name: forex-research
description: Tra cứu tin tức forex, đọc chỉ báo kỹ thuật, lướt website tài chính qua Chrome DevTools MCP. Tự động thu thập link bài viết và spawn team subagent để đọc song song rồi tổng hợp báo cáo.
---

## Tổng quan

Skill này biến Kuromi thành một forex research agent toàn diện, hoạt động trên **Chrome DevTools MCP** (CDP). Quy trình: mở trình duyệt → điều hướng đến các website tài chính → thu thập dữ liệu (tin tức, chỉ báo kỹ thuật, lịch kinh tế) → spawn team subagent đọc song song nhiều bài → tổng hợp báo cáo cho user.

---

## Phần 1 — Các website tài chính chính

### 1.1 Tin tức & Phân tích

| Website | URL | Dùng để |
|---------|-----|---------|
| ForexFactory | `https://www.forexfactory.com/news` | Tin forex mới nhất, lọc theo impact |
| Investing.com | `https://www.investing.com/news/forex-news` | Tin tổng hợp forex, commodities, crypto |
| FXStreet | `https://www.fxstreet.com/news` | Phân tích chuyên sâu, dự báo |
| DailyFX (→ IG) | `https://www.ig.com/uk/news-and-trade-ideas?source=dailyfx` | Tin tức + phân tích kỹ thuật (DailyFX merged vào IG Group) |
| Reuters | `https://www.reuters.com/markets/currencies/` | Tin tức macro, chính sách ngân hàng trung ương |
| Bloomberg | `https://www.bloomberg.com/markets/currencies` | Tin macro, geopolitical |

### 1.2 Lịch kinh tế

| Website | URL | Dùng để |
|---------|-----|---------|
| ForexFactory Calendar | `https://www.forexfactory.com/calendar` | Lịch sự kiện theo impact (vàng/cam/đỏ) |
| Investing.com Calendar | `https://www.investing.com/economic-calendar/` | Lịch kinh tế chi tiết với actual/forecast/previous |
| TradingEconomics | `https://tradingeconomics.com/calendar` | Lịch kinh tế đa quốc gia |

### 1.3 Chỉ báo kỹ thuật & Biểu đồ

| Website | URL | Dùng để |
|---------|-----|---------|
| TradingView | `https://www.tradingview.com/chart/` | Biểu đồ tương tác, chỉ báo kỹ thuật đầy đủ |
| Investing.com Technical | `https://www.investing.com/currencies/xau-usd-technical` | Tổng hợp chỉ báo (MA, RSI, MACD, Stochastic...) dạng bảng. URL: `/currencies/{symbol}-technical` |
| TradingView Technicals | `https://www.tradingview.com/symbols/XAUUSD/technicals/` | Gauge tổng hợp Buy/Sell/Neutral cho symbol cụ thể |

> **Lưu ý**: Thay `XAUUSD` trong URL TradingView bằng symbol cần tra (ví dụ: `EURUSD`, `BTCUSDT`, `FX:GBPUSD`).

---

## Phần 2 — Workflow lướt website qua Chrome DevTools MCP

### 2.1 Khởi động và điều hướng

```
Bước 1: Kiểm tra page hiện có
  → list_pages — xem danh sách tab đang mở

Bước 2: Mở trang mới hoặc điều hướng
  → new_page(url: "https://www.forexfactory.com/news")
  HOẶC navigate_page(url: "...", type: "url") nếu đã có tab

Bước 3: Chờ trang tải xong
  → wait_for(text: ["News", "Latest"]) — chờ nội dung chính xuất hiện

Bước 4: Chụp snapshot để hiểu cấu trúc
  → take_snapshot — lấy cây a11y với uid của từng phần tử
```

### 2.2 Thu thập danh sách bài viết / link tin tức

Sau khi có snapshot, dùng `evaluate_script` để trích xuất tất cả link bài viết:

```javascript
// Ví dụ cho trang tin tức — dùng data-test attrs khi có (Investing.com), fallback sang generic
() => {
  // Investing.com: stable data-test attributes
  let links = Array.from(document.querySelectorAll('a[data-test="article-title-link"]'))
    .map(a => ({ title: a.textContent?.trim().substring(0, 120), url: a.href }));

  // Fallback: generic selectors cho các site khác
  if (links.length === 0) {
    links = Array.from(document.querySelectorAll('article a, .news-item a, .title a, h3 a, h2 a'))
      .map(a => ({ title: a.textContent?.trim().substring(0, 120), url: a.href }));
  }

  return links
    .filter(item => item.url && item.title && item.title.length > 10)
    .filter((item, index, self) => self.findIndex(t => t.url === item.url) === index)
    .slice(0, 30);
}
```

**Các selector đã xác minh qua CDP (tháng 3/2026):**

| Website | Selector tin tức | Ghi chú |
|---------|----------------|---------|
| ForexFactory | `.flexposts__story-title a`, `.news__title a` | Xác minh OK |
| Investing.com | `a[data-test="article-title-link"]` | Stable `data-test` attrs. Container: `article[data-test="article-item"]`, List: `ul[data-test="news-list"]` |
| FXStreet | `article div.flex.flex-1 a[href*="fxstreet.com"]` | Tailwind-based, 10 articles/page. Không có cookie popup |
| DailyFX → IG | `h3.article-category-section-title a.primary.js_target` | **DailyFX redirect sang ig.com**, URL: `ig.com/uk/news-and-trade-ideas?source=dailyfx` |
| Reuters | `a[href*="/article/"]` | Có thể cần login |
| Google News | `main a:has(h3)` — title trong `h3`, URL từ `a.href` | Xác minh OK, thay đổi thường xuyên |

### FXStreet chi tiết

URL: `https://www.fxstreet.com/news`
- Article container: `<article>` (10 per page)
- Title link: `article div.flex.flex-1.flex-col.gap-1 > a[href*="fxstreet.com"]`
- Category badge: `span.text-overline.text-neutral-primary` (Breaking News, NEWS...)
- URL pattern: `/news/{slug}-{YYYYMMDDHHmm}`
- Không có cookie popup

### DailyFX (→ IG.com) chi tiết

DailyFX đã merge vào IG Group. URL redirect: `https://www.ig.com/uk/news-and-trade-ideas?source=dailyfx`
- Article container: `div.article-category-copy`
- Title link: `h3.article-category-section-title a.primary.js_target`
- Timestamp: `span.article-category-section-date.time` (attr `data-datetime` chứa ISO8601)
- Author: `span.article-category-section-author a.secondary`
- Section headings: `h2.articles-title` (category groups)
- URL pattern: `/uk/news-and-trade-ideas/{slug}-{YYMMDD}`
- Không có cookie popup blocking

> **Quan trọng**: Selector có thể thay đổi theo thời gian. Nếu evaluate_script trả về mảng rỗng, hãy dùng `take_snapshot` để xem lại cấu trúc DOM thực tế rồi điều chỉnh selector.

### 2.3 Đọc nội dung bài viết

Khi cần đọc chi tiết một bài viết, điều hướng tới URL rồi trích xuất nội dung:

```javascript
// Trích xuất nội dung chính của bài viết
() => {
  const selectors = ['article', '.article-body', '.caas-body', '.post-content', '[data-module="ArticleBody"]', '.story-body', 'main'];
  for (const sel of selectors) {
    const el = document.querySelector(sel);
    if (el && el.innerText.trim().length > 100) {
      return {
        title: document.title,
        content: el.innerText.trim().substring(0, 4000),
        url: window.location.href
      };
    }
  }
  return {
    title: document.title,
    content: document.body.innerText.trim().substring(0, 3000),
    url: window.location.href
  };
}
```

---

## Phần 3 — Đọc chỉ báo kỹ thuật

### 3.1 Investing.com Technical Summary

URL pattern: `https://www.investing.com/currencies/{symbol}-technical`

> **Lưu ý**: URL đúng là `/currencies/{symbol}-technical`, KHÔNG phải `/technical/{symbol}-technical-analysis` (sẽ 404).
> Investing.com có Cloudflare protection — curl trả về JS challenge. PHẢI dùng Chrome DevTools MCP.

Ví dụ:
- Vàng: `https://www.investing.com/currencies/xau-usd-technical`
- EUR/USD: `https://www.investing.com/currencies/eur-usd-technical`

```
1. navigate_page → URL trên
2. wait_for(text: ["Technical Indicators", "Moving Averages"])
3. evaluate_script để trích xuất dữ liệu:
```

```javascript
() => {
  // Investing.com dùng data-test="dynamic-table" cho tất cả bảng
  const tables = document.querySelectorAll('div[data-test="dynamic-table"] table');
  const result = { summary: '', oscillators: [], movingAverages: [], pivots: [] };

  // Summary heading chứa gauge result
  const summaryH2 = document.querySelector('h2');
  if (summaryH2) result.summary = summaryH2.innerText.trim();

  const parseTable = (table) => {
    return Array.from(table.querySelectorAll('tr')).slice(1).map(row => {
      const cells = Array.from(row.querySelectorAll('td'));
      return cells.map(c => c.innerText.trim());
    }).filter(r => r.length >= 2);
  };

  if (tables[0]) result.oscillators = parseTable(tables[0]);  // Summary table
  if (tables[1]) result.oscillators = parseTable(tables[1]);  // Oscillators detail
  if (tables[2]) result.movingAverages = parseTable(tables[2]); // MA detail
  if (tables[3]) result.pivots = parseTable(tables[3]);        // Pivots

  return result;
}
```

Kết quả mẫu (XAUUSD, tháng 3/2026):
- Summary: "Strong Sell" (MA: 0 Buy / 12 Sell, Indicators: 0 Buy / 9 Sell)
- RSI(14): 29.12 → Sell
- MACD(12,26): -26.35 → Sell
- MA200 SMA: 5141.69 → Sell

### 3.2 TradingView Scanner API (ưu tiên — không cần mở trang)

TradingView có Scanner API trả JSON trực tiếp — nhanh hơn và ổn định hơn DOM scraping (DOM dùng hashed CSS modules, đổi mỗi deploy).

**Dùng `evaluate_script` trên bất kỳ trang TradingView nào đã mở:**

```javascript
async () => {
  const symbol = 'OANDA:XAUUSD'; // Thay bằng symbol cần tra
  const fields = [
    'Recommend.All', 'Recommend.MA', 'Recommend.Other',
    'RSI', 'RSI[1]', 'Stoch.K', 'Stoch.D', 'CCI20', 'ADX', 'ADX+DI', 'ADX-DI',
    'AO', 'AO[1]', 'Mom', 'MACD.macd', 'MACD.signal',
    'Stoch.RSI.K', 'W.R', 'BBPower', 'UO',
    'EMA10', 'EMA20', 'EMA30', 'EMA50', 'EMA100', 'EMA200',
    'SMA10', 'SMA20', 'SMA30', 'SMA50', 'SMA100', 'SMA200',
    'Ichimoku.BLine', 'VWMA', 'HullMA9', 'close'
  ];
  const url = `https://scanner.tradingview.com/symbol?symbol=${symbol}&fields=${fields.join(',')}`;
  const resp = await fetch(url, {
    headers: { 'Origin': 'https://www.tradingview.com' }
  });
  return await resp.json();
}
```

**Rating scale cho `Recommend.*`:**

| Giá trị | Nhãn | Ý nghĩa |
|---------|------|---------|
| > 0.5 | Strong Buy | Tín hiệu mua mạnh |
| 0.1 → 0.5 | Buy | Tín hiệu mua |
| -0.1 → 0.1 | Neutral | Trung lập |
| -0.5 → -0.1 | Sell | Tín hiệu bán |
| < -0.5 | Strong Sell | Tín hiệu bán mạnh |

**Ví dụ kết quả XAUUSD (D1, tháng 3/2026):**
- `Recommend.All: -0.16` → Sell
- `RSI: 47.78`, `MACD: 47.93 / signal: 74.50`
- `EMA200: 4189.50`, `SMA200: 4050.13` (giá ~5019 → trên MA dài hạn)

### 3.3 TradingView — Fallback DOM (khi API không khả dụng)

URL pattern: `https://www.tradingview.com/symbols/{symbol}/technicals/`

TradingView dùng React + CSS Modules (class name hashed), nên CSS selectors KHÔNG ổn định. Dùng ARIA/semantic selectors thay thế:

```
1. navigate_page → URL trên
2. wait_for(text: ["Relative Strength Index", "Moving Averages"])
3. evaluate_script:
```

```javascript
() => {
  // Lấy toàn bộ text từ main content — reliable nhất cho hashed CSS
  const main = document.querySelector('main[aria-label="Main content"]') || document.querySelector('main');
  if (!main) return { error: 'main element not found' };

  const text = main.innerText;

  // Parse gauge summary từ text patterns
  const parseGauge = (sectionName) => {
    const idx = text.indexOf(sectionName);
    if (idx === -1) return null;
    const chunk = text.substring(idx, idx + 200);
    const sellMatch = chunk.match(/Sell\s+(\d+)/);
    const neutralMatch = chunk.match(/Neutral\s+(\d+)/);
    const buyMatch = chunk.match(/Buy\s+(\d+)/);
    return {
      sell: sellMatch ? parseInt(sellMatch[1]) : 0,
      neutral: neutralMatch ? parseInt(neutralMatch[1]) : 0,
      buy: buyMatch ? parseInt(buyMatch[1]) : 0,
    };
  };

  return {
    oscillators: parseGauge('Oscillators'),
    summary: parseGauge('Summary'),
    movingAverages: parseGauge('Moving Averages'),
    fullText: text.substring(0, 5000)
  };
}
```

### 3.4 Mapping symbol → URL / API

| HybridTrade Symbol | Investing.com path | TradingView page | Scanner API symbol |
|--------------------|--------------------|--------------------|-------------------|
| XAUUSD | `xau-usd` | `XAUUSD` | `OANDA:XAUUSD` |
| XAGUSD | `xag-usd` | `XAGUSD` | `OANDA:XAGUSD` |
| EURUSD | `eur-usd` | `EURUSD` | `FX:EURUSD` |
| GBPUSD | `gbp-usd` | `GBPUSD` | `FX:GBPUSD` |
| USNDAQ100 | `nq-100` | `NASDAQ:NDX` | `NASDAQ:NDX` |
| US30 | `us-30` | `TVC:DJI` | `TVC:DJI` |
| US500 | `us-spx-500` | `SP:SPX` | `SP:SPX` |
| UK100 | `uk-100` | `FTSE:UKX` | `FTSE:UKX` |
| WTI | `crude-oil` | `TVC:USOIL` | `TVC:USOIL` |
| BRENT | `brent-oil` | `TVC:UKOIL` | `TVC:UKOIL` |
| BTCUSDT | `btc-usd` | `BTCUSDT` | `BINANCE:BTCUSDT` |

---

## Phần 4 — Spawn team subagent đọc song song

Khi thu thập được danh sách nhiều bài viết (5+), thay vì đọc từng bài tuần tự, **spawn team subagent** để đọc song song và báo cáo tổng hợp.

### 4.1 Workflow tổng thể

```
┌─────────────────────────────────────────────────────────────────┐
│  KUROMI (Agent chính)                                           │
│                                                                 │
│  1. Mở website tin tức qua CDP                                  │
│  2. Thu thập danh sách link bài viết (evaluate_script)          │
│  3. Phân loại link theo chủ đề / tài sản                       │
│  4. Spawn team subagent — mỗi agent nhận 1 nhóm link           │
│  5. Nhận báo cáo tổng hợp → phân tích → update_dashboard       │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ Scout #1 │  │ Scout #2 │  │ Scout #3 │  │ Analyst  │        │
│  │ đọc 3-5  │  │ đọc 3-5  │  │ đọc 3-5  │  │ chỉ báo  │        │
│  │ bài Fed  │  │ bài Gold │  │ bài EUR  │  │ kỹ thuật │        │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │
│       │              │              │              │             │
│       └──────────────┴──────────────┴──────────────┘             │
│                        ↓                                        │
│              Báo cáo tổng hợp → Kuromi                          │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Cách spawn team

Dùng tool `spawn_team` với cấu trúc sau:

```json
{
  "mission": "Đọc và phân tích {N} bài viết forex từ {website} về chủ đề {topic}, trích xuất thông tin tác động tới các cặp tiền tệ và tài sản liên quan",
  "briefing": "Mỗi scout dùng Chrome DevTools MCP (CDP) để navigate tới link bài viết, đọc nội dung bằng evaluate_script, rồi tóm tắt. Analyst kiểm tra chỉ báo kỹ thuật trên TradingView/Investing.com.",
  "rounds": 2,
  "report_instruction": "Báo cáo ngắn gọn theo format:\n- Tài sản bị ảnh hưởng: [symbols]\n- Hướng tác động: bullish/bearish/neutral\n- Mức độ quan trọng: cao/trung bình/thấp\n- Tóm tắt: 2-3 câu\n- Nguồn: URL bài viết",
  "members": [
    {
      "name": "scout-macro",
      "responsibility": "Đọc các bài tin macro (Fed, ECB, BOJ, GDP, CPI, NFP)",
      "instructions": "Dùng CDP: new_page → navigate tới từng link → evaluate_script trích nội dung → tóm tắt tác động tới USD, vàng, chứng khoán. Danh sách link cần đọc:\n{links_macro}"
    },
    {
      "name": "scout-forex",
      "responsibility": "Đọc các bài phân tích cặp tiền tệ (EUR, GBP, JPY)",
      "instructions": "Dùng CDP: new_page → navigate tới từng link → evaluate_script trích nội dung → tóm tắt dự báo và mức giá quan trọng. Danh sách link:\n{links_forex}"
    },
    {
      "name": "scout-commodities",
      "responsibility": "Đọc các bài về vàng, dầu, crypto",
      "instructions": "Dùng CDP: new_page → navigate tới từng link → evaluate_script trích nội dung → tóm tắt xu hướng và mức giá. Danh sách link:\n{links_commodities}"
    },
    {
      "name": "technical-analyst",
      "responsibility": "Tra chỉ báo kỹ thuật cho các symbol chính",
      "instructions": "Dùng CDP vào TradingView technicals hoặc Investing.com technical analysis cho các symbol: {symbols}. Lấy RSI, MACD, MA summary, overall gauge (Strong Buy/Buy/Neutral/Sell/Strong Sell). Chụp screenshot gauge nếu cần."
    }
  ]
}
```

> **Lưu ý**: Thay `{links_macro}`, `{links_forex}`, `{links_commodities}`, `{symbols}` bằng dữ liệu thực thu thập được từ bước 2.

### 4.3 Phân bổ link cho subagent

Quy tắc phân bổ:
- **Tối đa 5 link/subagent** — nhiều hơn sẽ timeout hoặc quá tải context
- Phân loại theo keyword trong tiêu đề:
  - `Fed, ECB, BOJ, BOE, rate, GDP, CPI, NFP, inflation, employment, jobs` → **scout-macro**
  - `EUR, GBP, JPY, CHF, AUD, NZD, CAD, forex, currency, FX` → **scout-forex**
  - `gold, silver, oil, crude, OPEC, Bitcoin, crypto, XAU, WTI, BRENT` → **scout-commodities**
  - Link không phân loại được → gán vào scout có ít link nhất

### 4.4 Template evaluate_script cho subagent đọc bài

Mỗi subagent nên dùng quy trình sau cho mỗi link:

```
1. new_page(url: "{article_url}")
2. wait_for(text: ["..."]) — chờ content tải
3. evaluate_script:
   () => {
     const article = document.querySelector('article, .article-body, .post-content, main');
     const text = article ? article.innerText : document.body.innerText;
     return {
       title: document.title,
       content: text.substring(0, 3500),
       url: window.location.href,
       time: document.querySelector('time, [datetime], .date, .timestamp')?.textContent?.trim()
     };
   }
4. Phân tích nội dung → viết tóm tắt
```

---

## Phần 5 — Đọc lịch kinh tế trực tiếp từ website

### 5.1 ForexFactory Calendar

**Cách 1 — Free Public JSON API (ưu tiên, không cần scrape DOM):**

ForexFactory cung cấp dữ liệu calendar miễn phí ở nhiều format:

| Format | URL |
|--------|-----|
| JSON | `https://nfs.faireconomy.media/ff_calendar_thisweek.json` |
| CSV | `https://nfs.faireconomy.media/ff_calendar_thisweek.csv` |
| XML | `https://nfs.faireconomy.media/ff_calendar_thisweek.xml` |
| ICS | `https://nfs.faireconomy.media/ff_calendar_thisweek.ics` |

Dùng `evaluate_script` hoặc fetch trực tiếp:

```javascript
async () => {
  const resp = await fetch('https://nfs.faireconomy.media/ff_calendar_thisweek.json');
  return await resp.json();
}
```

Mỗi event trong JSON có: `id`, `name`, `currency`, `country`, `timeLabel`, `actual`, `forecast`, `previous`, `impactName`, `impactClass`, `date`, `url`.

**Cách 2 — JavaScript state object (chạy trên trang calendar đã load):**

```javascript
() => {
  // ForexFactory render calendar data vào window.calendarComponentStates
  const states = window.calendarComponentStates;
  if (!states || !states[1]) return { error: 'calendarComponentStates not found' };
  const events = [];
  states[1].days.forEach(day => {
    day.events.forEach(evt => {
      events.push({
        date: evt.date,
        time: evt.timeLabel,
        currency: evt.currency,
        country: evt.country,
        event: evt.name,
        impact: evt.impactClass?.includes('red') ? 'HIGH' : evt.impactClass?.includes('ora') ? 'MEDIUM' : 'LOW',
        actual: evt.actual,
        forecast: evt.forecast,
        previous: evt.previous
      });
    });
  });
  return events;
}
```

**Cách 3 — DOM scraping (backup nếu 2 cách trên lỗi):**

Selectors đã xác minh qua CDP (tháng 3/2026):

| Field | CSS Selector |
|-------|-------------|
| Calendar table | `table.calendar__table` |
| Event row | `tr.calendar__row` (loại trừ `tr.calendar__row--day-breaker`) |
| First event of day | `tr.calendar__row--new-day` |
| Date | `td.calendar__cell.calendar__date` |
| Time | `td.calendar__cell.calendar__time` |
| Currency | `td.calendar__cell.calendar__currency` |
| Impact icon | `td.calendar__cell.calendar__impact` → inner `span.icon.icon--ff-impact-[COLOR]` |
| Event name | `span.calendar__event-title` (inside `td.calendar__cell.calendar__event`) |
| Actual | `td.calendar__cell.calendar__actual` → inner span `.better` hoặc `.worse` |
| Forecast | `td.calendar__cell.calendar__forecast` |
| Previous | `td.calendar__cell.calendar__previous` → có thể có class `.revised` |

Actual/Forecast/Previous value states:
- Beat forecast: `<span class="better">3.0%</span>`
- Miss forecast: `<span class="worse">3.15T</span>`
- Revised: `<span class="revised worse" title="Revised from 4.5%">4.4%<span class="icon icon--revised"></span></span>`
- Chưa release: cell rỗng (no inner span)

Impact level theo icon class:
- `icon--ff-impact-red` → HIGH
- `icon--ff-impact-ora` → MEDIUM
- `icon--ff-impact-yel` → LOW
- `icon--ff-impact-gra` → Non-Economic / Holiday

```
1. navigate_page(url: "https://www.forexfactory.com/calendar")
2. wait_for(text: ["Impact", "Currency"])
3. evaluate_script:
```

```javascript
() => {
  const events = [];
  document.querySelectorAll('tr.calendar__row:not(.calendar__row--day-breaker)').forEach(row => {
    const time = row.querySelector('.calendar__time span')?.innerText?.trim();
    const currency = row.querySelector('.calendar__currency abbr span')?.innerText?.trim();
    const event = row.querySelector('span.calendar__event-title')?.innerText?.trim();
    const impactEl = row.querySelector('.calendar__impact span.icon');
    const impactClass = impactEl?.className || '';
    const actualEl = row.querySelector('.calendar__actual span');
    const actual = actualEl?.innerText?.trim();
    const actualState = actualEl?.classList.contains('better') ? 'BEAT' : actualEl?.classList.contains('worse') ? 'MISS' : '';
    const forecast = row.querySelector('.calendar__forecast span')?.innerText?.trim();
    const prevEl = row.querySelector('.calendar__previous span');
    const previous = prevEl?.innerText?.trim();
    const revised = prevEl?.classList.contains('revised');
    if (event && event.length > 2) {
      events.push({
        time, currency, event,
        impact: impactClass.includes('red') ? 'HIGH' : impactClass.includes('ora') ? 'MEDIUM' : impactClass.includes('yel') ? 'LOW' : 'NONE',
        actual, actualState, forecast, previous, revised: !!revised
      });
    }
  });
  return events;
}
```

### 5.2 Investing.com Economic Calendar

> **Lưu ý**: Investing.com có Cloudflare — PHẢI dùng Chrome DevTools MCP, curl sẽ bị chặn.

```
1. navigate_page(url: "https://www.investing.com/economic-calendar/")
2. wait_for(text: ["Economic Calendar", "Time"])
3. evaluate_script:
```

```javascript
() => {
  const events = [];
  // Investing.com dùng div[data-test="dynamic-table"] với row IDs encode eventId
  const rows = document.querySelectorAll('div[data-test="dynamic-table"] tr');
  rows.forEach(row => {
    const cells = Array.from(row.querySelectorAll('td'));
    if (cells.length < 5) return; // skip header/date rows

    const time = cells[0]?.innerText?.trim();
    const currency = cells[1]?.innerText?.trim();
    // Event cell chứa link tới detail page
    const eventLink = row.querySelector('a[href*="/economic-calendar/"]');
    const event = eventLink?.innerText?.trim() || cells[2]?.innerText?.trim();
    const eventUrl = eventLink?.href || '';
    // Impact: đếm icon elements (3 = HIGH, 2 = MEDIUM, 1 = LOW)
    const impactCell = cells[3];
    const impactIcons = impactCell?.querySelectorAll('i, span, img').length || 0;
    const actual = cells[4]?.innerText?.trim();
    const forecast = cells[5]?.innerText?.trim();
    const previous = cells[6]?.innerText?.trim();

    if (event && event.length > 3) {
      events.push({
        time, currency, event, eventUrl,
        importance: impactIcons >= 3 ? 'HIGH' : impactIcons >= 2 ? 'MEDIUM' : 'LOW',
        actual, forecast, previous
      });
    }
  });
  return events;
}
```

### 5.3 TradingEconomics Calendar

TradingEconomics dùng ASP.NET WebForms, bảng HTML chuẩn:

```
1. navigate_page(url: "https://tradingeconomics.com/calendar")
2. wait_for(text: ["Calendar", "Event"])
3. evaluate_script:
```

```javascript
() => {
  const events = [];
  const rows = document.querySelectorAll('table tr');
  let currentDate = '';
  rows.forEach(row => {
    const cells = Array.from(row.querySelectorAll('td'));
    if (cells.length < 4) {
      // Check if it's a date separator row
      const text = row.innerText?.trim();
      if (text && text.match(/\w+ \w+ \d+/)) currentDate = text;
      return;
    }
    const time = cells[0]?.innerText?.trim();
    const country = cells[1]?.innerText?.trim();
    const event = cells[2]?.innerText?.trim();
    const actual = cells[3]?.innerText?.trim();
    const previous = cells[4]?.innerText?.trim();
    const consensus = cells[5]?.innerText?.trim();
    const forecast = cells[6]?.innerText?.trim();
    if (event && event.length > 3) {
      events.push({ date: currentDate, time, country, event, actual, previous, consensus, forecast });
    }
  });
  return events;
}
```

---

## Phần 6 — Quy trình tổng hợp báo cáo

Sau khi team subagent hoàn tất, Kuromi nhận báo cáo và tổng hợp theo template:

```
📊 BÁO CÁO NGHIÊN CỨU FOREX — {ngày tháng}

━━━ TIN TỨC QUAN TRỌNG ━━━
{Tóm tắt từ scout-macro: tin gì vừa ra, tác động ra sao}

━━━ PHÂN TÍCH TỪNG TÀI SẢN ━━━
▸ XAUUSD (Vàng)
  Tin tức: {tóm tắt}
  Kỹ thuật: RSI={giá trị}, MACD={tín hiệu}, MA={tín hiệu}
  Gauge: {Strong Buy/Buy/Neutral/Sell/Strong Sell}
  Nhận định: {bullish/bearish/neutral} — {lý do ngắn}

▸ EURUSD
  ... (tương tự)

━━━ SỰ KIỆN SẮP TỚI ━━━
{Lịch kinh tế quan trọng trong 24h tới}

━━━ KẾT LUẬN ━━━
{Tâm lý thị trường chung: risk-on / risk-off}
{Top 3 cặp đáng chú ý và hướng dự kiến}
{Cảnh báo rủi ro nếu có}
```

Sau khi tổng hợp xong, dùng tool `update_dashboard` để cập nhật nhận định cho từng symbol lên dashboard.

---

## Phần 7 — Google Search để tìm tin tức và phân tích

Khi cần tìm tin tức mới nhất hoặc phân tích cụ thể mà các website trong danh sách không đủ, dùng Google Search qua CDP:

### 7.1 Tìm kiếm Google qua CDP

```
1. new_page(url: "https://www.google.com/search?q={query}&tbm=nws&hl=en")
   → &tbm=nws để tìm trong tab News, &hl=en để force English
   → Hoặc bỏ &tbm=nws để tìm general
2. wait_for(text: ["result", "Results"])
3. evaluate_script để trích link kết quả:
```

```javascript
// Google search results extraction — xác minh CDP tháng 3/2026
// Google thường xuyên đổi DOM, nên dùng kết hợp nhiều selector
() => {
  const results = [];
  // Approach 1: a:has(h3) — hoạt động ổn định nhất
  document.querySelectorAll('main a:has(h3)').forEach(a => {
    const h3 = a.querySelector('h3');
    if (!h3) return;
    const url = a.href;
    if (!url || url.includes('google.com') || url.includes('accounts.google')) return;
    const title = h3.innerText.trim();
    // Snippet: tìm text block gần nhất
    const container = a.closest('[data-hveid]') || a.parentElement?.parentElement;
    const snippetEls = container?.querySelectorAll('span[style*="line-clamp"], .VwiC3b, [data-sncf]') || [];
    const snippet = Array.from(snippetEls).map(n => n.innerText).join(' ').substring(0, 300);
    results.push({ title, url, snippet });
  });
  // Deduplicate
  const seen = new Set();
  return results.filter(r => {
    if (seen.has(r.url)) return false;
    seen.add(r.url);
    return r.title.length > 5;
  }).slice(0, 15);
}
```

### 7.2 Query template cho forex

Các query hiệu quả:

| Mục đích | Query |
|----------|-------|
| Tin vàng mới nhất | `XAUUSD gold price news today {year}` |
| Fed rate decision | `Fed interest rate decision {month} {year}` |
| ECB analysis | `ECB monetary policy analysis latest` |
| Oil outlook | `WTI crude oil price forecast this week` |
| Forex forecast | `{pair} forecast analysis this week` |
| Economic data | `US CPI data release today` |
| Geopolitical | `geopolitical risk markets impact today` |

> **Lưu ý**: Google có thể hiển thị CAPTCHA hoặc consent screen. Nếu gặp, thử:
> - `click` vào nút "Accept" / "Agree" nếu có
> - Hoặc chuyển sang DuckDuckGo: `https://duckduckgo.com/?q={query}&ia=news`

### 7.3 DuckDuckGo — Backup search engine

Nếu Google bị chặn hoặc yêu cầu CAPTCHA:

```
1. new_page(url: "https://duckduckgo.com/?q={query}&ia=news")
2. wait_for(text: ["results", "Results"])
3. evaluate_script:
```

```javascript
() => {
  const results = Array.from(document.querySelectorAll('.result__a, a.result__url, [data-testid="result-title-a"]'))
    .map(a => ({
      title: a.textContent?.trim().substring(0, 150),
      url: a.href,
    }))
    .filter(item => item.title && item.title.length > 5);
  return results.slice(0, 15);
}
```

---

## Phần 8 — Reusable Skills: Tự học và tái sử dụng

### 8.1 Concept

Mỗi lần Kuromi truy cập thành công một website tài chính, tớ **tự động lưu micro-skill** chứa selectors, scripts, và tips đã hoạt động cho domain đó. Lần sau quay lại, tớ **tự động load skill đã lưu** và dùng ngay — không cần dò lại từ đầu.

### 8.2 Workflow bắt buộc

```
TRƯỚC KHI NAVIGATE:
  reusable_skills(action: "match", url: "{target_url}")
  → Nếu found → dùng skill.pages[page_type].selectors và .extract_script
  → Nếu not found → dò thủ công bằng snapshot + evaluate_script

SAU KHI EXTRACT THÀNH CÔNG:
  reusable_skills(action: "save", domain: "{domain}", skill_data: {
    "name": "ForexFactory News Scraper",
    "description": "Extract tin tức từ ForexFactory news page",
    "pages": {
      "news": {
        "url_pattern": "/news",
        "selectors": {
          "article_links": ".flexposts__story-title a",
          "article_title": ".flexposts__story-title",
          "article_time": ".flexposts__time"
        },
        "extract_script": "() => { ... script đã test thành công ... }",
        "wait_for": ["Latest News", "News"],
        "cookie_dismiss": "#onetrust-accept-btn-handler"
      },
      "calendar": {
        "url_pattern": "/calendar",
        "selectors": {
          "event_row": ".calendar__row",
          "event_time": ".calendar__time",
          "event_title": ".calendar__event-title",
          "event_impact": ".calendar__impact span"
        },
        "extract_script": "() => { return window.calendarComponentStates || []; }",
        "wait_for": ["Impact", "Currency"]
      }
    },
    "tips": [
      "ForexFactory calendar có window.calendarComponentStates chứa structured data",
      "Cookie popup dùng OneTrust, dismiss bằng #onetrust-accept-btn-handler"
    ]
  })
```

### 8.3 Tự sửa skill khi selector cũ hỏng

```
1. reusable_skills(action: "match", url: "https://www.forexfactory.com/news")
2. Dùng saved selectors → evaluate_script trả về rỗng
3. take_snapshot → xem DOM mới → tìm selector mới
4. Test selector mới bằng evaluate_script → thành công
5. reusable_skills(action: "save", ...) → cập nhật skill với selector mới
```

### 8.4 Quản lý skills

- `reusable_skills(action: "list")` — xem tất cả skills đã học
- `reusable_skills(action: "load", domain: "forexfactory.com")` — load full skill cho domain
- `reusable_skills(action: "delete", domain: "...")` — xoá skill cũ/sai

---

## Phần 9 — Mẹo thực chiến

### Xử lý popup / cookie consent
Nhiều website tài chính hiển thị popup cookie. Xử lý:
```
take_snapshot → tìm nút "Accept" / "Agree" / "OK" → click(uid: "...")
```

### Website chặn hoặc yêu cầu đăng nhập
- Nếu bị chặn → chuyển sang website khác trong danh sách
- Nếu cần đăng nhập → bỏ qua, ưu tiên website miễn phí
- Luôn có ít nhất 3 nguồn backup cho mỗi loại dữ liệu

### Tối ưu tốc độ
- Dùng `evaluate_script` thay vì đọc từng phần tử qua snapshot — nhanh hơn nhiều
- Spawn team khi có 5+ bài cần đọc — nhanh hơn gấp 3-4 lần so với đọc tuần tự
- Giới hạn nội dung trích xuất 3000-4000 ký tự/bài — đủ để tóm tắt mà không tốn context

### Khi evaluate_script trả về rỗng
1. Chụp `take_screenshot` để xem trang thực tế
2. Dùng `take_snapshot` để xem lại cây DOM
3. Điều chỉnh selector dựa trên cấu trúc thực tế
4. Thử selector tổng quát hơn: `document.body.innerText.substring(0, 3000)`

### Kết hợp với tool native
- Trước khi lướt web, gọi `fetch_news` và `fetch_calendar` để có baseline
- So sánh kết quả từ web với dữ liệu native để cross-check
- Sau khi tổng hợp xong, dùng `update_dashboard` để đẩy kết quả lên dashboard
- **Luôn gọi `reusable_skills(action: "match")` trước khi navigate** — tiết kiệm thời gian dò selector
- **Luôn gọi `reusable_skills(action: "save")` sau khi extract thành công** — để lần sau tái sử dụng

### Subagent và reusable_skills
- Khi spawn subagent để đọc bài, mỗi subagent cũng nên gọi `reusable_skills(action: "match")` cho domain mà nó sẽ đọc
- Subagent sau khi tìm được selector mới cho một site, nên gọi `reusable_skills(action: "save")` để main agent và các subagent khác cũng được hưởng lợi
- Kết quả: hệ thống skills tự lớn dần theo thời gian sử dụng, mỗi website được tối ưu tự động
