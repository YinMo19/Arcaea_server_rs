export type ApiEnvelope<T> = {
  success: boolean
  value?: T
  error_code?: number
  extra?: Record<string, unknown>
}

export type AdminSession = {
  loggedIn: boolean
}

export type DashboardData = {
  onlineUsers: number
  onlineGrowth: number
  scoreSubmits: number
  scoreErrorRate: number
  presentCount: number
  alertCount: number
  recentOps: RecentOp[]
}

export type RecentOp = {
  name: string
  operator: string
  time: string
  status: string
}

export type UserRow = {
  userId: number
  name: string
  userCode: string
  ratingPtt: number
  ticket: number
  lastPlay: string
  banned: boolean
}

export type SongRow = {
  songId: string
  nameEn: string
  ratingPst: string
  ratingPrs: string
  ratingFtr: string
  ratingByd: string
  ratingEtr: string
}

export type SongPayload = {
  sid: string
  name_en: string
  rating_pst: string
  rating_prs: string
  rating_ftr: string
  rating_byd: string
  rating_etr: string
}

export type ItemRow = {
  itemId: string
  itemType: string
  isAvailable: number
}

export type ItemPayload = {
  item_id: string
  item_type: string
  is_available?: number
}

export type PurchaseRow = {
  purchaseName: string
  price: string
  origPrice: string
  discountFrom: string
  discountTo: string
  discountReason: string
  itemSummary: string
}

export type PurchasePayload = {
  purchase_name: string
  price?: string
  orig_price?: string
  discount_from?: string
  discount_to?: string
  discount_reason?: string
}

export type PurchaseItemRow = {
  purchaseName: string
  itemId: string
  itemType: string
  amount: string
}

export type PurchaseItemPayload = {
  purchase_name: string
  item_id: string
  item_type: string
  amount?: string
}

export type PurchaseData = {
  purchases: PurchaseRow[]
  purchaseItems: PurchaseItemRow[]
}

export type AdminOperation =
  | 'refresh_song_file_cache'
  | 'refresh_content_bundle_cache'
  | 'refresh_all_score_rating'

async function request<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const response = await fetch(path, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...init?.headers,
    },
    ...init,
  })

  const data = (await response.json()) as ApiEnvelope<T>
  if (!response.ok || !data.success) {
    throw new Error(`Request failed: ${data.error_code ?? response.status}`)
  }

  return data.value as T
}

function query(params: Record<string, string | undefined>) {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value) {
      search.set(key, value)
    }
  }
  const value = search.toString()
  return value ? `?${value}` : ''
}

export const adminApi = {
  session: () => request<AdminSession>('/web/api/session'),
  login: (username: string, password: string) =>
    request<AdminSession>('/web/api/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () =>
    request<void>('/web/api/logout', {
      method: 'POST',
    }),
  dashboard: () => request<DashboardData>('/web/api/dashboard'),
  operation: (operation: AdminOperation) =>
    request<void>(`/web/api/operations/${operation}`, {
      method: 'POST',
    }),
  users: (q?: string, status?: string) =>
    request<UserRow[]>(`/web/api/users${query({ q, status })}`),
  songs: (q?: string) => request<SongRow[]>(`/web/api/songs${query({ q })}`),
  createSong: (payload: SongPayload) =>
    request<void>('/web/api/songs', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateSong: (sid: string, payload: SongPayload) =>
    request<void>(`/web/api/songs/${encodeURIComponent(sid)}`, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deleteSong: (sid: string) =>
    request<void>('/web/api/songs', {
      method: 'DELETE',
      body: JSON.stringify({ sid }),
    }),
  items: (q?: string) => request<ItemRow[]>(`/web/api/items${query({ q })}`),
  createItem: (payload: ItemPayload) =>
    request<void>('/web/api/items', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateItem: (payload: ItemPayload) =>
    request<void>('/web/api/items', {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deleteItem: (item_id: string, item_type: string) =>
    request<void>('/web/api/items', {
      method: 'DELETE',
      body: JSON.stringify({ item_id, item_type }),
    }),
  purchases: (pq?: string, iq?: string) =>
    request<PurchaseData>(`/web/api/purchases${query({ pq, iq })}`),
  createPurchase: (payload: PurchasePayload) =>
    request<void>('/web/api/purchases', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updatePurchase: (purchaseName: string, payload: PurchasePayload) =>
    request<void>(`/web/api/purchases/${encodeURIComponent(purchaseName)}`, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deletePurchase: (purchase_name: string) =>
    request<void>('/web/api/purchases', {
      method: 'DELETE',
      body: JSON.stringify({ purchase_name }),
    }),
  createPurchaseItem: (payload: PurchaseItemPayload) =>
    request<void>('/web/api/purchase-items', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updatePurchaseItem: (payload: PurchaseItemPayload) =>
    request<void>('/web/api/purchase-items', {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deletePurchaseItem: (
    purchase_name: string,
    item_id: string,
    item_type: string,
  ) =>
    request<void>('/web/api/purchase-items', {
      method: 'DELETE',
      body: JSON.stringify({ purchase_name, item_id, item_type }),
    }),
}
