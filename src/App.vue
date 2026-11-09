<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { getVersion } from '@tauri-apps/api/app'
import { isEnabled as isAutostartEnabled, enable as enableAutostart, disable as disableAutostart } from '@tauri-apps/plugin-autostart'
import { t, activeLocale, currentLanguageChoice, setLanguage, type LanguageChoice, type MessageKey } from './i18n'
import { 
  BarChart3, 
  Clock, 
  Settings, 
  RefreshCw, 
  CheckCircle2, 
  Cloud, 
  AlertCircle,
  Play,
  Search,
  Plus,
  Trash2,
  Eye,
  EyeOff,
  Globe,
  Compass,
  LogIn,
  LogOut,
  ChevronDown,
  ChevronUp,
  X,
  CheckSquare,
  Square,
  History,
  Download,
  Upload,
  HardDrive,
  FolderOpen,
  Gamepad2,
  ArrowLeft,
  ExternalLink,
  Image as ImageIcon,
  Calendar,
  Building2,
  Tag,
  FileCode,
  Star,
  Sparkles,
  Layers,
  Info,
  Timer
} from 'lucide-vue-next'
import { gameMetadata, cleanExecutableName, type GameMetadata } from './lib/gameMetadata'

interface GameStats {
  game: string
  total_seconds: number
  formatted_time: string
}

interface GameSession {
  id?: number
  game: string
  start_time: string
  last_heartbeat: string
  end_time?: string
  duration_seconds: number
  is_synced: number
}

interface TrackerStatus {
  active_game: string | null
  active_duration_seconds: number
  api_enabled: boolean
  pending_events_count: number
  watched_games_count: number
}

interface AppConfig {
  db_path: string
  scan_interval_secs: number
  heartbeat_interval_secs: number
  sync_interval_secs: number
  config_refresh_interval_secs: number
  watched_games: string[]
  api_enabled: boolean
  api_base_url: string
  api_token: string | null
}

interface DiscoveredGame {
  name: string
  executable: string
  path: string
  source: string
}

interface PlaytimeImportCandidate {
  game_title: string
  executable: string
  source: string
  duration_seconds: number
  formatted_time: string
}

const activeTab = ref<'stats' | 'sessions' | 'settings'>('stats')

export interface GameLibraryItem {
  key: string
  displayName: string
  executable: string
  total_seconds: number
  formatted_playtime: string
  cover_url: string | null
  banner_url: string | null
  genres: string[]
  developer: string | null
  publisher: string | null
  release_date: string | null
  description: string | null
  short_description: string | null
  screenshots: Array<{ id: string; thumbnail: string; full: string }>
  steam_app_id: string | null
  is_monitored: boolean
  is_active: boolean
  last_played_timestamp: string | null
  session_count: number
}

// Library & Detail states
const selectedGameForDetail = ref<GameLibraryItem | null>(null)
const detailActiveTab = ref<'overview' | 'stats'>('overview')
const gameSessionsForDetail = ref<GameSession[]>([])
const isLoadingGameSessions = ref(false)
const librarySearchQuery = ref('')
const librarySort = ref<'playtime' | 'recent' | 'name'>('playtime')
const libraryStatusFilter = ref<'all' | 'played' | 'monitored'>('all')
const libraryGenreFilter = ref<string>('all')
const previewScreenshot = ref<string | null>(null)
const stats = ref<GameStats[]>([])
const recentSessions = ref<GameSession[]>([])
const status = ref<TrackerStatus>({
  active_game: null,
  active_duration_seconds: 0,
  api_enabled: false,
  pending_events_count: 0,
  watched_games_count: 0,
})

const formConfig = ref<AppConfig>({
  db_path: 'gaming.db',
  scan_interval_secs: 15,
  heartbeat_interval_secs: 900,
  sync_interval_secs: 30,
  config_refresh_interval_secs: 3600,
  watched_games: [],
  api_enabled: false,
  api_base_url: 'https://api.syntafin.de/v1/game',
  api_token: '',
})

// UI & Accordion States
const showAdvancedConfig = ref(false)
const showToken = ref(false)
const isTestingApi = ref(false)
const apiTestResult = ref<{ success: boolean; message: string } | null>(null)
const isSaving = ref(false)
const saveSuccess = ref(false)

// Modals
const showLoginModal = ref(false)
const loginTokenInput = ref('')
const loginUrlInput = ref('https://api.syntafin.de/v1/game')
const isLoggingIn = ref(false)
const loginError = ref<string | null>(null)

const showScannerModal = ref(false)
const isScanning = ref(false)
const discoveredGames = ref<DiscoveredGame[]>([])
const scannerSearch = ref('')
const selectedScannerGames = ref<Set<string>>(new Set())

// Playtime Importer Modal
const showImporterModal = ref(false)
const isScanningImports = ref(false)
const importCandidates = ref<PlaytimeImportCandidate[]>([])
const importSearch = ref('')
const selectedImportKeys = ref<Set<string>>(new Set())
const importAddToWatched = ref(true)
const isApplyingImport = ref(false)

// Multi-selection for monitored games
const selectedMonitoredGames = ref<Set<string>>(new Set())
const newGameInput = ref('')
const gameSearchQuery = ref('')

// Sync & Alerts
const isSyncing = ref(false)
const toastMessage = ref<{ type: 'success' | 'error'; text: string } | null>(null)

// In-App Updater
const appVersion = ref('')
const isCheckingUpdates = ref(false)
const updateInfo = ref<{ version: string; body?: string } | null>(null)
const updateStatusText = ref<string | null>(null)
const isDownloadingUpdate = ref(false)
let currentUpdateHandle: Update | null = null

async function handleCheckForUpdates() {
  isCheckingUpdates.value = true
  updateStatusText.value = null
  updateInfo.value = null
  try {
    const update = await check()
    if (update?.available) {
      currentUpdateHandle = update
      updateInfo.value = {
        version: update.version,
        body: update.body || '',
      }
    } else {
      updateStatusText.value = t('updater.up_to_date')
      setTimeout(() => {
        updateStatusText.value = null
      }, 4000)
    }
  } catch (err: any) {
    updateStatusText.value = t('updater.error', { error: String(err) })
    setTimeout(() => {
      updateStatusText.value = null
    }, 6000)
  } finally {
    isCheckingUpdates.value = false
  }
}

async function handleInstallUpdate() {
  if (!currentUpdateHandle) return
  isDownloadingUpdate.value = true
  try {
    await currentUpdateHandle.downloadAndInstall()
    await relaunch()
  } catch (err: any) {
    toastMessage.value = {
      type: 'error',
      text: t('updater.error', { error: String(err) }),
    }
  } finally {
    isDownloadingUpdate.value = false
  }
}

// Autostart
const isAutostart = ref(false)
const isUpdatingAutostart = ref(false)

async function checkAutostart() {
  try {
    isAutostart.value = await isAutostartEnabled()
  } catch (err) {
    console.warn('Failed to query autostart status:', err)
  }
}

async function toggleAutostart() {
  if (isUpdatingAutostart.value) return
  isUpdatingAutostart.value = true
  try {
    if (isAutostart.value) {
      await disableAutostart()
      isAutostart.value = false
    } else {
      await enableAutostart()
      isAutostart.value = true
    }
  } catch (err: any) {
    console.error('Failed to toggle autostart:', err)
    toastMessage.value = {
      type: 'error',
      text: t('settings.autostart_error', { error: String(err) }),
    }
    setTimeout(() => {
      toastMessage.value = null
    }, 4000)
  } finally {
    isUpdatingAutostart.value = false
  }
}

// Backup & Migration
const isExportingBackup = ref(false)
const isImportingBackup = ref(false)

interface ImportSummary {
  sessions_restored: number
  events_restored: number
  imported_playtime_restored: number
  games_restored: number
}

async function handleExportBackup() {
  isExportingBackup.value = true
  try {
    const savedPath = await invoke<string | null>('export_backup')
    if (savedPath) {
      toastMessage.value = {
        type: 'success',
        text: t('backup.export_success', { path: savedPath })
      }
    }
  } catch (err: any) {
    toastMessage.value = {
      type: 'error',
      text: t('backup.export_error', { error: String(err) })
    }
  } finally {
    isExportingBackup.value = false
  }
}

async function handleImportBackup() {
  isImportingBackup.value = true
  try {
    const summary = await invoke<ImportSummary | null>('import_backup')
    if (summary) {
      const totalRestored = summary.sessions_restored + summary.events_restored + summary.imported_playtime_restored + summary.games_restored
      if (totalRestored > 0) {
        toastMessage.value = {
          type: 'success',
          text: t('backup.import_success', {
            sessions: summary.sessions_restored,
            events: summary.events_restored,
            imported: summary.imported_playtime_restored,
            games: summary.games_restored
          })
        }
        await loadData()
      } else {
        toastMessage.value = {
          type: 'success',
          text: t('backup.import_empty')
        }
      }
    }
  } catch (err: any) {
    toastMessage.value = {
      type: 'error',
      text: t('backup.import_error', { error: String(err) })
    }
  } finally {
    isImportingBackup.value = false
  }
}

// Live timer
const liveTimerSeconds = ref(0)
let timerInterval: number | null = null
let unlistenTick: UnlistenFn | null = null

const formattedLiveTimer = computed(() => {
  const total = liveTimerSeconds.value
  const hrs = Math.floor(total / 3600).toString().padStart(2, '0')
  const mins = Math.floor((total % 3600) / 60).toString().padStart(2, '0')
  const secs = (total % 60).toString().padStart(2, '0')
  return `${hrs}:${mins}:${secs}`
})

const totalOverallPlaytimeHours = computed(() => {
  const totalSec = stats.value.reduce((acc, s) => acc + s.total_seconds, 0)
  return Math.round(totalSec / 3600)
})

const maxPlaytimeSeconds = computed(() => {
  if (stats.value.length === 0) return 1
  return Math.max(...stats.value.map(s => s.total_seconds))
})

function formatDurationLong(totalSeconds: number): string {
  if (totalSeconds <= 0) return '0 Min.'
  const days = Math.floor(totalSeconds / 86400)
  const remainingAfterDays = totalSeconds % 86400
  const hours = Math.floor(remainingAfterDays / 3600)
  const mins = Math.floor((remainingAfterDays % 3600) / 60)

  if (days > 0) {
    return `${days} T. ${hours} Std.`
  }
  if (hours > 0) {
    return `${hours} Std. ${mins} Min.`
  }
  return `${mins} Min.`
}

function formatRelativeTime(timestamp: string | null): string {
  if (!timestamp) return t('library.never_played')
  const date = new Date(timestamp)
  const now = new Date()
  const diffSec = Math.floor((now.getTime() - date.getTime()) / 1000)

  if (diffSec < 120) return 'vor wenigen Sekunden'
  if (diffSec < 3600) return `vor ${Math.floor(diffSec / 60)} Minuten`
  if (diffSec < 86400) {
    const hrs = Math.floor(diffSec / 3600)
    return hrs === 1 ? 'vor 1 Stunde' : `vor ${hrs} Stunden`
  }
  const days = Math.floor(diffSec / 86400)
  if (days === 1) return 'Gestern'
  if (days < 7) return `vor ${days} Tagen`
  if (days < 30) {
    const weeks = Math.floor(days / 7)
    return weeks === 1 ? 'vor 1 Woche' : `vor ${weeks} Wochen`
  }
  const months = Math.floor(days / 30)
  return months === 1 ? 'vor 1 Monat' : `vor ${months} Monaten`
}

