export interface NewsItem {
  id: string;
  title: string;
  releasedDateMs: number;
  important: boolean;
  tags: string[];
  path: string;
  smallImg: string | null;
}
