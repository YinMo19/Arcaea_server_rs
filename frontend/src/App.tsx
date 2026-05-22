import { Fragment, useCallback, useEffect, useMemo, useState } from 'react'
import type { CSSProperties, FormEvent, ReactNode } from 'react'
import {
  Activity,
  Boxes,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  ChartSpline,
  Database,
  Images,
  KeyRound,
  Link2,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Music2,
  PackagePlus,
  Pencil,
  Plus,
  RefreshCcw,
  Search,
  ShieldAlert,
  ShieldCheck,
  ShoppingBag,
  Trash2,
  UserRound,
  Users,
  X,
} from 'lucide-react'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  adminApi,
  type AdminChartTop,
  type AdminActionResult,
  type AdminOperation,
  type AdminScoreRow,
  type AdminSession,
  type AdminUserSummary,
  type AdminUserScores,
  type DashboardData,
  type ItemPayload,
  type ItemRow,
  type PageData,
  type PresentDeliverPayload,
  type PresentPayload,
  type PurchaseItemPayload,
  type PurchaseItemRow,
  type PurchasePayload,
  type PurchaseRow,
  type RedeemPayload,
  type ScoreDeletePayload,
  type ScoreImages,
  type SongPayload,
  type SongRow,
  type UserPurchasePayload,
  type UserRow,
  type UserSelectorPayload,
  type UserTicketPayload,
} from '@/lib/api'
import { cn } from '@/lib/utils'

const defaultAppTitle = 'Arcaea Server'
const githubUrl = 'https://github.com/YinMo19/Arcaea_server_rs'

type LoginPosition = 'left' | 'center' | 'right'
type LoginConfig = {
  title: string
  backgroundUrl?: string
  position: LoginPosition
  cardOpacity: number
  surfaceOpacity: number
}

const defaultLoginConfig: LoginConfig = {
  title: defaultAppTitle,
  position: 'center',
  cardOpacity: 1,
  surfaceOpacity: 1,
}

function normalizeLoginPosition(value?: string): LoginPosition {
  return value === 'left' || value === 'right' ? value : 'center'
}

function normalizeLoginCardOpacity(value?: number): number {
  if (value === undefined || !Number.isFinite(value)) {
    return 1
  }

  return Math.min(1, Math.max(0, value))
}

function opacityPercent(value: number): number {
  return Math.round(value * 1000) / 10
}

function loginConfigFromSession(session: AdminSession): LoginConfig {
  const title = session.appTitle?.trim() || defaultAppTitle
  const backgroundUrl = session.loginBackground?.trim() || undefined

  return {
    title,
    backgroundUrl,
    position: normalizeLoginPosition(session.loginPosition),
    cardOpacity: normalizeLoginCardOpacity(session.loginCardOpacity),
    surfaceOpacity: normalizeLoginCardOpacity(session.webSurfaceOpacity),
  }
}

type View =
  | 'dashboard'
  | MaintenanceView
  | 'users'
  | 'playerScores'
  | 'scoreImages'
  | 'chartTop'
  | 'userTicket'
  | 'userPassword'
  | 'userBan'
  | 'userPurchase'
  | 'scoreDelete'
  | 'presentCreate'
  | 'presentDeliver'
  | 'presentDelete'
  | 'redeemCreate'
  | 'redeemDelete'
  | 'redeemUsers'
  | 'songs'
  | 'items'
  | 'purchases'
  | 'purchaseItems'

type MaintenanceView =
  | 'refreshSongFileCache'
  | 'refreshContentBundleCache'
  | 'refreshAllScoreRating'

type MaintenanceOperationConfig = {
  operation: AdminOperation
  title: string
  description: string
  buttonLabel: string
  confirmText?: string
}

const maintenanceOperations: Record<MaintenanceView, MaintenanceOperationConfig> = {
  refreshSongFileCache: {
    operation: 'refresh_song_file_cache',
    title: '刷新 Song Hash',
    description: '重新扫描歌曲文件 hash 缓存',
    buttonLabel: '刷新 Song Hash',
  },
  refreshContentBundleCache: {
    operation: 'refresh_content_bundle_cache',
    title: '刷新 Bundle',
    description: '重新加载内容包缓存',
    buttonLabel: '刷新 Bundle',
  },
  refreshAllScoreRating: {
    operation: 'refresh_all_score_rating',
    title: '重算 Rating',
    description: '重新计算所有成绩 Rating',
    buttonLabel: '重算 Rating',
    confirmText: '重算所有成绩 Rating?',
  },
}

type NavItem = {
  id: View
  label: string
  icon: typeof Activity
}

const navSections: Array<{ label: string; items: NavItem[] }> = [
  {
    label: '概览',
    items: [{ id: 'dashboard', label: '总览', icon: Activity }],
  },
  {
    label: '查询',
    items: [
      { id: 'users', label: '玩家', icon: Users },
      { id: 'playerScores', label: '玩家成绩', icon: ChartSpline },
      { id: 'chartTop', label: '单曲榜', icon: Search },
      { id: 'redeemUsers', label: '兑换使用者', icon: Users },
    ],
  },
  {
    label: '账号',
    items: [
      { id: 'userTicket', label: '记忆源点', icon: Pencil },
      { id: 'userPassword', label: '重置密码', icon: KeyRound },
      { id: 'userBan', label: '封禁用户', icon: ShieldAlert },
      { id: 'userPurchase', label: '购买权限', icon: ShoppingBag },
    ],
  },
  {
    label: '成绩',
    items: [
      { id: 'scoreImages', label: '成绩图', icon: Images },
      { id: 'scoreDelete', label: '删除成绩', icon: Trash2 },
    ],
  },
  {
    label: '奖励',
    items: [
      { id: 'presentCreate', label: '新增奖励', icon: Plus },
      { id: 'presentDeliver', label: '分发奖励', icon: PackagePlus },
      { id: 'presentDelete', label: '删除奖励', icon: Trash2 },
    ],
  },
  {
    label: '兑换码',
    items: [
      { id: 'redeemCreate', label: '新增兑换码', icon: Plus },
      { id: 'redeemDelete', label: '删除兑换码', icon: Trash2 },
    ],
  },
  {
    label: '数据表',
    items: [
      { id: 'songs', label: '歌曲', icon: Music2 },
      { id: 'items', label: '物品', icon: Boxes },
      { id: 'purchases', label: '购买项', icon: ShoppingBag },
      { id: 'purchaseItems', label: '购买物品', icon: Link2 },
    ],
  },
  {
    label: '维护',
    items: [
      { id: 'refreshSongFileCache', label: '刷新 Song Hash', icon: RefreshCcw },
      { id: 'refreshContentBundleCache', label: '刷新 Bundle', icon: RefreshCcw },
      { id: 'refreshAllScoreRating', label: '重算 Rating', icon: RefreshCcw },
    ],
  },
]

const userAllowedViews = new Set<View>([
  'playerScores',
  'scoreImages',
  'chartTop',
  'songs',
  'items',
  'purchases',
])

type LoadState = 'idle' | 'loading' | 'ready' | 'error'
type ActionState = {
  kind: 'idle' | 'success' | 'error'
  message: string
}

const emptyAction: ActionState = { kind: 'idle', message: '' }
const defaultTablePageSize = 25
const pageSizeOptions = [10, 25, 50, 100]

const emptySongForm: SongPayload = {
  sid: '',
  name_en: '',
  rating_pst: '-1',
  rating_prs: '-1',
  rating_ftr: '-1',
  rating_byd: '-1',
  rating_etr: '-1',
}

const emptyItemForm: ItemPayload = {
  item_id: '',
  item_type: '',
  is_available: 1,
}

const emptyPurchaseForm: PurchasePayload = {
  purchase_name: '',
  price: '',
  orig_price: '',
  discount_from: '',
  discount_to: '',
  discount_reason: '',
}

const emptyPurchaseItemForm: PurchaseItemPayload = {
  purchase_name: '',
  item_id: '',
  item_type: '',
  amount: '1',
}

type UserSelectorForm = {
  userId: string
  name: string
  userCode: string
}

const emptyUserSelectorForm: UserSelectorForm = {
  userId: '',
  name: '',
  userCode: '',
}

type UserTicketForm = UserSelectorForm & {
  ticket: string
  allUsers: boolean
}

const emptyUserTicketForm: UserTicketForm = {
  ...emptyUserSelectorForm,
  ticket: '',
  allUsers: false,
}

type UserPasswordForm = UserSelectorForm & {
  password: string
}

const emptyUserPasswordForm: UserPasswordForm = {
  ...emptyUserSelectorForm,
  password: '',
}

type UserPurchaseForm = UserSelectorForm & {
  method: 'unlock' | 'lock'
  allUsers: boolean
  itemTypes: string[]
}

const defaultUserPurchaseItemTypes = ['pack', 'single']

const emptyUserPurchaseForm: UserPurchaseForm = {
  ...emptyUserSelectorForm,
  method: 'unlock',
  allUsers: false,
  itemTypes: defaultUserPurchaseItemTypes,
}

type ScoreDeleteForm = UserSelectorForm & {
  songId: string
  difficulty: string
}

const emptyScoreDeleteForm: ScoreDeleteForm = {
  ...emptyUserSelectorForm,
  songId: '',
  difficulty: '-1',
}

type PresentForm = {
  presentId: string
  expireTs: string
  description: string
  itemId: string
  itemType: string
  amount: string
}

const emptyPresentForm: PresentForm = {
  presentId: '',
  expireTs: '',
  description: '',
  itemId: '',
  itemType: '',
  amount: '1',
}

type PresentDeliverForm = UserSelectorForm & {
  presentId: string
  allUsers: boolean
}

const emptyPresentDeliverForm: PresentDeliverForm = {
  ...emptyUserSelectorForm,
  presentId: '',
  allUsers: false,
}

type RedeemForm = {
  code: string
  randomAmount: string
  redeemType: string
  itemId: string
  itemType: string
  amount: string
}

const emptyRedeemForm: RedeemForm = {
  code: '',
  randomAmount: '',
  redeemType: '0',
  itemId: '',
  itemType: '',
  amount: '1',
}

const purchaseItemTypeOptions = [
  'pack',
  'single',
  'world_song',
  'world_unlock',
  'course_banner',
  'online_banner',
]

