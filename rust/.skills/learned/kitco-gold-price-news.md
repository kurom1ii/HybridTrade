# Kitco Gold Price & News

## URL
- Charts: https://www.kitco.com/charts/gold
- Gold price today: https://www.kitco.com/gold-price-today-usa/ (redirects to /charts/gold)
- News: https://www.kitco.com/news/category/commodities/gold
- Opinion: https://www.kitco.com/opinion

## Key Selectors / A11y UIDs (snapshot-based)
- Live Gold Bid Price: heading "5,xxx.xx" (level 3) under "Live Gold Price" heading
- Day Change: StaticText "-xx.xx" + StaticText "-x.xx%"
- Ask Price: StaticText after "Ask"
- Day's Range: Two StaticTexts around "|" separator showing low and high (e.g. "5008.90" | "5129.30")
- Date/Time: StaticText "Mar 15, 2026 - 10:12 NY Time"
- News links: under heading "LATEST NEWS" level 2, links with timestamps

## Page Structure
- /charts/gold: Live spot price, day range, TradingView chart widget, 5 latest news links
- News article: Title (h1), author, date, full article text, tags
- Weekly Gold Survey article: Wall Street % bullish/bearish/neutral, Main Street poll results

## Important Data Points to Extract
1. Bid price (heading level 3 after "Live Gold Price")
2. Change amount and % (StaticText immediately after bid)
3. Day's Range: low | high separated by "|"
4. Latest 5 news headlines with dates
5. Weekly Gold Survey: Wall Street split, Main Street sentiment

## Workflow
1. Navigate to https://www.kitco.com/charts/gold
2. take_snapshot → extract price, range, news list
3. Click/navigate to 1-2 key news articles for detail
4. Look for Weekly Gold Survey article for expert opinions

## Tips
- Site may timeout (10000ms) but page still loads partially - use take_snapshot immediately
- TradingView chart widget embedded - cannot interact directly
- Jim Wyckoff (Kitco senior analyst) provides key technical levels in weekly articles
- Weekly Gold Survey published every Friday (price forecast for next week)
- Price Calculator widget also shows current price + change
