# FXStreet - Gold XAU/USD Data Extraction

## URLs
- Main Gold Page: https://www.fxstreet.com/markets/commodities/metals/gold
- Rates & Charts: https://www.fxstreet.com/rates-charts/xauusd
- Latest News: https://www.fxstreet.com/news/latest/asset?dFR[Category][0]=News&dFR[Tags][0]=XAUUSD

## Selectors Key
- Price data: navigate to /rates-charts/xauusd then `document.body.innerText` - price shown in text as "XAU/USD\n5,019.18\nUSD\n-61.25\n(-1.21%)"
- Article text: `document.querySelector('article')?.innerText` or `document.body.innerText`
- Technical overview: uid matching "XAU/USD Technical Overview" heading
- News list: uid matching "LATEST XAU/USD NEWS" heading

## Workflow
1. Navigate to /markets/commodities/metals/gold for overview, technical & fundamental analysis
2. Navigate to /rates-charts/xauusd for price data (Open, High, Low, Close, % change)
3. Click into latest weekly forecast article for in-depth technical levels
4. Use evaluate_script to extract body text

## Data Points Available
- Spot price, bid, ask, CHG, CHG%, Open, High, Low
- 1D, 1W, 1M, 3M, 6M, 1Y, YTD performance
- Technical overview (support/resistance, RSI, MACD, EMA)
- Fundamental overview (macro drivers)
- Weekly forecast (premium but summary visible)
- Latest analysis articles list

## Tips
- Premium articles show first 4000 chars free
- Login not required for overview page or price data
- Economic calendar embedded in article pages
- High-impact events listed at /rates-charts/xauusd
