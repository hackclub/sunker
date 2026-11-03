export const ORDER_STATUSES = ['pending', 'rejected', 'ordered', 'fulfilled'] as const;

export type OrderStatus = (typeof ORDER_STATUSES)[number];
