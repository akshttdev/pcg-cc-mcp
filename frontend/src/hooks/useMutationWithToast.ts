import { useMutation, useQueryClient, type UseMutationOptions } from '@tanstack/react-query';
import { toast } from 'sonner';

type MessageOrFn<TData> = string | ((data: TData) => string);

function resolveMessage<TData>(msg: MessageOrFn<TData> | undefined, data: TData): string | undefined {
  if (!msg) return undefined;
  return typeof msg === 'function' ? msg(data) : msg;
}

interface MutationWithToastOptions<TData, TError, TVariables, TContext>
  extends Omit<UseMutationOptions<TData, TError, TVariables, TContext>, 'onSuccess' | 'onError'> {
  /** Toast message on success. String or `(data) => string`. */
  successMessage?: MessageOrFn<TData>;
  /** Toast message on error. String or `(error) => string`. Defaults to generic message. */
  errorMessage?: MessageOrFn<TError>;
  /** Query keys to invalidate on success. */
  invalidateKeys?: readonly (readonly unknown[])[];
  /** Additional onSuccess callback (runs after toast + invalidation). */
  onSuccess?: (data: TData, variables: TVariables, context: TContext | undefined) => void;
  /** Additional onError callback (runs after toast). */
  onError?: (error: TError, variables: TVariables, context: TContext | undefined) => void;
}

/**
 * Wrapper around `useMutation` that handles:
 * - `toast.success` / `toast.error` on settle
 * - `queryClient.invalidateQueries` for one or more keys on success
 *
 * Usage:
 * ```ts
 * const deleteMutation = useMutationWithToast({
 *   mutationFn: (id: string) => api.delete(id),
 *   successMessage: 'Deleted successfully',
 *   errorMessage: 'Failed to delete',
 *   invalidateKeys: [queryKeys.tasks.all],
 * });
 * ```
 */
export function useMutationWithToast<
  TData = unknown,
  TError = Error,
  TVariables = void,
  TContext = unknown,
>(options: MutationWithToastOptions<TData, TError, TVariables, TContext>) {
  const queryClient = useQueryClient();

  const {
    successMessage,
    errorMessage,
    invalidateKeys,
    onSuccess: userOnSuccess,
    onError: userOnError,
    ...mutationOptions
  } = options;

  return useMutation<TData, TError, TVariables, TContext>({
    ...mutationOptions,
    onSuccess: (data, variables, context) => {
      const msg = resolveMessage(successMessage, data);
      if (msg) toast.success(msg);

      if (invalidateKeys?.length) {
        for (const key of invalidateKeys) {
          queryClient.invalidateQueries({ queryKey: key as unknown[] });
        }
      }

      userOnSuccess?.(data, variables, context);
    },
    onError: (error, variables, context) => {
      const msg = resolveMessage(errorMessage, error);
      if (msg) {
        toast.error(msg);
      } else {
        toast.error('Something went wrong');
      }

      userOnError?.(error, variables, context);
    },
  });
}
