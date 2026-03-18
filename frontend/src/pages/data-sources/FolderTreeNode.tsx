import { useState } from 'react';
import { ChevronRight, ChevronDown, Folder, FolderOpen } from 'lucide-react';
import type { FolderNode } from './types';

export const SECTION_ORDER = ['ACTIVE CLIENTS', 'PROJECTS', 'PROPOSALS', 'SALES', 'SOCIAL MEDIA', 'RESOURCES', 'ARCHIVE'];

export function FolderTreeNode({
  node, depth, selectedPath, onSelect,
}: {
  node: FolderNode; depth: number; selectedPath: string; onSelect: (path: string) => void;
}) {
  const hasChildren = Object.keys(node.children).length > 0;
  const isSelected = selectedPath === node.path;
  const isAncestor = selectedPath.startsWith(node.path + ' >');
  const [open, setOpen] = useState(depth < 1);

  return (
    <div>
      <button
        className={`flex items-center gap-1.5 w-full text-left px-2 py-1 rounded-md text-sm transition-colors hover:bg-muted/60
          ${isSelected ? 'bg-primary/10 text-primary font-medium' : ''}
          ${depth === 0 ? 'font-semibold text-[11px] tracking-wide uppercase mt-2 text-muted-foreground' : ''}
        `}
        style={{ paddingLeft: `${8 + depth * 12}px` }}
        onClick={() => {
          onSelect(node.path);
          if (hasChildren) setOpen(o => !o);
        }}
      >
        {hasChildren ? (
          open || isAncestor
            ? <ChevronDown className="h-3 w-3 shrink-0 text-muted-foreground" />
            : <ChevronRight className="h-3 w-3 shrink-0 text-muted-foreground" />
        ) : <span className="w-3 shrink-0" />}
        {depth === 0
          ? <FolderOpen className="h-3.5 w-3.5 shrink-0" />
          : <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        }
        <span className="truncate">{node.name}</span>
        <span className="ml-auto text-[10px] text-muted-foreground shrink-0">{node.count}</span>
      </button>
      {(open || isAncestor) && hasChildren && (
        <div>
          {Object.values(node.children)
            .sort((a, b) => b.count - a.count)
            .map(child => (
              <FolderTreeNode
                key={child.path}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                onSelect={setSelectedPath => onSelect(setSelectedPath)}
              />
            ))}
        </div>
      )}
    </div>
  );
}