function App() {
  const [session, setSession] = useState<AdminSession>()
  const [checkingSession, setCheckingSession] = useState(true)
  const [loginConfig, setLoginConfig] =
    useState<LoginConfig>(defaultLoginConfig)
  const [view, setView] = useState<View>('dashboard')
  const isAdmin = session?.role === 1
  const hasPageBackground = Boolean(loginConfig.backgroundUrl)
  const shellStyle = hasPageBackground
    ? ({
        '--web-surface-bg': `color-mix(in oklab, var(--card) ${opacityPercent(loginConfig.surfaceOpacity)}%, transparent)`,
        '--web-sidebar-bg': `color-mix(in oklab, var(--sidebar) ${opacityPercent(loginConfig.surfaceOpacity)}%, transparent)`,
        '--web-header-bg': `color-mix(in oklab, var(--background) ${opacityPercent(loginConfig.surfaceOpacity)}%, transparent)`,
        '--web-control-bg': `color-mix(in oklab, var(--background) ${opacityPercent(Math.min(1, loginConfig.surfaceOpacity + 0.15))}%, transparent)`,
      } as CSSProperties)
    : undefined
  const visibleNavSections = useMemo(
    () =>
      isAdmin
        ? navSections
        : navSections
            .map((section) => ({
              ...section,
              items: section.items.filter((item) => userAllowedViews.has(item.id)),
            }))
            .filter((section) => section.items.length > 0),
    [isAdmin],
  )
  const activeView =
    isAdmin || userAllowedViews.has(view) ? view : 'scoreImages'

  useEffect(() => {
    adminApi
      .session()
      .then((session) => {
        setLoginConfig(loginConfigFromSession(session))
        setSession(session.loggedIn ? session : undefined)
        if (session.loggedIn && session.role !== 1) {
          setView('scoreImages')
        }
      })
      .catch(() => setSession(undefined))
      .finally(() => setCheckingSession(false))
  }, [])

  if (checkingSession) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <LoaderCircle className="size-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!session) {
    return (
      <LoginScreen
        config={loginConfig}
        onLoggedIn={(session) => {
          setLoginConfig(loginConfigFromSession(session))
          setSession(session)
          if (session.role !== 1) {
            setView('scoreImages')
          }
        }}
      />
    )
  }

  return (
    <div
      className={cn(
        'web-shell min-h-svh bg-background text-foreground',
        hasPageBackground && 'web-shell-with-bg relative',
      )}
      style={shellStyle}
    >
      {loginConfig.backgroundUrl && (
        <>
          <div
            className="pointer-events-none fixed inset-0 bg-cover bg-center bg-no-repeat"
            style={{
              backgroundImage: `url(${JSON.stringify(loginConfig.backgroundUrl)})`,
            }}
          />
          <div className="pointer-events-none fixed inset-0 bg-background/25" />
        </>
      )}
      <aside className="web-shell-sidebar fixed inset-y-0 left-0 z-20 hidden w-64 border-r bg-sidebar px-4 py-5 lg:block">
        <div className="flex items-center gap-3 px-2">
          <div className="flex size-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Database className="size-5" />
          </div>
          <div>
            <div className="text-sm font-semibold">Arcaea Admin</div>
            <div className="text-xs text-muted-foreground">Operations</div>
          </div>
        </div>

        <nav className="mt-6 grid max-h-[calc(100svh-7rem)] gap-4 overflow-auto pr-1">
          {visibleNavSections.map((section) => (
            <div key={section.label} className="grid gap-1">
              <div className="px-3 text-xs font-medium text-muted-foreground">
                {section.label}
              </div>
              {section.items.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setView(item.id)}
                  className={cn(
                    'flex h-9 items-center gap-3 rounded-md px-3 text-left text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground',
                    activeView === item.id && 'bg-accent text-accent-foreground',
                  )}
                >
                  <item.icon className="size-4" />
                  {item.label}
                </button>
              ))}
            </div>
          ))}
        </nav>
      </aside>

      <div className="relative z-10 lg:pl-64">
        <header className="web-shell-header sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
          <div className="flex min-h-16 items-center justify-between gap-4 px-4 sm:px-6">
            <div>
              <h1 className="text-lg font-semibold">{viewTitle(activeView)}</h1>
              <p className="text-sm text-muted-foreground">
                {viewSubtitle(activeView)}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <select
                className="h-9 max-w-48 rounded-md border bg-background px-3 text-sm lg:hidden"
                value={activeView}
                onChange={(event) => setView(event.target.value as View)}
              >
                {visibleNavSections.map((section) => (
                  <optgroup key={section.label} label={section.label}>
                    {section.items.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.label}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  adminApi.logout().finally(() => setSession(undefined))
                }}
              >
                <LogOut />
                登出
              </Button>
            </div>
          </div>
        </header>

        <main className="px-4 py-5 sm:px-6">
          {isAdmin && activeView === 'dashboard' && <DashboardView />}
          {isAdmin && isMaintenanceView(activeView) && (
            <MaintenanceOperationView config={maintenanceOperations[activeView]} />
          )}
          {isAdmin && activeView === 'users' && <UsersView />}
          {activeView === 'playerScores' && <PlayerScoresView isAdmin={isAdmin} />}
          {activeView === 'scoreImages' && <ScoreImagesView isAdmin={isAdmin} />}
          {activeView === 'chartTop' && <ChartTopView />}
          {isAdmin && activeView === 'userTicket' && <UserTicketView />}
          {isAdmin && activeView === 'userPassword' && <UserPasswordView />}
          {isAdmin && activeView === 'userBan' && <UserBanView />}
          {isAdmin && activeView === 'userPurchase' && <UserPurchaseView />}
          {isAdmin && activeView === 'scoreDelete' && <ScoreDeleteView />}
          {isAdmin && activeView === 'presentCreate' && <PresentCreateView />}
          {isAdmin && activeView === 'presentDeliver' && <PresentDeliverView />}
          {isAdmin && activeView === 'presentDelete' && <PresentDeleteView />}
          {isAdmin && activeView === 'redeemCreate' && <RedeemCreateView />}
          {isAdmin && activeView === 'redeemDelete' && <RedeemDeleteView />}
          {isAdmin && activeView === 'redeemUsers' && <RedeemUsersView />}
          {activeView === 'songs' && <SongsView isAdmin={isAdmin} />}
          {activeView === 'items' && <ItemsView isAdmin={isAdmin} />}
          {activeView === 'purchases' && <PurchasesView isAdmin={isAdmin} />}
          {isAdmin && activeView === 'purchaseItems' && <PurchaseItemsView />}
        </main>
      </div>
    </div>
  )
}

function LoginScreen({
  config,
  onLoggedIn,
}: {
  config: LoginConfig
  onLoggedIn: (session: AdminSession) => void
}) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setError('')
    try {
      const session = await adminApi.login(username, password)
      onLoggedIn(session)
    } catch {
      setError('用户名或密码错误')
    } finally {
      setLoading(false)
    }
  }

  const cardAnchorClass = {
    left: 'lg:absolute lg:left-1/4 lg:top-1/2 lg:-translate-x-1/2 lg:-translate-y-1/2',
    center: '',
    right:
      'lg:absolute lg:left-3/4 lg:top-1/2 lg:-translate-x-1/2 lg:-translate-y-1/2',
  }[config.position]
  const shellWidthClass = config.position === 'center' ? 'max-w-md' : 'max-w-none'
  const cardBackgroundOpacity = opacityPercent(config.cardOpacity)

  return (
    <div className="relative min-h-svh overflow-hidden bg-background px-4 py-6 text-foreground sm:px-6">
      {config.backgroundUrl ? (
        <>
          <div
            className="pointer-events-none absolute inset-0 bg-cover bg-center bg-no-repeat"
            style={{
              backgroundImage: `url(${JSON.stringify(config.backgroundUrl)})`,
            }}
          />
          <div className="pointer-events-none absolute inset-0 bg-background/25" />
        </>
      ) : (
        <>
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(115deg,rgba(15,23,42,0.07),transparent_42%),linear-gradient(180deg,rgba(255,255,255,0.85),rgba(226,232,240,0.58))]" />
          <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(rgba(15,23,42,0.045)_1px,transparent_1px),linear-gradient(90deg,rgba(15,23,42,0.045)_1px,transparent_1px)] bg-[size:40px_40px]" />
        </>
      )}

      <div
        className={cn(
          'relative mx-auto flex min-h-[calc(100svh-3rem)] w-full flex-col',
          shellWidthClass,
        )}
      >
        <div className="relative flex w-full flex-1 items-center justify-center">
          <Card
            className={cn('w-full max-w-md border shadow-lg backdrop-blur', cardAnchorClass)}
            style={{
              backgroundColor: `color-mix(in oklab, var(--card) ${cardBackgroundOpacity}%, transparent)`,
            }}
          >
            <CardHeader className="gap-5 p-6">
              <div className="flex items-center gap-3">
                <div className="flex size-11 items-center justify-center rounded-md bg-primary text-primary-foreground shadow-sm">
                  <KeyRound className="size-5" />
                </div>
                <div className="min-w-0">
                  <CardTitle className="truncate text-xl">{config.title}</CardTitle>
                  <CardDescription className="mt-1">进入 Web 控制台</CardDescription>
                </div>
              </div>
            </CardHeader>
            <CardContent className="p-6 pt-0">
              <form className="grid gap-4" onSubmit={onSubmit}>
                <label className="grid gap-1.5 text-sm font-medium">
                  Username
                  <div className="relative">
                    <UserRound className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      className="h-11 bg-background pl-10"
                      value={username}
                      autoComplete="username"
                      onChange={(event) => setUsername(event.target.value)}
                      required
                    />
                  </div>
                </label>
                <label className="grid gap-1.5 text-sm font-medium">
                  Password
                  <div className="relative">
                    <LockKeyhole className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      className="h-11 bg-background pl-10"
                      value={password}
                      type="password"
                      autoComplete="current-password"
                      onChange={(event) => setPassword(event.target.value)}
                      required
                    />
                  </div>
                </label>
                {error && (
                  <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                    {error}
                  </div>
                )}
                <Button className="h-11 w-full" type="submit" disabled={loading}>
                  {loading ? (
                    <LoaderCircle className="animate-spin" />
                  ) : (
                    <ShieldCheck />
                  )}
                  登录
                </Button>
              </form>
            </CardContent>
          </Card>
        </div>
        <footer className="flex justify-center pb-1 pt-5">
          <a
            className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
            href={githubUrl}
            target="_blank"
            rel="noreferrer"
          >
            <GithubMark className="size-4" />
            By YinMo19
          </a>
        </footer>
      </div>
    </div>
  )
}

function GithubMark({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      role="img"
      aria-hidden="true"
      fill="currentColor"
    >
      <path d="M12 1.85C6.35 1.85 1.78 6.42 1.78 12.07c0 4.52 2.93 8.35 7 9.7.51.09.7-.22.7-.49v-1.8c-2.85.62-3.45-1.22-3.45-1.22-.47-1.18-1.14-1.49-1.14-1.49-.93-.64.07-.63.07-.63 1.03.07 1.57 1.06 1.57 1.06.92 1.57 2.4 1.12 2.98.85.09-.66.36-1.12.65-1.37-2.27-.26-4.66-1.14-4.66-5.06 0-1.12.4-2.03 1.05-2.75-.1-.26-.46-1.3.1-2.71 0 0 .86-.27 2.81 1.05.82-.23 1.69-.34 2.56-.35.87.01 1.75.12 2.56.35 1.95-1.32 2.81-1.05 2.81-1.05.56 1.41.2 2.45.1 2.71.66.72 1.05 1.63 1.05 2.75 0 3.93-2.39 4.79-4.67 5.05.37.32.69.94.69 1.9v2.82c0 .27.18.59.7.49a10.23 10.23 0 0 0 7-9.7C22.22 6.42 17.65 1.85 12 1.85Z" />
    </svg>
  )
}

