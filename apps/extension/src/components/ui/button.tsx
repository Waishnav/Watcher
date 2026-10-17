import React from 'react';
import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const variants = cva('inline-flex items-center justify-center gap-2 rounded-lg text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary', {
  variants: {
    variant: {
      default: 'bg-primary text-primary-foreground hover:opacity-90',
      outline: 'border border-border bg-background hover:bg-muted',
      ghost: 'hover:bg-muted',
    },
    size: { default: 'h-9 px-4', sm: 'h-8 px-3 text-xs', icon: 'size-9' },
  },
  defaultVariants: { variant: 'default', size: 'default' },
});
export type ButtonProps = React.ComponentProps<'button'> & VariantProps<typeof variants> & { asChild?: boolean };
export function Button({ className, variant, size, asChild = false, ...props }: ButtonProps) {
  const Comp = asChild ? Slot : 'button';
  return <Comp data-slot="button" className={cn(variants({ variant, size, className }))} {...props} />;
}
