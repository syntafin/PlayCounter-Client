import rawGamesMetadata from '../assets/games_metadata.json'

export interface GameGenre {
  id: number
  name: string
  slug: string
}

export interface GameScreenshot {
  id: string
  thumbnail: string
  full: string
}

export interface GameMetadata {
  id?: number
  name: string
  slug?: string
  process_name?: string | null
  short_description?: string | null
  description?: string | null
  image_path?: string | null
  banner_path?: string | null
  developer?: string | null
  publisher?: string | null
  release_date?: string | null
  steam_app_id?: string | null
  tags?: string[] | null
  genres?: GameGenre[]
  screenshots?: GameScreenshot[] | null
  game_cover_url?: string | null
  game_banner_url?: string | null
}

// Known process/alias to friendly title dictionary
const KNOWN_GAME_ALIASES: Record<string, { title: string; dev?: string; genres?: string[] }> = {
  'starcitizen': { title: 'Star Citizen', dev: 'Cloud Imperium Games', genres: ['Space Sim', 'MMO', 'Sci-Fi'] },
  'starcitizen.exe': { title: 'Star Citizen', dev: 'Cloud Imperium Games', genres: ['Space Sim', 'MMO', 'Sci-Fi'] },
  'starcitizen-win64-shipping': { title: 'Star Citizen', dev: 'Cloud Imperium Games', genres: ['Space Sim', 'MMO', 'Sci-Fi'] },
  'ffxiv_dx11': { title: 'FINAL FANTASY XIV Online', dev: 'Square Enix', genres: ['MMORPG', 'RPG', 'Fantasy'] },
  'ffxiv_dx11.exe': { title: 'FINAL FANTASY XIV Online', dev: 'Square Enix', genres: ['MMORPG', 'RPG', 'Fantasy'] },
  'ffxiv': { title: 'FINAL FANTASY XIV Online', dev: 'Square Enix', genres: ['MMORPG', 'RPG', 'Fantasy'] },
  'ffxiv.exe': { title: 'FINAL FANTASY XIV Online', dev: 'Square Enix', genres: ['MMORPG', 'RPG', 'Fantasy'] },
  'helldivers2': { title: 'HELLDIVERS II', dev: 'Arrowhead Game Studios', genres: ['Shooter', 'Action', 'Co-op'] },
  'helldivers2.exe': { title: 'HELLDIVERS II', dev: 'Arrowhead Game Studios', genres: ['Shooter', 'Action', 'Co-op'] },
  'endfield': { title: 'Arknights: Endfield', dev: 'Hypergryph / Gryphline', genres: ['3D RPG', 'Strategy', 'Sci-Fi'] },
  'endfield.exe': { title: 'Arknights: Endfield', dev: 'Hypergryph / Gryphline', genres: ['3D RPG', 'Strategy', 'Sci-Fi'] },
  'arknights': { title: 'Arknights', dev: 'Hypergryph / Yostar', genres: ['Tower Defense', 'RPG', 'Strategy'] },
  'arknights.exe': { title: 'Arknights', dev: 'Hypergryph / Yostar', genres: ['Tower Defense', 'RPG', 'Strategy'] },
  'factorygamesteam': { title: 'Satisfactory', dev: 'Coffee Stain Studios', genres: ['Building', 'Automation', 'Open World'] },
  'factorygamesteam.exe': { title: 'Satisfactory', dev: 'Coffee Stain Studios', genres: ['Building', 'Automation', 'Open World'] },
  'tetriseffect-win64-shipping': { title: 'Tetris Effect Connected', dev: 'Monstars Inc. / Resonair', genres: ['Puzzle', 'Arcade', 'Music'] },
  'tetriseffect-win64-shipping.exe': { title: 'Tetris Effect Connected', dev: 'Monstars Inc. / Resonair', genres: ['Puzzle', 'Arcade', 'Music'] },
  'riftbreaker_win_release': { title: 'The Riftbreaker', dev: 'EXOR Studios', genres: ['Base Building', 'Survival', 'Action-RPG'] },
  'riftbreaker_win_release.exe': { title: 'The Riftbreaker', dev: 'EXOR Studios', genres: ['Base Building', 'Survival', 'Action-RPG'] },
  'farthest frontier': { title: 'Farthest Frontier', dev: 'Crate Entertainment', genres: ['City Builder', 'Survival', 'Strategy'] },
  'farthest frontier.exe': { title: 'Farthest Frontier', dev: 'Crate Entertainment', genres: ['City Builder', 'Survival', 'Strategy'] },
  'maplestory': { title: 'MapleStory', dev: 'Nexon', genres: ['MMORPG', '2D', 'Adventure'] },
  'maplestory.exe': { title: 'MapleStory', dev: 'Nexon', genres: ['MMORPG', '2D', 'Adventure'] },
  'starsandisland': { title: 'Starsand Island', dev: 'Creatime', genres: ['Simulation', 'Adventure', 'Co-op'] },
  'starsandisland.exe': { title: 'Starsand Island', dev: 'Creatime', genres: ['Simulation', 'Adventure', 'Co-op'] },
  'voiceofcardstheisledragonroars': { title: 'Voice of Cards: The Isle Dragon Roars', dev: 'Square Enix', genres: ['Tabletop RPG', 'Card Game', 'Fantasy'] },
  'voiceofcardstheisledragonroars.exe': { title: 'Voice of Cards: The Isle Dragon Roars', dev: 'Square Enix', genres: ['Tabletop RPG', 'Card Game', 'Fantasy'] },
  'remake': { title: 'Final Fantasy VII Remake', dev: 'Square Enix', genres: ['Action-RPG', 'Sci-Fi', 'Fantasy'] },
  'rebirth': { title: 'Final Fantasy VII Rebirth', dev: 'Square Enix', genres: ['Action-RPG', 'Open World', 'Fantasy'] },
  'ffxvi': { title: 'Final Fantasy XVI', dev: 'Square Enix', genres: ['Action-RPG', 'Dark Fantasy'] },
  'replicant': { title: 'NieR Replicant ver.1.22474487139...', dev: 'Toylogic / Square Enix', genres: ['Action-RPG', 'Sci-Fi'] },
  'automatagame': { title: 'NieR:Automata', dev: 'PlatinumGames / Square Enix', genres: ['Action-RPG', 'Hack and Slash', 'Sci-Fi'] },
  'starrail': { title: 'Honkai: Star Rail', dev: 'HoYoverse', genres: ['Turn-based RPG', 'Space Fantasy'] },
  'starrail.exe': { title: 'Honkai: Star Rail', dev: 'HoYoverse', genres: ['Turn-based RPG', 'Space Fantasy'] },
  'genshinimpact': { title: 'Genshin Impact', dev: 'HoYoverse', genres: ['Action-RPG', 'Open World'] },
  'genshinimpact.exe': { title: 'Genshin Impact', dev: 'HoYoverse', genres: ['Action-RPG', 'Open World'] },
  'palworld-win64-shipping': { title: 'Palworld', dev: 'Pocketpair', genres: ['Survival', 'Crafting', 'Open World'] },
  'palworld-win64-shipping.exe': { title: 'Palworld', dev: 'Pocketpair', genres: ['Survival', 'Crafting', 'Open World'] },
  'forzahorizon6': { title: 'Forza Horizon 6', dev: 'Playground Games', genres: ['Racing', 'Open World'] },
  'squadron42': { title: 'Squadron 42', dev: 'Cloud Imperium Games', genres: ['Space Sim', 'Cinematic', 'Sci-Fi'] },
  'wwm': { title: 'Where Winds Meet', dev: 'Everstone Studio', genres: ['Wuxia', 'Action-RPG', 'Open World'] },
  'htgame': { title: 'Hytale', dev: 'Hypixel Studios', genres: ['Sandbox', 'Adventure', 'RPG'] },
  'sandfall-win64-shipping': { title: 'Clair Obscur: Expedition 33', dev: 'Sandfall Interactive', genres: ['Turn-based RPG', 'Fantasy'] },
  'aniimo': { title: 'Aniimo', dev: 'Aniimo', genres: ['Indie', 'Adventure'] },
  'cs2': { title: 'Counter-Strike 2', dev: 'Valve', genres: ['FPS', 'Competitive', 'Tactical'] },
  'cs2.exe': { title: 'Counter-Strike 2', dev: 'Valve', genres: ['FPS', 'Competitive', 'Tactical'] },
  'cyberpunk2077': { title: 'Cyberpunk 2077', dev: 'CD PROJEKT RED', genres: ['Open World RPG', 'Cyberpunk', 'Action'] },
  'cyberpunk2077.exe': { title: 'Cyberpunk 2077', dev: 'CD PROJEKT RED', genres: ['Open World RPG', 'Cyberpunk', 'Action'] },
  'witcher3': { title: 'The Witcher 3: Wild Hunt', dev: 'CD PROJEKT RED', genres: ['Open World RPG', 'Fantasy'] },
  'witcher3.exe': { title: 'The Witcher 3: Wild Hunt', dev: 'CD PROJEKT RED', genres: ['Open World RPG', 'Fantasy'] },
  'factorio': { title: 'Factorio', dev: 'Wube Software', genres: ['Automation', 'Building', 'Strategy'] },
  'factorio.exe': { title: 'Factorio', dev: 'Wube Software', genres: ['Automation', 'Building', 'Strategy'] },
  'hl2': { title: 'Half-Life 2', dev: 'Valve', genres: ['FPS', 'Sci-Fi', 'Classic'] },
  'hl2.exe': { title: 'Half-Life 2', dev: 'Valve', genres: ['FPS', 'Sci-Fi', 'Classic'] },
  'steam_40100': { title: 'Supreme Commander', dev: 'Gas Powered Games', genres: ['RTS', 'Sci-Fi', 'Strategy'] },
  'steam_40100.exe': { title: 'Supreme Commander', dev: 'Gas Powered Games', genres: ['RTS', 'Sci-Fi', 'Strategy'] },
  'banished': { title: 'Banished', dev: 'Shining Rock Software', genres: ['City Builder', 'Survival', 'Simulation'] },
  'banished.exe': { title: 'Banished', dev: 'Shining Rock Software', genres: ['City Builder', 'Survival', 'Simulation'] },
}

