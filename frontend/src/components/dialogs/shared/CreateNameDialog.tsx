import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { useState } from 'react';

export interface CreateNameDialogProps {
  title: string;
  label?: string;
  placeholder?: string;
  submitText?: string;
}

export interface CreateNameDialogResult {
  name: string;
}

const CreateNameDialog = NiceModal.create<CreateNameDialogProps>((props) => {
  const modal = useModal();
  const {
    title,
    label = 'Name',
    placeholder = 'Enter a name...',
    submitText = 'Create',
  } = props;
  const [name, setName] = useState('');

  const handleSubmit = () => {
    if (!name.trim()) return;
    modal.resolve({ name: name.trim() } as CreateNameDialogResult);
    modal.hide();
  };

  const handleCancel = () => {
    modal.reject();
    modal.hide();
  };

  return (
    <Dialog open={modal.visible} onOpenChange={(open) => { if (!open) handleCancel(); }}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className="py-2">
          <Label htmlFor="create-name-input">{label}</Label>
          <Input
            id="create-name-input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={placeholder}
            className="mt-1.5"
            autoFocus
            onKeyDown={(e) => { if (e.key === 'Enter') handleSubmit(); }}
          />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleCancel}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={!name.trim()}>{submitText}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export { CreateNameDialog };
