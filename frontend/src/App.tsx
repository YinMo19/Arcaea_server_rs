import { Fragment, useCallback, useEffect, useMemo, useState } from 'react'
import type { FormEvent } from 'react'
import {
  Activity,
  Boxes,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  ChartSpline,
  Database,
  KeyRound,
  Link2,
  LoaderCircle,
  LogOut,
  Music2,
  PackagePlus,
  Pencil,
  Plus,
  RefreshCcw,
  Search,
  ShieldAlert,
  ShoppingBag,
  Trash2,
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
  type AdminOperation,
  type DashboardData,
  type ItemPayload,
  type ItemRow,
  type PageData,
  type PurchaseItemPayload,
  type PurchaseItemRow,
  type PurchasePayload,
  type PurchaseRow,
  type SongPayload,
  type SongRow,
  type UserRow,
} from '@/lib/api'
import { cn } from '@/lib/utils'

type View = 'dashboard' | 'users' | 'songs' | 'items' | 'purchases' | 'purchaseItems'

const navItems: Array<{
  id: View
  label: string
  icon: typeof Activity
}> = [
  { id: 'dashboard', label: '总览', icon: Activity },
  { id: 'users', label: '玩家', icon: Users },
  { id: 'songs', label: '歌曲', icon: Music2 },
  { id: 'items', label: '物品', icon: Boxes },
  { id: 'purchases', label: '购买项', icon: ShoppingBag },
  { id: 'purchaseItems', label: '购买物品', icon: Link2 },
]

const adminOperations: Array<{
  id: AdminOperation
  label: string
}> = [
  { id: 'refresh_song_file_cache', label: '刷新 Song Hash' },
  { id: 'refresh_content_bundle_cache', label: '刷新 Bundle' },
  { id: 'refresh_all_score_rating', label: '重算 Rating' },
]

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

function App() {
  const [loggedIn, setLoggedIn] = useState(false)
  const [checkingSession, setCheckingSession] = useState(true)
  const [view, setView] = useState<View>('dashboard')

  useEffect(() => {
    adminApi
      .session()
      .then((session) => setLoggedIn(session.loggedIn))
      .catch(() => setLoggedIn(false))
      .finally(() => setCheckingSession(false))
  }, [])

  if (checkingSession) {
    return (
      <div className="flex min-h-svh items-center justify-center bg-background">
        <LoaderCircle className="size-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!loggedIn) {
    return <LoginScreen onLoggedIn={() => setLoggedIn(true)} />
  }

  return (
    <div className="min-h-svh bg-background text-foreground">
      <aside className="fixed inset-y-0 left-0 hidden w-64 border-r bg-sidebar px-4 py-5 lg:block">
        <div className="flex items-center gap-3 px-2">
          <div className="flex size-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Database className="size-5" />
          </div>
          <div>
            <div className="text-sm font-semibold">Arcaea Admin</div>
            <div className="text-xs text-muted-foreground">Operations</div>
          </div>
        </div>

        <nav className="mt-8 grid gap-1">
          {navItems.map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setView(item.id)}
              className={cn(
                'flex h-10 items-center gap-3 rounded-md px-3 text-left text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground',
                view === item.id && 'bg-accent text-accent-foreground',
              )}
            >
              <item.icon className="size-4" />
              {item.label}
            </button>
          ))}
        </nav>
      </aside>

      <div className="lg:pl-64">
        <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
          <div className="flex min-h-16 items-center justify-between gap-4 px-4 sm:px-6">
            <div>
              <h1 className="text-lg font-semibold">{viewTitle(view)}</h1>
              <p className="text-sm text-muted-foreground">
                服务状态与运营数据
              </p>
            </div>
            <div className="flex items-center gap-2">
              <div className="hidden gap-1 sm:flex lg:hidden">
                {navItems.map((item) => (
                  <Button
                    key={item.id}
                    type="button"
                    size="icon"
                    variant={view === item.id ? 'secondary' : 'ghost'}
                    onClick={() => setView(item.id)}
                    title={item.label}
                  >
                    <item.icon />
                  </Button>
                ))}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  adminApi.logout().finally(() => setLoggedIn(false))
                }}
              >
                <LogOut />
                登出
              </Button>
            </div>
          </div>
        </header>

        <main className="px-4 py-5 sm:px-6">
          {view === 'dashboard' && <DashboardView />}
          {view === 'users' && <UsersView />}
          {view === 'songs' && <SongsView />}
          {view === 'items' && <ItemsView />}
          {view === 'purchases' && <PurchasesView />}
          {view === 'purchaseItems' && <PurchaseItemsView />}
        </main>
      </div>
    </div>
  )
}

