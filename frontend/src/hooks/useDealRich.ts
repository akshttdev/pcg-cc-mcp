// NOTE: useDealRich is currently unused (PR #36 review item C5).
// Commenting out rather than deleting — restore when needed.
//
// import { useQuery } from '@tanstack/react-query';
// import { crmDealsApi } from '@/lib/api';
//
// export function useDealRich(dealId: string | undefined) {
//   return useQuery({
//     queryKey: ['deal-rich', dealId],
//     queryFn: () => crmDealsApi.getDealRich(dealId!),
//     enabled: !!dealId,
//     staleTime: 30_000,
//   });
// }
