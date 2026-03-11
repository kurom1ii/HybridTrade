import { LandingHeader } from "@/components/landing/header";
import { Hero } from "@/components/landing/hero";
import { PhilosophySection } from "@/components/landing/philosophy-section";
import { LiveMarkets } from "@/components/landing/live-markets";
import { CTASection } from "@/components/landing/cta-section";
import { LandingFooter } from "@/components/landing/footer";

export default function HomePage() {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <LandingHeader />
      <Hero />
      <PhilosophySection />
      <LiveMarkets />
      <CTASection />
      <LandingFooter />
    </main>
  );
}
