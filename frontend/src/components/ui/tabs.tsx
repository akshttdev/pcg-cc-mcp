import * as React from 'react';
import * as TabsPrimitive from '@radix-ui/react-tabs';

import { cn } from '@/lib/utils';

const Tabs = TabsPrimitive.Root;

const TabsList = React.forwardRef<
  React.ElementRef<typeof TabsPrimitive.List>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.List
    ref={ref}
    className={cn(
      'inline-flex h-9 items-center gap-1 border-b border-border/40 text-muted-foreground',
      className
    )}
    {...props}
  />
));
TabsList.displayName = TabsPrimitive.List.displayName;

const TabsTrigger = React.forwardRef<
  React.ElementRef<typeof TabsPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, value, ...props }, ref) => (
  <TabsPrimitive.Trigger
    ref={ref}
    value={value}
    data-testid={`tab-${value}`}
    className={cn(
      'inline-flex items-center justify-center whitespace-nowrap px-3 py-2 text-sm font-medium',
      'relative -mb-px border-b-2 border-transparent',
      'ring-offset-background transition-all duration-150',
      'hover:text-foreground',
      'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
      'disabled:pointer-events-none disabled:opacity-50',
      'data-[state=active]:text-foreground data-[state=active]:border-primary',
      className
    )}
    {...props}
  />
));
TabsTrigger.displayName = TabsPrimitive.Trigger.displayName;

const TabsContent = React.forwardRef<
  React.ElementRef<typeof TabsPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, value, ...props }, ref) => (
  <TabsPrimitive.Content
    ref={ref}
    value={value}
    data-testid={`tab-content-${value}`}
    className={cn(
      'mt-3 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
      'animate-fade-in',
      className
    )}
    {...props}
  />
));
TabsContent.displayName = TabsPrimitive.Content.displayName;

// ── TabPanel: Declarative tab panel with built-in testability ────────────────

export interface TabDefinition {
  value: string;
  label: string;
  icon?: React.ReactNode;
  /** Status dot color (shown top-right of tab) */
  indicator?: 'green' | 'amber' | null;
  /** Count badge (shown inline after label) */
  badge?: number;
  disabled?: boolean;
}

interface TabPanelProps {
  /** Tab definitions — order determines display order */
  tabs: TabDefinition[];
  /** Controlled value */
  value?: string;
  /** Default value (uncontrolled) */
  defaultValue?: string;
  /** Change handler */
  onValueChange?: (value: string) => void;
  /** Tab content keyed by tab value */
  children: React.ReactNode;
  /** Class for the root Tabs element */
  className?: string;
  /** Class for the TabsList */
  listClassName?: string;
  /** Class for each TabsTrigger */
  triggerClassName?: string;
  /** data-testid prefix (default: "tab-panel") */
  testId?: string;
}

/**
 * Declarative tab panel with built-in test attributes.
 *
 * Each tab trigger gets `data-testid="tab-{value}"` automatically.
 * Each content panel gets `data-testid="tab-content-{value}"`.
 *
 * Usage:
 * ```tsx
 * <TabPanel
 *   tabs={[
 *     { value: 'overview', label: 'Overview' },
 *     { value: 'intel', label: 'Intel', indicator: 'green' },
 *   ]}
 *   value={activeTab}
 *   onValueChange={setActiveTab}
 * >
 *   <TabsContent value="overview">...</TabsContent>
 *   <TabsContent value="intel">...</TabsContent>
 * </TabPanel>
 * ```
 */
function TabPanel({
  tabs,
  value,
  defaultValue,
  onValueChange,
  children,
  className,
  listClassName,
  triggerClassName,
  testId = 'tab-panel',
}: TabPanelProps) {
  return (
    <Tabs
      value={value}
      defaultValue={defaultValue}
      onValueChange={onValueChange}
      className={className}
      data-testid={testId}
    >
      <TabsList className={listClassName}>
        {tabs.map((tab) => (
          <TabsTrigger
            key={tab.value}
            value={tab.value}
            disabled={tab.disabled}
            className={cn('relative', triggerClassName)}
          >
            {tab.icon && <span className="mr-1.5">{tab.icon}</span>}
            {tab.label}
            {tab.badge != null && tab.badge > 0 && (
              <span className="ml-1.5 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-primary">
                {tab.badge}
              </span>
            )}
            {tab.indicator && (
              <span
                className={cn(
                  'absolute top-1.5 right-1 w-1.5 h-1.5 rounded-full',
                  tab.indicator === 'green' ? 'bg-green-500' : 'bg-amber-500 animate-pulse'
                )}
              />
            )}
          </TabsTrigger>
        ))}
      </TabsList>
      {children}
    </Tabs>
  );
}

export { Tabs, TabsList, TabsTrigger, TabsContent, TabPanel };
