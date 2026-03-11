"use client";
import { motion } from "motion/react";
import { StaggerGrid, SlideIn } from "@/components/dashboard/motion-primitives";
import { StatsCard } from "@/components/dashboard/stats-card";
import { PageTitle } from "@/components/dashboard/page-title";

const filterTabs = ["ALL", "PENDING", "LIMIT", "STOP", "FILLED", "CANCELLED"];

const orders = [
 { id: "#ORD-001", pair: "EUR/USD", type: "BUY LIMIT", price: "1.0850", size: "1.0 Lot", status: "Pending", time: "10:24 AM", sl: "1.0800", tp: "1.0950" },
 { id: "#ORD-002", pair: "GBP/USD", type: "SELL STOP", price: "1.2600", size: "0.5 Lot", status: "Pending", time: "10:18 AM", sl: "1.2650", tp: "1.2500" },
 { id: "#ORD-003", pair: "USD/JPY", type: "BUY LIMIT", price: "149.50", size: "0.8 Lot", status: "Pending", time: "09:45 AM", sl: "149.00", tp: "150.50" },
 { id: "#ORD-004", pair: "BTC/USD", type: "BUY STOP", price: "44,500", size: "0.05 BTC", status: "Pending", time: "09:30 AM", sl: "43,500", tp: "46,000" },
 { id: "#ORD-005", pair: "EUR/GBP", type: "SELL LIMIT", price: "0.8620", size: "1.2 Lot", status: "Filled", time: "08:55 AM", sl: "0.8660", tp: "0.8550" },
 { id: "#ORD-006", pair: "XAU/USD", type: "BUY LIMIT", price: "2,020.0", size: "0.3 Lot", status: "Filled", time: "08:30 AM", sl: "2,005.0", tp: "2,060.0" },
 { id: "#ORD-007", pair: "AUD/USD", type: "SELL STOP", price: "0.6480", size: "1.0 Lot", status: "Cancelled", time: "Yesterday", sl: "0.6520", tp: "0.6400" },
 { id: "#ORD-008", pair: "NZD/USD", type: "BUY LIMIT", price: "0.6100", size: "0.5 Lot", status: "Cancelled", time: "Yesterday", sl: "0.6060", tp: "0.6180" },
];