// Clean and normalize strings for matching
function normalizeKey(str: string): string {
  return str.toLowerCase().replace(/\.exe$/i, '').replace(/[-_\s]/g, '')
}

// Format raw executable into a pleasant human-readable name when no metadata entry exists
export function cleanExecutableName(raw: string): string {
  const withoutExt = raw.replace(/\.exe$/i, '').trim()
  if (withoutExt.toLowerCase().startsWith('steam_')) {
    const appId = withoutExt.replace(/^steam_/i, '')
    return `Steam App (${appId})`
  }
  // Strip common shipping suffixes
  const stripped = withoutExt
    .replace(/-win64-shipping/i, '')
    .replace(/_win64_shipping/i, '')
    .replace(/_win_release/i, '')
    .replace(/_dx11/i, '')
    .replace(/_dx12/i, '')
  
  // Insert space before uppercase letters in CamelCase/PascalCase
  const spaced = stripped.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/[-_]/g, ' ')
  
  // Title-case words
  return spaced
    .split(' ')
    .filter(Boolean)
    .map(w => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ')
}

class GameMetadataService {
  private games: GameMetadata[] = []
  private lookupMap = new Map<string, GameMetadata>()

  constructor() {
    this.init(rawGamesMetadata as unknown as GameMetadata[])
  }

