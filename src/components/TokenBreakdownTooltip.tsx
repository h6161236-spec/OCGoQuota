import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

export function TokenBreakdownTooltip({
  children,
  uncachedInput,
  cacheHit,
  cacheWrite,
  output,
}: {
  children: ReactNode;
  uncachedInput: number;
  cacheHit: number;
  cacheWrite: number;
  output: number;
}) {
  const { t } = useTranslation();
  const totalInput = uncachedInput + cacheHit + cacheWrite;
  const hitRate = totalInput > 0 ? ((cacheHit / totalInput) * 100).toFixed(1) : '0.0';

  return (
    <details className="dropdown dropdown-end">
      <summary className="list-none cursor-help w-fit [&::-webkit-details-marker]:hidden">
        {children}
      </summary>
      <div className="dropdown-content z-30 mt-2 w-56 rounded-lg border border-base-300 bg-base-100 p-3 text-xs shadow-lg">
        <div className="font-semibold mb-2">{t('tokenStats.breakdownTitle')}</div>
        {[
          [t('tokenStats.uncachedInput'), uncachedInput],
          [t('tokenStats.cacheHit'), cacheHit],
          [t('tokenStats.cacheWrite'), cacheWrite],
          [t('tokenStats.totalInput'), totalInput],
          [t('tokenStats.output'), output],
        ].map(([label, value], index) => (
          <div
            key={String(label)}
            className={`flex justify-between gap-4 py-0.5 ${index === 3 ? 'border-t border-base-200 mt-1 pt-1' : ''}`}
          >
            <span className="text-base-content/60">{label}</span>
            <span className="tabular-nums">{Number(value).toLocaleString()}</span>
          </div>
        ))}
        <div className="text-base-content/40 mt-1">
          {t('tokenStats.cacheHitRate', { rate: hitRate })}
        </div>
      </div>
    </details>
  );
}

