export type FilterOperator =
  | 'equals'
  | 'not_equals'
  | 'contains'
  | 'not_contains'
  | 'in' // For arrays/tags
  | 'not_in'
  | 'before' // For dates
  | 'after'
  | 'between'
  | 'greater_than' // For numbers
  | 'less_than';

export type FilterableField =
  | 'title'
  | 'description'
  | 'status'
  | 'priority'
  | 'assignee'
  | 'tags'
  | 'created_at'
  | 'updated_at';

export interface FilterCondition {
  id: string; // nanoid for removal
  field: FilterableField;
  operator: FilterOperator;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- polymorphic; narrowing requires discriminated union refactor
  value: any;
}

export interface FilterGroup {
  id: string;
  logic: 'AND' | 'OR';
  conditions: FilterCondition[];
}

export interface FilterPreset {
  id: string;
  name: string;
  projectId: string;
  groups: FilterGroup[];
  createdAt: Date;
  updatedAt: Date;
}

export interface CreateFilterPreset {
  name: string;
  projectId: string;
  groups: FilterGroup[];
}