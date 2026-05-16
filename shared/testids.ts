export const dealCard = {} as Record<string, string>;
export const pipeline = {} as Record<string, string>;
export const dealDetail = {} as Record<string, string>;
export const agentHistory = {} as Record<string, string>;
export const callScheduling = {} as Record<string, string>;
export const deck = {} as Record<string, string>;
export const discovery = {} as Record<string, string>;
export const pipelineSettings = {} as Record<string, string>;
export const review = {} as Record<string, string>;

export const avatars = {
  page: 'avatars-page',
  newAvatarCard: 'avatars-new-card',
  newAvatarDropzone: 'avatars-new-dropzone',
  newAvatarNameInput: 'avatars-new-name',
  newAvatarSubmit: 'avatars-new-submit',
  card: (id: string) => `avatars-card-${id}`,
  generateProfile: (id: string) => `avatars-generate-profile-${id}`,
  shot: (slot: string) => `avatars-shot-${slot}`,
  bibleSection: 'avatars-bible',
} as const;
