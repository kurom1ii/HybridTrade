# DailyFX Gold Price - Redirect Behavior

## Trạng thái (cập nhật 2026-03-16)
- **DailyFX đã redirect 100% sang IG.com**
- URL `https://www.dailyfx.com/gold-price` → redirect `https://www.ig.com/uk/commodities/markets-commodities/gold?source=dailyfx`
- URL `https://www.dailyfx.com/gold` → redirect `https://www.ig.com/uk?source=dailyfx`
- URL `https://www.dailyfx.com/` → redirect `https://www.ig.com/uk?source=dailyfx`

## Sử dụng thay thế
- Dữ liệu gold lấy từ: `https://www.ig.com/uk/commodities/markets-commodities/gold?source=dailyfx`
- Client sentiment available: Long% vs Short%
- Price data: BUY/SELL spread, High/Low range

## IG Gold Page - Key Selectors
- Heading: "Spot Gold" (uid=1_20)
- BUY price: StaticText after "BUY" link
- SELL price: StaticText after "SELL" link
- High/Low: "H 5192.03" / "L 5009.49" format
- Sentiment: "84%" Long, "16%" Short pattern
- Change: "-138.00" and "-2.68%" pattern

## ForexFactory Calendar
- URL: `https://www.forexfactory.com/calendar`
- JSON API: `https://nfs.faireconomy.media/ff_calendar_thisweek.json`
- Impact levels: High/Medium/Low
- Snapshot shows events by day with currency flags

## Tips
- ForexFactory JSON API accessible via bash curl (không fetch từ browser do CORS)
- Events tuần hiện tại: 102 events
- Currency field trong JSON có thể để trống, dùng snapshot để lấy USD events
