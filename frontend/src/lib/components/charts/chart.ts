// chart.js with the pieces the charts use, registered once.
import {
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Chart,
  DoughnutController,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  Title,
  Tooltip,
} from 'chart.js';

Chart.register(
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  DoughnutController,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  Title,
  Tooltip,
);

export { Chart };

/** The tooltip every chart shares. */
export const TOOLTIP_STYLE = {
  backgroundColor: 'rgba(24, 24, 27, 0.95)',
  borderColor: 'rgba(63, 63, 70, 0.6)',
  borderWidth: 1,
  titleColor: '#f4f4f5',
  bodyColor: '#d4d4d8',
  titleFont: { family: 'monospace' },
  bodyFont: { family: 'monospace' },
};
