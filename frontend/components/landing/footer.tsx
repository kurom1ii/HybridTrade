import Link from "next/link";

const footerLinks = {
  Product: ["Dashboard", "Trading Agents", "Market Data", "News Feed", "API Access"],
  Company: ["About", "Careers", "Blog", "Press", "Contact"],
  Legal: ["Terms of Service", "Privacy Policy", "Risk Disclaimer"],
  Resources: ["Documentation", "API Reference", "Tutorials", "Status"],
};

export function LandingFooter() {
  return (
    <footer className="relative border-t border-border/50 bg-panel/50 backdrop-blur-sm">
      {/* Top gradient accent */}
      <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-cyan/20 to-transparent" />

      <div className="mx-auto max-w-[1100px] px-6 py-14">
        <div className="grid grid-cols-2 gap-8 md:grid-cols-5">
          {/* Brand Column */}
          <div className="col-span-2 md:col-span-1">
            <Link href="/" className="flex items-center gap-2">
              <div className="relative flex h-7 w-7 items-center justify-center rounded-lg bg-gradient-to-br from-cyan to-[#00b4d8]">
                <svg width="14" height="14" viewBox="0 0 20 20" fill="none">
                  <path d="M5 15L8 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
                  <path d="M12 15L15 5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
                  <path d="M6.5 10H13.5" stroke="black" strokeWidth="2.5" strokeLinecap="round" />
                </svg>
              </div>
              <div className="flex items-baseline gap-0.5">
                <span className="text-[14px] font-bold tracking-[0.5px] text-cyan">HYBRID</span>
                <span className="text-[14px] font-light tracking-[0.5px] text-text-secondary">TRADE</span>
              </div>
            </Link>
            <p className="mt-3 text-[11px] leading-[1.8] text-text-secondary/60">
              AI-powered algorithmic trading across forex and crypto markets.
            </p>
            <div className="mt-3 flex items-center gap-1.5">
              <span className="h-1.5 w-1.5 rounded-full bg-profit" />
              <span className="text-[10px] text-profit/80">All systems operational</span>
            </div>
          </div>

          {/* Link columns */}
          {Object.entries(footerLinks).map(([heading, links]) => (
            <div key={heading}>
              <h4 className="text-[10px] font-bold tracking-[1.5px] text-text-secondary/50">{heading.toUpperCase()}</h4>
              <ul className="mt-3 space-y-2">
                {links.map((link) => (
                  <li key={link}>
                    <a href="#" className="text-[12px] text-text-secondary/70 transition-colors hover:text-foreground">
                      {link}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        {/* Divider */}
        <div className="mt-10 h-px bg-gradient-to-r from-transparent via-border/50 to-transparent" />

        {/* Bottom bar */}
        <div className="mt-6 flex items-center justify-between text-[10px] text-text-secondary/40">
          <span>&copy; 2026 HybridTrade. All rights reserved.</span>
          <div className="flex gap-4">
            {["Twitter", "Discord", "GitHub", "Telegram"].map((s) => (
              <a key={s} href="#" className="transition-colors hover:text-foreground">{s}</a>
            ))}
          </div>
        </div>

        {/* Risk Disclaimer */}
        <p className="mt-4 text-center text-[9px] leading-[1.6] text-text-secondary/30">
          Trading forex and cryptocurrencies involves substantial risk. Past performance is not indicative of future results.
        </p>
      </div>
    </footer>
  );
}
