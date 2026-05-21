import { useEffect, useMemo, useState } from 'react'
import type { FormEvent } from 'react'
import {
  Activity,
  Boxes,
  ChartSpline,
  Database,
  KeyRound,
  LoaderCircle,
  LogOut,
  Music2,
  RefreshCcw,
  Search,
  ShieldAlert,
  ShoppingBag,
  Users,
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
  type DashboardData,
  type ItemPayload,
  type ItemRow,
  type PurchaseData,
  type PurchaseItemPayload,
  type PurchasePayload,
  type SongPayload,
  type SongRow,
  type UserRow,
} from '@/lib/api'
import { cn } from '@/lib/utils'

type View = 'dashboard' | 'users' | 'songs' | 'items' | 'purchases'

const navItems: Array<{
  id: View
  label: string
  icon: typeof Activity
}> = [
  { id: 'dashboard', label: '总览', icon: Activity },
  { id: 'users', label: '玩家', icon: Users },
  { id: 'songs', label: '歌曲', icon: Music2 },
  { id: 'items', label: '物品', icon: Boxes },
  { id: 'purchases', label: '购买', icon: ShoppingBag },
]

type LoadState = 'idle' | 'loading' | 'ready' | 'error'
type ActionState = {
  kind: 'idle' | 'success' | 'error'
  message: string
}

