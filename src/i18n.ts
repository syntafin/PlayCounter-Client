import { ref, computed } from 'vue'

export type LanguageChoice = 'system' | 'de' | 'en'

const STORAGE_KEY = 'playcounter_language'

const messages = {
  de: {
    // Navigation
    'nav.stats': 'Statistiken & Spiele',
    'nav.sessions': 'Sitzungen',
    'nav.settings': 'Einstellungen',

    // Status & Header
    'status.tracking': 'Tracking: {game}',
    'status.idle': 'Im Leerlauf',
    'status.sync': 'Synchronisieren',
    'status.syncing': 'Synchronisiere...',

    // Active Game Banner
    'active.game_running': 'Spiel läuft gerade',
    'active.current_playtime': 'Aktuelle Spielzeit',

    // Stats
    'stats.total_time': 'Gesamtspielzeit',
    'stats.hours': 'Stunden',
    'stats.tracked_games': 'Erfasste Spiele',
    'stats.pending_events': 'Ausstehende Synchronisation',
    'stats.waiting_sync': 'Warten auf Übertragung',
    'stats.all_synced': 'Vollständig synchronisiert',
    'stats.top_games': 'Meistgespielte Titel',
    'stats.local_db': 'Lokal in SQLite erfasst',
    'stats.no_data': 'Noch keine Spielzeiten erfasst. Starte ein überwachtes Spiel, um das Tracking zu beginnen.',

    // Sessions
    'sessions.title': 'Letzte Gaming-Sitzungen',
    'sessions.subtitle': 'Die letzten 20 Sitzungen',
    'sessions.game': 'Spiel',
    'sessions.start': 'Startzeit',
    'sessions.duration': 'Dauer',
    'sessions.status': 'Cloud-Status',
    'sessions.synced': 'Synchronisiert',
    'sessions.local': 'Nur lokal',
    'sessions.no_sessions': 'Keine beendeten Sitzungen vorhanden.',

    // Settings: Account & Cloud
    'settings.cloud_title': 'Cloud-Konto & Synchronisation',
    'settings.logged_in': 'Angemeldet',
    'settings.not_logged_in': 'Nicht angemeldet',
    'settings.login_btn': 'Anmelden',
    'settings.logout_btn': 'Abmelden',
    'settings.connected_to': 'Verbunden mit: {url}',
    'settings.local_mode_hint': 'Du nutzt PlayCounter aktuell im rein lokalen Modus. Deine Daten bleiben sicher auf diesem PC.',
    'settings.advanced_config': 'Erweiterte Server-Einstellungen',
    'settings.api_url': 'Server-Adresse / Endpunkt',
    'settings.api_token': 'API-Schlüssel / Token',
    'settings.api_token_placeholder': 'Schlüssel hier eingeben...',
    'settings.test_connection': 'Verbindung prüfen',
    'settings.testing': 'Prüfe Verbindung...',
    'settings.test_success': 'Verbindung erfolgreich! {count} Spiele gefunden.',
    'settings.test_failed': 'Verbindungsfehler: {error}',
    'settings.save_config': 'Speichern',
    'settings.saved': 'Gespeichert!',

    // Settings: General
    'settings.general_title': 'Allgemein',
    'settings.language': 'Sprache',
    'settings.lang_system': 'Systemsprache (Standard)',
    'settings.lang_de': 'Deutsch',
    'settings.lang_en': 'English',

    // Updater
    'updater.current_version': 'Installierte Version',
    'updater.check_btn': 'Nach Updates suchen',
    'updater.checking': 'Suche...',
    'updater.up_to_date': 'PlayCounter ist auf dem neuesten Stand.',
    'updater.update_available': 'Version v{version} verfügbar!',
    'updater.install_btn': 'Jetzt aktualisieren & neu starten',
    'updater.installing': 'Lade Update...',
    'updater.error': 'Update-Server nicht erreichbar oder kein Update vorhanden ({error})',

    // Settings: Watched Games
    'settings.games_title': 'Überwachte Spiele ({count})',
    'settings.games_subtitle': 'Spiele, deren Spielzeit automatisch erfasst wird',
    'settings.scan_pc': 'PC nach Spielen durchsuchen',
    'settings.browse_file': 'Datei auswählen',
    'settings.browse_folder': 'Ordner durchsuchen',
    'settings.file_selected_success': 'Spiel "{game}" erfolgreich hinzugefügt!',
    'settings.add_single': 'Hinzufügen',
    'settings.add_placeholder': 'z.B. game.exe oder vollständiger Pfad',
    'settings.filter_placeholder': 'Spiele filtern...',
    'settings.select_all': 'Alle auswählen',
    'settings.delete_selected': 'Ausgewählte entfernen ({count})',
    'settings.no_games': 'Keine Spiele hinterlegt. Durchsuche deinen PC oder wähle Spieldateien aus.',
    'settings.active_badge': 'AKTIV',

    // Settings: Import
    'settings.import_title': 'Spielzeiten importieren',
    'settings.import_subtitle': 'Übernehme historische Spielzeiten von Steam oder AMD Adrenalin (inkl. Epic/Standalone)',
    'settings.import_btn': 'Historische Spielzeiten scannen',

    // Settings: Backup & Migration
    'settings.backup_title': 'Datensicherung & Migration',
    'settings.backup_subtitle': 'Exportiere oder importiere alle Sessions, Spielzeiten und Einstellungen zur Mitnahme auf einen neuen PC',
    'settings.export_btn': 'Daten exportieren',
    'settings.import_backup_btn': 'Daten wiederherstellen',
    'backup.exporting': 'Exportiere Daten...',
    'backup.export_success': 'Datensicherung erfolgreich gespeichert: {path}',
    'backup.importing': 'Stelle Daten wieder her...',
    'backup.import_success': 'Sicherung erfolgreich importiert! ({sessions} Sitzungen, {events} Events, {imported} externe Zeiten, {games} neue Spiele hinzugefügt)',
    'backup.import_empty': 'Keine neuen Datensätze in der Sicherungsdatei gefunden.',
    'backup.import_error': 'Fehler beim Import: {error}',
    'backup.export_error': 'Fehler beim Export: {error}',

    // Scanner Modal
    'scanner.title': 'Gefundene Spiele auf diesem PC',
    'scanner.subtitle': 'Wähle die Titel aus, die automatisch erfasst werden sollen',
    'scanner.scanning': 'Durchsuche Steam, Epic und Standalone-Installationen...',
    'scanner.found_count': '{count} Spiele auf deinem PC gefunden',
    'scanner.search': 'Gefundene Spiele durchsuchen...',
    'scanner.add_selected': '{count} Spiele übernehmen',
    'scanner.cancel': 'Abbrechen',
    'scanner.no_results': 'Keine neuen Spiele gefunden.',
    'scanner.already_added': 'Bereits überwacht',

    // Importer Modal
    'importer.title': 'Spielzeiten importieren',
    'importer.subtitle': 'Wähle aus, welche Spielzeiten in deine lokale Statistik übernommen werden sollen',
    'importer.scanning': 'Durchsuche Steam-Konfiguration und AMD Adrenalin...',
    'importer.found_count': '{count} Spielzeiten gefunden',
    'importer.search': 'Spiele filtern...',
    'importer.select_all': 'Alle auswählen',
    'importer.add_to_watched': 'Spiele auch zu überwachten Titeln hinzufügen',
    'importer.import_selected': '{count} Spielzeiten importieren',
    'importer.cancel': 'Abbrechen',
    'importer.no_results': 'Keine importierbaren Spielzeiten gefunden.',
    'importer.success': 'Erfolgreich {count} Spielzeiten importiert!',

    // Login Modal
    'login.title': 'Bei PlayCounter Cloud anmelden',
    'login.subtitle': 'Synchronisiere deine Spielzeiten geräteübergreifend mit deiner Datenbank',
    'login.token_label': 'API-Schlüssel / Persönliches Token',
    'login.token_hint': 'Trage hier dein PlayCounter-Token ein, um die Cloud-Synchronisation zu aktivieren.',
    'login.load_secret_conf': 'Aus bestehender secret.conf laden',
    'login.advanced_url': 'Eigener Server (optional)',
    'login.submit': 'Anmelden & Verbinden',
    'login.cancel': 'Abbrechen',
    'login.success': 'Erfolgreich angemeldet! {count} Spiele aus der Cloud geladen.',

    // Errors & Success
    'error.api_disabled': 'Die Cloud-Synchronisation ist nicht aktiviert oder es ist kein Token hinterlegt.',
    'error.sync_failed': 'Synchronisationsfehler: {error}',
    'error.empty_token': 'Bitte gib einen gültigen Schlüssel ein.',
    'error.empty_name': 'Der Dateiname des Spiels darf nicht leer sein.',
    'success.sync_done': 'Erfolgreich {count} Event(s) synchronisiert!',
  },
  en: {
    // Navigation
    'nav.stats': 'Statistics & Games',
    'nav.sessions': 'Sessions',
    'nav.settings': 'Settings',

    // Status & Header
    'status.tracking': 'Tracking: {game}',
    'status.idle': 'Idle',
    'status.sync': 'Sync',
    'status.syncing': 'Syncing...',

    // Active Game Banner
    'active.game_running': 'Game currently running',
    'active.current_playtime': 'Current Playtime',

    // Stats
    'stats.total_time': 'Total Playtime',
    'stats.hours': 'Hours',
    'stats.tracked_games': 'Tracked Games',
    'stats.pending_events': 'Pending Synchronization',
    'stats.waiting_sync': 'Waiting for sync',
    'stats.all_synced': 'Fully synchronized',
    'stats.top_games': 'Top Played Games',
    'stats.local_db': 'Tracked locally in SQLite',
    'stats.no_data': 'No playtime recorded yet. Launch a monitored game to start tracking.',

    // Sessions
    'sessions.title': 'Recent Gaming Sessions',
    'sessions.subtitle': 'Last 20 sessions',
    'sessions.game': 'Game',
    'sessions.start': 'Start Time',
    'sessions.duration': 'Duration',
    'sessions.status': 'Cloud Status',
    'sessions.synced': 'Synced',
    'sessions.local': 'Local only',
    'sessions.no_sessions': 'No completed sessions found.',

    // Settings: Account & Cloud
    'settings.cloud_title': 'Cloud Account & Synchronization',
    'settings.logged_in': 'Connected',
    'settings.not_logged_in': 'Not connected',
    'settings.login_btn': 'Connect',
    'settings.logout_btn': 'Disconnect',
    'settings.connected_to': 'Connected to: {url}',
    'settings.local_mode_hint': 'PlayCounter is currently running in local-only mode. All data remains stored safely on this machine.',
    'settings.advanced_config': 'Advanced Server Settings',
    'settings.api_url': 'Server Address / Endpoint',
    'settings.api_token': 'API Key / Token',
    'settings.api_token_placeholder': 'Enter token here...',
    'settings.test_connection': 'Test Connection',
    'settings.testing': 'Testing connection...',
    'settings.test_success': 'Connection successful! Found {count} games.',
    'settings.test_failed': 'Connection error: {error}',
    'settings.save_config': 'Save',
    'settings.saved': 'Saved!',

    // Settings: General
    'settings.general_title': 'General',
    'settings.language': 'Language',
    'settings.lang_system': 'System Default',
    'settings.lang_de': 'Deutsch',
    'settings.lang_en': 'English',

    // Updater
    'updater.current_version': 'Installed Version',
    'updater.check_btn': 'Check for Updates',
    'updater.checking': 'Checking...',
    'updater.up_to_date': 'PlayCounter is up to date.',
    'updater.update_available': 'Version v{version} available!',
    'updater.install_btn': 'Update & Restart now',
    'updater.installing': 'Downloading update...',
    'updater.error': 'Update server unavailable or no update found ({error})',

    // Settings: Watched Games
    'settings.games_title': 'Monitored Games ({count})',
    'settings.games_subtitle': 'Games monitored for automated playtime tracking',
    'settings.scan_pc': 'Scan PC for Games',
    'settings.browse_file': 'Select File',
    'settings.browse_folder': 'Browse Folder',
    'settings.file_selected_success': 'Added game "{game}"!',
    'settings.add_single': 'Add',
    'settings.add_placeholder': 'e.g. game.exe or full path',
    'settings.filter_placeholder': 'Filter games...',
    'settings.select_all': 'Select all',
    'settings.delete_selected': 'Remove selected ({count})',
    'settings.no_games': 'No games configured. Scan your computer or pick game files.',
    'settings.active_badge': 'ACTIVE',

    // Settings: Import
    'settings.import_title': 'Import Playtime',
    'settings.import_subtitle': 'Import historical playtime from Steam or AMD Adrenalin (incl. Epic/Standalone)',
    'settings.import_btn': 'Scan Historical Playtime',

    // Settings: Backup & Migration
    'settings.backup_title': 'Backup & Migration',
    'settings.backup_subtitle': 'Export or import all sessions, playtime history, and settings to transfer to a new PC',
    'settings.export_btn': 'Export Backup',
    'settings.import_backup_btn': 'Restore Backup',
    'backup.exporting': 'Exporting data...',
    'backup.export_success': 'Backup successfully saved to: {path}',
    'backup.importing': 'Restoring data...',
    'backup.import_success': 'Backup successfully restored! ({sessions} sessions, {events} events, {imported} external records, {games} new games added)',
    'backup.import_empty': 'No new records found in the backup file.',
    'backup.import_error': 'Import error: {error}',
    'backup.export_error': 'Export error: {error}',

    // Scanner Modal
    'scanner.title': 'Installed Games Found on PC',
    'scanner.subtitle': 'Choose the titles you would like to track automatically',
    'scanner.scanning': 'Scanning Steam, Epic, and standalone games...',
    'scanner.found_count': 'Found {count} games on your system',
    'scanner.search': 'Search discovered games...',
    'scanner.add_selected': 'Add {count} Games',
    'scanner.cancel': 'Cancel',
    'scanner.no_results': 'No new games discovered.',
    'scanner.already_added': 'Already tracked',

    // Importer Modal
    'importer.title': 'Import Playtime',
    'importer.subtitle': 'Choose which playtime records to import into your local statistics',
    'importer.scanning': 'Scanning Steam config and AMD Adrenalin...',
    'importer.found_count': 'Found {count} playtime records',
    'importer.search': 'Filter games...',
    'importer.select_all': 'Select all',
    'importer.add_to_watched': 'Also add games to monitored list',
    'importer.import_selected': 'Import {count} Records',
    'importer.cancel': 'Cancel',
    'importer.no_results': 'No importable playtime records found.',
    'importer.success': 'Successfully imported {count} playtime records!',

    // Login Modal
    'login.title': 'Connect to PlayCounter Cloud',
    'login.subtitle': 'Synchronize playtime across devices with your central database',
    'login.token_label': 'API Key / Personal Token',
    'login.token_hint': 'Enter your PlayCounter key to enable cloud synchronization.',
    'login.load_secret_conf': 'Load from local secret.conf',
    'login.advanced_url': 'Custom Server URL (optional)',
    'login.submit': 'Connect & Synchronize',
    'login.cancel': 'Cancel',
    'login.success': 'Successfully connected! Loaded {count} games from the cloud.',

    // Errors & Success
    'error.api_disabled': 'Cloud synchronization is disabled or no valid key is configured.',
    'error.sync_failed': 'Sync error: {error}',
    'error.empty_token': 'Please enter a valid key.',
    'error.empty_name': 'Game executable name cannot be empty.',
    'success.sync_done': 'Successfully synchronized {count} event(s)!',
  }
} as const

export type MessageKey = keyof typeof messages['de']

function getSystemLanguage(): 'de' | 'en' {
  if (typeof navigator !== 'undefined') {
    const lang = (navigator.language || '').toLowerCase()
    if (lang.startsWith('de')) return 'de'
  }
  return 'en'
}

const savedLang = (typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null) as LanguageChoice | null
export const currentLanguageChoice = ref<LanguageChoice>(savedLang || 'system')

export const activeLocale = computed<'de' | 'en'>(() => {
  if (currentLanguageChoice.value === 'system') {
    return getSystemLanguage()
  }
  return currentLanguageChoice.value
})

export function setLanguage(choice: LanguageChoice) {
  currentLanguageChoice.value = choice
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, choice)
  }
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const loc = activeLocale.value
  let text: string = (messages[loc] as Record<string, string>)[key] || (messages['en'] as Record<string, string>)[key] || key

  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
    }
  }

  return text
}