  public init(dataset: GameMetadata[]) {
    this.games = dataset
    this.lookupMap.clear()

    for (const g of dataset) {
      if (g.name) {
        this.lookupMap.set(normalizeKey(g.name), g)
        this.lookupMap.set(g.name.toLowerCase(), g)
      }
      if (g.process_name) {
        this.lookupMap.set(normalizeKey(g.process_name), g)
        this.lookupMap.set(g.process_name.toLowerCase(), g)
      }
      if (g.slug) {
        this.lookupMap.set(normalizeKey(g.slug), g)
        this.lookupMap.set(g.slug.toLowerCase(), g)
      }
    }
  }

  public getAllMetadata(): GameMetadata[] {
    return this.games
  }

  public findGame(key: string): GameMetadata | undefined {
    if (!key) return undefined
    const clean = key.trim()
    const norm = normalizeKey(clean)

    // 1. Direct lookup in map
    if (this.lookupMap.has(norm)) {
      return this.lookupMap.get(norm)
    }
    if (this.lookupMap.has(clean.toLowerCase())) {
      return this.lookupMap.get(clean.toLowerCase())
    }

    // 2. Known alias check
    const alias = KNOWN_GAME_ALIASES[clean.toLowerCase()] || KNOWN_GAME_ALIASES[clean.toLowerCase().replace(/\.exe$/, '')]
    if (alias) {
      const matched = this.lookupMap.get(normalizeKey(alias.title))
      if (matched) return matched
      return {
        name: alias.title,
        developer: alias.dev,
        genres: alias.genres?.map((name, id) => ({ id, name, slug: name.toLowerCase().replace(/\s+/g, '-') })),
        process_name: clean,
      }
    }

    // 3. Substring matching in library
    for (const g of this.games) {
      const gNorm = normalizeKey(g.name)
      if (gNorm.length > 3 && (norm.includes(gNorm) || gNorm.includes(norm))) {
        return g
      }
    }

    return undefined
  }