const emptyAction: ActionState = { kind: 'idle', message: '' }

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

  function load(showLoading = true) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .users(query, status)
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  useEffect(() => {
    adminApi
      .users()
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [])

  return (
    <DataPanel
      title="玩家列表"
      description="账号状态、票券和最近游玩记录"
      state={state}
      onSearch={() => load()}
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
          {rows.map((row) => (
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
    </DataPanel>
  )
}

function SongsView() {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<SongRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [form, setForm] = useState<SongPayload>(emptySongForm)
  const [editingSid, setEditingSid] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)

  function load(showLoading = true) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .songs(query)
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function edit(row: SongRow) {
    setEditingSid(row.songId)
    setForm({
      sid: row.songId,
      name_en: row.nameEn,
      rating_pst: row.ratingPst,
      rating_prs: row.ratingPrs,
      rating_ftr: row.ratingFtr,
      rating_byd: row.ratingByd,
      rating_etr: row.ratingEtr,
    })
  }

  function resetForm() {
    setEditingSid('')
    setForm(emptySongForm)
    setAction(emptyAction)
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      if (editingSid) {
        await adminApi.updateSong(editingSid, form)
      } else {
        await adminApi.createSong(form)
      }
      setAction({ kind: 'success', message: editingSid ? '歌曲已更新' : '歌曲已新增' })
      resetForm()
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
      .songs()
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [])

  return (
    <DataPanel
      title="歌曲表"
      description="曲目名称和谱面定数"
      state={state}
      onSearch={() => load()}
      searchValue={query}
      onSearchChange={setQuery}
    >
      <form className="mb-5 grid gap-3 rounded-md border p-3" onSubmit={submit}>
        <div className="grid gap-3 lg:grid-cols-8">
          <Input
            value={form.sid}
            disabled={Boolean(editingSid)}
            onChange={(event) => setForm({ ...form, sid: event.target.value })}
            placeholder="song_id"
            required
          />
          <Input
            className="lg:col-span-2"
            value={form.name_en}
            onChange={(event) => setForm({ ...form, name_en: event.target.value })}
            placeholder="name_en"
            required
          />
          {(['rating_pst', 'rating_prs', 'rating_ftr', 'rating_byd', 'rating_etr'] as const).map((field) => (
            <Input
              key={field}
              value={form[field]}
              onChange={(event) => setForm({ ...form, [field]: event.target.value })}
              placeholder={field.replace('rating_', '').toUpperCase()}
              required
            />
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm">
            {editingSid ? '更新歌曲' : '新增歌曲'}
          </Button>
          {editingSid && (
            <Button type="button" size="sm" variant="outline" onClick={resetForm}>
              取消编辑
            </Button>
          )}
          <ActionMessage action={action} />
        </div>
      </form>
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
          {rows.map((row) => (
            <TableRow key={row.songId}>
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
                    编辑
                  </Button>
                  <Button type="button" size="sm" variant="destructive" onClick={() => remove(row)}>
                    删除
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </DataPanel>
  )
}

function ItemsView() {
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<ItemRow[]>([])
  const [state, setState] = useState<LoadState>('loading')
  const [form, setForm] = useState<ItemPayload>(emptyItemForm)
  const [editingKey, setEditingKey] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)

  function load(showLoading = true) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .items(query)
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function edit(row: ItemRow) {
    setEditingKey(`${row.itemId}:${row.itemType}`)
    setForm({
      item_id: row.itemId,
      item_type: row.itemType,
      is_available: row.isAvailable,
    })
  }

  function resetForm() {
    setEditingKey('')
    setForm(emptyItemForm)
    setAction(emptyAction)
  }

  async function submit(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      if (editingKey) {
        await adminApi.updateItem(form)
      } else {
        await adminApi.createItem(form)
      }
      setAction({ kind: 'success', message: editingKey ? '物品已更新' : '物品已新增' })
      resetForm()
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
      .items()
      .then((value) => {
        setRows(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [])

  return (
    <DataPanel
      title="物品表"
      description="物品类型和可用状态"
      state={state}
      onSearch={() => load()}
      searchValue={query}
      onSearchChange={setQuery}
    >
      <form className="mb-5 grid gap-3 rounded-md border p-3" onSubmit={submit}>
        <div className="grid gap-3 sm:grid-cols-[1fr_1fr_160px]">
          <Input
            value={form.item_id}
            disabled={Boolean(editingKey)}
            onChange={(event) => setForm({ ...form, item_id: event.target.value })}
            placeholder="item_id"
            required
          />
          <Input
            value={form.item_type}
            disabled={Boolean(editingKey)}
            onChange={(event) => setForm({ ...form, item_type: event.target.value })}
            placeholder="type"
            required
          />
          <select
            className="h-9 rounded-md border bg-background px-3 text-sm"
            value={form.is_available ?? 0}
            onChange={(event) =>
              setForm({ ...form, is_available: Number(event.target.value) })
            }
          >
            <option value={1}>可用</option>
            <option value={0}>不可用</option>
          </select>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" size="sm">
            {editingKey ? '更新物品' : '新增物品'}
          </Button>
          {editingKey && (
            <Button type="button" size="sm" variant="outline" onClick={resetForm}>
              取消编辑
            </Button>
          )}
          <ActionMessage action={action} />
        </div>
      </form>
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
          {rows.map((row) => (
            <TableRow key={`${row.itemId}-${row.itemType}`}>
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
                    编辑
                  </Button>
                  <Button type="button" size="sm" variant="destructive" onClick={() => remove(row)}>
                    删除
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </DataPanel>
  )
}

function PurchasesView() {
  const [query, setQuery] = useState('')
  const [data, setData] = useState<PurchaseData>({
    purchases: [],
    purchaseItems: [],
  })
  const [state, setState] = useState<LoadState>('loading')
  const [purchaseForm, setPurchaseForm] = useState<PurchasePayload>(emptyPurchaseForm)
  const [editingPurchase, setEditingPurchase] = useState('')
  const [purchaseItemForm, setPurchaseItemForm] =
    useState<PurchaseItemPayload>(emptyPurchaseItemForm)
  const [editingPurchaseItem, setEditingPurchaseItem] = useState('')
  const [action, setAction] = useState<ActionState>(emptyAction)

  function load(showLoading = true) {
    if (showLoading) {
      setState('loading')
    }
    adminApi
      .purchases(query, query)
      .then((value) => {
        setData(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }

  function editPurchase(row: PurchaseData['purchases'][number]) {
    setEditingPurchase(row.purchaseName)
    setPurchaseForm({
      purchase_name: row.purchaseName,
      price: row.price,
      orig_price: row.origPrice,
      discount_from: row.discountFrom,
      discount_to: row.discountTo,
      discount_reason: row.discountReason,
    })
  }

  function resetPurchaseForm() {
    setEditingPurchase('')
    setPurchaseForm(emptyPurchaseForm)
    setAction(emptyAction)
  }

  async function submitPurchase(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      if (editingPurchase) {
        await adminApi.updatePurchase(editingPurchase, purchaseForm)
      } else {
        await adminApi.createPurchase(purchaseForm)
      }
      setAction({
        kind: 'success',
        message: editingPurchase ? '购买项已更新' : '购买项已新增',
      })
      resetPurchaseForm()
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function removePurchase(row: PurchaseData['purchases'][number]) {
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

  function editPurchaseItem(row: PurchaseData['purchaseItems'][number]) {
    setEditingPurchaseItem(`${row.purchaseName}:${row.itemId}:${row.itemType}`)
    setPurchaseItemForm({
      purchase_name: row.purchaseName,
      item_id: row.itemId,
      item_type: row.itemType,
      amount: row.amount,
    })
  }

  function resetPurchaseItemForm() {
    setEditingPurchaseItem('')
    setPurchaseItemForm(emptyPurchaseItemForm)
    setAction(emptyAction)
  }

  async function submitPurchaseItem(event: FormEvent) {
    event.preventDefault()
    setAction(emptyAction)
    try {
      if (editingPurchaseItem) {
        await adminApi.updatePurchaseItem(purchaseItemForm)
      } else {
        await adminApi.createPurchaseItem(purchaseItemForm)
      }
      setAction({
        kind: 'success',
        message: editingPurchaseItem ? '购买物品已更新' : '购买物品已新增',
      })
      resetPurchaseItemForm()
      load(false)
    } catch (error) {
      setAction({ kind: 'error', message: errorMessage(error) })
    }
  }

  async function removePurchaseItem(row: PurchaseData['purchaseItems'][number]) {
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
      .purchases()
      .then((value) => {
        setData(value)
        setState('ready')
      })
      .catch(() => setState('error'))
  }, [])

  return (
    <DataPanel
      title="购买配置"
      description="购买项、价格和关联物品"
      state={state}
      onSearch={() => load()}
      searchValue={query}
      onSearchChange={setQuery}
    >
      <div className="grid gap-5">
        <form className="grid gap-3 rounded-md border p-3" onSubmit={submitPurchase}>
          <div className="grid gap-3 xl:grid-cols-6">
            <Input
              value={purchaseForm.purchase_name}
              disabled={Boolean(editingPurchase)}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, purchase_name: event.target.value })
              }
              placeholder="purchase_name"
              required
            />
            <Input
              value={purchaseForm.price}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, price: event.target.value })
              }
              placeholder="price"
            />
            <Input
              value={purchaseForm.orig_price}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, orig_price: event.target.value })
              }
              placeholder="orig_price"
            />
            <Input
              type="datetime-local"
              value={purchaseForm.discount_from}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, discount_from: event.target.value })
              }
            />
            <Input
              type="datetime-local"
              value={purchaseForm.discount_to}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, discount_to: event.target.value })
              }
            />
            <Input
              value={purchaseForm.discount_reason}
              onChange={(event) =>
                setPurchaseForm({ ...purchaseForm, discount_reason: event.target.value })
              }
              placeholder="discount_reason"
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button type="submit" size="sm">
              {editingPurchase ? '更新购买项' : '新增购买项'}
            </Button>
            {editingPurchase && (
              <Button type="button" size="sm" variant="outline" onClick={resetPurchaseForm}>
                取消编辑
              </Button>
            )}
            <ActionMessage action={action} />
          </div>
        </form>

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
            {data.purchases.map((row) => (
              <TableRow key={row.purchaseName}>
                <TableCell className="font-mono">{row.purchaseName}</TableCell>
                <TableCell>{row.price || '-'}</TableCell>
                <TableCell>{row.origPrice || '-'}</TableCell>
                <TableCell className="min-w-52">
                  {row.discountFrom || '-'} / {row.discountTo || '-'}
                </TableCell>
                <TableCell className="max-w-xl truncate">{row.itemSummary}</TableCell>
                <TableCell>
                  <div className="flex gap-2">
                    <Button type="button" size="sm" variant="outline" onClick={() => editPurchase(row)}>
                      编辑
                    </Button>
                    <Button type="button" size="sm" variant="destructive" onClick={() => removePurchase(row)}>
                      删除
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>

        <div>
          <div className="mb-2 text-sm font-medium">Purchase Items</div>
          <form className="mb-3 grid gap-3 rounded-md border p-3" onSubmit={submitPurchaseItem}>
            <div className="grid gap-3 lg:grid-cols-[1fr_1fr_1fr_120px]">
              <Input
                value={purchaseItemForm.purchase_name}
                disabled={Boolean(editingPurchaseItem)}
                onChange={(event) =>
                  setPurchaseItemForm({
                    ...purchaseItemForm,
                    purchase_name: event.target.value,
                  })
                }
                placeholder="purchase_name"
                required
              />
              <Input
                value={purchaseItemForm.item_id}
                disabled={Boolean(editingPurchaseItem)}
                onChange={(event) =>
                  setPurchaseItemForm({ ...purchaseItemForm, item_id: event.target.value })
                }
                placeholder="item_id"
                required
              />
              <Input
                value={purchaseItemForm.item_type}
                disabled={Boolean(editingPurchaseItem)}
                onChange={(event) =>
                  setPurchaseItemForm({ ...purchaseItemForm, item_type: event.target.value })
                }
                placeholder="type"
                required
              />
              <Input
                value={purchaseItemForm.amount}
                onChange={(event) =>
                  setPurchaseItemForm({ ...purchaseItemForm, amount: event.target.value })
                }
                placeholder="amount"
                required
              />
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button type="submit" size="sm">
                {editingPurchaseItem ? '更新购买物品' : '新增购买物品'}
              </Button>
              {editingPurchaseItem && (
                <Button type="button" size="sm" variant="outline" onClick={resetPurchaseItemForm}>
                  取消编辑
                </Button>
              )}
            </div>
          </form>
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
              {data.purchaseItems.map((row) => (
                <TableRow
                  key={`${row.purchaseName}-${row.itemId}-${row.itemType}`}
                >
                  <TableCell className="font-mono">{row.purchaseName}</TableCell>
                  <TableCell>{row.itemId}</TableCell>
                  <TableCell>{row.itemType}</TableCell>
                  <TableCell>{row.amount}</TableCell>
                  <TableCell>
                    <div className="flex gap-2">
                      <Button type="button" size="sm" variant="outline" onClick={() => editPurchaseItem(row)}>
                        编辑
                      </Button>
                      <Button type="button" size="sm" variant="destructive" onClick={() => removePurchaseItem(row)}>
                        删除
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
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

function ActionMessage({ action }: { action: ActionState }) {
  if (action.kind === 'idle') {
    return null
  }

  return (
    <span
      className={cn(
        'text-sm',
        action.kind === 'success' ? 'text-emerald-700' : 'text-destructive',
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
      return '购买配置'
  }
}

export default App
