export interface CalendarEvent {
  id: string;
  title: string;
  country: string;
  countryImg: string | null;
  releasedDate: number; // Unix seconds
  star: number; // 1-3 importance
  type: "data" | "event" | "holiday";
  actual: string | null;
  consensus: string | null;
  previous: string | null;
  unit: string | null;
  path: string | null;
}
