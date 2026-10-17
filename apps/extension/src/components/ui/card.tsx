import type React from 'react';
import { cn } from '../../lib/utils';
export function Card({ className, ...props }: React.ComponentProps<'section'>) {
  return <section data-slot="card" className={cn('rounded-xl border border-border bg-card p-4 shadow-xs', className)} {...props} />;
}
export function CardTitle({ className, ...props }: React.ComponentProps<'h2'>) {
  return <h2 data-slot="card-title" className={cn('font-semibold tracking-tight', className)} {...props} />;
}