function formatSessionDate(dtStr: string): string {
  try {
    const d = new Date(dtStr)
    return d.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' })
  } catch {
    return dtStr
  }
}

function formatSessionTimespan(startStr: string, endStr?: string): string {
  try {
    const start = new Date(startStr).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })
    if (endStr) {
      const end = new Date(endStr).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })
      return `${start} - ${end} Uhr`
    }
    return `ab ${start} Uhr`
  } catch {
    return startStr
  }
}

async function openGameDetail(game: GameLibraryItem) {
  selectedGameForDetail.value = game
  detailActiveTab.value = 'overview'
  isLoadingGameSessions.value = true
  try {
    const sessions = await invoke<GameSession[]>('get_game_sessions', {
      game: game.executable || game.key,
      limit: 100
    })
    gameSessionsForDetail.value = sessions
  } catch (err) {
    console.error('Failed to load game sessions', err)
    gameSessionsForDetail.value = []
  } finally {
    isLoadingGameSessions.value = false
  }
}

async function handleToggleWatchGame(executable: string) {
  if (!executable) return
  const isCurrentlyWatched = formConfig.value.watched_games.some(g => g.toLowerCase() === executable.toLowerCase())
  if (isCurrentlyWatched) {
    await handleRemoveSingleGame(executable)
    if (selectedGameForDetail.value) {
      selectedGameForDetail.value.is_monitored = false
    }
  } else {
    try {
      await invoke('add_custom_game', { game: executable })
      formConfig.value.watched_games.push(executable)
      if (selectedGameForDetail.value) {
        selectedGameForDetail.value.is_monitored = true
      }
      toastMessage.value = {
        type: 'success',
        text: t('settings.file_selected_success', { game: gameMetadata.getDisplayName(executable) })
      }
    } catch (err: any) {
      toastMessage.value = {
        type: 'error',
        text: String(err)
      }
    }
  }
}

const libraryItems = computed<GameLibraryItem[]>(() => {
  const map = new Map<string, GameLibraryItem>()

  // 1. Initial metadata games
  for (const meta of gameMetadata.getAllMetadata()) {
    const key = (meta.process_name || meta.slug || meta.name).toLowerCase()
    const exec = meta.process_name || (meta.slug ? `${meta.slug}.exe` : '')
    const genres = meta.genres?.map(g => g.name) || []
    const isMonitored = formConfig.value.watched_games.some(w => 
      w.toLowerCase() === key || 
      (meta.process_name && w.toLowerCase() === meta.process_name.toLowerCase()) ||
      (meta.slug && w.toLowerCase() === meta.slug.toLowerCase()) ||
      w.toLowerCase() === meta.name.toLowerCase()
    )

    map.set(key, {
      key,
      displayName: meta.name,
      executable: exec,
      total_seconds: 0,
      formatted_playtime: '0 Min.',
      cover_url: gameMetadata.getCoverUrl(meta, formConfig.value.api_base_url),
      banner_url: gameMetadata.getBannerUrl(meta, formConfig.value.api_base_url),
      genres,
      developer: meta.developer || null,
      publisher: meta.publisher || null,
      release_date: meta.release_date ? meta.release_date.split('T')[0] : null,
      description: meta.description || meta.short_description || null,
      short_description: meta.short_description || null,
      screenshots: meta.screenshots || [],
      steam_app_id: meta.steam_app_id || null,
      is_monitored: isMonitored,
      is_active: Boolean(status.value.active_game && (status.value.active_game.toLowerCase() === key || (meta.process_name && status.value.active_game.toLowerCase() === meta.process_name.toLowerCase()))),
      last_played_timestamp: null,
      session_count: 0,
    })
  }

  // 2. Merge stats from SQLite
  for (const s of stats.value) {
    const meta = gameMetadata.findGame(s.game)
    const key = (meta?.process_name || meta?.slug || s.game).toLowerCase()
    let item = map.get(key)
    if (!item) {
      const displayName = gameMetadata.getDisplayName(s.game)
      const isMonitored = formConfig.value.watched_games.some(w => w.toLowerCase() === s.game.toLowerCase())
      item = {
        key,
        displayName,
        executable: s.game,
        total_seconds: s.total_seconds,
        formatted_playtime: formatDurationLong(s.total_seconds),
        cover_url: meta ? gameMetadata.getCoverUrl(meta, formConfig.value.api_base_url) : null,
        banner_url: meta ? gameMetadata.getBannerUrl(meta, formConfig.value.api_base_url) : null,
        genres: meta?.genres?.map(g => g.name) || [],
        developer: meta?.developer || null,
        publisher: meta?.publisher || null,
        release_date: meta?.release_date ? meta.release_date.split('T')[0] : null,
        description: meta?.description || meta?.short_description || null,
        short_description: meta?.short_description || null,
        screenshots: meta?.screenshots || [],
        steam_app_id: meta?.steam_app_id || null,
        is_monitored: isMonitored,
        is_active: Boolean(status.value.active_game && status.value.active_game.toLowerCase() === s.game.toLowerCase()),
        last_played_timestamp: null,
        session_count: 0,
      }
      map.set(key, item)
    } else {
      item.total_seconds = s.total_seconds
      item.formatted_playtime = formatDurationLong(s.total_seconds)
    }
  }

  // 3. Ensure watched games are present
  for (const w of formConfig.value.watched_games) {
    const meta = gameMetadata.findGame(w)
    const key = (meta?.process_name || meta?.slug || w).toLowerCase()
    let item = map.get(key)
    if (!item) {
      const displayName = gameMetadata.getDisplayName(w)
      item = {
        key,
        displayName,
        executable: w,
        total_seconds: 0,
        formatted_playtime: '0 Min.',
        cover_url: meta ? gameMetadata.getCoverUrl(meta, formConfig.value.api_base_url) : null,
        banner_url: meta ? gameMetadata.getBannerUrl(meta, formConfig.value.api_base_url) : null,
        genres: meta?.genres?.map(g => g.name) || [],
        developer: meta?.developer || null,
        publisher: meta?.publisher || null,
        release_date: meta?.release_date ? meta.release_date.split('T')[0] : null,
        description: meta?.description || meta?.short_description || null,
        short_description: meta?.short_description || null,
        screenshots: meta?.screenshots || [],
        steam_app_id: meta?.steam_app_id || null,
        is_monitored: true,
        is_active: Boolean(status.value.active_game && status.value.active_game.toLowerCase() === w.toLowerCase()),
        last_played_timestamp: null,
        session_count: 0,
      }
      map.set(key, item)
    } else {
      item.is_monitored = true
    }
  }

  // 4. Update last_played_timestamp and session count from recentSessions
  for (const sess of recentSessions.value) {
    for (const item of map.values()) {
      if (
        sess.game.toLowerCase() === item.key ||
        sess.game.toLowerCase() === item.executable.toLowerCase() ||
        sess.game.toLowerCase() === item.displayName.toLowerCase()
      ) {
        if (!item.last_played_timestamp) {
          item.last_played_timestamp = sess.end_time || sess.last_heartbeat || sess.start_time
        }
        item.session_count++
      }
    }
  }

  return Array.from(map.values())
})

const availableGenres = computed(() => {
  const set = new Set<string>()
  for (const item of libraryItems.value) {
    for (const g of item.genres) {
      if (g) set.add(g)
    }
  }
  return Array.from(set).sort()
})

const filteredLibraryGames = computed(() => {
  let list = libraryItems.value

  if (librarySearchQuery.value.trim()) {
    const q = librarySearchQuery.value.toLowerCase().trim()
    list = list.filter(g => 
      g.displayName.toLowerCase().includes(q) || 
      g.executable.toLowerCase().includes(q) ||
      (g.developer && g.developer.toLowerCase().includes(q)) ||
      g.genres.some(genre => genre.toLowerCase().includes(q))
    )
  }

  if (libraryStatusFilter.value === 'played') {
    list = list.filter(g => g.total_seconds > 0)
  } else if (libraryStatusFilter.value === 'monitored') {
    list = list.filter(g => g.is_monitored)
  }

  if (libraryGenreFilter.value !== 'all') {
    list = list.filter(g => g.genres.some(genre => genre.toLowerCase() === libraryGenreFilter.value.toLowerCase()))
  }

  return [...list].sort((a, b) => {
    if (a.is_active && !b.is_active) return -1
    if (!a.is_active && b.is_active) return 1

    if (librarySort.value === 'playtime') {
      return b.total_seconds - a.total_seconds
    }
    if (librarySort.value === 'recent') {
      const timeA = a.last_played_timestamp ? new Date(a.last_played_timestamp).getTime() : 0
      const timeB = b.last_played_timestamp ? new Date(b.last_played_timestamp).getTime() : 0
      return timeB - timeA
    }
    if (librarySort.value === 'name') {
      return a.displayName.localeCompare(b.displayName)
    }
    return 0
  })
})

const detailStats = computed(() => {
  if (!selectedGameForDetail.value) {
    return {
      totalPlaytime: '0 Min.',
      totalHours: 0,
      lastSessionDuration: '—',
      avgSessionDuration: '—',
      totalSessionsCount: 0,
      lastPlayedDate: '—',
    }
  }

  const game = selectedGameForDetail.value
  const sessions = gameSessionsForDetail.value

  const totalSec = game.total_seconds || sessions.reduce((acc, s) => acc + s.duration_seconds, 0)
  const totalHours = Math.round(totalSec / 3600)
  const totalSessionsCount = sessions.length

  const lastSessionDuration = sessions.length > 0 && sessions[0].duration_seconds > 0
    ? formatDurationLong(sessions[0].duration_seconds)
    : '—'

  const avgSeconds = totalSessionsCount > 0 ? Math.round(totalSec / totalSessionsCount) : 0
  const avgSessionDuration = avgSeconds > 0 ? formatDurationLong(avgSeconds) : '—'

  const lastPlayedDate = game.last_played_timestamp ? formatRelativeTime(game.last_played_timestamp) : t('library.never_played')

  return {
    totalPlaytime: formatDurationLong(totalSec),
    totalHours,
    lastSessionDuration,
    avgSessionDuration,
    totalSessionsCount,
    lastPlayedDate,
  }
})

const filteredWatchedGames = computed(() => {
  if (!formConfig.value.watched_games) return []
  if (!gameSearchQuery.value.trim()) return formConfig.value.watched_games
  const q = gameSearchQuery.value.toLowerCase()
  return formConfig.value.watched_games.filter(g => g.toLowerCase().includes(q))
})

const filteredScannerGames = computed(() => {
  if (!scannerSearch.value.trim()) return discoveredGames.value
  const q = scannerSearch.value.toLowerCase()
  return discoveredGames.value.filter(g => 
    g.name.toLowerCase().includes(q) || g.executable.toLowerCase().includes(q)
  )
})

const filteredImportCandidates = computed(() => {
  if (!importSearch.value.trim()) return importCandidates.value
  const q = importSearch.value.toLowerCase()
  return importCandidates.value.filter(c => 
    c.game_title.toLowerCase().includes(q) || 
    c.executable.toLowerCase().includes(q) ||
    c.source.toLowerCase().includes(q)
  )
})

const allFilteredMonitoredSelected = computed(() => {
  if (filteredWatchedGames.value.length === 0) return false
  return filteredWatchedGames.value.every(g => selectedMonitoredGames.value.has(g))
})