export default function OrdersPage() {
 return (
 <div className="flex gap-6 h-full overflow-y-auto p-6">
 <div className="flex-1 min-w-0 space-y-6">
 <PageTitle title="Orders" subtitle="Manage pending and historical orders" breadcrumb="ORDERS / ORDER MANAGEMENT" />

 {/* Filter Tabs */}
 <div className="flex gap-1">
 {filterTabs.map((tab) => (
 <button
 key={tab}
 className={` px-4 py-1.5 text-[12px] font-semibold tracking-wider transition-colors ${
 tab === "ALL"
 ? "bg-cyan/10 text-cyan"
 : "text-muted-foreground hover:bg-secondary"
 }`}
 >
 {tab}
 </button>
 ))}
 </div>

 {/* Stats Row */}
 <StaggerGrid className="grid grid-cols-4 gap-4">
 <StatsCard title="Pending Orders" value="4" change="2 buy, 2 sell" changeType="neutral" />
 <StatsCard title="Filled Today" value="2" change="100% fill rate" changeType="profit" />
 <StatsCard title="Cancelled" value="2" change="Expired" changeType="neutral" />
 <StatsCard title="Total Value" value="$8,450" change="Pending margin" changeType="neutral" />
 </StaggerGrid>

 {/* Orders Table */}
 <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5, delay: 0.2 }}>
 <div className=" border border-border bg-card">
 <div className="border-b border-border px-4 py-3">
 <h3 className="text-sm font-semibold">All Orders</h3>
 </div>
 <div className="overflow-x-auto">
 <table className="w-full text-[13px]">
 <thead>
 <tr className="border-b border-border text-[11px] uppercase tracking-wider text-muted-foreground">
 <th className="px-4 py-3 text-left font-medium">Order ID</th>
 <th className="px-4 py-3 text-left font-medium">Pair</th>
 <th className="px-4 py-3 text-left font-medium">Type</th>
 <th className="px-4 py-3 text-right font-medium">Price</th>
 <th className="px-4 py-3 text-right font-medium">Size</th>
 <th className="px-4 py-3 text-right font-medium">S/L</th>
 <th className="px-4 py-3 text-right font-medium">T/P</th>
 <th className="px-4 py-3 text-left font-medium">Status</th>
 <th className="px-4 py-3 text-right font-medium">Time</th>
 </tr>
 </thead>
 <tbody>
 {orders.map((order, i) => (
 <motion.tr key={i} initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.3, delay: 0.1 + i * 0.04 }} className="border-b border-border/50 transition-colors hover:bg-card-alt">
 <td className="px-4 py-3 text-muted-foreground">{order.id}</td>
 <td className="px-4 py-3 font-semibold">{order.pair}</td>
 <td className="px-4 py-3">
 <span className={`px-2 py-0.5 text-[11px] font-semibold ${
 order.type.includes("BUY") ? "bg-profit/10 text-profit" : "bg-loss/10 text-loss"
 }`}>
 {order.type}
 </span>
 </td>
 <td className="px-4 py-3 text-right font-medium">{order.price}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{order.size}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{order.sl}</td>
 <td className="px-4 py-3 text-right text-muted-foreground">{order.tp}</td>
 <td className="px-4 py-3">
 <span className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${
 order.status === "Pending"
 ? "bg-cyan/10 text-cyan"
 : order.status === "Filled"
 ? "bg-profit/10 text-profit"
 : "bg-muted text-muted-foreground"
 }`}>
 {order.status}
 </span>
 </td>
 <td className="px-4 py-3 text-right text-muted-foreground">{order.time}</td>
 </motion.tr>
 ))}
 </tbody>
 </table>
 </div>
 </div>
 </motion.div>
 </div>

 {/* Quick Order Panel */}
 <SlideIn direction="right" delay={0.3}>
 <div className="w-[300px] shrink-0">
 <div className=" border border-border bg-card">
 <div className="border-b border-border px-4 py-3">
 <h3 className="text-sm font-semibold">Quick Order</h3>
 </div>
 <div className="p-4 space-y-4">
 {/* Pair selector */}
 <div>
 <label className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Instrument</label>
 <select className="mt-1 w-full border border-border bg-secondary/50 px-3 py-2 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan">
 <option>EUR/USD</option>
 <option>GBP/USD</option>
 <option>USD/JPY</option>
 <option>BTC/USD</option>
 </select>
 </div>

 {/* BUY / SELL toggle */}
 <div className="grid grid-cols-2 gap-2">
 <button className=" bg-profit/10 py-2.5 text-[13px] font-bold text-profit transition-colors hover:bg-profit/20">
 BUY
 </button>
 <button className=" bg-secondary py-2.5 text-[13px] font-bold text-muted-foreground transition-colors hover:bg-secondary/80">
 SELL
 </button>
 </div>

 {/* Order Type */}
 <div>
 <label className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Order Type</label>
 <select className="mt-1 w-full border border-border bg-secondary/50 px-3 py-2 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan">
 <option>Market</option>
 <option>Limit</option>
 <option>Stop</option>
 </select>
 </div>

 {/* Lot size */}
 <div>
 <label className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Lot Size</label>
 <input
 type="text"
 defaultValue="1.00"
 className="mt-1 w-full border border-border bg-secondary/50 px-3 py-2 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
 />
 </div>

 {/* S/L and T/P */}
 <div className="grid grid-cols-2 gap-2">
 <div>
 <label className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Stop Loss</label>
 <input
 type="text"
 placeholder="Price"
 className="mt-1 w-full border border-border bg-secondary/50 px-3 py-2 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
 />
 </div>
 <div>
 <label className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">Take Profit</label>
 <input
 type="text"
 placeholder="Price"
 className="mt-1 w-full border border-border bg-secondary/50 px-3 py-2 text-[13px] focus:outline-none focus:ring-1 focus:ring-cyan"
 />
 </div>
 </div>

 {/* Place order */}
 <button className="w-full bg-cyan py-2.5 text-[13px] font-bold text-black transition-colors hover:bg-cyan/90">
 PLACE ORDER
 </button>
 </div>
 </div>
 </div>
 </SlideIn>
 </div>
 );
}
