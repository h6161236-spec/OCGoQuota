import { useTranslation } from 'react-i18next';

export type TimeRange = 'today' | '7d' | '30d' | 'all';

const STORAGE_KEY = 'ocgoquota.timeRange';

export function getStoredTimeRange(): TimeRange {
  if (typeof window === 'undefined') return '30d';
  const value = window.localStorage.getItem(STORAGE_KEY);
  return value === 'today' || value === '7d' || value === '30d' || value === 'all' ? value : '30d';
}

export function storeTimeRange(value: TimeRange) {
  if (typeof window !== 'undefined') window.localStorage.setItem(STORAGE_KEY, value);
}

const OPTIONS: Array<{ value: TimeRange; key: string }> = [
  { value: 'today', key: 'timeRange.today' },
  { value: '7d', key: 'timeRange.7days' },
  { value: '30d', key: 'timeRange.30days' },
  { value: 'all', key: 'timeRange.all' },
];

export function TimeRangeTabs({
  value,
  onChange,
  size = 'sm',
}: {
  value: TimeRange;
  onChange: (value: TimeRange) => void;
  size?: 'sm' | 'xs';
}) {
  const { t } = useTranslation();
  const compact = size === 'xs';
  return (
    <div className={`inline-flex items-center gap-0.5 rounded-lg bg-base-200 ${compact ? 'p-0.5' : 'p-1'}`}>
      {OPTIONS.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={value === option.value}
          className={`rounded-md font-medium transition-colors whitespace-nowrap focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-base-200 ${
            compact ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-[11px]'
          } ${
            value === option.value
              ? 'bg-primary text-primary-content shadow-sm'
              : 'text-base-content/60 hover:bg-base-100/70 hover:text-base-content'
          }`}
          onClick={() => onChange(option.value)}
        >
          {t(option.key)}
        </button>
      ))}
    </div>
  );
}