async function loadData() {
  try {
    stats.value = await invoke<GameStats[]>('get_historical_stats')
    recentSessions.value = await invoke<GameSession[]>('get_recent_sessions', { limit: 20 })
    const currentStatus = await invoke<TrackerStatus>('get_tracker_status')
    status.value = currentStatus
    liveTimerSeconds.value = currentStatus.active_duration_seconds

    const cfg = await invoke<AppConfig>('get_config')
    formConfig.value = {
      ...cfg,
      api_token: cfg.api_token || '',
    }
  } catch (err) {
    console.error('Failed to load tracker data:', err)
  }
}

async function handleManualSync() {
  if (isSyncing.value) return
  isSyncing.value = true
  toastMessage.value = null
  try {
    const count = await invoke<number>('trigger_sync')
    toastMessage.value = {
      type: 'success',
      text: t('success.sync_done', { count }),
    }
    await loadData()
  } catch (err: any) {
    const errMsg = String(err)
    if (errMsg.includes('ERR_API_DISABLED') || errMsg.includes('disabled')) {
      toastMessage.value = {
        type: 'error',
        text: t('error.api_disabled'),
      }
    } else {
      toastMessage.value = {
        type: 'error',
        text: t('error.sync_failed', { error: errMsg }),
      }
    }
  } finally {
    isSyncing.value = false
    setTimeout(() => {
      toastMessage.value = null
    }, 4500)
  }
}

async function handleLogin() {
  const token = loginTokenInput.value.trim()
  if (!token) {
    loginError.value = t('error.empty_token')
    return
  }
  isLoggingIn.value = true
  loginError.value = null
  try {
    const count = await invoke<number>('login_with_token', {
      token,
      url: loginUrlInput.value.trim() || null,
    })
    showLoginModal.value = false
    loginTokenInput.value = ''
    toastMessage.value = {
      type: 'success',
      text: t('login.success', { count }),
    }
    await loadData()
  } catch (err: any) {
    loginError.value = String(err)
  } finally {
    isLoggingIn.value = false
  }
}

async function handleLogout() {
  try {
    await invoke('logout_api')
    toastMessage.value = {
      type: 'success',
      text: t('settings.local_mode_hint'),
    }
    await loadData()
  } catch (err) {
    console.error('Logout failed:', err)
  }
}

async function handleTestApi() {
  isTestingApi.value = true
  apiTestResult.value = null
  try {
    const count = await invoke<number>('test_api_connection', {
      url: formConfig.value.api_base_url,
      token: formConfig.value.api_token || '',
    })
    apiTestResult.value = {
      success: true,
      message: t('settings.test_success', { count }),
    }
  } catch (err: any) {
    apiTestResult.value = {
      success: false,
      message: t('settings.test_failed', { error: String(err) }),
    }
  } finally {
    isTestingApi.value = false
  }
}

async function handleSaveSettings() {
  isSaving.value = true
  saveSuccess.value = false
  try {
    const payload = {
      ...formConfig.value,
      api_token: formConfig.value.api_token?.trim() || null,
    }
    await invoke('save_config', { newConfig: payload })
    saveSuccess.value = true
    await loadData()
    setTimeout(() => {
      saveSuccess.value = false
    }, 3000)
  } catch (err: any) {
    toastMessage.value = {
      type: 'error',
      text: String(err),
    }
  } finally {
    isSaving.value = false
  }
}

let activeScanSessionId = 0

function cancelScan() {
  activeScanSessionId++
  isScanning.value = false
  showScannerModal.value = false
}

async function startPcScan() {
  const currentId = ++activeScanSessionId
  showScannerModal.value = true
  isScanning.value = true
  discoveredGames.value = []
  selectedScannerGames.value.clear()
  try {
    const games = await invoke<DiscoveredGame[]>('scan_installed_games')
    if (activeScanSessionId !== currentId) {
      return
    }
    discoveredGames.value = games
    // Preselect games that are not yet tracked
    const trackedSet = new Set(formConfig.value.watched_games.map(g => g.toLowerCase()))
    for (const g of games) {
      if (!trackedSet.has(g.executable.toLowerCase())) {
        selectedScannerGames.value.add(g.executable)
      }
    }
  } catch (err) {
    if (activeScanSessionId === currentId) {
      console.error('Scan failed:', err)
    }
  } finally {
    if (activeScanSessionId === currentId) {
      isScanning.value = false
    }
  }
}

async function handleApplyScannerSelection() {
  if (selectedScannerGames.value.size === 0) return
  const toAdd = Array.from(selectedScannerGames.value)
  try {
    const updated = await invoke<string[]>('add_multiple_games', { games: toAdd })
    formConfig.value.watched_games = updated
    showScannerModal.value = false
    await loadData()
  } catch (err) {
    console.error('Failed to add scanned games:', err)
  }
}

let activeImportScanId = 0

function cancelImportScan() {
  activeImportScanId++
  isScanningImports.value = false
  showImporterModal.value = false
}

async function startPlaytimeScan() {
  const currentId = ++activeImportScanId
  showImporterModal.value = true
  isScanningImports.value = true
  importCandidates.value = []
  selectedImportKeys.value.clear()
  try {
    const candidates = await invoke<PlaytimeImportCandidate[]>('scan_playtime_imports')
    if (activeImportScanId !== currentId) {
      return
    }
    importCandidates.value = candidates
    for (const c of candidates) {
      selectedImportKeys.value.add(`${c.executable}:::${c.source}`)
    }
  } catch (err) {
    if (activeImportScanId === currentId) {
      console.error('Scan imports failed:', err)
    }
  } finally {
    if (activeImportScanId === currentId) {
      isScanningImports.value = false
    }
  }
}

function toggleImportSelectAll() {
  if (selectedImportKeys.value.size === filteredImportCandidates.value.length) {
    selectedImportKeys.value.clear()
  } else {
    for (const c of filteredImportCandidates.value) {
      selectedImportKeys.value.add(`${c.executable}:::${c.source}`)
    }
  }
}

async function handleApplyImports() {
  if (selectedImportKeys.value.size === 0) return
  isApplyingImport.value = true
  try {
    const itemsToImport = importCandidates.value
      .filter(c => selectedImportKeys.value.has(`${c.executable}:::${c.source}`))
      .map(c => ({
        game_title: c.game_title,
        executable: c.executable,
        source: c.source,
        duration_seconds: c.duration_seconds,
      }))

    const count = await invoke<number>('apply_playtime_imports', {
      items: itemsToImport,
      addToWatched: importAddToWatched.value,
    })

    showImporterModal.value = false
    toastMessage.value = {
      type: 'success',
      text: t('importer.success', { count }),
    }
    await loadData()
  } catch (err: any) {
    toastMessage.value = {
      type: 'error',
      text: String(err),
    }
  } finally {
    isApplyingImport.value = false
  }
}

function toggleScannerSelectAll() {
  if (selectedScannerGames.value.size === filteredScannerGames.value.length) {
    selectedScannerGames.value.clear()
  } else {
    for (const g of filteredScannerGames.value) {
      selectedScannerGames.value.add(g.executable)
    }
  }
}

function toggleMonitoredSelectAll() {
  if (allFilteredMonitoredSelected.value) {
    for (const g of filteredWatchedGames.value) {
      selectedMonitoredGames.value.delete(g)
    }
  } else {
    for (const g of filteredWatchedGames.value) {
      selectedMonitoredGames.value.add(g)
    }
  }
}

function toggleGameSelection(game: string) {
  if (selectedMonitoredGames.value.has(game)) {
    selectedMonitoredGames.value.delete(game)
  } else {
    selectedMonitoredGames.value.add(game)
  }
}

async function handleRemoveSelectedGames() {
  if (selectedMonitoredGames.value.size === 0) return
  const toRemove = Array.from(selectedMonitoredGames.value)
  try {
    const updated = await invoke<string[]>('remove_multiple_games', { games: toRemove })
    formConfig.value.watched_games = updated
    selectedMonitoredGames.value.clear()
    await loadData()
  } catch (err) {
    console.error('Failed to remove selected games:', err)
  }
}

async function handleAddSingleGame() {
  const name = newGameInput.value.trim()
  if (!name) return
  try {
    const updated = await invoke<string[]>('add_custom_game', { game: name })
    formConfig.value.watched_games = updated
    newGameInput.value = ''
    await loadData()
  } catch (err) {
    console.error('Failed to add game:', err)
  }
}

async function handleBrowseGameFile() {
  try {
    const selected = await invoke<string | null>('pick_game_file')
    if (selected) {
      const parts = selected.split(/[\\/]/)
      const fileName = parts[parts.length - 1]
      const updated = await invoke<string[]>('add_custom_game', { game: fileName })
      formConfig.value.watched_games = updated
      toastMessage.value = {
        type: 'success',
        text: t('settings.file_selected_success', { game: fileName })
      }
      await loadData()
    }
  } catch (err) {
    console.error('Browse game file failed:', err)
  }
}

async function handleRemoveSingleGame(game: string) {
  try {
    const updated = await invoke<string[]>('remove_custom_game', { game })
    formConfig.value.watched_games = updated
    selectedMonitoredGames.value.delete(game)
    await loadData()
  } catch (err) {
    console.error('Failed to remove game:', err)
  }
}

function isGameAlreadyTracked(executable: string): boolean {
  const target = executable.toLowerCase()
  return formConfig.value.watched_games.some(g => g.toLowerCase() === target)
}

