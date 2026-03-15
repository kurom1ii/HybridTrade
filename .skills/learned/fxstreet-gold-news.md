# FXStreet Gold/Commodities News

Lấy tin tức và phân tích XAUUSD từ FXStreet.

**Domain:** fxstreet.com
**Last verified:** 2026-03-15

## Pages

### Gold News (`/markets/commodities/metals/gold`)

**Selectors:**
- `article_title`: `h2, h3, [class*='title']`
- `article_time`: `time, [class*='time'], [class*='date']`
- `price`: `[class*='price']`

**Extract script:**
```javascript
const articles = [];
const items = document.querySelectorAll('article, [class*="article"], [class*="news-item"], [class*="card"]');
items.forEach(item => {
  const title = item.querySelector('h2, h3, h4, [class*="title"]');
  const time = item.querySelector('time, [class*="time"], [class*="date"]');
  if (title && title.innerText.trim()) {
    articles.push({
      title: title.innerText.trim(),
      time: time ? time.innerText.trim() : ''
    });
  }
});
return articles.slice(0, 15);
```

**Wait for:** `Gold`, `XAU/USD`

### Commodities News (`/news?category=commodities`)

**Selectors:**
- `article_links`: `a[href*='/news/']`
- `pagination`: `nav[aria-label='pagination']`

**Extract script:**
```javascript
const articles = [];
const links = document.querySelectorAll('a[href*="/news/"]');
links.forEach(a => {
  if (a.innerText.trim().length > 20)
    articles.push({ title: a.innerText.trim(), url: a.href });
});
return [...new Map(articles.map(a => [a.title, a])).values()].slice(0, 15);
```

**Wait for:** `Showing`, `results`

## Tips

- URL `/currencies/gold` trả về 404 — dùng `/markets/commodities/metals/gold`
- URL `/news?category=commodities` cho tin tổng hợp commodities
- Trang có nhiều quảng cáo iframe, bỏ qua
