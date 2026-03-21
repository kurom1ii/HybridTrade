# Skill: Investing.com - Gold Price & Technical Analysis

## URL Patterns
- https://www.investing.com/commodities/gold
- https://www.investing.com/commodities/gold-technical
- https://www.investing.com/commodities/gold-news

## Key Data Locations (A11y Snapshot)
- **Price**: StaticText near heading "Gold (GCJ6)" - giá dạng "5,061.70"
- **Change**: StaticText "-64.10" và "(-1.25%)"
- **Day's Range**: StaticText "5,014.10" và "5,132.40"
- **Prev Close / Open**: Các StaticText sau label tương ứng
- **Volume**: StaticText sau "Volume"

## Technical Tab (gold-technical)
- Tabs: 30 Min, Hourly, 5 Hours, Daily, Weekly, Monthly (có ghi sẵn signal vào tên tab)
- Technical Indicators table: RSI(14), STOCH, STOCHRSI, MACD, ADX, Williams %R, CCI, ATR, etc.
- Moving Averages table: MA5/10/20/50/100/200 (SMA & EMA)
- Pivot Points: Classic, Fibonacci, Camarilla, Woodie's, DeMark's

## Selectors Notes
- Tab Daily: uid dạng "2_234" (tab "Daily Sell" selectable)
- Tab Hourly: uid "2_232" 
- Login popup xuất hiện khi click tab 1Min/5Min/15Min - nhấn Escape để đóng

## Workflow
1. Truy cập /commodities/gold → lấy giá, change%, range, volume, performance %
2. Truy cập /commodities/gold-technical → đọc tab summary (Daily mặc định visible)
3. Truy cập /commodities/gold-news → danh sách tin tức mới nhất

## Tips
- Không cần login cho khung 30Min/Hourly/Daily/Weekly/Monthly
- Pivot Points nằm ở cuối trang technical
- Tên tab đã encode signal sẵn: "Daily Sell", "Hourly Strong Sell", "Weekly Strong Buy"
