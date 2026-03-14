import { NextResponse } from "next/server";

const FASTBULL_API =
  "https://api.fastbull.com/fastbull-quotes-service/api/postSnapshotByIds";

/** Map from our display symbol → FastBull stockId */
const SYMBOL_MAP: Record<string, string> = {
  "XAUUSD": "8500_XAUUSD",
  "XAGUSD": "8500_XAGUSD",
  "EURUSD": "8100_EURUSD",
  "GBPUSD": "8100_GBPUSD",
  "USDJPY": "8100_USDJPY",
  "GBPJPY": "8200_GBPJPY",
  "USDCAD": "8100_USDCAD",
  "AUDUSD": "8100_AUDUSD",
  "NZDUSD": "8100_NZDUSD",
  "USDCHF": "8100_USDCHF",
  "BTCUSDT": "6100_BTC-USDT",
  "ETHUSDT": "6100_ETH-USDT",
  "SOLUSDT": "6100_SOL-USDT",
  "USNDAQ100": "8700_USNDAQ100",
  "US30": "8700_US30",
  "US500": "8700_USSPX500",
  "UK100": "8700_UK100",
  "JP225": "8700_Japan225",
  "WTI": "8600_WTI",
  "BRENT": "8600_BRENT",
  "NATGAS": "8600_NAT.GAS",
  "XPTUSD": "8500_XPTUSD",
  "XPDUSD": "8500_XPDUSD",
  "COPPER": "8800_COPPER",
};

/** Reverse map: FastBull stockId → our display symbol */
const STOCK_ID_TO_SYMBOL: Record<string, string> = {};
for (const [sym, sid] of Object.entries(SYMBOL_MAP)) {
  STOCK_ID_TO_SYMBOL[sid] = sym;
}

const DEFAULT_SYMBOLS = [
  "XAUUSD", "XAGUSD", "EURUSD", "GBPUSD", "USDJPY",
  "BTCUSDT", "ETHUSDT", "US500", "WTI",
];

export interface PriceSnapshot {
  symbol: string;
  name: string;
  price: number;
  bid: number;
  ask: number;
  change: number;
  changePct: number;
  high: number;
  low: number;
  open: number;
  prev: number;
  precision: number;
  category: string;
  stockId: string;
}

function categoryFromMainType(mainType: string): string {
  switch (mainType) {
    case "101": return "FOREX";
    case "102": return "COMMODITIES";
    case "103": return "INDICES";
    case "104": return "CRYPTO";
    default: return mainType;
  }
}

function categoryFromMarketType(marketType: string): string {
  if (marketType.startsWith("81") || marketType.startsWith("82")) return "FOREX";
  if (marketType.startsWith("85") || marketType.startsWith("86") || marketType.startsWith("88") || marketType.startsWith("89")) return "COMMODITIES";
  if (marketType.startsWith("61") || marketType.startsWith("63")) return "CRYPTO";
  if (marketType.startsWith("87") || marketType.startsWith("91")) return "INDICES";
  return "OTHER";
}

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const symbolsParam = searchParams.get("symbols");

  const requestedSymbols = symbolsParam
    ? symbolsParam.split(",").map((s) => s.trim().toUpperCase())
    : DEFAULT_SYMBOLS;

  const stockList = requestedSymbols
    .map((sym) => {
      const stockId = SYMBOL_MAP[sym];
      return stockId ? { dataPlatform: "FB", stockId } : null;
    })
    .filter(Boolean);

  if (stockList.length === 0) {
    return NextResponse.json({ error: "No valid symbols provided" }, { status: 400 });
  }

  try {
    const res = await fetch(FASTBULL_API, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ stockSnapshotList: stockList }),
      cache: "no-store",
    });

    if (!res.ok) {
      return NextResponse.json(
        { error: `FastBull API returned ${res.status}` },
        { status: 502 },
      );
    }

    const data = await res.json();
    if (data.code !== 0) {
      return NextResponse.json(
        { error: `FastBull error: ${data.message}` },
        { status: 502 },
      );
    }

    const items: unknown[] =
      typeof data.bodyMessage === "string"
        ? JSON.parse(data.bodyMessage)
        : data.bodyMessage;

    const prices: PriceSnapshot[] = items.map((raw: any) => ({
      symbol: STOCK_ID_TO_SYMBOL[raw.stockId] || raw.symbol,
      name: raw.stockName,
      price: raw.close ?? raw.askPrice,
      bid: raw.bidPrice,
      ask: raw.askPrice,
      change: raw.change,
      changePct: raw.changeRate,
      high: raw.high,
      low: raw.low,
      open: raw.openPrice,
      prev: raw.prePrice,
      precision: raw.precision,
      category: categoryFromMainType(raw.mainType) !== raw.mainType
        ? categoryFromMainType(raw.mainType)
        : categoryFromMarketType(raw.marketType),
      stockId: raw.stockId,
    }));

    return NextResponse.json({ prices, ts: Date.now() }, {
      headers: { "Cache-Control": "no-store, max-age=0" },
    });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : "Unknown error";
    return NextResponse.json(
      { error: `Failed to fetch prices: ${message}` },
      { status: 502 },
    );
  }
}
