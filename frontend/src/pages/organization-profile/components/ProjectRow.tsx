import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Badge } from '@/components/ui/badge';
import { FolderOpen, ExternalLink, ChevronDown, ChevronRight } from 'lucide-react';

export function ProjectRow({ project, folderName, depth = 0 }: { project: any; folderName?: string; depth?: number }) {
  const hasChildren = project.children && project.children.length > 0;
  const [expanded, setExpanded] = useState(false);

  return (
    <div>
      <div className="flex items-center gap-1">
        {hasChildren && (
          <button
            onClick={() => setExpanded(!expanded)}
            className="p-0.5 hover:bg-muted rounded shrink-0"
          >
            {expanded ? <ChevronDown className="h-3 w-3 text-muted-foreground" /> : <ChevronRight className="h-3 w-3 text-muted-foreground" />}
          </button>
        )}
        <Link
          to={`/projects/${project.id}`}
          className="flex items-center justify-between p-2 rounded-md hover:bg-muted transition-colors flex-1 min-w-0"
          style={hasChildren ? undefined : { marginLeft: depth > 0 ? '0' : '1.25rem' }}
        >
          <div className="flex items-center gap-2 min-w-0">
            <FolderOpen className="h-4 w-4 text-muted-foreground shrink-0" />
            <span className="text-sm font-medium truncate">{project.name}</span>
            {folderName && <Badge variant="secondary" className="text-xs shrink-0">{folderName}</Badge>}
            {hasChildren && <Badge variant="outline" className="text-[10px] shrink-0">{project.children.length}</Badge>}
          </div>
          <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
        </Link>
      </div>
      {hasChildren && expanded && (
        <div className="pl-4 border-l border-border/50 ml-3 mt-0.5 space-y-0.5">
          {project.children.map((child: any) => (
            <ProjectRow key={child.id} project={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
}