function formatDate(isoStr: string) {
  try {
    const d = new Date(isoStr)
    const localeStr = activeLocale.value === 'de' ? 'de-DE' : 'en-US'
    return d.toLocaleString(localeStr, {
      day: '2-digit',
      month: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return isoStr
  }
}

function formatDuration(sec: number) {
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

onMounted(async () => {
  try {
    appVersion.value = await getVersion()
  } catch {}
  await loadData()
  await checkAutostart()

  timerInterval = window.setInterval(() => {
    if (status.value.active_game) {
      liveTimerSeconds.value += 1
    }
  }, 1000)

  unlistenTick = await listen('tracker-tick', (event: any) => {
    if (event.payload) {
      status.value.active_game = event.payload.game
    } else {
      status.value.active_game = null
      liveTimerSeconds.value = 0
    }
    loadData()
  })
})

onUnmounted(() => {
  if (timerInterval) clearInterval(timerInterval)
  if (unlistenTick) unlistenTick()
})
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-950 text-slate-100 font-sans selection:bg-indigo-600 selection:text-white">
    <!-- Single Clean Navigation Bar (No redundant duplicate logo/title!) -->
    <header class="h-12 border-b border-slate-800 bg-slate-900/60 backdrop-blur px-4 flex items-center justify-between shrink-0">
      <!-- Tabs on the Left -->
      <nav class="flex items-center gap-1">
        <button 
          @click="activeTab = 'stats'; selectedGameForDetail = null"
          class="px-3 py-1.5 rounded-md text-xs font-medium flex items-center gap-2 transition"
          :class="activeTab === 'stats' ? 'bg-indigo-600 text-white font-semibold shadow-sm' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'">
          <Gamepad2 class="w-3.5 h-3.5" />
          <span>{{ t('nav.stats') }}</span>
        </button>

        <button 
          @click="activeTab = 'sessions'"
          class="px-3 py-1.5 rounded-md text-xs font-medium flex items-center gap-2 transition"
          :class="activeTab === 'sessions' ? 'bg-indigo-600 text-white font-semibold shadow-sm' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'">
          <Clock class="w-3.5 h-3.5" />
          <span>{{ t('nav.sessions') }}</span>
        </button>

        <button 
          @click="activeTab = 'settings'"
          class="px-3 py-1.5 rounded-md text-xs font-medium flex items-center gap-2 transition"
          :class="activeTab === 'settings' ? 'bg-indigo-600 text-white font-semibold shadow-sm' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'">
          <Settings class="w-3.5 h-3.5" />
          <span>{{ t('nav.settings') }}</span>
        </button>
      </nav>

      <!-- Status Indicator & Sync Action on the Right -->
      <div class="flex items-center gap-3">
        <!-- Live Status Badge -->
        <div 
          class="flex items-center gap-2 px-2.5 py-1 rounded-full border text-xs"
          :class="status.active_game ? 'bg-emerald-950/60 border-emerald-500/40 text-emerald-300' : 'bg-slate-800/60 border-slate-700/60 text-slate-400'">
          <span class="relative flex h-2 w-2">
            <span v-if="status.active_game" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
            <span class="relative inline-flex rounded-full h-2 w-2" :class="status.active_game ? 'bg-emerald-500' : 'bg-slate-500'"></span>
          </span>
          <span class="font-medium text-[11px]">
            {{ status.active_game ? t('status.tracking', { game: gameMetadata.getDisplayName(status.active_game) }) : t('status.idle') }}
          </span>
        </div>

        <!-- Sync Button -->
        <button 
          @click="handleManualSync"
          :disabled="isSyncing"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 border border-slate-700 transition disabled:opacity-50">
          <RefreshCw class="w-3.5 h-3.5 text-indigo-400" :class="{ 'animate-spin': isSyncing }" />
          <span>{{ isSyncing ? t('status.syncing') : t('status.sync') }}</span>
        </button>
      </div>
    </header>

    <!-- Active Game Banner (if currently tracking a game) -->
    <div v-if="status.active_game" class="bg-slate-900/90 border-b border-emerald-500/30 px-5 py-2.5 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3">
        <div class="p-1.5 rounded-md bg-emerald-500/20 text-emerald-400">
          <Play class="w-4 h-4 fill-current" />
        </div>
        <div>
          <div class="text-[10px] font-semibold text-emerald-400 uppercase tracking-wider">
            {{ t('active.game_running') }}
          </div>
          <div class="flex items-center gap-2">
            <span class="text-xs font-bold text-white">
              {{ gameMetadata.getDisplayName(status.active_game) }}
            </span>
            <span class="text-[10px] font-mono text-emerald-400/80 bg-emerald-950/60 px-1.5 py-0.5 rounded border border-emerald-500/20">
              {{ status.active_game }}
            </span>
          </div>
        </div>
      </div>
      <div class="text-right">
        <div class="text-[10px] text-slate-400">{{ t('active.current_playtime') }}</div>
        <div class="text-lg font-mono font-bold text-emerald-300">
          {{ formattedLiveTimer }}
        </div>
      </div>
    </div>

    <!-- Alert / Toast Banner -->
    <div 
      v-if="toastMessage" 
      class="px-5 py-2 text-xs flex items-center justify-between border-b"
      :class="toastMessage.type === 'success' ? 'bg-emerald-950/50 border-emerald-500/30 text-emerald-300' : 'bg-rose-950/50 border-rose-500/30 text-rose-300'">
      <div class="flex items-center gap-2">
        <CheckCircle2 v-if="toastMessage.type === 'success'" class="w-3.5 h-3.5 shrink-0" />
        <AlertCircle v-else class="w-3.5 h-3.5 shrink-0" />
        <span>{{ toastMessage.text }}</span>
      </div>
      <button @click="toastMessage = null" class="opacity-60 hover:opacity-100">&times;</button>
    </div>

    <!-- Main Content Area -->
    <main class="flex-1 overflow-y-auto p-5">
      <!-- 1. SPIELEBIBLIOTHEK & DETAILANSICHT (Matches User Website Screenshots 2, 3 & 4) -->
      <div v-if="activeTab === 'stats'" class="space-y-4">
        
        <!-- DETAIL PAGE VIEW (Screenshots 3 & 4) -->
        <div v-if="selectedGameForDetail" class="space-y-4">
          <!-- Back button & Detail Header -->
          <div class="flex items-center justify-between border-b border-slate-800 pb-3">
            <div class="flex items-center gap-3">
              <button 
                @click="selectedGameForDetail = null"
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-800/80 hover:bg-slate-700 text-xs text-slate-200 border border-slate-700 transition">
                <ArrowLeft class="w-3.5 h-3.5" />
                <span>{{ t('detail.back') }}</span>
              </button>
              <div>
                <h1 class="text-xl sm:text-2xl font-black text-white tracking-tight flex items-center gap-2">
                  <span>{{ selectedGameForDetail.displayName }}</span>
                  <span 
                    v-if="selectedGameForDetail.is_active"
                    class="text-[10px] font-bold px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-400 border border-emerald-500/30">
                    {{ t('library.now_playing') }}
                  </span>
                </h1>
                <div class="text-xs font-mono text-slate-400 mt-0.5">
                  {{ selectedGameForDetail.executable }}
                </div>
              </div>
            </div>

            <!-- Action: Toggle Monitoring -->
            <button 
              @click="handleToggleWatchGame(selectedGameForDetail.executable)"
              class="px-3 py-1.5 rounded-lg text-xs font-medium border flex items-center gap-1.5 transition"
              :class="selectedGameForDetail.is_monitored 
                ? 'bg-indigo-950/60 text-indigo-300 border-indigo-500/40 hover:bg-rose-950/40 hover:text-rose-300 hover:border-rose-500/40' 
                : 'bg-slate-800 text-slate-300 border-slate-700 hover:bg-slate-700'">
              <Star class="w-3.5 h-3.5" :class="{ 'fill-current': selectedGameForDetail.is_monitored }" />
              <span>{{ selectedGameForDetail.is_monitored ? t('detail.watching_active') : t('detail.watching_inactive') }}</span>
            </button>
          </div>

          <!-- Detail Navigation Tabs -->
          <div class="flex items-center gap-2 border-b border-slate-800 pb-2">
            <button 
              @click="detailActiveTab = 'overview'"
              class="px-3 py-1.5 rounded-md text-xs font-medium flex items-center gap-1.5 transition"
              :class="detailActiveTab === 'overview' ? 'bg-indigo-600 text-white font-semibold shadow' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'">
              <Info class="w-3.5 h-3.5" />
              <span>{{ t('detail.tab_overview') }}</span>
            </button>
            <button 
              @click="detailActiveTab = 'stats'"
              class="px-3 py-1.5 rounded-md text-xs font-medium flex items-center gap-1.5 transition"
              :class="detailActiveTab === 'stats' ? 'bg-indigo-600 text-white font-semibold shadow' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'">
              <BarChart3 class="w-3.5 h-3.5" />
              <span>{{ t('detail.tab_stats') }}</span>
              <span v-if="gameSessionsForDetail.length > 0" class="ml-1 text-[10px] px-1.5 py-0.2 rounded-full bg-indigo-950 text-indigo-300 border border-indigo-500/30">
                {{ gameSessionsForDetail.length }}
              </span>
            </button>
          </div>

          <!-- Grid: Main Left, Sidebar Right (Screenshots 3 & 4) -->
          <div class="grid grid-cols-1 lg:grid-cols-3 gap-5">
            
            <!-- Left Main Column -->
            <div class="lg:col-span-2 space-y-4">
              
              <!-- Tab 1: Overview (Screenshot 3) -->
              <div v-if="detailActiveTab === 'overview'" class="space-y-4">
                <!-- Description Card -->
                <div class="p-4 rounded-xl bg-slate-900/60 border border-slate-800 space-y-2.5">
                  <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                    <Info class="w-3.5 h-3.5 text-indigo-400" />
                    <span>{{ t('detail.about_title') }}</span>
                  </h2>
                  <p class="text-xs text-slate-300 leading-relaxed whitespace-pre-line">
                    {{ selectedGameForDetail.description || selectedGameForDetail.short_description || 'Keine zusätzliche Spielbeschreibung hinterlegt.' }}
                  </p>
                </div>

                <!-- Screenshots Gallery (Screenshot 3) -->
                <div v-if="selectedGameForDetail.screenshots && selectedGameForDetail.screenshots.length > 0" class="p-4 rounded-xl bg-slate-900/60 border border-slate-800 space-y-3">
                  <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                    <ImageIcon class="w-3.5 h-3.5 text-indigo-400" />
                    <span>{{ t('detail.screenshots_title') }}</span>
                  </h2>
                  <div class="grid grid-cols-2 sm:grid-cols-3 gap-2.5">
                    <div 
                      v-for="sc in selectedGameForDetail.screenshots.slice(0, 6)" 
                      :key="sc.id"
                      @click="previewScreenshot = sc.full || sc.thumbnail"
                      class="rounded-lg overflow-hidden border border-slate-800 bg-slate-950 aspect-video group cursor-pointer relative shadow">
                      <img 
                        :src="sc.thumbnail || sc.full" 
                        loading="lazy"
                        class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300" />
                    </div>
                  </div>
                </div>
              </div>

              <!-- Tab 2: Statistics & Session History (Screenshot 4) -->
              <div v-if="detailActiveTab === 'stats'" class="space-y-4">
                <!-- 4 Top Metric Cards (Screenshot 4) -->
                <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
                  <div class="p-3.5 rounded-xl bg-slate-900/70 border border-slate-800">
                    <div class="text-[11px] text-slate-400">{{ t('detail.total_playtime') }}</div>
                    <div class="mt-1 text-base sm:text-lg font-bold text-white">{{ detailStats.totalPlaytime }}</div>
                    <div class="text-[10px] text-slate-500 mt-0.5">{{ detailStats.totalHours }} {{ t('stats.hours') }}</div>
                  </div>

                  <div class="p-3.5 rounded-xl bg-slate-900/70 border border-slate-800">
                    <div class="text-[11px] text-slate-400">{{ t('detail.last_session') }}</div>
                    <div class="mt-1 text-base sm:text-lg font-bold text-white">{{ detailStats.lastSessionDuration }}</div>
                    <div class="text-[10px] text-slate-500 mt-0.5">{{ detailStats.lastPlayedDate }}</div>
                  </div>

                  <div class="p-3.5 rounded-xl bg-slate-900/70 border border-slate-800">
                    <div class="text-[11px] text-slate-400">{{ t('detail.avg_session') }}</div>
                    <div class="mt-1 text-base sm:text-lg font-bold text-white">{{ detailStats.avgSessionDuration }}</div>
                    <div class="text-[10px] text-slate-500 mt-0.5">pro Sitzung</div>
                  </div>

                  <div class="p-3.5 rounded-xl bg-slate-900/70 border border-slate-800">
                    <div class="text-[11px] text-slate-400">{{ t('detail.session_count') }}</div>
                    <div class="mt-1 text-base sm:text-lg font-bold text-white">{{ detailStats.totalSessionsCount }}</div>
                    <div class="text-[10px] text-slate-500 mt-0.5">Erfasste Sitzungen</div>
                  </div>
                </div>

                <!-- Session History Card (Screenshot 4) -->
                <div class="p-4 rounded-xl bg-slate-900/60 border border-slate-800 space-y-3">
                  <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                    <History class="w-3.5 h-3.5 text-indigo-400" />
                    <span>{{ t('detail.history_title') }}</span>
                  </h2>

                  <div v-if="isLoadingGameSessions" class="py-8 text-center text-slate-500 text-xs">
                    Lade Sitzungen...
                  </div>

                  <div v-else-if="gameSessionsForDetail.length === 0" class="py-8 text-center text-slate-500 text-xs">
                    {{ t('detail.no_sessions_yet') }}
                  </div>

                  <div v-else class="rounded-lg border border-slate-800/80 overflow-hidden bg-slate-950/40">
                    <table class="w-full text-left text-xs">
                      <thead class="bg-slate-900/80 text-slate-400 uppercase tracking-wider text-[10px] border-b border-slate-800">
                        <tr>
                          <th class="py-2 px-3">{{ t('detail.date_col') }}</th>
                          <th class="py-2 px-3">{{ t('detail.timespan_col') }}</th>
                          <th class="py-2 px-3 text-right">{{ t('detail.duration_col') }}</th>
                        </tr>
                      </thead>
                      <tbody class="divide-y divide-slate-800/60">
                        <tr v-for="sess in gameSessionsForDetail" :key="sess.id" class="hover:bg-slate-800/30 transition">
                          <td class="py-2 px-3 text-slate-300 font-medium">{{ formatSessionDate(sess.start_time) }}</td>
                          <td class="py-2 px-3 text-slate-400 font-mono text-[11px]">{{ formatSessionTimespan(sess.start_time, sess.end_time) }}</td>
                          <td class="py-2 px-3 text-right">
                            <span class="inline-flex px-2 py-0.5 rounded-full text-[11px] font-semibold bg-indigo-950/60 text-indigo-300 border border-indigo-500/20">
                              {{ formatDurationLong(sess.duration_seconds) }}
                            </span>
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            </div>

            <!-- Right Sidebar Column (Screenshots 3 & 4) -->
            <div class="space-y-4">
              <!-- Game Poster -->
              <div class="rounded-xl overflow-hidden bg-slate-900 border border-slate-800 shadow-xl aspect-[3/4] relative">
                <img 
                  v-if="selectedGameForDetail.cover_url" 
                  :src="selectedGameForDetail.cover_url" 
                  :alt="selectedGameForDetail.displayName"
                  class="w-full h-full object-cover" 
                  @error="(e: any) => { e.target.style.display = 'none'; if (e.target.nextElementSibling) e.target.nextElementSibling.style.display = 'flex'; }" />
                <div 
                  :style="{ display: selectedGameForDetail.cover_url ? 'none' : 'flex' }"
                  class="w-full h-full flex flex-col items-center justify-center p-6 text-center bg-gradient-to-br from-indigo-950 to-slate-900 text-slate-500">
                  <Gamepad2 class="w-16 h-16 text-indigo-400/50 mb-2" />
                  <span class="text-sm font-bold text-white">{{ selectedGameForDetail.displayName }}</span>
                </div>
              </div>

              <!-- Metadata Card (Screenshots 3 & 4) -->
              <div class="p-4 rounded-xl bg-slate-900/60 border border-slate-800 space-y-3 text-xs">
                <h3 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                  <Building2 class="w-3.5 h-3.5 text-indigo-400" />
                  <span>{{ t('detail.metadata_title') }}</span>
                </h3>

                <div class="space-y-2 pt-1 divide-y divide-slate-800/60">
                  <div v-if="selectedGameForDetail.release_date" class="pt-2 flex justify-between">
                    <span class="text-slate-400">{{ t('detail.release_date') }}</span>
                    <span class="font-medium text-slate-200">{{ selectedGameForDetail.release_date }}</span>
                  </div>

                  <div v-if="selectedGameForDetail.developer" class="pt-2 flex justify-between">
                    <span class="text-slate-400">{{ t('detail.developer') }}</span>
                    <span class="font-medium text-slate-200 text-right">{{ selectedGameForDetail.developer }}</span>
                  </div>

                  <div v-if="selectedGameForDetail.publisher" class="pt-2 flex justify-between">
                    <span class="text-slate-400">{{ t('detail.publisher') }}</span>
                    <span class="font-medium text-slate-200 text-right">{{ selectedGameForDetail.publisher }}</span>
                  </div>

                  <div v-if="selectedGameForDetail.steam_app_id" class="pt-2 flex justify-between items-center">
                    <span class="text-slate-400">{{ t('detail.steam_app_id') }}</span>
                    <span class="font-mono text-indigo-400 bg-indigo-950/60 px-1.5 py-0.5 rounded border border-indigo-500/20">
                      {{ selectedGameForDetail.steam_app_id }}
                    </span>
                  </div>

                  <div class="pt-2 flex justify-between items-center">
                    <span class="text-slate-400">{{ t('detail.process_name') }}</span>
                    <span class="font-mono text-slate-300 text-[11px] truncate max-w-[150px]">
                      {{ selectedGameForDetail.executable }}
                    </span>
                  </div>

                  <div v-if="selectedGameForDetail.genres && selectedGameForDetail.genres.length > 0" class="pt-2 space-y-1.5">
                    <span class="text-slate-400 block">{{ t('detail.genres_tags') }}</span>
                    <div class="flex flex-wrap gap-1">
                      <span 
                        v-for="genre in selectedGameForDetail.genres" 
                        :key="genre" 
                        class="px-2 py-0.5 rounded bg-slate-800 text-[10px] text-slate-300 border border-slate-700/60">
                        {{ genre }}
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <!-- Quick Stats Summary Box -->
              <div class="p-4 rounded-xl bg-slate-900/60 border border-slate-800 space-y-2.5 text-xs">
                <h3 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                  <Clock class="w-3.5 h-3.5 text-indigo-400" />
                  <span>{{ t('detail.stats_summary') }}</span>
                </h3>
                <div class="space-y-1.5 pt-1">
                  <div class="flex justify-between">
                    <span class="text-slate-400">{{ t('detail.total_playtime') }}</span>
                    <span class="font-bold text-indigo-300">{{ detailStats.totalPlaytime }}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-slate-400">{{ t('detail.last_session') }}</span>
                    <span class="text-slate-200">{{ detailStats.lastSessionDuration }}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="text-slate-400">{{ t('library.last_played') }}</span>
                    <span class="text-slate-400">{{ detailStats.lastPlayedDate }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- LIBRARY GRID VIEW (Screenshot 2) -->
        <div v-else class="space-y-4">
          <!-- Top Stats Bar -->
          <div class="grid grid-cols-3 gap-3">
            <div class="p-3.5 rounded-xl bg-slate-900/60 border border-slate-800">
              <span class="text-xs text-slate-400">{{ t('stats.total_time') }}</span>
              <div class="mt-1 text-xl font-bold text-white">
                {{ totalOverallPlaytimeHours }} <span class="text-xs font-normal text-slate-400">{{ t('stats.hours') }}</span>
              </div>
            </div>
            <div class="p-3.5 rounded-xl bg-slate-900/60 border border-slate-800">
              <span class="text-xs text-slate-400">{{ t('stats.tracked_games') }}</span>
              <div class="mt-1 text-xl font-bold text-white">
                {{ libraryItems.length }}
              </div>
            </div>
            <div class="p-3.5 rounded-xl bg-slate-900/60 border border-slate-800">
              <span class="text-xs text-slate-400">{{ t('stats.pending_events') }}</span>
              <div class="mt-1 text-xl font-bold flex items-baseline gap-2" :class="status.pending_events_count > 0 ? 'text-amber-400' : 'text-emerald-400'">
                {{ status.pending_events_count }}
                <span class="text-[11px] font-normal text-slate-400">
                  {{ status.pending_events_count > 0 ? t('stats.waiting_sync') : t('stats.all_synced') }}
                </span>
              </div>
            </div>
          </div>

          <!-- Library Filter Toolbar (Screenshot 2) -->
          <div class="p-3.5 rounded-xl bg-slate-900/60 border border-slate-800 flex flex-wrap items-center justify-between gap-3">
            <!-- Search Input -->
            <div class="relative flex-1 min-w-[200px]">
              <Search class="absolute left-3 top-2.5 w-3.5 h-3.5 text-slate-400" />
              <input 
                v-model="librarySearchQuery"
                type="text" 
                :placeholder="t('library.search_placeholder')"
                class="w-full pl-9 pr-3 py-1.5 bg-slate-800/80 border border-slate-700 rounded-lg text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500" />
            </div>

            <!-- Sort & Filters -->
            <div class="flex items-center gap-2 flex-wrap">
              <!-- Sort Select -->
              <div class="flex items-center gap-1.5 text-xs text-slate-400">
                <span>{{ t('library.sort_label') }}:</span>
                <select 
                  v-model="librarySort"
                  class="bg-slate-800 border border-slate-700 text-xs text-slate-200 rounded-lg px-2.5 py-1.5 focus:outline-none focus:border-indigo-500">
                  <option value="playtime">{{ t('library.sort_playtime') }}</option>
                  <option value="recent">{{ t('library.sort_recent') }}</option>
                  <option value="name">{{ t('library.sort_name') }}</option>
                </select>
              </div>

              <!-- Status Filter Select -->
              <div class="flex items-center gap-1.5 text-xs text-slate-400">
                <span>{{ t('library.filter_status') }}:</span>
                <select 
                  v-model="libraryStatusFilter"
                  class="bg-slate-800 border border-slate-700 text-xs text-slate-200 rounded-lg px-2.5 py-1.5 focus:outline-none focus:border-indigo-500">
                  <option value="all">{{ t('library.filter_all') }}</option>
                  <option value="played">{{ t('library.filter_played') }}</option>
                  <option value="monitored">{{ t('library.filter_monitored') }}</option>
                </select>
              </div>

              <!-- Genre Filter Select -->
              <div v-if="availableGenres.length > 0" class="flex items-center gap-1.5 text-xs text-slate-400">
                <span>{{ t('library.filter_genre') }}:</span>
                <select 
                  v-model="libraryGenreFilter"
                  class="bg-slate-800 border border-slate-700 text-xs text-slate-200 rounded-lg px-2.5 py-1.5 focus:outline-none focus:border-indigo-500">
                  <option value="all">{{ t('library.filter_all_genres') }}</option>
                  <option v-for="g in availableGenres" :key="g" :value="g">{{ g }}</option>
                </select>
              </div>
            </div>
          </div>

          <!-- Empty state -->
          <div v-if="filteredLibraryGames.length === 0" class="py-12 text-center text-slate-500 text-xs bg-slate-900/40 rounded-xl border border-slate-800">
            {{ t('library.no_games_found') }}
          </div>

          <!-- Game Cards Grid (Screenshot 2) -->
          <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3.5">
            <div 
              v-for="game in filteredLibraryGames" 
              :key="game.key"
              @click="openGameDetail(game)"
              class="group relative flex gap-3.5 p-3 rounded-xl bg-slate-900/60 hover:bg-slate-900/90 border border-slate-800 hover:border-indigo-500/50 cursor-pointer transition-all duration-200 shadow-sm hover:shadow-indigo-500/10 hover:shadow-lg">
              
              <!-- Game Poster / Cover -->
              <div class="w-20 h-28 sm:w-24 sm:h-32 shrink-0 rounded-lg overflow-hidden bg-slate-800 border border-slate-700/60 relative">
                <img 
                  v-if="game.cover_url" 
                  :src="game.cover_url" 
                  :alt="game.displayName"
                  loading="lazy"
                  class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300" 
                  @error="(e: any) => { e.target.style.display = 'none'; if (e.target.nextElementSibling) e.target.nextElementSibling.style.display = 'flex'; }" />
                <div 
                  :style="{ display: game.cover_url ? 'none' : 'flex' }"
                  class="w-full h-full flex flex-col items-center justify-center p-2 text-center bg-gradient-to-br from-indigo-950/60 to-slate-900 text-slate-500">
                  <Gamepad2 class="w-7 h-7 text-indigo-400/60 mb-1" />
                  <span class="text-[9px] font-medium leading-tight text-slate-400 line-clamp-2">{{ game.displayName }}</span>
                </div>

                <!-- Active live badge on poster -->
                <div v-if="game.is_active" class="absolute top-1.5 left-1.5 flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-emerald-500 text-slate-950 text-[9px] font-bold shadow-md animate-pulse">
                  <span class="h-1.5 w-1.5 rounded-full bg-slate-950"></span>
                  <span>LIVE</span>
                </div>
              </div>

              <!-- Game Info -->
              <div class="flex-1 min-w-0 flex flex-col justify-between py-0.5">
                <div>
                  <!-- Title & Badges -->
                  <div class="flex items-start justify-between gap-1.5">
                    <h3 class="text-xs sm:text-sm font-bold text-white group-hover:text-indigo-300 transition truncate">
                      {{ game.displayName }}
                    </h3>
                    <span 
                      v-if="game.is_monitored" 
                      class="shrink-0 text-[9px] px-1.5 py-0.5 rounded bg-indigo-950/80 text-indigo-300 border border-indigo-500/30 font-medium">
                      {{ t('detail.watching_active') }}
                    </span>
                  </div>

                  <!-- Process name -->
                  <div class="text-[10px] font-mono text-slate-500 truncate mt-0.5">
                    {{ game.executable }}
                  </div>

                  <!-- Genres Tags -->
                  <div v-if="game.genres && game.genres.length > 0" class="flex flex-wrap gap-1 mt-1.5">
                    <span 
                      v-for="genre in game.genres.slice(0, 3)" 
                      :key="genre" 
                      class="px-1.5 py-0.5 rounded bg-slate-800/80 text-[10px] text-slate-300">
                      {{ genre }}
                    </span>
                  </div>

                  <!-- Description snippet -->
                  <p v-if="game.short_description || game.description" class="text-[11px] text-slate-400 line-clamp-2 mt-1.5 leading-relaxed">
                    {{ game.short_description || game.description }}
                  </p>
                </div>

                <!-- Footer: Playtime & Last Played (Screenshot 2) -->
                <div class="flex items-center justify-between pt-2 border-t border-slate-800/60 mt-2 text-xs">
                  <div class="flex items-center gap-1.5 font-semibold text-indigo-300 bg-indigo-950/40 px-2 py-0.5 rounded border border-indigo-500/20 text-[11px]">
                    <Clock class="w-3 h-3 text-indigo-400" />
                    <span>{{ game.formatted_playtime }}</span>
                  </div>
                  <div class="text-[10px] text-slate-400 flex items-center gap-1">
                    <span>{{ formatRelativeTime(game.last_played_timestamp) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 2. SESSIONS TAB -->
      <div v-if="activeTab === 'sessions'" class="space-y-3">
        <div class="flex items-center justify-between">
          <h2 class="text-xs font-semibold text-white uppercase tracking-wider">{{ t('sessions.title') }}</h2>
          <span class="text-[11px] text-slate-400">{{ t('sessions.subtitle') }}</span>
        </div>

        <div class="rounded-lg border border-slate-800 overflow-hidden bg-slate-900/40">
          <table class="w-full text-left text-xs">
            <thead class="bg-slate-900/80 text-slate-400 uppercase tracking-wider text-[10px] border-b border-slate-800">
              <tr>
                <th class="py-2.5 px-3.5">{{ t('sessions.game') }}</th>
                <th class="py-2.5 px-3.5">{{ t('sessions.start') }}</th>
                <th class="py-2.5 px-3.5">{{ t('sessions.duration') }}</th>
                <th class="py-2.5 px-3.5 text-right">{{ t('sessions.status') }}</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800/60">
              <tr v-if="recentSessions.length === 0">
                <td colspan="4" class="py-6 text-center text-slate-500">
                  {{ t('sessions.no_sessions') }}
                </td>
              </tr>
              <tr v-for="s in recentSessions" :key="s.id" class="hover:bg-slate-800/30 transition">
                <td class="py-2.5 px-3.5">
                  <div class="font-medium text-slate-200">{{ gameMetadata.getDisplayName(s.game) }}</div>
                  <div class="text-[10px] font-mono text-slate-500">{{ s.game }}</div>
                </td>
                <td class="py-2.5 px-3.5 text-slate-400">{{ formatDate(s.start_time) }}</td>
                <td class="py-2.5 px-3.5 font-mono text-slate-300">{{ formatDuration(s.duration_seconds) }}</td>
                <td class="py-2.5 px-3.5 text-right">
                  <span 
                    class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px]"
                    :class="s.is_synced ? 'bg-emerald-950/60 text-emerald-400' : 'bg-slate-800 text-slate-400'">
                    <CheckCircle2 v-if="s.is_synced" class="w-3 h-3" />
                    <span>{{ s.is_synced ? t('sessions.synced') : t('sessions.local') }}</span>
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- 3. SETTINGS TAB: Ergonomic 2-Column Balanced Layout -->
      <div v-if="activeTab === 'settings'" class="grid grid-cols-1 lg:grid-cols-2 gap-5 items-start">
        <!-- COLUMN 1: Account, Cloud Sync & General -->
        <div class="space-y-4">
          <!-- Cloud Account Card with Login/Logout -->
          <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 space-y-3.5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <Cloud class="w-4 h-4 text-indigo-400" />
                <h2 class="text-xs font-semibold text-white uppercase tracking-wider">{{ t('settings.cloud_title') }}</h2>
              </div>
              <span 
                class="text-[10px] px-2 py-0.5 rounded-full font-medium"
                :class="status.api_enabled ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-500/30' : 'bg-slate-800 text-slate-400'">
                {{ status.api_enabled ? t('settings.logged_in') : t('settings.not_logged_in') }}
              </span>
            </div>

            <!-- Connected state -->
            <div v-if="status.api_enabled" class="space-y-3">
              <div class="p-3 rounded-md bg-emerald-950/20 border border-emerald-500/20 text-xs text-emerald-300 flex items-center justify-between">
                <div class="truncate mr-2">
                  <div class="font-medium">{{ t('settings.connected_to', { url: formConfig.api_base_url }) }}</div>
                </div>
                <button 
                  @click="handleLogout"
                  class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-xs text-slate-200 border border-slate-700 flex items-center gap-1 shrink-0 transition">
                  <LogOut class="w-3 h-3 text-rose-400" />
                  <span>{{ t('settings.logout_btn') }}</span>
                </button>
              </div>
            </div>

            <!-- Not connected state: Simple Login Call-to-Action -->
            <div v-else class="space-y-3">
              <p class="text-xs text-slate-400 leading-relaxed">
                {{ t('settings.local_mode_hint') }}
              </p>
              <button 
                @click="showLoginModal = true"
                class="w-full py-2 bg-indigo-600 hover:bg-indigo-500 text-xs font-medium text-white rounded-md transition shadow flex items-center justify-center gap-2">
                <LogIn class="w-3.5 h-3.5" />
                <span>{{ t('settings.login_btn') }}</span>
              </button>
            </div>

            <!-- Advanced Server Settings (Collapsible) -->
            <div class="pt-2 border-t border-slate-800/80">
              <button 
                @click="showAdvancedConfig = !showAdvancedConfig"
                class="text-xs text-slate-400 hover:text-slate-200 flex items-center justify-between w-full py-1">
                <span>{{ t('settings.advanced_config') }}</span>
                <ChevronDown v-if="!showAdvancedConfig" class="w-3.5 h-3.5" />
                <ChevronUp v-else class="w-3.5 h-3.5" />
              </button>

              <div v-if="showAdvancedConfig" class="mt-3 space-y-3 pt-2">
                <div class="space-y-1">
                  <label class="text-[11px] font-medium text-slate-400">{{ t('settings.api_url') }}</label>
                  <input 
                    v-model="formConfig.api_base_url" 
                    type="text" 
                    class="w-full px-2.5 py-1 bg-slate-800/80 border border-slate-700 rounded text-xs font-mono text-slate-200 focus:outline-none focus:border-indigo-500" />
                </div>

                <div class="space-y-1">
                  <label class="text-[11px] font-medium text-slate-400">{{ t('settings.api_token') }}</label>
                  <div class="relative">
                    <input 
                      v-model="formConfig.api_token" 
                      :type="showToken ? 'text' : 'password'" 
                      :placeholder="t('settings.api_token_placeholder')"
                      class="w-full pl-2.5 pr-8 py-1 bg-slate-800/80 border border-slate-700 rounded text-xs font-mono text-slate-200 focus:outline-none focus:border-indigo-500" />
                    <button 
                      type="button" 
                      @click="showToken = !showToken"
                      class="absolute right-2 top-1.5 text-slate-400 hover:text-slate-200">
                      <EyeOff v-if="showToken" class="w-3 h-3" />
                      <Eye v-else class="w-3 h-3" />
                    </button>
                  </div>
                </div>

                <div 
                  v-if="apiTestResult" 
                  class="p-2 rounded text-xs flex items-center gap-1.5 border"
                  :class="apiTestResult.success ? 'bg-emerald-950/40 border-emerald-500/30 text-emerald-300' : 'bg-rose-950/40 border-rose-500/30 text-rose-300'">
                  <span>{{ apiTestResult.message }}</span>
                </div>

                <div class="flex items-center justify-between pt-1">
                  <button 
                    type="button"
                    @click="handleTestApi"
                    :disabled="isTestingApi || !formConfig.api_token"
                    class="px-2.5 py-1 bg-slate-800 hover:bg-slate-700 text-xs text-slate-300 rounded border border-slate-700 transition disabled:opacity-40">
                    {{ isTestingApi ? t('settings.testing') : t('settings.test_connection') }}
                  </button>
                  <button 
                    type="button"
                    @click="handleSaveSettings"
                    :disabled="isSaving"
                    class="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 text-xs font-medium text-white rounded transition disabled:opacity-50">
                    {{ saveSuccess ? t('settings.saved') : t('settings.save_config') }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- General & Language Card -->
          <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 space-y-3">
            <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
              <Globe class="w-4 h-4 text-indigo-400" />
              <span>{{ t('settings.general_title') }}</span>
            </h2>

            <div class="flex items-center justify-between pt-1">
              <label class="text-xs text-slate-300">{{ t('settings.language') }}</label>
              <select 
                :value="currentLanguageChoice"
                @change="e => setLanguage((e.target as HTMLSelectElement).value as LanguageChoice)"
                class="bg-slate-800 border border-slate-700 text-xs text-slate-200 rounded px-2.5 py-1 focus:outline-none focus:border-indigo-500">
                <option value="system">{{ t('settings.lang_system') }}</option>
                <option value="de">{{ t('settings.lang_de') }}</option>
                <option value="en">{{ t('settings.lang_en') }}</option>
              </select>
            </div>

            <!-- Autostart Row -->
            <div class="pt-3 border-t border-slate-800/80 flex items-center justify-between">
              <div>
                <div class="text-xs font-medium text-slate-200">{{ t('settings.autostart') }}</div>
                <div class="text-[11px] text-slate-400">{{ t('settings.autostart_desc') }}</div>
              </div>
              <button 
                type="button"
                @click="toggleAutostart"
                :disabled="isUpdatingAutostart"
                :class="[
                  isAutostart ? 'bg-indigo-600' : 'bg-slate-800 border border-slate-700',
                  'relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors duration-200 ease-in-out focus:outline-none disabled:opacity-50'
                ]">
                <span 
                  :class="[
                    isAutostart ? 'translate-x-4 bg-white' : 'translate-x-0.5 bg-slate-400',
                    'pointer-events-none inline-block h-4 w-4 transform rounded-full shadow ring-0 transition duration-200 ease-in-out mt-0.5'
                  ]" 
                />
              </button>
            </div>

            <!-- In-App Updater Row -->
            <div class="pt-3 border-t border-slate-800/80 space-y-2">
              <div class="flex items-center justify-between">
                <div>
                  <div class="text-[11px] text-slate-400">{{ t('updater.current_version') }}</div>
                  <div class="text-xs font-mono text-slate-200">{{ appVersion ? `v${appVersion}` : '...' }}</div>
                </div>
                <button 
                  @click="handleCheckForUpdates" 
                  :disabled="isCheckingUpdates"
                  class="px-2.5 py-1 bg-slate-800 hover:bg-slate-700 text-xs text-slate-300 border border-slate-700 rounded transition disabled:opacity-50">
                  {{ isCheckingUpdates ? t('updater.checking') : t('updater.check_btn') }}
                </button>
              </div>

              <!-- Update notification / feedback -->
              <div v-if="updateStatusText" class="text-[11px] text-slate-400 bg-slate-800/50 p-2 rounded">
                {{ updateStatusText }}
              </div>

              <div v-if="updateInfo" class="p-2.5 rounded bg-emerald-950/30 border border-emerald-500/30 text-xs space-y-2">
                <div class="flex items-center justify-between text-emerald-300">
                  <span class="font-medium">{{ t('updater.update_available', { version: updateInfo.version }) }}</span>
                  <button 
                    @click="handleInstallUpdate"
                    :disabled="isDownloadingUpdate"
                    class="px-2.5 py-1 rounded bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-[11px] transition disabled:opacity-50">
                    {{ isDownloadingUpdate ? t('updater.installing') : t('updater.install_btn') }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Playtime Import Card -->
          <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 space-y-3">
            <div>
              <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                <History class="w-4 h-4 text-indigo-400" />
                <span>{{ t('settings.import_title') }}</span>
              </h2>
              <p class="text-[11px] text-slate-400 mt-0.5">{{ t('settings.import_subtitle') }}</p>
            </div>

            <button 
              @click="startPlaytimeScan"
              class="w-full py-2 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-indigo-300 border border-indigo-500/30 rounded-md transition flex items-center justify-center gap-2 shadow-sm">
              <Download class="w-3.5 h-3.5 text-indigo-400" />
              <span>{{ t('settings.import_btn') }}</span>
            </button>
          </div>

          <!-- Backup & Migration Card -->
          <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 space-y-3">
            <div>
              <h2 class="text-xs font-semibold text-white uppercase tracking-wider flex items-center gap-2">
                <HardDrive class="w-4 h-4 text-indigo-400" />
                <span>{{ t('settings.backup_title') }}</span>
              </h2>
              <p class="text-[11px] text-slate-400 mt-0.5">{{ t('settings.backup_subtitle') }}</p>
            </div>

            <div class="grid grid-cols-2 gap-2 pt-1">
              <button 
                @click="handleExportBackup"
                :disabled="isExportingBackup"
                class="py-2 px-3 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 border border-slate-700 rounded-md transition flex items-center justify-center gap-2 shadow-sm disabled:opacity-50">
                <Upload class="w-3.5 h-3.5 text-slate-400" :class="{ 'animate-pulse': isExportingBackup }" />
                <span>{{ isExportingBackup ? t('backup.exporting') : t('settings.export_btn') }}</span>
              </button>

              <button 
                @click="handleImportBackup"
                :disabled="isImportingBackup"
                class="py-2 px-3 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 border border-slate-700 rounded-md transition flex items-center justify-center gap-2 shadow-sm disabled:opacity-50">
                <Download class="w-3.5 h-3.5 text-slate-400" :class="{ 'animate-pulse': isImportingBackup }" />
                <span>{{ isImportingBackup ? t('backup.importing') : t('settings.import_backup_btn') }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- COLUMN 2: Monitored Games Management with Scanner & Multi-Selection -->
        <div class="space-y-4">
          <div class="p-4 rounded-lg bg-slate-900/60 border border-slate-800 space-y-3">
            <!-- Header & PC Scanner Action -->
            <div class="flex items-center justify-between">
              <div>
                <h2 class="text-xs font-semibold text-white uppercase tracking-wider">
                  {{ t('settings.games_title', { count: formConfig.watched_games.length }) }}
                </h2>
                <p class="text-[11px] text-slate-400">{{ t('settings.games_subtitle') }}</p>
              </div>

              <!-- PC Scan Button -->
              <button 
                @click="startPcScan"
                class="px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 border border-slate-700 rounded-md transition flex items-center gap-1.5 shadow-sm">
                <Search class="w-3.5 h-3.5 text-slate-400" />
                <span>{{ t('settings.scan_pc') }}</span>
              </button>
            </div>

            <!-- Single Game Add Input & File Picker -->
            <div class="flex gap-2 pt-1">
              <input 
                v-model="newGameInput"
                @keyup.enter="handleAddSingleGame"
                type="text" 
                :placeholder="t('settings.add_placeholder')"
                class="flex-1 px-2.5 py-1.5 bg-slate-800/80 border border-slate-700 rounded-md text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500" />
              <button 
                @click="handleAddSingleGame"
                :disabled="!newGameInput.trim()"
                class="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-200 border border-slate-700 rounded-md transition flex items-center gap-1 disabled:opacity-40">
                <Plus class="w-3.5 h-3.5" />
                <span>{{ t('settings.add_single') }}</span>
              </button>
              <button 
                @click="handleBrowseGameFile"
                class="px-2.5 py-1.5 bg-slate-800 hover:bg-slate-700 text-xs font-medium text-indigo-300 border border-indigo-500/30 rounded-md transition flex items-center gap-1.5 shrink-0 shadow-sm"
                :title="t('settings.browse_file')">
                <FolderOpen class="w-3.5 h-3.5 text-indigo-400" />
                <span>{{ t('settings.browse_file') }}</span>
              </button>
            </div>

            <!-- Filter & Multi-Selection Toolbar -->
            <div class="flex items-center justify-between gap-2 pt-2 border-t border-slate-800/60">
              <!-- Select All Checkbox -->
              <div class="flex items-center gap-2">
                <button 
                  @click="toggleMonitoredSelectAll"
                  :disabled="filteredWatchedGames.length === 0"
                  class="flex items-center gap-1.5 text-xs text-slate-400 hover:text-slate-200 transition disabled:opacity-30">
                  <CheckSquare v-if="allFilteredMonitoredSelected" class="w-3.5 h-3.5 text-indigo-400" />
                  <Square v-else class="w-3.5 h-3.5" />
                  <span>{{ t('settings.select_all') }}</span>
                </button>

                <!-- Bulk Remove Button -->
                <button 
                  v-if="selectedMonitoredGames.size > 0"
                  @click="handleRemoveSelectedGames"
                  class="px-2 py-0.5 rounded bg-rose-950/60 hover:bg-rose-900/80 text-rose-300 border border-rose-500/40 text-[11px] font-medium flex items-center gap-1 transition">
                  <Trash2 class="w-3 h-3" />
                  <span>{{ t('settings.delete_selected', { count: selectedMonitoredGames.size }) }}</span>
                </button>
              </div>

              <!-- Filter Search -->
              <div class="relative w-40">
                <Search class="w-3 h-3 absolute left-2 top-2 text-slate-400" />
                <input 
                  v-model="gameSearchQuery"
                  type="text" 
                  :placeholder="t('settings.filter_placeholder')"
                  class="w-full pl-6 pr-2 py-1 bg-slate-800 border border-slate-700 rounded text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500" />
              </div>
            </div>

            <!-- Games List with Checkboxes -->
            <div v-if="filteredWatchedGames.length === 0" class="py-10 text-center text-slate-500 text-xs">
              {{ t('settings.no_games') }}
            </div>

            <div v-else class="space-y-1.5 max-h-64 overflow-y-auto pr-1">
              <div 
                v-for="game in filteredWatchedGames" 
                :key="game"
                class="px-2.5 py-1.5 rounded-md bg-slate-800/40 border border-slate-700/40 text-xs text-slate-300 flex items-center justify-between hover:bg-slate-800/80 transition">
                <div class="flex items-center gap-2 truncate">
                  <!-- Checkbox -->
                  <input 
                    type="checkbox"
                    :checked="selectedMonitoredGames.has(game)"
                    @change="toggleGameSelection(game)"
                    class="rounded bg-slate-800 border-slate-700 text-indigo-600 focus:ring-0 focus:ring-offset-0 w-3.5 h-3.5 cursor-pointer" />
                  <span class="truncate font-mono text-[11px]">{{ game }}</span>
                  <span v-if="status.active_game === game" class="text-[9px] px-1 rounded bg-emerald-500/20 text-emerald-400 font-bold shrink-0">
                    {{ t('settings.active_badge') }}
                  </span>
                </div>
                <button 
                  @click="handleRemoveSingleGame(game)"
                  class="text-slate-500 hover:text-rose-400 p-0.5 transition"
                  title="Entfernen">
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- LOGIN MODAL -->
    <div v-if="showLoginModal" class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="w-full max-w-md bg-slate-900 border border-slate-800 rounded-xl p-5 space-y-4 shadow-2xl">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Cloud class="w-4 h-4 text-indigo-400" />
            <h2 class="text-sm font-semibold text-white">{{ t('login.title') }}</h2>
          </div>
          <button @click="showLoginModal = false" class="text-slate-400 hover:text-white">
            <X class="w-4 h-4" />
          </button>
        </div>

        <p class="text-xs text-slate-400 leading-relaxed">
          {{ t('login.subtitle') }}
        </p>

        <div class="space-y-3">
          <!-- Token Input -->
          <div class="space-y-1">
            <label class="text-[11px] font-medium text-slate-300">{{ t('login.token_label') }}</label>
            <input 
              v-model="loginTokenInput" 
              type="password" 
              placeholder="z.B. 9wdad9..."
              class="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-md text-xs font-mono text-slate-200 focus:outline-none focus:border-indigo-500" />
            <p class="text-[10px] text-slate-500">{{ t('login.token_hint') }}</p>
          </div>

          <!-- Custom URL -->
          <div class="space-y-1">
            <label class="text-[11px] font-medium text-slate-400">{{ t('login.advanced_url') }}</label>
            <input 
              v-model="loginUrlInput" 
              type="text" 
              class="w-full px-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs font-mono text-slate-300 focus:outline-none focus:border-indigo-500" />
          </div>

          <!-- Error Alert -->
          <div v-if="loginError" class="p-2.5 rounded-md bg-rose-950/60 border border-rose-500/40 text-rose-300 text-xs">
            {{ loginError }}
          </div>
        </div>

        <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
          <button 
            type="button" 
            @click="showLoginModal = false"
            class="px-3 py-1.5 rounded-md text-xs text-slate-400 hover:text-white">
            {{ t('login.cancel') }}
          </button>
          <button 
            type="button" 
            @click="handleLogin"
            :disabled="isLoggingIn || !loginTokenInput.trim()"
            class="px-4 py-1.5 rounded-md bg-indigo-600 hover:bg-indigo-500 text-xs font-medium text-white transition flex items-center gap-1.5 disabled:opacity-50">
            <RefreshCw v-if="isLoggingIn" class="w-3.5 h-3.5 animate-spin" />
            <span>{{ isLoggingIn ? 'Verbinde...' : t('login.submit') }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- PC GAME SCANNER MODAL -->
    <div v-if="showScannerModal" class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="w-full max-w-xl bg-slate-900 border border-slate-800 rounded-xl p-5 space-y-4 shadow-2xl flex flex-col max-h-[85vh]">
        <!-- Modal Header -->
        <div class="flex items-center justify-between shrink-0">
          <div>
            <h2 class="text-sm font-semibold text-white">{{ t('scanner.title') }}</h2>
            <p class="text-[11px] text-slate-400">{{ t('scanner.subtitle') }}</p>
          </div>
          <button @click="cancelScan" class="text-slate-400 hover:text-white p-1 rounded hover:bg-slate-800 transition">
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Scanning State -->
        <div v-if="isScanning" class="py-12 flex flex-col items-center justify-center gap-3 text-slate-400 text-xs">
          <div class="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
          <span class="text-slate-300 font-medium">{{ t('scanner.scanning') }}</span>
          <button 
            type="button" 
            @click="cancelScan"
            class="mt-2 px-3 py-1.5 rounded-md bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-300 border border-slate-700 transition">
            {{ t('scanner.cancel') }}
          </button>
        </div>

        <!-- Scanner Content -->
        <div v-else class="flex-1 flex flex-col min-h-0 space-y-3">
          <!-- Toolbar -->
          <div class="flex items-center justify-between gap-3 shrink-0">
            <button 
              @click="toggleScannerSelectAll"
              class="text-xs text-indigo-400 hover:text-indigo-300 font-medium">
              {{ selectedScannerGames.size === filteredScannerGames.length ? 'Auswahl aufheben' : 'Alle auswählen' }}
            </button>

            <!-- Search -->
            <div class="relative w-56">
              <Search class="w-3 h-3 absolute left-2.5 top-2 text-slate-400" />
              <input 
                v-model="scannerSearch"
                type="text" 
                :placeholder="t('scanner.search')"
                class="w-full pl-7 pr-2.5 py-1 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500" />
            </div>
          </div>

          <!-- Discovered List -->
          <div class="flex-1 overflow-y-auto border border-slate-800 rounded-lg p-2 space-y-1 bg-slate-950/40">
            <div v-if="filteredScannerGames.length === 0" class="py-8 text-center text-slate-500 text-xs">
              {{ t('scanner.no_results') }}
            </div>

            <div 
              v-for="game in filteredScannerGames" 
              :key="game.executable"
              class="px-3 py-2 rounded-md bg-slate-800/40 border border-slate-800 flex items-center justify-between text-xs hover:bg-slate-800/70 transition">
              <div class="flex items-center gap-2.5 truncate">
                <input 
                  type="checkbox"
                  :checked="selectedScannerGames.has(game.executable)"
                  @change="selectedScannerGames.has(game.executable) ? selectedScannerGames.delete(game.executable) : selectedScannerGames.add(game.executable)"
                  class="rounded bg-slate-800 border-slate-700 text-indigo-600 focus:ring-0 focus:ring-offset-0 w-3.5 h-3.5 cursor-pointer" />
                <div class="truncate">
                  <div class="font-medium text-slate-200 truncate">{{ game.name }}</div>
                  <div class="text-[10px] font-mono text-slate-400 truncate">{{ game.executable }} &bull; {{ game.source }}</div>
                </div>
              </div>

              <span v-if="isGameAlreadyTracked(game.executable)" class="text-[9px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 shrink-0">
                {{ t('scanner.already_added') }}
              </span>
            </div>
          </div>
        </div>

        <!-- Modal Actions (only shown once scan is completed) -->
        <div v-if="!isScanning" class="flex items-center justify-between pt-2 border-t border-slate-800 shrink-0">
          <span class="text-xs text-slate-400">
            {{ t('scanner.found_count', { count: discoveredGames.length }) }}
          </span>

          <div class="flex gap-2">
            <button 
              type="button" 
              @click="cancelScan"
              class="px-3 py-1.5 rounded-md text-xs text-slate-400 hover:text-white">
              {{ t('scanner.cancel') }}
            </button>
            <button 
              type="button" 
              @click="handleApplyScannerSelection"
              :disabled="selectedScannerGames.size === 0"
              class="px-4 py-1.5 rounded-md bg-indigo-600 hover:bg-indigo-500 text-xs font-medium text-white transition disabled:opacity-50">
              {{ t('scanner.add_selected', { count: selectedScannerGames.size }) }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- PLAYTIME IMPORTER MODAL -->
    <div v-if="showImporterModal" class="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="w-full max-w-xl bg-slate-900 border border-slate-800 rounded-xl p-5 space-y-4 shadow-2xl flex flex-col max-h-[85vh]">
        <!-- Modal Header -->
        <div class="flex items-center justify-between shrink-0">
          <div>
            <h2 class="text-sm font-semibold text-white">{{ t('importer.title') }}</h2>
            <p class="text-[11px] text-slate-400">{{ t('importer.subtitle') }}</p>
          </div>
          <button @click="cancelImportScan" class="text-slate-400 hover:text-white p-1 rounded hover:bg-slate-800 transition">
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Scanning State -->
        <div v-if="isScanningImports" class="py-12 flex flex-col items-center justify-center gap-3 text-slate-400 text-xs">
          <div class="w-6 h-6 border-2 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
          <span class="text-slate-300 font-medium">{{ t('importer.scanning') }}</span>
          <button 
            type="button" 
            @click="cancelImportScan"
            class="mt-2 px-3 py-1.5 rounded-md bg-slate-800 hover:bg-slate-700 text-xs font-medium text-slate-300 border border-slate-700 transition">
            {{ t('importer.cancel') }}
          </button>
        </div>

        <!-- Importer Content -->
        <div v-else class="flex-1 flex flex-col min-h-0 space-y-3">
          <!-- Toolbar & Options -->
          <div class="flex items-center justify-between gap-3 shrink-0">
            <button 
              @click="toggleImportSelectAll"
              class="text-xs text-indigo-400 hover:text-indigo-300 font-medium">
              {{ selectedImportKeys.size === filteredImportCandidates.length ? 'Auswahl aufheben' : t('importer.select_all') }}
            </button>

            <!-- Search -->
            <div class="relative w-56">
              <Search class="w-3 h-3 absolute left-2.5 top-2 text-slate-400" />
              <input 
                v-model="importSearch"
                type="text" 
                :placeholder="t('importer.search')"
                class="w-full pl-7 pr-2.5 py-1 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500" />
            </div>
          </div>

          <!-- Add to watched checkbox -->
          <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer select-none bg-slate-800/40 border border-slate-800/60 p-2 rounded-md">
            <input 
              type="checkbox" 
              v-model="importAddToWatched"
              class="rounded bg-slate-800 border-slate-700 text-indigo-600 focus:ring-0 w-3.5 h-3.5 cursor-pointer" />
            <span>{{ t('importer.add_to_watched') }}</span>
          </label>

          <!-- Candidate List -->
          <div class="flex-1 overflow-y-auto border border-slate-800 rounded-lg p-2 space-y-1 bg-slate-950/40">
            <div v-if="filteredImportCandidates.length === 0" class="py-8 text-center text-slate-500 text-xs">
              {{ t('importer.no_results') }}
            </div>

            <div 
              v-for="candidate in filteredImportCandidates" 
              :key="`${candidate.executable}:::${candidate.source}`"
              class="px-3 py-2 rounded-md bg-slate-800/40 border border-slate-800 flex items-center justify-between text-xs hover:bg-slate-800/70 transition">
              <div class="flex items-center gap-2.5 truncate mr-3">
                <input 
                  type="checkbox"
                  :checked="selectedImportKeys.has(`${candidate.executable}:::${candidate.source}`)"
                  @change="selectedImportKeys.has(`${candidate.executable}:::${candidate.source}`) ? selectedImportKeys.delete(`${candidate.executable}:::${candidate.source}`) : selectedImportKeys.add(`${candidate.executable}:::${candidate.source}`)"
                  class="rounded bg-slate-800 border-slate-700 text-indigo-600 focus:ring-0 focus:ring-offset-0 w-3.5 h-3.5 cursor-pointer shrink-0" />
                <div class="truncate">
                  <div class="font-medium text-slate-200 truncate">{{ candidate.game_title }}</div>
                  <div class="text-[10px] font-mono text-slate-400 truncate">{{ candidate.executable }}</div>
                </div>
              </div>

              <div class="flex items-center gap-2.5 shrink-0">
                <span 
                  class="text-[9px] px-1.5 py-0.5 rounded font-medium border"
                  :class="candidate.source.includes('AMD') ? 'bg-red-950/50 text-red-300 border-red-500/30' : 'bg-sky-950/50 text-sky-300 border-sky-500/30'">
                  {{ candidate.source }}
                </span>
                <span class="font-mono font-medium text-indigo-300 text-xs">
                  {{ candidate.formatted_time }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Modal Actions (only shown once scan is completed) -->
        <div v-if="!isScanningImports" class="flex items-center justify-between pt-2 border-t border-slate-800 shrink-0">
          <span class="text-xs text-slate-400">
            {{ t('importer.found_count', { count: importCandidates.length }) }}
          </span>

          <div class="flex gap-2">
            <button 
              type="button" 
              @click="cancelImportScan"
              class="px-3 py-1.5 rounded-md text-xs text-slate-400 hover:text-white">
              {{ t('importer.cancel') }}
            </button>
            <button 
              type="button" 
              @click="handleApplyImports"
              :disabled="selectedImportKeys.size === 0 || isApplyingImport"
              class="px-4 py-1.5 rounded-md bg-indigo-600 hover:bg-indigo-500 text-xs font-medium text-white transition flex items-center gap-1.5 disabled:opacity-50">
              <RefreshCw v-if="isApplyingImport" class="w-3.5 h-3.5 animate-spin" />
              <span>{{ t('importer.import_selected', { count: selectedImportKeys.size }) }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Screenshot Preview Lightbox Modal -->
    <div 
      v-if="previewScreenshot" 
      @click="previewScreenshot = null"
      class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="relative max-w-5xl max-h-[90vh] bg-slate-900 rounded-xl overflow-hidden border border-slate-800 shadow-2xl" @click.stop>
        <button 
          @click="previewScreenshot = null" 
          class="absolute top-3 right-3 p-1.5 rounded-full bg-slate-950/80 text-white hover:bg-slate-800 transition z-10">
          <X class="w-4 h-4" />
        </button>
        <img :src="previewScreenshot" class="max-w-full max-h-[85vh] object-contain rounded-lg" />
      </div>
    </div>
  </div>
</template>
