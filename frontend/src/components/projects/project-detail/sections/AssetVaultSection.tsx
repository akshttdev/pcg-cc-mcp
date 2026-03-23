import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { AlertCircle, Folder, Loader2, Plus, Trash2 } from 'lucide-react';
import type { ProjectAsset, ProjectBoard } from 'shared/types';
import { formatByteSize, formatStatusLabel } from '../helpers';
import { BOARD_TYPE_LABELS } from '../helpers';

interface AssetVaultSectionProps {
  assets: ProjectAsset[];
  assetsLoading: boolean;
  assetsError: string;
  boardById: Map<string, ProjectBoard>;
  onAddAsset: () => void;
  onDeleteAsset: (assetId: string) => void;
}

function formatBoardLabel(value: ProjectBoard['board_type']) {
  return BOARD_TYPE_LABELS[value] ?? formatStatusLabel(value);
}

export function AssetVaultSection({
  assets,
  assetsLoading,
  assetsError,
  boardById,
  onAddAsset,
  onDeleteAsset,
}: AssetVaultSectionProps) {
  return (
    <Card className="h-full">
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <CardTitle className="flex items-center gap-2">
            <Folder className="h-5 w-5" />
            Asset Vault
          </CardTitle>
          <CardDescription>
            Logos, guides, transcripts, and collateral linked to this brand.
          </CardDescription>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={onAddAsset}
        >
          <Plus className="mr-1 h-4 w-4" /> Add Asset
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {assetsLoading ? (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading assets...
          </div>
        ) : assetsError ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{assetsError}</AlertDescription>
          </Alert>
        ) : assets.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No assets have been catalogued yet. Upload brand kits,
            strategy docs, call transcripts, and web repos here as you
            onboard the client.
          </p>
        ) : (
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Scope</TableHead>
                  <TableHead className="hidden xl:table-cell">
                    Board
                  </TableHead>
                  <TableHead className="hidden xl:table-cell">
                    Size
                  </TableHead>
                  <TableHead className="text-right">
                    Updated
                  </TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {assets.map((asset) => (
                  <TableRow key={asset.id}>
                    <TableCell className="max-w-[160px] truncate font-medium">
                      {asset.name}
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="uppercase">
                        {asset.category}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="uppercase">
                        {asset.scope}
                      </Badge>
                    </TableCell>
                    <TableCell className="hidden xl:table-cell text-sm text-muted-foreground">
                      {asset.board_id
                        ? boardById.get(asset.board_id)?.name ||
                          formatBoardLabel(
                            boardById.get(asset.board_id)?.board_type ??
                              'custom'
                          )
                        : '\u2014'}
                    </TableCell>
                    <TableCell className="hidden xl:table-cell text-sm text-muted-foreground">
                      {formatByteSize(asset.byte_size)}
                    </TableCell>
                    <TableCell className="text-right text-sm text-muted-foreground">
                      {new Date(asset.updated_at).toLocaleDateString()}
                    </TableCell>
                    <TableCell className="text-right">
                      <IconButton
                        variant="ghost" className="text-muted-foreground hover:text-destructive"
                        onClick={() => onDeleteAsset(asset.id)}
                        icon={Trash2}
                        label={`Delete asset ${asset.name}`}
                      />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
