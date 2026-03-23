import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from '@tanstack/react-table';
import { ChevronDown, ChevronUp, ChevronsUpDown, ExternalLink, MoreHorizontal, Edit, Copy, Trash2, Bot, Calendar } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Checkbox } from '@/components/ui/checkbox';
import { format } from 'date-fns';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import type { TaskWithAttemptStatus } from 'shared/types';
import { cn } from '@/lib/utils';
import { useBulkSelectionStore } from '@/stores/useBulkSelectionStore';

interface TableViewProps {
  tasks: TaskWithAttemptStatus[];
  projectId: string;
  onEditTask?: (task: TaskWithAttemptStatus) => void;
  onDeleteTask?: (taskId: string) => void;
  onDuplicateTask?: (task: TaskWithAttemptStatus) => void;
}

const columnHelper = createColumnHelper<TaskWithAttemptStatus>();

const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'completed':
      return 'bg-green-500/10 text-green-700 dark:text-green-400';
    case 'in_progress':
      return 'bg-blue-500/10 text-blue-700 dark:text-blue-400';
    case 'blocked':
      return 'bg-red-500/10 text-red-700 dark:text-red-400';
    case 'pending':
      return 'bg-yellow-500/10 text-yellow-700 dark:text-yellow-400';
    default:
      return 'bg-gray-500/10 text-gray-700 dark:text-gray-400';
  }
};

const getPriorityColor = (priority: string) => {
  switch (priority.toLowerCase()) {
    case 'critical':
      return 'bg-red-500/10 text-red-700 dark:text-red-400';
    case 'high':
      return 'bg-orange-500/10 text-orange-700 dark:text-orange-400';
    case 'medium':
      return 'bg-yellow-500/10 text-yellow-700 dark:text-yellow-400';
    case 'low':
      return 'bg-blue-500/10 text-blue-700 dark:text-blue-400';
    default:
      return 'bg-gray-500/10 text-gray-700 dark:text-gray-400';
  }
};

const getApprovalColor = (status: string | null | undefined) => {
  if (!status) return '';
  switch (status.toLowerCase()) {
    case 'approved':
      return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    case 'pending':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    case 'rejected':
      return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
    case 'changesrequested':
      return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
    default:
      return '';
  }
};

