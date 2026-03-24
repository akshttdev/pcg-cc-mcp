import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { FormField } from '@/components/ui/form-field';
import { EditorType } from 'shared/types';
import { toPrettyCase } from '@/utils/string';

interface EditorSectionProps {
  t: (key: string) => string;
  draft: { editor: { editor_type: EditorType } } | null;
  updateDraft: (patch: Record<string, unknown>) => void;
}

export function EditorSection({
  t,
  draft,
  updateDraft,
}: EditorSectionProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.general.editor.title')}</CardTitle>
        <CardDescription>
          {t('settings.general.editor.description')}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <FormField
          label={t('settings.general.editor.type.label')}
          htmlFor="editor-type"
          description={t('settings.general.editor.type.helper')}
        >
          <Select
            value={draft?.editor.editor_type}
            onValueChange={(value: EditorType) =>
              updateDraft({
                editor: { ...draft!.editor, editor_type: value },
              })
            }
          >
            <SelectTrigger id="editor-type">
              <SelectValue
                placeholder={t('settings.general.editor.type.placeholder')}
              />
            </SelectTrigger>
            <SelectContent>
              {Object.values(EditorType).map((editor) => (
                <SelectItem key={editor} value={editor}>
                  {toPrettyCase(editor)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </FormField>
      </CardContent>
    </Card>
  );
}
