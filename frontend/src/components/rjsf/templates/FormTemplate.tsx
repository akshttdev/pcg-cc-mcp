import type { ReactNode } from 'react';

export const FormTemplate = (props: { children: ReactNode }) => {
  const { children } = props;

  return <div className="w-full">{children}</div>;
};