function LoginScreen({ onLoggedIn }: { onLoggedIn: () => void }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setError('')
    try {
      await adminApi.login(username, password)
      onLoggedIn()
    } catch {
      setError('用户名或密码错误')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-background px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <div className="mb-2 flex size-10 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <KeyRound className="size-5" />
          </div>
          <CardTitle>管理员登录</CardTitle>
          <CardDescription>进入管理后台</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="grid gap-4" onSubmit={onSubmit}>
            <label className="grid gap-1.5 text-sm font-medium">
              Username
              <Input
                value={username}
                autoComplete="username"
                onChange={(event) => setUsername(event.target.value)}
                required
              />
            </label>
            <label className="grid gap-1.5 text-sm font-medium">
              Password
              <Input
                value={password}
                type="password"
                autoComplete="current-password"
                onChange={(event) => setPassword(event.target.value)}
                required
              />
            </label>
            {error && (
              <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                {error}
              </div>
            )}
            <Button type="submit" disabled={loading}>
              {loading && <LoaderCircle className="animate-spin" />}
              登录
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}

function DashboardView() {
  const [data, setData] = useState<DashboardData>()
  const [state, setState] = useState<LoadState>('loading')
  const [operationAction, setOperationAction] = useState<ActionState>(emptyAction)
  const [operationBusy, setOperationBusy] = useState<AdminOperation | ''>('')

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

  async function runOperation(operation: AdminOperation) {
    setOperationBusy(operation)
    setOperationAction(emptyAction)
    try {
      await adminApi.operation(operation)
      setOperationAction({ kind: 'success', message: '操作已完成' })
      load(false)
    } catch (error) {
      setOperationAction({ kind: 'error', message: errorMessage(error) })
    } finally {
      setOperationBusy('')
    }
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
        <CardHeader>
          <CardTitle>维护操作</CardTitle>
          <CardDescription>资源缓存与成绩 Rating 维护</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-center gap-2">
            {adminOperations.map((operation) => (
              <Button
                key={operation.id}
                type="button"
                variant="outline"
                size="sm"
                disabled={Boolean(operationBusy)}
                onClick={() => runOperation(operation.id)}
              >
                {operationBusy === operation.id ? (
                  <LoaderCircle className="animate-spin" />
                ) : (
                  <RefreshCcw />
                )}
                {operation.label}
              </Button>
            ))}
            <ActionMessage action={operationAction} />
          </div>
        </CardContent>
      </Card>

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

function SongsView() {
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
      <ActionMessage action={action} className="mb-3 block" />
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
                <TableHead>操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => (
                <Fragment key={row.songId}>
                  {editingSid === row.songId && (
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
                    <TableCell>
                      <div className="flex gap-2">
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

function ItemsView() {
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
      <ActionMessage action={action} className="mb-3 block" />
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
                <TableHead>操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visibleRows.map((row) => {
                const key = `${row.itemId}:${row.itemType}`
                return (
                  <Fragment key={key}>
                    {editingKey === key && (
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
                      <TableCell>
                        <div className="flex gap-2">
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

function PurchasesView() {
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
        <ActionMessage action={action} />

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
                  <TableHead>操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {visibleRows.map((row) => (
                  <Fragment key={row.purchaseName}>
                    {editingPurchase === row.purchaseName && (
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
                      <TableCell>
                        <div className="flex gap-2">
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
                <TableHead>操作</TableHead>
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
                      <TableCell>
                        <div className="flex gap-2">
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
  extraControl?: React.ReactNode
  children: React.ReactNode
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
  renderTable: (rows: T[]) => React.ReactNode
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
  switch (view) {
    case 'dashboard':
      return '总览'
    case 'users':
      return '玩家管理'
    case 'songs':
      return '歌曲管理'
    case 'items':
      return '物品管理'
    case 'purchases':
      return '购买项配置'
    case 'purchaseItems':
      return '购买物品配置'
  }
}

export default App
