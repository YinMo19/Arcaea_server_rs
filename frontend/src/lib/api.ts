export type ApiEnvelope<T> = {
  success: boolean
  value?: T
  error_code?: number
  message?: string
  extra?: Record<string, unknown>
}

export type AdminSession = {
  loggedIn: boolean
  role: number
  appTitle?: string
  loginBackground?: string
  loginPosition?: string
  user?: AdminUserSummary
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

export type PageData<T> = {
  rows: T[]
  total: number
  page: number
  pageSize: number
}

export type PageParams = {
  page: number
  pageSize: number
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

export type AdminUserSummary = {
  userId: number
  name: string
  userCode: string
}

export type AdminScoreRow = {
  userId: number
  name?: string
  songId: string
  difficulty: number
  score: number
  shinyPerfectCount: number
  perfectCount: number
  nearCount: number
  missCount: number
  clearType: number
  bestClearType: number
  rating: number
  timePlayed: string
}

export type AdminUserScoreStats = {
  best30Sum: number
  recent10Sum: number
  potential: number
}

export type AdminUserScores = {
  user: AdminUserSummary
  stats: AdminUserScoreStats
  b30: AdminScoreRow[]
  r10: AdminScoreRow[]
}

export type ScoreImage = {
  mode: string
  title: string
  entryCount: number
  url: string
}

export type ScoreImages = {
  user: AdminUserSummary
  images: ScoreImage[]
}

export type AdminChartTop = {
  songId: string
  nameEn: string
  difficulty: number
  scores: AdminScoreRow[]
}

export type AdminActionResult = {
  message: string
  affectedRows: number
}

export type AdminRedeemUsers = {
  code: string
  users: AdminUserSummary[]
}

export type UserSelectorPayload = {
  user_id?: number
  name?: string
  user_code?: string
}

export type UserTicketPayload = UserSelectorPayload & {
  ticket: number
  all_users?: boolean
}

export type UserPasswordPayload = UserSelectorPayload & {
  password: string
}

export type UserPurchasePayload = UserSelectorPayload & {
  method: 'unlock' | 'lock'
  all_users?: boolean
  item_types?: string[]
}

export type ScoreDeletePayload = UserSelectorPayload & {
  song_id?: string
  difficulty?: number
}

export type PresentPayload = {
  present_id: string
  expire_ts?: string
  description?: string
  item_id: string
  item_type: string
  amount?: string
}

export type PresentDeliverPayload = UserSelectorPayload & {
  present_id: string
  all_users?: boolean
}

export type RedeemPayload = {
  code?: string
  random_amount?: number
  redeem_type: number
  item_id: string
  item_type: string
  amount?: string
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
    throw new Error(data.message ?? `Request failed: ${data.error_code ?? response.status}`)
  }

  return data.value as T
}

function query(params: Record<string, string | number | undefined>) {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== '') {
      search.set(key, String(value))
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
  users: (params: PageParams & { q?: string; status?: string }) =>
    request<PageData<UserRow>>(
      `/web/api/users${query({
        q: params.q,
        status: params.status,
        page: params.page,
        page_size: params.pageSize,
      })}`,
    ),
  songs: (params: PageParams & { q?: string }) =>
    request<PageData<SongRow>>(
      `/web/api/songs${query({
        q: params.q,
        page: params.page,
        page_size: params.pageSize,
      })}`,
    ),
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
  items: (params: PageParams & { q?: string }) =>
    request<PageData<ItemRow>>(
      `/web/api/items${query({
        q: params.q,
        page: params.page,
        page_size: params.pageSize,
      })}`,
    ),
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
  purchases: (params: PageParams & { pq?: string }) =>
    request<PageData<PurchaseRow>>(
      `/web/api/purchases${query({
        pq: params.pq,
        page: params.page,
        page_size: params.pageSize,
      })}`,
    ),
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
  purchaseItems: (params: PageParams & { iq?: string }) =>
    request<PageData<PurchaseItemRow>>(
      `/web/api/purchase-items${query({
        iq: params.iq,
        page: params.page,
        page_size: params.pageSize,
      })}`,
    ),
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
  userScores: (params: UserSelectorPayload) =>
    request<AdminUserScores>(
      `/web/api/user-scores${query({
        user_id: params.user_id,
        name: params.name,
        user_code: params.user_code,
      })}`,
    ),
  scoreImages: (params: UserSelectorPayload) =>
    request<ScoreImages>(
      `/web/api/score-images${query({
        user_id: params.user_id,
        name: params.name,
        user_code: params.user_code,
      })}`,
    ),
  chartTop: (params: { sid: string; difficulty: number; limit?: number }) =>
    request<AdminChartTop>(
      `/web/api/chart-top${query({
        sid: params.sid,
        difficulty: params.difficulty,
        limit: params.limit,
      })}`,
    ),
  updateUserTicket: (payload: UserTicketPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/user-ticket', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  resetUserPassword: (payload: UserPasswordPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/user-password', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  banUser: (payload: UserSelectorPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/user-ban', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateUserPurchase: (payload: UserPurchasePayload) =>
    request<AdminActionResult>('/web/api/admin-actions/user-purchase', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  deleteScores: (payload: ScoreDeletePayload) =>
    request<AdminActionResult>('/web/api/admin-actions/scores/delete', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  redeemUsers: (code: string) =>
    request<AdminRedeemUsers>(
      `/web/api/redeem-users${query({
        code,
      })}`,
    ),
  createPresent: (payload: PresentPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/presents', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  deletePresent: (present_id: string) =>
    request<AdminActionResult>('/web/api/admin-actions/presents', {
      method: 'DELETE',
      body: JSON.stringify({ present_id }),
    }),
  deliverPresent: (payload: PresentDeliverPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/presents/deliver', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  createRedeem: (payload: RedeemPayload) =>
    request<AdminActionResult>('/web/api/admin-actions/redeems', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  deleteRedeem: (code: string) =>
    request<AdminActionResult>('/web/api/admin-actions/redeems', {
      method: 'DELETE',
      body: JSON.stringify({ code }),
    }),
}