function DashboardView() {
  const [data, setData] = useState<DashboardData>()
  const [state, setState] = useState<LoadState>('loading')

  function load(showLoading = true) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .dashboard()
      .then((value) => {
        setData(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  useEffect(() => {
    adminApi
      .dashboard()
      .then((value) => {
        setData(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [])

  if (!data) {
    return <LoadPanel state={state} onRetry={() => load()} />
  }

  return (
    <div className="grid gap-5">
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="24h 活跃"
          value={data.onlineUsers}
          sub={`${data.onlineGrowth.toFixed(1)}% vs previous day`}
          icon={Users}
        />
        <MetricCard
          label="成绩记录"
          value={data.scoreSubmits}
          sub="best_score rows"
          icon={ChartSpline}
        />
        <MetricCard
          label="奖励发放"
          value={data.presentCount}
          sub="user_present rows"
          icon={ShoppingBag}
        />
        <MetricCard
          label="风险账号"
          value={data.alertCount}
          sub="empty password accounts"
          icon={ShieldAlert}
        />
      </div>

      <Card>
        <CardHeader className="flex-row items-center justify-between">
          <div>
            <CardTitle>最近事件</CardTitle>
            <CardDescription>登录与系统事件</CardDescription>
          </div>
          <Button type="button" variant="outline" size="sm" onClick={() => load()}>
            <RefreshCcw />
            刷新
          </Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>事件</TableHead>
                <TableHead>操作者</TableHead>
                <TableHead>时间</TableHead>
                <TableHead>状态</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.recentOps.map((op, index) => (
                <TableRow key={`${op.name}-${op.time}-${index}`}>
                  <TableCell className="font-medium">{op.name}</TableCell>
                  <TableCell>{op.operator}</TableCell>
                  <TableCell>{op.time}</TableCell>
                  <TableCell>
                    <Badge variant="secondary">{op.status}</Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  )
}

function UsersView() {
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState('')
  const [rows, setRows] = useState<UserRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const pagination = useServerPagination(rows, defaultTablePageSize)
  const { setMeta } = pagination

  function load(showLoading = true, page = pagination.page, pageSize = pagination.pageSize) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .users({ q: query, status, page, pageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function search() {
    load(true, 1, pagination.pageSize)
  }

  useEffect(() => {
    adminApi
      .users({ page: 1, pageSize: defaultTablePageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [setMeta])

  return (
    <DataPanel
      title="玩家列表"
      description="账号状态、票券和最近游玩记录"
      state={state}
      onSearch={search}
      searchValue={query}
      onSearchChange={setQuery}
      extraControl={
        <select
          className="h-9 rounded-md border bg-background px-3 text-sm"
          value={status}
          onChange={(event) => setStatus(event.target.value)}
        >
          <option value="">全部状态</option>
          <option value="normal">正常</option>
          <option value="banned">封禁</option>
        </select>
      }
    >
      <TableBlock
        pagination={pagination}
        onPageChange={(page) => load(true, page, pagination.pageSize)}
        onPageSizeChange={(pageSize) => load(true, 1, pageSize)}
        emptyText="没有玩家数据"
        renderTable={(visibleRows) => (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>名称</TableHead>
                <TableHead>User Code</TableHead>
                <TableHead>PTT</TableHead>
                <TableHead>Ticket</TableHead>
                <TableHead>最近游玩</TableHead>
                <TableHead>状态</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => (
                <TableRow key={row.userId}>
                  <TableCell className="font-mono">{row.userId}</TableCell>
                  <TableCell className="font-medium">{row.name || '-'}</TableCell>
                  <TableCell>{row.userCode || '-'}</TableCell>
                  <TableCell>{(row.ratingPtt / 100).toFixed(2)}</TableCell>
                  <TableCell>{row.ticket}</TableCell>
                  <TableCell>{row.lastPlay}</TableCell>
                  <TableCell>
                    <Badge variant={row.banned ? 'destructive' : 'secondary'}>
                      {row.banned ? '封禁' : '正常'}
                    </Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      />
    </DataPanel>
  )
}

function MaintenanceOperationView({
  config,
}: {
  config: MaintenanceOperationConfig
}) {
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function runOperation() {
    if (config.confirmText && !confirm(config.confirmText)) {
      return
    }
    setLoading(true)
    setAction(emptyAction)
    try {
      await adminApi.operation(config.operation)
      setAction({ kind: 'success', message: '操作已完成' })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title={config.title} description={config.description}>
      <div className="flex flex-wrap items-center gap-2">
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={loading}
          onClick={runOperation}
        >
          {loading ? <LoaderCircle className="animate-spin" /> : <RefreshCcw />}
          {config.buttonLabel}
        </Button>
        <ActionMessage action={action} />
      </div>
    </ActionCard>
  )
}

function ActionCard({
  title,
  description,
  children,
  className,
  contentClassName,
}: {
  title: string
  description: string
  children: ReactNode
  className?: string
  contentClassName?: string
}) {
  return (
    <Card className={cn('w-full', className)}>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className={cn('flex flex-col gap-4', contentClassName)}>
        {children}
      </CardContent>
    </Card>
  )
}

function PlayerScoresView({ isAdmin }: { isAdmin: boolean }) {
  const [form, setForm] = useState({ ...emptyUserSelectorForm })
  const [scores, setScores] = useState<AdminUserScores>()
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)
  const best30Average = scores?.b30.length
    ? scores.stats.best30Sum / scores.b30.length
    : 0
  const recent10Average = scores?.r10.length
    ? scores.stats.recent10Sum / scores.r10.length
    : 0

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const result = await adminApi.userScores({
        ...(isAdmin ? buildUserSelectorPayload(form) : {}),
      })
      setScores(result)
      setAction({
        kind: 'success',
        message: `${result.user.name || result.user.userId} · B30 ${result.b30.length} · R10 ${result.r10.length}`,
      })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard
      title="玩家成绩"
      description="singleplayer"
      className="flex min-h-[calc(100svh-6.5rem)] flex-col"
      contentClassName="min-h-0 flex-1"
    >
      <form className="grid gap-3" onSubmit={onSubmit}>
        {isAdmin && (
          <UserSelectorFields
            value={form}
            onChange={(value) => setForm({ ...form, ...value })}
          />
        )}
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Search />}
            查询
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
      {scores && (
        <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-2">
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <span className="font-medium">
              {scores.user.name || '-'} · {scores.user.userId} ·{' '}
              {scores.user.userCode || '-'}
            </span>
            <Badge variant="secondary">PTT {scores.stats.potential.toFixed(4)}</Badge>
            <Badge variant="outline">B30 Avg {best30Average.toFixed(4)}</Badge>
            <Badge variant="outline">R10 Avg {recent10Average.toFixed(4)}</Badge>
          </div>
          <div className="grid min-h-0 gap-3 xl:grid-cols-2">
            <ScoreSection title="B30" scores={scores.b30} />
            <ScoreSection title="R10" scores={scores.r10} />
          </div>
        </div>
      )}
    </ActionCard>
  )
}

function ScoreImagesView({ isAdmin }: { isAdmin: boolean }) {
  const [form, setForm] = useState({ ...emptyUserSelectorForm })
  const [result, setResult] = useState<ScoreImages>()
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const value = await adminApi.scoreImages({
        ...(isAdmin ? buildUserSelectorPayload(form) : {}),
      })
      setResult(value)
      setAction({
        kind: 'success',
        message: `${value.user.name || value.user.userId} · ${value.images.length} 张`,
      })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard
      title="成绩图"
      description="生成 B30 / AP30 / Sex30"
      className="flex min-h-[calc(100svh-6.5rem)] flex-col"
      contentClassName="min-h-0 flex-1"
    >
      <form className="grid gap-3" onSubmit={onSubmit}>
        {isAdmin && (
          <UserSelectorFields
            value={form}
            onChange={(value) => setForm({ ...form, ...value })}
          />
        )}
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Images />}
            生成
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>

      {result && (
        <div className="grid min-h-0 flex-1 gap-4">
          <div className="text-sm font-medium">
            {result.user.name || '-'} · {result.user.userId} ·{' '}
            {result.user.userCode || '-'}
          </div>
          <div className="grid gap-4 xl:grid-cols-3">
            {result.images.map((image) => (
              <div
                key={image.mode}
                className="grid min-w-0 gap-2 rounded-md border bg-card p-3"
              >
                <div className="flex items-center justify-between gap-2">
                  <div>
                    <div className="text-sm font-medium">{image.title}</div>
                    <div className="text-xs text-muted-foreground">
                      {image.entryCount} 条
                    </div>
                  </div>
                  <Button asChild variant="outline" size="sm">
                    <a href={image.url} download={`${image.mode}.png`}>
                      下载
                    </a>
                  </Button>
                </div>
                <img
                  className="w-full rounded border bg-muted"
                  src={image.url}
                  alt={image.title}
                />
              </div>
            ))}
          </div>
        </div>
      )}
    </ActionCard>
  )
}

function ChartTopView() {
  const [form, setForm] = useState({ sid: '', difficulty: '2', limit: '50' })
  const [chartTop, setChartTop] = useState<AdminChartTop>()
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const sid = requireTrimmed(form.sid, 'song_id')
      const result = await adminApi.chartTop({
        sid,
        difficulty: parseDifficulty(form.difficulty, 2),
        limit: parseOptionalPositiveInt(form.limit, 'limit'),
      })
      setChartTop(result)
      setAction({
        kind: 'success',
        message: `${result.songId} · ${difficultyLabel(result.difficulty)} · ${result.scores.length} 条`,
      })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard
      title="单曲排行榜"
      description="singlecharttop"
      className="flex min-h-[calc(100svh-6.5rem)] flex-col"
      contentClassName="min-h-0 flex-1"
    >
      <form className="grid gap-3" onSubmit={onSubmit}>
        <div className="grid gap-3 sm:grid-cols-[1fr_140px_120px]">
          <Input
            value={form.sid}
            onChange={(event) => setForm({ ...form, sid: event.target.value })}
            placeholder="song_id / name"
            required
          />
          <DifficultySelect
            value={form.difficulty}
            onChange={(difficulty) => setForm({ ...form, difficulty })}
          />
          <Input
            value={form.limit}
            onChange={(event) => setForm({ ...form, limit: event.target.value })}
            placeholder="limit"
          />
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Search />}
            查询
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
      {chartTop && (
        <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-2">
          <div className="text-sm font-medium">
            {chartTop.nameEn || chartTop.songId} · {chartTop.songId} ·{' '}
            {difficultyLabel(chartTop.difficulty)}
          </div>
          <ScoreResultsTable scores={chartTop.scores} showUser />
        </div>
      )}
    </ActionCard>
  )
}

function UserTicketView() {
  const [form, setForm] = useState<UserTicketForm>(emptyUserTicketForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const payload: UserTicketPayload = {
        ...(form.allUsers ? {} : buildUserSelectorPayload(form)),
        ticket: parseRequiredInt(form.ticket, 'ticket'),
        all_users: form.allUsers,
      }
      const result = await adminApi.updateUserTicket(payload)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="记忆源点" description="changeuser">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <ToggleLabel
          checked={form.allUsers}
          onChange={(checked) => setForm({ ...form, allUsers: checked })}
          label="全部用户"
        />
        <UserSelectorFields
          value={form}
          disabled={form.allUsers}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="w-32"
            value={form.ticket}
            onChange={(event) => setForm({ ...form, ticket: event.target.value })}
            placeholder="ticket"
            required
          />
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Pencil />}
            更新
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function UserPasswordView() {
  const [form, setForm] = useState<UserPasswordForm>(emptyUserPasswordForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const result = await adminApi.resetUserPassword({
        ...buildUserSelectorPayload(form),
        password: form.password,
      })
      setForm(emptyUserPasswordForm)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="重置密码" description="changeuserpwd">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <UserSelectorFields
          value={form}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="w-56"
            value={form.password}
            type="password"
            autoComplete="new-password"
            onChange={(event) => setForm({ ...form, password: event.target.value })}
            placeholder="password"
            required
          />
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <KeyRound />}
            重置
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function UserBanView() {
  const [form, setForm] = useState<UserSelectorForm>(emptyUserSelectorForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      const payload = buildUserSelectorPayload(form)
      if (!confirm('封禁该用户?')) {
        return
      }
      setLoading(true)
      const result = await adminApi.banUser(payload)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="封禁用户" description="banuser">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <UserSelectorFields
          value={form}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" variant="destructive" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <ShieldAlert />}
            封禁
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function UserPurchaseView() {
  const [form, setForm] = useState<UserPurchaseForm>(emptyUserPurchaseForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      if (form.itemTypes.length <= 0) {
        throw new Error('至少选择一种 item type')
      }
      const payload: UserPurchasePayload = {
        ...(form.allUsers ? {} : buildUserSelectorPayload(form)),
        method: form.method,
        all_users: form.allUsers,
        item_types: form.itemTypes,
      }
      const verb = form.method === 'unlock' ? '解锁' : '锁定'
      if (!confirm(`${verb}${form.allUsers ? '全部用户' : '该用户'}购买内容?`)) {
        return
      }
      setLoading(true)
      const result = await adminApi.updateUserPurchase(payload)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="购买权限" description="changeuserpurchase">
      <form className="grid gap-4" onSubmit={onSubmit}>
        <div className="flex flex-wrap items-center gap-2">
          <select
            className="h-9 rounded-md border bg-background px-3 text-sm"
            value={form.method}
            onChange={(event) =>
              setForm({ ...form, method: event.target.value as 'unlock' | 'lock' })
            }
          >
            <option value="unlock">解锁</option>
            <option value="lock">锁定</option>
          </select>
          <ToggleLabel
            checked={form.allUsers}
            onChange={(checked) => setForm({ ...form, allUsers: checked })}
            label="全部用户"
          />
        </div>
        <UserSelectorFields
          value={form}
          disabled={form.allUsers}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="grid gap-2 sm:grid-cols-2">
          {purchaseItemTypeOptions.map((itemType) => (
            <ToggleLabel
              key={itemType}
              checked={form.itemTypes.includes(itemType)}
              onChange={(checked) =>
                setForm({
                  ...form,
                  itemTypes: checked
                    ? [...form.itemTypes, itemType]
                    : form.itemTypes.filter((value) => value !== itemType),
                })
              }
              label={itemType}
            />
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <ShoppingBag />}
            应用
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function ScoreDeleteView() {
  const [form, setForm] = useState<ScoreDeleteForm>(emptyScoreDeleteForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      const payload = buildScoreDeletePayload(form)
      if (!confirm('删除匹配成绩?')) {
        return
      }
      setLoading(true)
      const result = await adminApi.deleteScores(payload)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="成绩删除" description="changescore / deleteuserscore">
      <form className="grid gap-4" onSubmit={onSubmit}>
        <UserSelectorFields
          value={form}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="grid gap-3 sm:grid-cols-[1fr_140px]">
          <Input
            value={form.songId}
            onChange={(event) => setForm({ ...form, songId: event.target.value })}
            placeholder="song_id"
          />
          <DifficultySelect
            value={form.difficulty}
            includeAll
            onChange={(difficulty) => setForm({ ...form, difficulty })}
          />
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" variant="destructive" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Trash2 />}
            删除成绩
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function PresentCreateView() {
  const [form, setForm] = useState<PresentForm>(emptyPresentForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const payload: PresentPayload = {
        present_id: requireTrimmed(form.presentId, 'present_id'),
        expire_ts: requireTrimmed(form.expireTs, 'expire_ts'),
        description: form.description.trim(),
        item_id: requireTrimmed(form.itemId, 'item_id'),
        item_type: requireTrimmed(form.itemType, 'type'),
        amount: form.amount,
      }
      const result = await adminApi.createPresent(payload)
      setForm(emptyPresentForm)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="新增奖励" description="changepresent">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <div className="grid gap-3 lg:grid-cols-3">
          <Input
            value={form.presentId}
            onChange={(event) => setForm({ ...form, presentId: event.target.value })}
            placeholder="present_id"
            required
          />
          <Input
            type="datetime-local"
            value={form.expireTs}
            onChange={(event) => setForm({ ...form, expireTs: event.target.value })}
            required
          />
          <Input
            value={form.description}
            onChange={(event) => setForm({ ...form, description: event.target.value })}
            placeholder="description"
          />
          <Input
            value={form.itemId}
            onChange={(event) => setForm({ ...form, itemId: event.target.value })}
            placeholder="item_id"
            required
          />
          <Input
            value={form.itemType}
            onChange={(event) => setForm({ ...form, itemType: event.target.value })}
            placeholder="type"
            required
          />
          <Input
            value={form.amount}
            onChange={(event) => setForm({ ...form, amount: event.target.value })}
            placeholder="amount"
            required
          />
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Plus />}
            新增奖励
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function PresentDeliverView() {
  const [form, setForm] = useState<PresentDeliverForm>(emptyPresentDeliverForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      const payload: PresentDeliverPayload = {
        ...(form.allUsers ? {} : buildUserSelectorPayload(form)),
        present_id: requireTrimmed(form.presentId, 'present_id'),
        all_users: form.allUsers,
      }
      if (!confirm(`分发奖励 ${payload.present_id}?`)) {
        return
      }
      setLoading(true)
      const result = await adminApi.deliverPresent(payload)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="分发奖励" description="deliverpresent">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <ToggleLabel
          checked={form.allUsers}
          onChange={(checked) => setForm({ ...form, allUsers: checked })}
          label="全部用户"
        />
        <Input
          value={form.presentId}
          onChange={(event) => setForm({ ...form, presentId: event.target.value })}
          placeholder="present_id"
          required
        />
        <UserSelectorFields
          value={form}
          disabled={form.allUsers}
          onChange={(value) => setForm({ ...form, ...value })}
        />
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <PackagePlus />}
            分发
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function PresentDeleteView() {
  const [presentId, setPresentId] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      const value = requireTrimmed(presentId, 'present_id')
      if (!confirm(`删除奖励 ${value}?`)) {
        return
      }
      setLoading(true)
      const result = await adminApi.deletePresent(value)
      setPresentId('')
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="删除奖励" description="changepresent/deletepresent">
      <form className="flex flex-col gap-3 sm:flex-row" onSubmit={onSubmit}>
        <Input
          value={presentId}
          onChange={(event) => setPresentId(event.target.value)}
          placeholder="present_id"
          required
        />
        <Button type="submit" size="sm" variant="destructive" disabled={loading}>
          {loading ? <LoaderCircle className="animate-spin" /> : <Trash2 />}
          删除奖励
        </Button>
      </form>
      <ActionMessage action={action} />
    </ActionCard>
  )
}

function RedeemCreateView() {
  const [form, setForm] = useState<RedeemForm>(emptyRedeemForm)
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const payload: RedeemPayload = {
        code: form.code.trim() || undefined,
        random_amount: parseOptionalPositiveInt(form.randomAmount, 'random_amount'),
        redeem_type: parseRequiredInt(form.redeemType, 'redeem_type'),
        item_id: requireTrimmed(form.itemId, 'item_id'),
        item_type: requireTrimmed(form.itemType, 'type'),
        amount: form.amount,
      }
      const result = await adminApi.createRedeem(payload)
      setForm(emptyRedeemForm)
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="新增兑换码" description="changeredeem/addredeem">
      <form className="grid gap-3" onSubmit={onSubmit}>
        <div className="grid gap-3 lg:grid-cols-3">
          <Input
            value={form.code}
            onChange={(event) => setForm({ ...form, code: event.target.value })}
            placeholder="code"
          />
          <Input
            value={form.randomAmount}
            onChange={(event) => setForm({ ...form, randomAmount: event.target.value })}
            placeholder="random_amount"
          />
          <select
            className="h-9 rounded-md border bg-background px-3 text-sm"
            value={form.redeemType}
            onChange={(event) => setForm({ ...form, redeemType: event.target.value })}
          >
            <option value="0">全局一次</option>
            <option value="1">每用户一次</option>
          </select>
          <Input
            value={form.itemId}
            onChange={(event) => setForm({ ...form, itemId: event.target.value })}
            placeholder="item_id"
            required
          />
          <Input
            value={form.itemType}
            onChange={(event) => setForm({ ...form, itemType: event.target.value })}
            placeholder="type"
            required
          />
          <Input
            value={form.amount}
            onChange={(event) => setForm({ ...form, amount: event.target.value })}
            placeholder="amount"
            required
          />
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm" disabled={loading}>
            {loading ? <LoaderCircle className="animate-spin" /> : <Plus />}
            新增兑换码
          </Button>
          <ActionMessage action={action} />
        </div>
      </form>
    </ActionCard>
  )
}

function RedeemDeleteView() {
  const [code, setCode] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      const value = requireTrimmed(code, 'code')
      if (!confirm(`删除兑换码 ${value}?`)) {
        return
      }
      setLoading(true)
      const result = await adminApi.deleteRedeem(value)
      setCode('')
      setAction({ kind: 'success', message: formatActionResult(result) })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="删除兑换码" description="changeredeem/deleteredeem">
      <form className="flex flex-col gap-3 sm:flex-row" onSubmit={onSubmit}>
        <Input
          value={code}
          onChange={(event) => setCode(event.target.value)}
          placeholder="code"
          required
        />
        <Button type="submit" size="sm" variant="destructive" disabled={loading}>
          {loading ? <LoaderCircle className="animate-spin" /> : <Trash2 />}
          删除
        </Button>
      </form>
      <ActionMessage action={action} />
    </ActionCard>
  )
}

function RedeemUsersView() {
  const [code, setCode] = useState('')
  const [users, setUsers] = useState<AdminUserSummary[]>([])
  const [action, setAction] = useState<ActionState>(emptyAction)
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setAction(emptyAction)
    try {
      const result = await adminApi.redeemUsers(requireTrimmed(code, 'code'))
      setUsers(result.users)
      setAction({
        kind: 'success',
        message: `${result.code} · ${result.users.length} 个用户`,
      })
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setLoading(false)
    }
  }

  return (
    <ActionCard title="兑换使用者" description="redeem/<code>">
      <form className="flex flex-col gap-3 sm:flex-row" onSubmit={onSubmit}>
        <Input
          value={code}
          onChange={(event) => setCode(event.target.value)}
          placeholder="code"
          required
        />
        <Button type="submit" size="sm" variant="outline" disabled={loading}>
          {loading ? <LoaderCircle className="animate-spin" /> : <Search />}
          查询
        </Button>
      </form>
      <ActionMessage action={action} />
      {users.length > 0 && (
        <div className="max-h-64 overflow-auto rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>Name</TableHead>
                <TableHead>User Code</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {users.map((user) => (
                <TableRow key={user.userId}>
                  <TableCell className="font-mono">{user.userId}</TableCell>
                  <TableCell>{user.name || '-'}</TableCell>
                  <TableCell>{user.userCode || '-'}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </ActionCard>
  )
}

function DifficultySelect({
  value,
  onChange,
  includeAll = false,
}: {
  value: string
  onChange: (value: string) => void
  includeAll?: boolean
}) {
  return (
    <select
      className="h-9 rounded-md border bg-background px-3 text-sm"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    >
      {includeAll && <option value="-1">全部难度</option>}
      <option value="0">PST</option>
      <option value="1">PRS</option>
      <option value="2">FTR</option>
      <option value="3">BYD</option>
      <option value="4">ETR</option>
    </select>
  )
}

function SongsView({ isAdmin }: { isAdmin: boolean }) {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<SongRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [createForm, setCreateForm] = useState<SongPayload>(emptySongForm)
  const [editForm, setEditForm] = useState<SongPayload>(emptySongForm)
  const [editingSid, setEditingSid] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const pagination = useServerPagination(rows, defaultTablePageSize)
  const { setMeta } = pagination

  function load(showLoading = true, page = pagination.page, pageSize = pagination.pageSize) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .songs({ q: query, page, pageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function search() {
    load(true, 1, pagination.pageSize)
  }

  function edit(row: SongRow) {
    setEditingSid(row.songId)
    setAction(emptyAction)
    setEditForm({
      sid: row.songId,
      name_en: row.nameEn,
      rating_pst: row.ratingPst,
      rating_prs: row.ratingPrs,
      rating_ftr: row.ratingFtr,
      rating_byd: row.ratingByd,
      rating_etr: row.ratingEtr,
    })
  }

  function resetEdit(clearAction = true) {
    setEditingSid('')
    setEditForm(emptySongForm)
    if (clearAction) {
      setAction(emptyAction)
    }
  }

  async function submitCreate(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      await adminApi.createSong(createForm)
      setCreateForm(emptySongForm)
      setAction({ kind: 'success', message: '歌曲已新增' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function submitEdit(event: FormEvent) {
    event.preventDefault()
    if (!editingSid) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.updateSong(editingSid, editForm)
      resetEdit(false)
      setAction({ kind: 'success', message: '歌曲已更新' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function remove(row: SongRow) {
    if (!confirm(`删除歌曲 ${row.songId}?`)) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.deleteSong(row.songId)
      setAction({ kind: 'success', message: '歌曲已删除' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  useEffect(() => {
    adminApi
      .songs({ page: 1, pageSize: defaultTablePageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [setMeta])

  return (
    <DataPanel
      title="歌曲表"
      description="曲目名称和谱面定数"
      state={state}
      onSearch={search}
      searchValue={query}
      onSearchChange={setQuery}
    >
      {isAdmin && (
        <form className="mb-5 grid gap-3 rounded-md border p-3" onSubmit={submitCreate}>
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium">新增歌曲</div>
          </div>
          <div className="grid gap-3 lg:grid-cols-8">
            <Input
              value={createForm.sid}
              onChange={(event) => setCreateForm({ ...createForm, sid: event.target.value })}
              placeholder="song_id"
              required
            />
            <Input
              className="lg:col-span-2"
              value={createForm.name_en}
              onChange={(event) => setCreateForm({ ...createForm, name_en: event.target.value })}
              placeholder="name_en"
              required
            />
            {(['rating_pst', 'rating_prs', 'rating_ftr', 'rating_byd', 'rating_etr'] as const).map((field) => (
              <Input
                key={field}
                value={createForm[field]}
                onChange={(event) => setCreateForm({ ...createForm, [field]: event.target.value })}
                placeholder={field.replace('rating_', '').toUpperCase()}
                required
              />
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" size="sm">
              <Plus />
              新增歌曲
            </Button>
          </div>
        </form>
      )}
      {isAdmin && <ActionMessage action={action} className="mb-3 block" />}
      <TableBlock
        pagination={pagination}
        onPageChange={(page) => load(true, page, pagination.pageSize)}
        onPageSizeChange={(pageSize) => load(true, 1, pageSize)}
        emptyText="没有歌曲数据"
        renderTable={(visibleRows) => (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Song ID</TableHead>
                <TableHead>Name</TableHead>
                <TableHead>PST</TableHead>
                <TableHead>PRS</TableHead>
                <TableHead>FTR</TableHead>
                <TableHead>BYD</TableHead>
                <TableHead>ETR</TableHead>
                {isAdmin && <TableHead className="w-0 text-right">操作</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => (
                <Fragment key={row.songId}>
                  {isAdmin && editingSid === row.songId && (
                    <TableRow className="bg-muted/40 hover:bg-muted/40">
                      <TableCell colSpan={8} className="p-3">
                        <form className="grid gap-3 rounded-md border bg-background p-3" onSubmit={submitEdit}>
                          <div className="flex items-center justify-between gap-3">
                            <div className="text-sm font-medium">编辑歌曲 {editingSid}</div>
                            <Button type="button" size="sm" variant="ghost" onClick={() => resetEdit()}>
                              <X />
                              取消
                            </Button>
                          </div>
                          <div className="grid gap-3 lg:grid-cols-8">
                            <Input value={editForm.sid} disabled placeholder="song_id" />
                            <Input
                              className="lg:col-span-2"
                              value={editForm.name_en}
                              onChange={(event) => setEditForm({ ...editForm, name_en: event.target.value })}
                              placeholder="name_en"
                              required
                            />
                            {(['rating_pst', 'rating_prs', 'rating_ftr', 'rating_byd', 'rating_etr'] as const).map((field) => (
                              <Input
                                key={field}
                                value={editForm[field]}
                                onChange={(event) => setEditForm({ ...editForm, [field]: event.target.value })}
                                placeholder={field.replace('rating_', '').toUpperCase()}
                                required
                              />
                            ))}
                          </div>
                          <div className="flex flex-wrap items-center gap-2">
                            <Button type="submit" size="sm">
                              <Pencil />
                              保存修改
                            </Button>
                          </div>
                        </form>
                      </TableCell>
                    </TableRow>
                  )}
                  <TableRow data-state={editingSid === row.songId ? 'selected' : undefined}>
                    <TableCell className="font-mono">{row.songId}</TableCell>
                    <TableCell className="font-medium">{row.nameEn || '-'}</TableCell>
                    <TableCell>{row.ratingPst}</TableCell>
                    <TableCell>{row.ratingPrs}</TableCell>
                    <TableCell>{row.ratingFtr}</TableCell>
                    <TableCell>{row.ratingByd}</TableCell>
                    <TableCell>{row.ratingEtr}</TableCell>
                    {isAdmin && (
                      <TableCell className="w-0 whitespace-nowrap">
                        <div className="flex justify-end gap-2">
                          <Button type="button" size="sm" variant="outline" onClick={() => edit(row)}>
                            <Pencil />
                            编辑
                          </Button>
                          <Button type="button" size="sm" variant="destructive" onClick={() => remove(row)}>
                            <Trash2 />
                            删除
                          </Button>
                        </div>
                      </TableCell>
                    )}
                  </TableRow>
                </Fragment>
              ))}
            </TableBody>
          </Table>
        )}
      />
    </DataPanel>
  )
}

function ItemsView({ isAdmin }: { isAdmin: boolean }) {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<ItemRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [createForm, setCreateForm] = useState<ItemPayload>(emptyItemForm)
  const [editForm, setEditForm] = useState<ItemPayload>(emptyItemForm)
  const [editingKey, setEditingKey] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const pagination = useServerPagination(rows, defaultTablePageSize)
  const { setMeta } = pagination

  function load(showLoading = true, page = pagination.page, pageSize = pagination.pageSize) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .items({ q: query, page, pageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function search() {
    load(true, 1, pagination.pageSize)
  }

  function edit(row: ItemRow) {
    setEditingKey(`${row.itemId}:${row.itemType}`)
    setAction(emptyAction)
    setEditForm({
      item_id: row.itemId,
      item_type: row.itemType,
      is_available: row.isAvailable,
    })
  }

  function resetEdit(clearAction = true) {
    setEditingKey('')
    setEditForm(emptyItemForm)
    if (clearAction) {
      setAction(emptyAction)
    }
  }

  async function submitCreate(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      await adminApi.createItem(createForm)
      setCreateForm(emptyItemForm)
      setAction({ kind: 'success', message: '物品已新增' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function submitEdit(event: FormEvent) {
    event.preventDefault()
    if (!editingKey) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.updateItem(editForm)
      resetEdit(false)
      setAction({ kind: 'success', message: '物品已更新' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function remove(row: ItemRow) {
    if (!confirm(`删除物品 ${row.itemId}:${row.itemType}?`)) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.deleteItem(row.itemId, row.itemType)
      setAction({ kind: 'success', message: '物品已删除' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  useEffect(() => {
    adminApi
      .items({ page: 1, pageSize: defaultTablePageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [setMeta])

  return (
    <DataPanel
      title="物品表"
      description="物品类型和可用状态"
      state={state}
      onSearch={search}
      searchValue={query}
      onSearchChange={setQuery}
    >
      {isAdmin && (
        <form className="mb-5 grid gap-3 rounded-md border p-3" onSubmit={submitCreate}>
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium">新增物品</div>
          </div>
          <div className="grid gap-3 sm:grid-cols-[1fr_1fr_160px]">
            <Input
              value={createForm.item_id}
              onChange={(event) => setCreateForm({ ...createForm, item_id: event.target.value })}
              placeholder="item_id"
              required
            />
            <Input
              value={createForm.item_type}
              onChange={(event) => setCreateForm({ ...createForm, item_type: event.target.value })}
              placeholder="type"
              required
            />
            <select
              className="h-9 rounded-md border bg-background px-3 text-sm"
              value={createForm.is_available ?? 0}
              onChange={(event) =>
                setCreateForm({ ...createForm, is_available: Number(event.target.value) })
              }
            >
              <option value={1}>可用</option>
              <option value={0}>不可用</option>
            </select>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" size="sm">
              <Plus />
              新增物品
            </Button>
          </div>
        </form>
      )}
      {isAdmin && <ActionMessage action={action} className="mb-3 block" />}
      <TableBlock
        pagination={pagination}
        onPageChange={(page) => load(true, page, pagination.pageSize)}
        onPageSizeChange={(pageSize) => load(true, 1, pageSize)}
        emptyText="没有物品数据"
        renderTable={(visibleRows) => (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Item ID</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>可用</TableHead>
                {isAdmin && <TableHead className="w-0 text-right">操作</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => {
                const key = `${row.itemId}:${row.itemType}`
                return (
                  <Fragment key={key}>
                    {isAdmin && editingKey === key && (
                      <TableRow className="bg-muted/40 hover:bg-muted/40">
                        <TableCell colSpan={4} className="p-3">
                          <form className="grid gap-3 rounded-md border bg-background p-3" onSubmit={submitEdit}>
                            <div className="flex items-center justify-between gap-3">
                              <div className="text-sm font-medium">编辑物品 {editingKey}</div>
                              <Button type="button" size="sm" variant="ghost" onClick={() => resetEdit()}>
                                <X />
                                取消
                              </Button>
                            </div>
                            <div className="grid gap-3 sm:grid-cols-[1fr_1fr_160px]">
                              <Input value={editForm.item_id} disabled placeholder="item_id" />
                              <Input value={editForm.item_type} disabled placeholder="type" />
                              <select
                                className="h-9 rounded-md border bg-background px-3 text-sm"
                                value={editForm.is_available ?? 0}
                                onChange={(event) =>
                                  setEditForm({ ...editForm, is_available: Number(event.target.value) })
                                }
                              >
                                <option value={1}>可用</option>
                                <option value={0}>不可用</option>
                              </select>
                            </div>
                            <div className="flex flex-wrap items-center gap-2">
                              <Button type="submit" size="sm">
                                <Pencil />
                                保存修改
                              </Button>
                            </div>
                          </form>
                        </TableCell>
                      </TableRow>
                    )}
                    <TableRow data-state={editingKey === key ? 'selected' : undefined}>
                      <TableCell className="font-mono">{row.itemId}</TableCell>
                      <TableCell>{row.itemType}</TableCell>
                      <TableCell>
                        <Badge variant={row.isAvailable ? 'secondary' : 'outline'}>
                          {row.isAvailable ? 'Yes' : 'No'}
                        </Badge>
                      </TableCell>
                      {isAdmin && (
                        <TableCell className="w-0 whitespace-nowrap">
                          <div className="flex justify-end gap-2">
                            <Button type="button" size="sm" variant="outline" onClick={() => edit(row)}>
                              <Pencil />
                              编辑
                            </Button>
                            <Button type="button" size="sm" variant="destructive" onClick={() => remove(row)}>
                              <Trash2 />
                              删除
                            </Button>
                          </div>
                        </TableCell>
                      )}
                    </TableRow>
                  </Fragment>
                )
              })}
            </TableBody>
          </Table>
        )}
      />
    </DataPanel>
  )
}

function PurchasesView({ isAdmin }: { isAdmin: boolean }) {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<PurchaseRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [createForm, setCreateForm] = useState<PurchasePayload>(emptyPurchaseForm)
  const [editForm, setEditForm] = useState<PurchasePayload>(emptyPurchaseForm)
  const [editingPurchase, setEditingPurchase] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const pagination = useServerPagination(rows, defaultTablePageSize)
  const { setMeta } = pagination

  function load(showLoading = true, page = pagination.page, pageSize = pagination.pageSize) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .purchases({ pq: query, page, pageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function search() {
    load(true, 1, pagination.pageSize)
  }

  function editPurchase(row: PurchaseRow) {
    setEditingPurchase(row.purchaseName)
    setAction(emptyAction)
    setEditForm({
      purchase_name: row.purchaseName,
      price: row.price,
      orig_price: row.origPrice,
      discount_from: row.discountFrom,
      discount_to: row.discountTo,
      discount_reason: row.discountReason,
    })
  }

  function resetEdit(clearAction = true) {
    setEditingPurchase('')
    setEditForm(emptyPurchaseForm)
    if (clearAction) {
      setAction(emptyAction)
    }
  }

  async function submitCreate(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      await adminApi.createPurchase(createForm)
      setCreateForm(emptyPurchaseForm)
      setAction({ kind: 'success', message: '购买项已新增' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function submitEdit(event: FormEvent) {
    event.preventDefault()
    if (!editingPurchase) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.updatePurchase(editingPurchase, editForm)
      resetEdit(false)
      setAction({ kind: 'success', message: '购买项已更新' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function removePurchase(row: PurchaseRow) {
    if (!confirm(`删除购买项 ${row.purchaseName}?`)) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.deletePurchase(row.purchaseName)
      setAction({ kind: 'success', message: '购买项已删除' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  useEffect(() => {
    adminApi
      .purchases({ page: 1, pageSize: defaultTablePageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [setMeta])

  return (
    <DataPanel
      title="购买配置"
      description="购买项、价格和折扣配置"
      state={state}
      onSearch={search}
      searchValue={query}
      onSearchChange={setQuery}
    >
      <div className="grid gap-5">
        {isAdmin && (
          <form className="grid gap-3 rounded-md border p-3" onSubmit={submitCreate}>
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium">新增购买项</div>
            </div>
            <div className="grid gap-3 xl:grid-cols-6">
              <Input
                value={createForm.purchase_name}
                onChange={(event) =>
                  setCreateForm({ ...createForm, purchase_name: event.target.value })
                }
                placeholder="purchase_name"
                required
              />
              <Input
                value={createForm.price}
                onChange={(event) =>
                  setCreateForm({ ...createForm, price: event.target.value })
                }
                placeholder="price"
              />
              <Input
                value={createForm.orig_price}
                onChange={(event) =>
                  setCreateForm({ ...createForm, orig_price: event.target.value })
                }
                placeholder="orig_price"
              />
              <Input
                type="datetime-local"
                value={createForm.discount_from}
                onChange={(event) =>
                  setCreateForm({ ...createForm, discount_from: event.target.value })
                }
              />
              <Input
                type="datetime-local"
                value={createForm.discount_to}
                onChange={(event) =>
                  setCreateForm({ ...createForm, discount_to: event.target.value })
                }
              />
              <Input
                value={createForm.discount_reason}
                onChange={(event) =>
                  setCreateForm({ ...createForm, discount_reason: event.target.value })
                }
                placeholder="discount_reason"
              />
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button type="submit" size="sm">
                <Plus />
                新增购买项
              </Button>
            </div>
          </form>
        )}
        {isAdmin && <ActionMessage action={action} />}

        <TableBlock
          pagination={pagination}
          onPageChange={(page) => load(true, page, pagination.pageSize)}
          onPageSizeChange={(pageSize) => load(true, 1, pageSize)}
          emptyText="没有购买项数据"
          renderTable={(visibleRows) => (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Purchase</TableHead>
                  <TableHead>Price</TableHead>
                  <TableHead>Orig</TableHead>
                  <TableHead>Discount</TableHead>
                  <TableHead>Items</TableHead>
                  {isAdmin && <TableHead className="w-0 text-right">操作</TableHead>}
                </TableRow>
              </TableHeader>
              <TableBody>
                {visibleRows.map((row) => (
                  <Fragment key={row.purchaseName}>
                    {isAdmin && editingPurchase === row.purchaseName && (
                      <TableRow className="bg-muted/40 hover:bg-muted/40">
                        <TableCell colSpan={6} className="p-3">
                          <form className="grid gap-3 rounded-md border bg-background p-3" onSubmit={submitEdit}>
                            <div className="flex items-center justify-between gap-3">
                              <div className="text-sm font-medium">编辑购买项 {editingPurchase}</div>
                              <Button type="button" size="sm" variant="ghost" onClick={() => resetEdit()}>
                                <X />
                                取消
                              </Button>
                            </div>
                            <div className="grid gap-3 xl:grid-cols-6">
                              <Input value={editForm.purchase_name} disabled placeholder="purchase_name" />
                              <Input
                                value={editForm.price}
                                onChange={(event) => setEditForm({ ...editForm, price: event.target.value })}
                                placeholder="price"
                              />
                              <Input
                                value={editForm.orig_price}
                                onChange={(event) => setEditForm({ ...editForm, orig_price: event.target.value })}
                                placeholder="orig_price"
                              />
                              <Input
                                type="datetime-local"
                                value={editForm.discount_from}
                                onChange={(event) => setEditForm({ ...editForm, discount_from: event.target.value })}
                              />
                              <Input
                                type="datetime-local"
                                value={editForm.discount_to}
                                onChange={(event) => setEditForm({ ...editForm, discount_to: event.target.value })}
                              />
                              <Input
                                value={editForm.discount_reason}
                                onChange={(event) => setEditForm({ ...editForm, discount_reason: event.target.value })}
                                placeholder="discount_reason"
                              />
                            </div>
                            <div className="flex flex-wrap items-center gap-2">
                              <Button type="submit" size="sm">
                                <Pencil />
                                保存修改
                              </Button>
                            </div>
                          </form>
                        </TableCell>
                      </TableRow>
                    )}
                    <TableRow data-state={editingPurchase === row.purchaseName ? 'selected' : undefined}>
                      <TableCell className="font-mono">{row.purchaseName}</TableCell>
                      <TableCell>{row.price || '-'}</TableCell>
                      <TableCell>{row.origPrice || '-'}</TableCell>
                      <TableCell className="min-w-52">
                        {row.discountFrom || '-'} / {row.discountTo || '-'}
                      </TableCell>
                      <TableCell className="max-w-xl truncate" title={row.itemSummary}>
                        {row.itemSummary}
                      </TableCell>
                      {isAdmin && (
                        <TableCell className="w-0 whitespace-nowrap">
                          <div className="flex justify-end gap-2">
                            <Button type="button" size="sm" variant="outline" onClick={() => editPurchase(row)}>
                              <Pencil />
                              编辑
                            </Button>
                            <Button type="button" size="sm" variant="destructive" onClick={() => removePurchase(row)}>
                              <Trash2 />
                              删除
                            </Button>
                          </div>
                        </TableCell>
                      )}
                    </TableRow>
                  </Fragment>
                ))}
              </TableBody>
            </Table>
          )}
        />
      </div>
    </DataPanel>
  )
}

function PurchaseItemsView() {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<PurchaseItemRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [createForm, setCreateForm] =
    useState<PurchaseItemPayload>(emptyPurchaseItemForm)
  const [editForm, setEditForm] =
    useState<PurchaseItemPayload>(emptyPurchaseItemForm)
  const [editingPurchaseItem, setEditingPurchaseItem] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)
  const pagination = useServerPagination(rows, defaultTablePageSize)
  const { setMeta } = pagination

  function load(showLoading = true, page = pagination.page, pageSize = pagination.pageSize) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .purchaseItems({ iq: query, page, pageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function search() {
    load(true, 1, pagination.pageSize)
  }

  function editPurchaseItem(row: PurchaseItemRow) {
    setEditingPurchaseItem(`${row.purchaseName}:${row.itemId}:${row.itemType}`)
    setAction(emptyAction)
    setEditForm({
      purchase_name: row.purchaseName,
      item_id: row.itemId,
      item_type: row.itemType,
      amount: row.amount,
    })
  }

  function resetEdit(clearAction = true) {
    setEditingPurchaseItem('')
    setEditForm(emptyPurchaseItemForm)
    if (clearAction) {
      setAction(emptyAction)
    }
  }

  async function submitCreate(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      await adminApi.createPurchaseItem(createForm)
      setCreateForm(emptyPurchaseItemForm)
      setAction({ kind: 'success', message: '购买物品已新增' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function submitEdit(event: FormEvent) {
    event.preventDefault()
    if (!editingPurchaseItem) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.updatePurchaseItem(editForm)
      resetEdit(false)
      setAction({ kind: 'success', message: '购买物品已更新' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function removePurchaseItem(row: PurchaseItemRow) {
    if (!confirm(`删除购买物品 ${row.purchaseName}:${row.itemId}:${row.itemType}?`)) {
      return
    }
    setAction(emptyAction)
    try {
      await adminApi.deletePurchaseItem(row.purchaseName, row.itemId, row.itemType)
      setAction({ kind: 'success', message: '购买物品已删除' })
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  useEffect(() => {
    adminApi
      .purchaseItems({ page: 1, pageSize: defaultTablePageSize })
      .then((value) => {
        setRows(value.rows)
        setMeta(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [setMeta])

  return (
    <DataPanel
      title="购买物品"
      description="购买项和物品的关联关系"
      state={state}
      onSearch={search}
      searchValue={query}
      onSearchChange={setQuery}
    >
      <div className="grid gap-5">
        <form className="grid gap-3 rounded-md border p-3" onSubmit={submitCreate}>
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm font-medium">新增购买物品</div>
          </div>
          <div className="grid gap-3 lg:grid-cols-[1fr_1fr_1fr_120px]">
            <Input
              value={createForm.purchase_name}
              onChange={(event) =>
                setCreateForm({
                  ...createForm,
                  purchase_name: event.target.value,
                })
              }
              placeholder="purchase_name"
              required
            />
            <Input
              value={createForm.item_id}
              onChange={(event) =>
                setCreateForm({ ...createForm, item_id: event.target.value })
              }
              placeholder="item_id"
              required
            />
            <Input
              value={createForm.item_type}
              onChange={(event) =>
                setCreateForm({ ...createForm, item_type: event.target.value })
              }
              placeholder="type"
              required
            />
            <Input
              value={createForm.amount}
              onChange={(event) =>
                setCreateForm({ ...createForm, amount: event.target.value })
              }
              placeholder="amount"
              required
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" size="sm">
              <PackagePlus />
              新增购买物品
            </Button>
          </div>
        </form>
        <ActionMessage action={action} />

        <TableBlock
          pagination={pagination}
          onPageChange={(page) => load(true, page, pagination.pageSize)}
          onPageSizeChange={(pageSize) => load(true, 1, pageSize)}
          emptyText="没有购买物品数据"
          renderTable={(visibleRows) => (
            <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Purchase</TableHead>
                <TableHead>Item</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Amount</TableHead>
                <TableHead className="w-0 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => {
                const key = `${row.purchaseName}:${row.itemId}:${row.itemType}`
                return (
                  <Fragment key={key}>
                    {editingPurchaseItem === key && (
                      <TableRow className="bg-muted/40 hover:bg-muted/40">
                        <TableCell colSpan={5} className="p-3">
                          <form className="grid gap-3 rounded-md border bg-background p-3" onSubmit={submitEdit}>
                            <div className="flex items-center justify-between gap-3">
                              <div className="text-sm font-medium">编辑购买物品 {editingPurchaseItem}</div>
                              <Button type="button" size="sm" variant="ghost" onClick={() => resetEdit()}>
                                <X />
                                取消
                              </Button>
                            </div>
                            <div className="grid gap-3 lg:grid-cols-[1fr_1fr_1fr_120px]">
                              <Input value={editForm.purchase_name} disabled placeholder="purchase_name" />
                              <Input value={editForm.item_id} disabled placeholder="item_id" />
                              <Input value={editForm.item_type} disabled placeholder="type" />
                              <Input
                                value={editForm.amount}
                                onChange={(event) => setEditForm({ ...editForm, amount: event.target.value })}
                                placeholder="amount"
                                required
                              />
                            </div>
                            <div className="flex flex-wrap items-center gap-2">
                              <Button type="submit" size="sm">
                                <Pencil />
                                保存修改
                              </Button>
                            </div>
                          </form>
                        </TableCell>
                      </TableRow>
                    )}
                    <TableRow data-state={editingPurchaseItem === key ? 'selected' : undefined}>
                      <TableCell className="font-mono">{row.purchaseName}</TableCell>
                      <TableCell>{row.itemId}</TableCell>
                      <TableCell>{row.itemType}</TableCell>
                      <TableCell>{row.amount}</TableCell>
                      <TableCell className="w-0 whitespace-nowrap">
                        <div className="flex justify-end gap-2">
                          <Button type="button" size="sm" variant="outline" onClick={() => editPurchaseItem(row)}>
                            <Pencil />
                            编辑
                          </Button>
                          <Button type="button" size="sm" variant="destructive" onClick={() => removePurchaseItem(row)}>
                            <Trash2 />
                            删除
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  </Fragment>
                )
              })}
            </TableBody>
          </Table>
          )}
        />
      </div>
    </DataPanel>
  )
}

function DataPanel({
  title,
  description,
  state,
  searchValue,
  onSearchChange,
  onSearch,
  extraControl,
  children,
}: {
  title: string
  description: string
  state: LoadState
  searchValue: string
  onSearchChange: (value: string) => void
  onSearch: () => void
  extraControl?: ReactNode
  children: ReactNode
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <div>
            <CardTitle>{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </div>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <div className="relative">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                className="w-full pl-9 sm:w-72"
                value={searchValue}
                onChange={(event) => onSearchChange(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    onSearch()
                  }
                }}
                placeholder="搜索"
              />
            </div>
            {extraControl}
            <Button type="button" variant="outline" onClick={onSearch}>
              {state === 'loading' ? (
                <LoaderCircle className="animate-spin" />
              ) : (
                <RefreshCcw />
              )}
              刷新
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {state === 'error' ? (
          <LoadPanel state={state} onRetry={onSearch} />
        ) : (
          children
        )}
      </CardContent>
    </Card>
  )
}

type PaginationState<T> = {
  page: number
  pageCount: number
  pageSize: number
  rows: T[]
  total: number
  start: number
  end: number
  canPrevious: boolean
  canNext: boolean
  setMeta: (data: PageData<T>) => void
}

function useServerPagination<T>(rows: T[], initialPageSize = 25): PaginationState<T> {
  const [page, setPageState] = useState(1)
  const [pageSize, setPageSizeState] = useState(initialPageSize)
  const [total, setTotal] = useState(0)
  const pageCount = Math.max(1, Math.ceil(total / pageSize))
  const start = total === 0 ? 0 : (page - 1) * pageSize + 1
  const end = Math.min((page - 1) * pageSize + rows.length, total)

  const setMeta = useCallback((data: PageData<T>) => {
    setPageState(data.page)
    setPageSizeState(data.pageSize)
    setTotal(data.total)
  }, [])

  return {
    page,
    pageCount,
    pageSize,
    rows,
    total,
    start,
    end,
    canPrevious: page > 1,
    canNext: page < pageCount,
    setMeta,
  }
}

function TableBlock<T>({
  pagination,
  onPageChange,
  onPageSizeChange,
  emptyText,
  renderTable,
}: {
  pagination: PaginationState<T>
  onPageChange: (page: number) => void
  onPageSizeChange: (pageSize: number) => void
  emptyText: string
  renderTable: (rows: T[]) => ReactNode
}) {
  if (pagination.total === 0) {
    return (
      <div className="flex min-h-32 items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
        {emptyText}
      </div>
    )
  }

  return (
    <div className="grid gap-3">
      {renderTable(pagination.rows)}
      <PaginationControls
        pagination={pagination}
        onPageChange={onPageChange}
        onPageSizeChange={onPageSizeChange}
      />
    </div>
  )
}

function UserSelectorFields({
  value,
  onChange,
  disabled = false,
}: {
  value: UserSelectorForm
  onChange: (value: Partial<UserSelectorForm>) => void
  disabled?: boolean
}) {
  return (
    <div className="grid gap-3 lg:grid-cols-3">
      <Input
        value={value.userId}
        disabled={disabled}
        onChange={(event) => onChange({ userId: event.target.value })}
        placeholder="user_id"
      />
      <Input
        value={value.name}
        disabled={disabled}
        onChange={(event) => onChange({ name: event.target.value })}
        placeholder="name"
      />
      <Input
        value={value.userCode}
        disabled={disabled}
        onChange={(event) => onChange({ userCode: event.target.value })}
        placeholder="user_code"
      />
    </div>
  )
}

function ToggleLabel({
  checked,
  onChange,
  label,
}: {
  checked: boolean
  onChange: (checked: boolean) => void
  label: string
}) {
  return (
    <label className="inline-flex h-9 items-center gap-2 rounded-md border px-3 text-sm">
      <input
        className="size-4 accent-primary"
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  )
}

function ScoreSection({
  title,
  scores,
}: {
  title: string
  scores: AdminScoreRow[]
}) {
  const totalRating = scores.reduce((sum, score) => sum + score.rating, 0)

  return (
    <div className="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] gap-2">
      <div className="flex items-center justify-between gap-2">
        <div className="text-sm font-medium">{title}</div>
        <div className="font-mono text-xs text-muted-foreground">
          {scores.length} · {totalRating.toFixed(4)}
        </div>
      </div>
      <ScoreResultsTable scores={scores} />
    </div>
  )
}

function ScoreResultsTable({
  scores,
  showUser = false,
}: {
  scores: AdminScoreRow[]
  showUser?: boolean
}) {
  if (scores.length === 0) {
    return (
      <div className="flex h-full min-h-28 items-center justify-center rounded-md border border-dashed text-sm text-muted-foreground">
        无成绩
      </div>
    )
  }

  return (
    <div className="min-h-0 overflow-x-hidden overflow-y-auto rounded-md border">
      <Table
        className={cn(
          'table-fixed leading-tight',
          showUser ? 'min-w-[820px] text-[11px]' : 'w-full text-xs',
        )}
      >
        <colgroup>
          {showUser && <col className="w-[14%]" />}
          <col className={showUser ? 'w-[19%]' : 'w-[24%]'} />
          <col className={showUser ? 'w-[7%]' : 'w-[8%]'} />
          <col className={showUser ? 'w-[12%]' : 'w-[15%]'} />
          <col className={showUser ? 'w-[14%]' : 'w-[18%]'} />
          <col className={showUser ? 'w-[7%]' : 'w-[8%]'} />
          <col className={showUser ? 'w-[10%]' : 'w-[13%]'} />
          <col className={showUser ? 'w-[17%]' : 'w-[14%]'} />
        </colgroup>
        <TableHeader>
          <TableRow>
            {showUser && <TableHead className="px-1.5">User</TableHead>}
            <TableHead className="px-1.5">Song</TableHead>
            <TableHead className="px-1.5">Diff</TableHead>
            <TableHead className="px-1.5">Score</TableHead>
            <TableHead className="px-1.5">BP/LP/F/L</TableHead>
            <TableHead className="px-1.5">Clear</TableHead>
            <TableHead className="px-1.5">Rating</TableHead>
            <TableHead className="px-1.5">Time</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {scores.map((score) => (
            <TableRow
              key={`${score.userId}:${score.songId}:${score.difficulty}:${score.timePlayed}`}
            >
              {showUser && (
                <TableCell className="px-1.5 py-2">
                  <div className="font-medium">{score.name || '-'}</div>
                  <div className="font-mono text-xs text-muted-foreground">
                    {score.userId}
                  </div>
              </TableCell>
              )}
              <TableCell className="min-w-0 px-1.5 py-2 font-mono">
                <span className="score-song-id" title={score.songId}>
                  {score.songId}
                </span>
              </TableCell>
              <TableCell className="whitespace-nowrap px-1.5 py-2">
                {difficultyLabel(score.difficulty)}
              </TableCell>
              <TableCell className="whitespace-nowrap px-1.5 py-2 font-mono">
                {score.score.toLocaleString()}
              </TableCell>
              <TableCell className="whitespace-nowrap px-1.5 py-2 font-mono">
                {score.shinyPerfectCount}/
                {Math.max(score.perfectCount - score.shinyPerfectCount, 0)}/
                {score.nearCount}/{score.missCount}
              </TableCell>
              <TableCell className="whitespace-nowrap px-1.5 py-2">
                {score.clearType}/{score.bestClearType}
              </TableCell>
              <TableCell className="whitespace-nowrap px-1.5 py-2 font-mono">
                {score.rating.toFixed(4)}
              </TableCell>
              <TableCell className="break-all px-1.5 py-2 font-mono">
                {score.timePlayed}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function PaginationControls<T>({
  pagination,
  onPageChange,
  onPageSizeChange,
}: {
  pagination: PaginationState<T>
  onPageChange: (page: number) => void
  onPageSizeChange: (pageSize: number) => void
}) {
  return (
    <div className="flex flex-col gap-3 border-t pt-3 text-sm text-muted-foreground md:flex-row md:items-center md:justify-between">
      <div>
        显示 {pagination.start}-{pagination.end} / {pagination.total}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <select
          className="h-8 rounded-md border bg-background px-2 text-sm text-foreground"
          value={pagination.pageSize}
          onChange={(event) => onPageSizeChange(Number(event.target.value))}
        >
          {pageSizeOptions.map((size) => (
            <option key={size} value={size}>
              {size} / 页
            </option>
          ))}
        </select>
        <div className="text-foreground">
          第 {pagination.page} / {pagination.pageCount} 页
        </div>
        <div className="flex gap-1">
          <Button
            type="button"
            size="icon"
            variant="outline"
            disabled={!pagination.canPrevious}
            onClick={() => onPageChange(1)}
            title="第一页"
          >
            <ChevronsLeft />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="outline"
            disabled={!pagination.canPrevious}
            onClick={() => onPageChange(pagination.page - 1)}
            title="上一页"
          >
            <ChevronLeft />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="outline"
            disabled={!pagination.canNext}
            onClick={() => onPageChange(pagination.page + 1)}
            title="下一页"
          >
            <ChevronRight />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="outline"
            disabled={!pagination.canNext}
            onClick={() => onPageChange(pagination.pageCount)}
            title="最后一页"
          >
            <ChevronsRight />
          </Button>
        </div>
      </div>
    </div>
  )
}

function LoadPanel({
  state,
  onRetry,
}: {
  state: LoadState
  onRetry: () => void
}) {
  if (state === 'error') {
    return (
      <div className="flex min-h-44 flex-col items-center justify-center gap-3 rounded-md border border-dashed text-sm text-muted-foreground">
        数据加载失败
        <Button type="button" variant="outline" size="sm" onClick={onRetry}>
          重试
        </Button>
      </div>
    )
  }

  return (
    <div className="flex min-h-44 items-center justify-center rounded-md border border-dashed text-muted-foreground">
      <LoaderCircle className="size-5 animate-spin" />
    </div>
  )
}

function ActionMessage({
  action,
  className,
}: {
  action: ActionState
  className?: string
}) {
  if (action.kind === 'idle') {
    return null
  }

  return (
    <span
      className={cn(
        'text-sm',
        action.kind === 'success' ? 'text-emerald-700' : 'text-destructive',
        className,
      )}
    >
      {action.message}
    </span>
  )
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '操作失败'
}

function buildUserSelectorPayload(form: UserSelectorForm): UserSelectorPayload {
  const payload: UserSelectorPayload = {}
  const userId = form.userId.trim()
  if (userId) {
    payload.user_id = parseRequiredInt(userId, 'user_id')
  }
  const name = form.name.trim()
  if (name) {
    payload.name = name
  }
  const userCode = form.userCode.trim()
  if (userCode) {
    payload.user_code = userCode
  }
  if (!payload.user_id && !payload.name && !payload.user_code) {
    throw new Error('需要 user_id、name 或 user_code')
  }
  return payload
}

function requireTrimmed(value: string, label: string) {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new Error(`${label} 不能为空`)
  }
  return trimmed
}

function buildScoreDeletePayload(form: ScoreDeleteForm): ScoreDeletePayload {
  const payload: ScoreDeletePayload = {}
  const selector = buildUserSelectorPayloadAllowEmpty(form)
  Object.assign(payload, selector)
  const songId = form.songId.trim()
  if (songId) {
    payload.song_id = songId
  }
  if (form.difficulty !== '-1') {
    payload.difficulty = parseDifficulty(form.difficulty, -1)
  }
  if (!payload.user_id && !payload.name && !payload.user_code && !payload.song_id && payload.difficulty === undefined) {
    throw new Error('至少提供一个删除条件')
  }
  return payload
}

function buildUserSelectorPayloadAllowEmpty(
  form: UserSelectorForm,
): UserSelectorPayload {
  const payload: UserSelectorPayload = {}
  const userId = form.userId.trim()
  if (userId) {
    payload.user_id = parseRequiredInt(userId, 'user_id')
  }
  const name = form.name.trim()
  if (name) {
    payload.name = name
  }
  const userCode = form.userCode.trim()
  if (userCode) {
    payload.user_code = userCode
  }
  return payload
}

function parseRequiredInt(value: string, label: string) {
  const parsed = Number.parseInt(value.trim(), 10)
  if (!Number.isFinite(parsed)) {
    throw new Error(`${label} 必须是整数`)
  }
  return parsed
}

function parseOptionalPositiveInt(value: string, label: string) {
  const trimmed = value.trim()
  if (!trimmed) {
    return undefined
  }
  const parsed = parseRequiredInt(trimmed, label)
  if (parsed <= 0) {
    throw new Error(`${label} 必须大于 0`)
  }
  return parsed
}

function parseDifficulty(value: string, fallback: number) {
  const parsed = Number.parseInt(value.trim(), 10)
  if (!Number.isFinite(parsed)) {
    return fallback
  }
  return Math.min(4, Math.max(0, parsed))
}

function difficultyLabel(difficulty: number) {
  return ['PST', 'PRS', 'FTR', 'BYD', 'ETR'][difficulty] ?? String(difficulty)
}

function formatActionResult(result: AdminActionResult) {
  return `${result.message} · ${result.affectedRows} 行`
}

function isMaintenanceView(view: View): view is MaintenanceView {
  return view in maintenanceOperations
}

function MetricCard({
  label,
  value,
  sub,
  icon: Icon,
}: {
  label: string
  value: number
  sub: string
  icon: typeof Activity
}) {
  const formatted = useMemo(() => value.toLocaleString(), [value])

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {label}
        </CardTitle>
        <Icon className="size-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-semibold">{formatted}</div>
        <p className="mt-1 text-xs text-muted-foreground">{sub}</p>
      </CardContent>
    </Card>
  )
}

function viewTitle(view: View) {
  if (isMaintenanceView(view)) {
    return maintenanceOperations[view].title
  }

  switch (view) {
    case 'dashboard':
      return '总览'
    case 'playerScores':
      return '玩家成绩'
    case 'scoreImages':
      return '成绩图'
    case 'chartTop':
      return '单曲排行榜'
    case 'userTicket':
      return '记忆源点'
    case 'userPassword':
      return '重置密码'
    case 'userBan':
      return '封禁用户'
    case 'userPurchase':
      return '购买权限'
    case 'scoreDelete':
      return '删除成绩'
    case 'presentCreate':
      return '新增奖励'
    case 'presentDeliver':
      return '分发奖励'
    case 'presentDelete':
      return '删除奖励'
    case 'redeemCreate':
      return '新增兑换码'
    case 'redeemDelete':
      return '删除兑换码'
    case 'redeemUsers':
      return '兑换使用者'
    case 'users':
      return '玩家管理'
    case 'songs':
      return '歌曲表'
    case 'items':
      return '物品表'
    case 'purchases':
      return '购买项'
    case 'purchaseItems':
      return '购买物品配置'
  }
}

function viewSubtitle(view: View) {
  if (isMaintenanceView(view)) {
    return maintenanceOperations[view].description
  }

  switch (view) {
    case 'dashboard':
      return '服务状态与运营数据'
    case 'playerScores':
      return '查询单个玩家的成绩记录'
    case 'scoreImages':
      return '生成 B30 / AP30 / Sex30 成绩图'
    case 'chartTop':
      return '查询单曲指定难度排行榜'
    case 'userTicket':
      return '更新玩家记忆源点'
    case 'userPassword':
      return '重置玩家登录密码'
    case 'userBan':
      return '封禁指定玩家账号'
    case 'userPurchase':
      return '调整玩家购买权限'
    case 'scoreDelete':
      return '按条件删除成绩记录'
    case 'presentCreate':
      return '创建一个奖励定义'
    case 'presentDeliver':
      return '向玩家分发已有奖励'
    case 'presentDelete':
      return '删除奖励定义'
    case 'redeemCreate':
      return '创建兑换码'
    case 'redeemDelete':
      return '删除兑换码'
    case 'redeemUsers':
      return '查询兑换码使用者'
    case 'users':
      return '账号状态、票券和最近游玩记录'
    case 'songs':
      return '曲目名称和谱面定数'
    case 'items':
      return '物品类型和可用状态'
    case 'purchases':
      return '购买项、价格和折扣配置'
    case 'purchaseItems':
      return '购买项和物品的关联关系'
  }
}

export default App
