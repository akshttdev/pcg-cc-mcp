// Stub testid helpers. The real registry was deleted; this file restores
// enough surface area that every consumer compiles and renders without
// throwing. Each property can be used directly as a string
// (`data-testid={tid.menu}`) or called with one or more string arguments
// to produce a qualified id (`data-testid={tid.menu(deal.id)}`).
//
// Both forms render to a stable, hyphenated id derived from the prefix +
// property name + arguments — adequate for development and E2E selectors
// that don't pin the exact id format.

type Testid = string & ((...args: Array<string | number>) => string);
type TestidGroup = Readonly<Record<string, Testid>>;

const RESERVED = new Set<string | symbol>([
  Symbol.toPrimitive,
  Symbol.toStringTag,
  Symbol.iterator,
  'then',
  'toString',
  'toJSON',
  'valueOf',
  'constructor',
]);

function makeGroup(prefix: string): TestidGroup {
  return new Proxy({} as TestidGroup, {
    get(_target, prop) {
      if (RESERVED.has(prop) || typeof prop === 'symbol') return undefined;
      const base = `${prefix}-${String(prop)}`;
      const fn = (...args: Array<string | number>) =>
        args.length ? `${base}-${args.map(String).join('-')}` : base;
      (fn as unknown as { toString(): string }).toString = () => base;
      Object.defineProperty(fn, Symbol.toPrimitive, { value: () => base });
      return fn as Testid;
    },
  });
}

export const dealCard = makeGroup('deal-card');
export const pipeline = makeGroup('pipeline');
export const dealDetail = makeGroup('deal-detail');
export const agentHistory = makeGroup('agent-history');
export const callScheduling = makeGroup('call-scheduling');
export const deck = makeGroup('deck');
export const discovery = makeGroup('discovery');
export const pipelineSettings = makeGroup('pipeline-settings');
export const review = makeGroup('review');