  public getDisplayName(key: string): string {
    const meta = this.findGame(key)
    if (meta?.name) return meta.name

    const alias = KNOWN_GAME_ALIASES[key.toLowerCase()] || KNOWN_GAME_ALIASES[key.toLowerCase().replace(/\.exe$/, '')]
    if (alias?.title) return alias.title

    return cleanExecutableName(key)
  }

  public getCoverUrl(game: GameMetadata | string, apiBaseUrl?: string): string | null {
    let meta: GameMetadata | undefined
    if (typeof game === 'string') {
      meta = this.findGame(game)
    } else {
      meta = game
    }

    if (!meta) return null

    if (meta.game_cover_url) {
      return meta.game_cover_url
    }

    if (meta.image_path) {
      if (meta.image_path.startsWith('http://') || meta.image_path.startsWith('https://')) {
        return meta.image_path
      }
      // Point to syntafin backend storage
      const base = (apiBaseUrl || 'https://api.syntafin.de').replace(/\/v1\/game\/?$/, '').replace(/\/+$/, '')
      return `${base}/storage/${meta.image_path.replace(/^\/+/, '')}`
    }

    return null
  }

  public getBannerUrl(game: GameMetadata | string, apiBaseUrl?: string): string | null {
    let meta: GameMetadata | undefined
    if (typeof game === 'string') {
      meta = this.findGame(game)
    } else {
      meta = game
    }

    if (!meta) return null

    if (meta.game_banner_url) {
      return meta.game_banner_url
    }

    if (meta.banner_path) {
      if (meta.banner_path.startsWith('http://') || meta.banner_path.startsWith('https://')) {
        return meta.banner_path
      }
      const base = (apiBaseUrl || 'https://api.syntafin.de').replace(/\/v1\/game\/?$/, '').replace(/\/+$/, '')
      return `${base}/storage/${meta.banner_path.replace(/^\/+/, '')}`
    }

    return null
  }
}

export const gameMetadata = new GameMetadataService()