export function TableView({ tasks, projectId, onEditTask, onDeleteTask, onDuplicateTask }: TableViewProps) {
  const navigate = useNavigate();
  const [sorting, setSorting] = useState<SortingState>([]);
  const { selectionMode, isSelected, toggleTask, selectAll, clearSelection } =
    useBulkSelectionStore();

  const columns = useMemo(
    () => [
      // Checkbox column for selection mode
      ...(selectionMode
        ? [
            columnHelper.display({
              id: 'select',
              header: ({ table }) => (
                <Checkbox
                  checked={
                    table.getRowModel().rows.length > 0 &&
                    table.getRowModel().rows.every((row) => isSelected(row.original.id))
                  }
                  onCheckedChange={(checked) => {
                    if (checked === true) {
                      selectAll(
                        table.getRowModel().rows.map((row) => row.original.id)
                      );
                    } else {
                      clearSelection();
                    }
                  }}
                />
              ),
              cell: ({ row }) => (
                <div
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={(e) => e.stopPropagation()}
                >
                  <Checkbox
                    checked={isSelected(row.original.id)}
                    onCheckedChange={() => toggleTask(row.original.id)}
                  />
                </div>
              ),
            }),
          ]
        : []),
      columnHelper.accessor('title', {
        header: 'Task',
        cell: (info) => (
          <div className="flex items-center gap-2">
            <span className="font-medium truncate max-w-md">{info.getValue()}</span>
            <IconButton
              variant="ghost" className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={(e) => {
                e.stopPropagation();
                navigate(`/projects/${projectId}/tasks/${info.row.original.id}`);
              }}
              icon={ExternalLink}
              label="Open task"
              iconClassName="h-3 w-3"
            />
          </div>
        ),
        size: 400,
      }),
      columnHelper.accessor('status', {
        header: 'Status',
        cell: (info) => (
          <Badge className={cn('capitalize', getStatusColor(info.getValue() || 'pending'))}>
            {(info.getValue() || 'pending').replace('_', ' ')}
          </Badge>
        ),
        size: 120,
      }),
      columnHelper.accessor('priority', {
        header: 'Priority',
        cell: (info) => (
          <Badge className={cn('capitalize', getPriorityColor(info.getValue() || 'medium'))}>
            {info.getValue() || 'medium'}
          </Badge>
        ),
        size: 100,
      }),
      columnHelper.accessor('assignee_id', {
        header: 'Assignee',
        cell: (info) => (
          <span className="text-sm text-muted-foreground">
            {info.getValue() || 'Unassigned'}
          </span>
        ),
        size: 150,
      }),
      columnHelper.accessor('assigned_agent', {
        header: 'Agent',
        cell: (info) => info.getValue() ? (
          <div className="flex items-center gap-1">
            <Bot className="h-3 w-3 text-purple-600" />
            <Badge variant="outline" className="text-xs bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">
              {info.getValue()}
            </Badge>
          </div>
        ) : (
          <span className="text-sm text-muted-foreground">-</span>
        ),
        size: 120,
      }),
      columnHelper.accessor('assigned_mcps', {
        header: 'MCPs',
        cell: (info) => {
          const mcps = info.getValue();
          if (!mcps) return <span className="text-sm text-muted-foreground">-</span>;
          const mcpList = JSON.parse(mcps);
          if (mcpList.length === 0) return <span className="text-sm text-muted-foreground">-</span>;
          return (
            <div className="flex flex-wrap gap-1">
              {mcpList.slice(0, 2).map((mcp: string) => (
                <Badge key={mcp} variant="outline" className="text-[10px] bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                  {mcp}
                </Badge>
              ))}
              {mcpList.length > 2 && (
                <Badge variant="secondary" className="text-[10px]">+{mcpList.length - 2}</Badge>
              )}
            </div>
          );
        },
        size: 140,
      }),
      columnHelper.accessor('tags', {
        header: 'Tags',
        cell: (info) => {
          const tags = info.getValue();
          if (!tags) return <span className="text-sm text-muted-foreground">-</span>;
          const tagList = JSON.parse(tags);
          if (tagList.length === 0) return <span className="text-sm text-muted-foreground">-</span>;
          return (
            <div className="flex flex-wrap gap-1">
              {tagList.slice(0, 2).map((tag: string) => (
                <Badge key={tag} variant="secondary" className="text-[10px]">
                  {tag}
                </Badge>
              ))}
              {tagList.length > 2 && (
                <Badge variant="secondary" className="text-[10px]">+{tagList.length - 2}</Badge>
              )}
            </div>
          );
        },
        size: 140,
      }),
      columnHelper.accessor('due_date', {
        header: 'Due Date',
        cell: (info) => info.getValue() ? (
          <div className="flex items-center gap-1 text-sm text-muted-foreground">
            <Calendar className="h-3 w-3" />
            <span>{format(new Date(info.getValue()!), 'MMM d')}</span>
          </div>
        ) : (
          <span className="text-sm text-muted-foreground">-</span>
        ),
        size: 110,
      }),
      columnHelper.accessor('approval_status', {
        header: 'Approval',
        cell: (info) => {
          const status = info.getValue();
          if (!status) return <span className="text-sm text-muted-foreground">-</span>;
          return (
            <Badge className={cn('text-xs capitalize', getApprovalColor(status))}>
              {status === 'changesrequested' ? 'Changes Requested' : status}
            </Badge>
          );
        },
        size: 130,
      }),
      columnHelper.accessor('created_at', {
        header: 'Created',
        cell: (info) => (
          <span className="text-sm text-muted-foreground">
            {new Date(info.getValue()).toLocaleDateString()}
          </span>
        ),
        size: 120,
      }),
      columnHelper.accessor('updated_at', {
        header: 'Updated',
        cell: (info) => (
          <span className="text-sm text-muted-foreground">
            {new Date(info.getValue()).toLocaleDateString()}
          </span>
        ),
        size: 120,
      }),
      // Actions column
      columnHelper.display({
        id: 'actions',
        header: '',
        cell: ({ row }) => (
          <div
            onClick={(e) => e.stopPropagation()}
            onPointerDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
          >
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 p-0"
                >
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {onEditTask && (
                  <DropdownMenuItem onClick={() => onEditTask(row.original)}>
                    <Edit className="h-4 w-4 mr-2" />
                    Edit
                  </DropdownMenuItem>
                )}
                {onDuplicateTask && (
                  <DropdownMenuItem onClick={() => onDuplicateTask(row.original)}>
                    <Copy className="h-4 w-4 mr-2" />
                    Duplicate
                  </DropdownMenuItem>
                )}
                {onDeleteTask && (
                  <DropdownMenuItem
                    onClick={() => onDeleteTask(row.original.id)}
                    className="text-destructive"
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        ),
        size: 60,
      }),
    ],
    [navigate, projectId, selectionMode, isSelected, toggleTask, selectAll, clearSelection, onEditTask, onDeleteTask, onDuplicateTask]
  );

  const table = useReactTable({
    data: tasks,
    columns,
    state: {
      sorting,
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  return (
    <div className="border rounded-lg overflow-hidden bg-background">
      <Table>
        <TableHeader>
          {table.getHeaderGroups().map((headerGroup) => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map((header) => (
                <TableHead
                  key={header.id}
                  style={{ width: header.column.getSize() }}
                  className="bg-muted/50"
                >
                  {header.isPlaceholder ? null : (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="-ml-3 h-8 data-[state=open]:bg-accent"
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      <span>
                        {flexRender(
                          header.column.columnDef.header,
                          header.getContext()
                        )}
                      </span>
                      {{
                        asc: <ChevronUp className="ml-2 h-4 w-4" />,
                        desc: <ChevronDown className="ml-2 h-4 w-4" />,
                      }[header.column.getIsSorted() as string] ?? (
                        <ChevronsUpDown className="ml-2 h-4 w-4 opacity-50" />
                      )}
                    </Button>
                  )}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows?.length ? (
            table.getRowModel().rows.map((row) => (
              <TableRow
                key={row.id}
                data-state={row.getIsSelected() && 'selected'}
                className="cursor-pointer group hover:bg-muted/50"
                onClick={() => navigate(`/projects/${projectId}/tasks/${row.original.id}`)}
              >
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell colSpan={columns.length} className="h-24 text-center">
                No tasks found.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
