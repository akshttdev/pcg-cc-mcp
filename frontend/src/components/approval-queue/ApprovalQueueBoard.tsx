import { useMemo, useState } from 'react';
import { ApprovalCard } from './ApprovalCard';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Loader } from '@/components/ui/loader';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Clock,
  CheckCircle2,
  Search,
  Filter,
  Inbox,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  ArtifactReview,
  ExecutionArtifact,
  ReviewType,
} from 'shared/types';

interface ApprovalQueueBoardProps {
  reviews: ArtifactReview[];
  artifacts?: Map<string, ExecutionArtifact>;
  isLoading?: boolean;
  onApprove?: (reviewId: string, feedback?: string, rating?: number) => void;
  onReject?: (reviewId: string, feedback: string) => void;
  onRequestRevision?: (reviewId: string, notes: string) => void;
  onViewArtifact?: (artifactId: string) => void;
  className?: string;
}

export function ApprovalQueueBoard({
  reviews,
  artifacts = new Map(),
  isLoading,
  onApprove,
  onReject,
  onRequestRevision,
  onViewArtifact,
  className,
}: ApprovalQueueBoardProps) {
  const [activeTab, setActiveTab] = useState<'pending' | 'completed' | 'all'>('pending');
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<ReviewType | 'all'>('all');

  // Group reviews by status
  const { pendingReviews, completedReviews, allReviews } = useMemo(() => {
    const pending: ArtifactReview[] = [];
    const completed: ArtifactReview[] = [];

    reviews.forEach((review) => {
      if (review.status === 'pending') {
        pending.push(review);
      } else {
        completed.push(review);
      }
    });

    // Sort by date (newest first for pending, newest resolved first for completed)
    pending.sort(
      (a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
    completed.sort(
      (a, b) =>
        new Date(b.resolved_at || b.created_at).getTime() -
        new Date(a.resolved_at || a.created_at).getTime()
    );

    return {
      pendingReviews: pending,
      completedReviews: completed,
      allReviews: [...pending, ...completed],
    };
  }, [reviews]);

  // Apply filters
  const filterReviews = (reviewList: ArtifactReview[]) => {
    return reviewList.filter((review) => {
      // Search filter
      if (searchQuery) {
        const artifact = artifacts.get(review.artifact_id);
        const query = searchQuery.toLowerCase();
        const matchesArtifact =
          artifact?.title?.toLowerCase().includes(query) ||
          artifact?.artifact_type.toLowerCase().includes(query);
        const matchesReviewer =
          review.reviewer_name?.toLowerCase().includes(query);
        if (!matchesArtifact && !matchesReviewer) return false;
      }

      // Type filter
      if (filterType !== 'all' && review.review_type !== filterType) {
        return false;
      }

      return true;
    });
  };

  const getDisplayReviews = () => {
    switch (activeTab) {
      case 'pending':
        return filterReviews(pendingReviews);
      case 'completed':
        return filterReviews(completedReviews);
      default:
        return filterReviews(allReviews);
    }
  };

  const displayReviews = getDisplayReviews();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader />
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header */}
      <div className="p-4 border-b bg-background">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Approval Queue</h2>
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="text-sm">
              <Clock className="h-3 w-3 mr-1" />
              {pendingReviews.length} Pending
            </Badge>
          </div>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search reviews..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>

          <Select
            value={filterType}
            onValueChange={(value) => setFilterType(value as ReviewType | 'all')}
          >
            <SelectTrigger className="w-48">
              <Filter className="h-4 w-4 mr-2" />
              <SelectValue placeholder="Filter by type" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Types</SelectItem>
              <SelectItem value="approval">Approval</SelectItem>
              <SelectItem value="feedback">Feedback</SelectItem>
              <SelectItem value="revision_request">Revision Request</SelectItem>
              <SelectItem value="quality_check">Quality Check</SelectItem>
              <SelectItem value="compliance_check">Compliance Check</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Tabs */}
      <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as 'pending' | 'completed' | 'all')}
        className="flex-1 flex flex-col"
      >
        <div className="px-4 pt-2 border-b">
          <TabsList>
            <TabsTrigger value="pending" className="gap-2">
              <Clock className="h-4 w-4" />
              Pending
              <Badge variant="secondary" className="ml-1">
                {pendingReviews.length}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="completed" className="gap-2">
              <CheckCircle2 className="h-4 w-4" />
              Completed
              <Badge variant="secondary" className="ml-1">
                {completedReviews.length}
              </Badge>
            </TabsTrigger>
            <TabsTrigger value="all" className="gap-2">
              <Inbox className="h-4 w-4" />
              All
              <Badge variant="secondary" className="ml-1">
                {allReviews.length}
              </Badge>
            </TabsTrigger>
          </TabsList>
        </div>

        <ScrollArea className="flex-1">
          <div className="p-4 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {displayReviews.map((review) => (
              <ApprovalCard
                key={review.id}
                review={review}
                artifact={artifacts.get(review.artifact_id)}
                onApprove={(feedback, rating) =>
                  onApprove?.(review.id, feedback, rating)
                }
                onReject={(feedback) => onReject?.(review.id, feedback)}
                onRequestRevision={(notes) =>
                  onRequestRevision?.(review.id, notes)
                }
                onViewArtifact={() => onViewArtifact?.(review.artifact_id)}
              />
            ))}

            {displayReviews.length === 0 && (
              <div className="col-span-full flex flex-col items-center justify-center h-64 text-muted-foreground">
                <Inbox className="h-12 w-12 mb-4" />
                <p>
                  {activeTab === 'pending'
                    ? 'No pending reviews'
                    : 'No reviews found'}
                </p>
                {searchQuery && (
                  <Button
                    variant="link"
                    className="mt-2"
                    onClick={() => setSearchQuery('')}
                  >
                    Clear search
                  </Button>
                )}
              </div>
            )}
          </div>
        </ScrollArea>
      </Tabs>
    </div>
  );
}
