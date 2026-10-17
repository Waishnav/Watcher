import React, { useEffect, useState } from 'react';
import { browser } from 'wxt/browser';
import { Activity, BarChart3, CircleCheck, HardDrive, RefreshCcw, ShieldCheck } from 'lucide-react';
import { Button } from './components/ui/button';
import { Card, CardTitle } from './components/ui/card';
import { Badge } from './components/ui/badge';
import type { DomainTime } from './lib/db';

type Dashboard = {
  local: { domains: DomainTime[]; pending: number };
  online: boolean;
  device: { active_ms: number; idle_ms: number; unknown_ms: number; apps: { app: string; duration_ms: number }[] } | null;
};

function duration(ms: number): string {
  const minutes = Math.floor(ms / 60_000);
  if (!minutes) return '<1m';
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
}
function UsageRows({ items }: { items: { name: string; ms: number }[] }) {
  const biggest = Math.max(1, ...items.map((item) => item.ms));
  if (!items.length) return <p className="py-8 text-center text-sm text-muted-foreground">Nothing recorded for today yet.</p>;
  return <div className="space-y-4">{items.slice(0, 10).map((item) => <div key={item.name} className="space-y-1.5">
    <div className="flex items-center justify-between gap-3 text-xs"><span className="truncate font-medium">{item.name}</span><span className="shrink-0 tabular-nums text-muted-foreground">{duration(item.ms)}</span></div>
    <div className="h-1.5 overflow-hidden rounded-full bg-muted"><div className="h-full rounded-full bg-primary" style={{ width: `${item.ms / biggest * 100}%` }}/></div>
  </div>)}</div>;
}

export default function App() {
  const [data, setData] = useState<Dashboard | null>(null);
  const [section, setSection] = useState<'browser' | 'device'>('browser');
  const [issue, setIssue] = useState(false);
  async function refresh() {
    try {
      setData(await browser.runtime.sendMessage({ type: 'watcher-dashboard' }) as Dashboard);
      setIssue(false);
    } catch { setIssue(true); }
  }
  useEffect(() => {
    void refresh();
    const timer = setInterval(() => { void refresh(); }, 15_000);
    return () => clearInterval(timer);
  }, []);
  const domains = data?.local.domains ?? [];
  const browserMs = domains.reduce((sum, row) => sum + row.duration_ms, 0);
  const device = data?.device;
  return <main className="mx-auto max-w-md space-y-4 px-4 pb-8 pt-5">
    <header className="flex items-center justify-between gap-2">
      <div className="flex items-center gap-3"><div className="flex size-10 items-center justify-center rounded-xl bg-primary text-primary-foreground"><Activity size={20}/></div>
        <div><h1 className="text-lg font-bold leading-tight">Watcher</h1><p className="text-[11px] text-muted-foreground">Browser & device analytics</p></div></div>
      <Button variant="ghost" size="icon" aria-label="Refresh analytics" onClick={() => void refresh()}><RefreshCcw size={17}/></Button>
    </header>
    <div className="flex items-center justify-between gap-2">
      <Badge><span className={`size-1.5 rounded-full ${data?.online ? 'bg-emerald-500' : 'bg-amber-500'}`}/>{data?.online ? 'Daemon connected' : 'Browser-only mode'}</Badge>
      <span className="text-[11px] tabular-nums text-muted-foreground">{data?.local.pending ?? '—'} pending sync</span>
    </div>
    <Card className="space-y-2 bg-gradient-to-b from-card to-muted/40">
      <p className="text-xs font-medium text-muted-foreground">Browser-observed today</p>
      <div className="text-3xl font-semibold tracking-tight tabular-nums">{data ? duration(browserMs) : '—'}</div>
      <p className="text-[11px] leading-relaxed text-muted-foreground">Recorded locally. Not added to verified device screen time.</p>
    </Card>
    <div className="grid grid-cols-2 rounded-xl bg-muted p-1" role="tablist" aria-label="Analytics category">
      <Button role="tab" aria-selected={section === 'browser'} variant={section === 'browser' ? 'outline' : 'ghost'} size="sm" onClick={() => setSection('browser')}><BarChart3 size={15}/>Browser</Button>
      <Button role="tab" aria-selected={section === 'device'} variant={section === 'device' ? 'outline' : 'ghost'} size="sm" onClick={() => setSection('device')}><HardDrive size={15}/>Device</Button>
    </div>
    {section === 'browser' ? <Card className="space-y-4"><div className="flex items-center justify-between"><CardTitle>Top websites</CardTitle><span className="text-xs text-muted-foreground">Today</span></div>
      <UsageRows items={domains.map((row) => ({ name: row.domain, ms: row.duration_ms }))}/></Card>
      : <Card className="space-y-4"><CardTitle>Device activity</CardTitle>{device
        ? <><div className="text-2xl font-semibold tabular-nums">{duration(device.active_ms)}</div><p className="text-xs text-muted-foreground">OS-verified interaction · {duration(device.idle_ms)} idle · {duration(device.unknown_ms)} unknown</p>
          <UsageRows items={device.apps.map(({ app, duration_ms }) => ({ name: app, ms: duration_ms }))}/></>
        : <div className="space-y-3 py-7 text-center"><HardDrive size={28} className="mx-auto text-muted-foreground"/><p className="text-sm font-medium">Companion daemon not connected</p><p className="text-xs leading-relaxed text-muted-foreground">Browser observations remain on this device and will sync when Watcher becomes available.</p></div>}</Card>}
    <div className="flex items-start gap-2 text-xs text-muted-foreground"><ShieldCheck size={16} className="mt-0.5 shrink-0"/><p>No URLs, search queries or page titles are stored. Incognito tracking is disabled by default in browser settings.</p></div>
    {issue && <p className="text-xs text-amber-600">Background collector unavailable. Reopen the panel or reload the extension.</p>}
    <footer className="flex items-center justify-center gap-1 text-[11px] text-muted-foreground"><CircleCheck size={12}/>Local-first · No account required</footer>
  </main>;
}
