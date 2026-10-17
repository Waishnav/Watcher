import type React from 'react';
import { cn } from '../../lib/utils';
export function Badge({ className, ...props }: React.ComponentProps<'span'>) {
  return <span data-slot="badge" className={cn('inline-flex items-center gap-1 rounded-full border border-border bg-muted px-2 py-1 text-[11px] font-medium text-muted-foreground', className)} {...props} />;
}
