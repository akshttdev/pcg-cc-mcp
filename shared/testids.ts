type AnyTestid = string & ((...args: unknown[]) => string);

function slugify(v: unknown): string {
  return String(v)
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function makeNamespace(ns: string): Record<string, AnyTestid> {
  return new Proxy({} as Record<string, AnyTestid>, {
    get(_target, prop: string | symbol): AnyTestid | undefined {
      if (typeof prop !== 'string') return undefined;
      if (prop === 'then') return undefined;
      const key = slugify(prop);
      const base = `${ns}-${key}`;
      const fn = ((...args: unknown[]) => {
        const suffix = args.map(slugify).filter(Boolean).join('-');
        return suffix ? `${base}-${suffix}` : base;
      }) as AnyTestid;
      Object.defineProperty(fn, Symbol.toPrimitive, { value: () => base });
      Object.defineProperty(fn, 'toString', { value: () => base });
      return fn;
    },
  });
}

export const dealCard = makeNamespace('deal-card');
export const pipeline = makeNamespace('pipeline');
export const dealDetail = makeNamespace('deal-detail');
export const agentHistory = makeNamespace('agent-history');
export const callScheduling = makeNamespace('call-scheduling');
export const deck = makeNamespace('deck');
export const discovery = makeNamespace('discovery');
export const pipelineSettings = makeNamespace('pipeline-settings');
export const review = makeNamespace('review');
